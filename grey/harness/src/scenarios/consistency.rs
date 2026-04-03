//! Scenario: verify all validators converge on the same finalized state.
//!
//! Starts from validator 0's RPC endpoint, then checks all testnet validators
//! for matching finalized hashes, matching state roots at that finalized hash,
//! identical pixels-service storage, and bounded head-slot skew.

use std::time::{Duration, Instant};

use tracing::info;

use crate::poll::submit_and_verify_pixel;
use crate::rpc::{MultiRpcClient, RpcClient};
use crate::scenarios::{LatencySample, ScenarioResult};

const VALIDATOR_COUNT: u16 = 6;
const RPC_HOST: &str = "127.0.0.1";
const BASE_RPC_PORT: u16 = 9933;
const SERVICE_ID: u32 = 2000;
const STORAGE_KEY: &str = "00";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const SETTLE_TIMEOUT: Duration = Duration::from_secs(180);
const PIXEL_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_HEAD_SPREAD: u32 = 2;
const MIN_SETTLED_HEAD: u32 = 20;

const PIXELS: [(u8, u8, u8, u8, u8); 5] = [
    (60, 10, 255, 128, 0),
    (61, 11, 0, 255, 255),
    (62, 12, 200, 30, 180),
    (63, 13, 80, 120, 255),
    (64, 14, 40, 220, 90),
];

#[derive(Debug, Clone)]
struct ValidatorSnapshot {
    index: usize,
    head_slot: u32,
    finalized_slot: u32,
    finalized_hash: String,
    finalized_state_root: String,
}

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();
    let mut latencies = Vec::new();

    match run_inner(client, &mut latencies).await {
        Ok(()) => ScenarioResult {
            name: "consistency",
            pass: true,
            duration: start.elapsed(),
            error: None,
            latencies,
        },
        Err(e) => ScenarioResult {
            name: "consistency",
            pass: false,
            duration: start.elapsed(),
            error: Some(e),
            latencies,
        },
    }
}

async fn run_inner(client: &RpcClient, latencies: &mut Vec<LatencySample>) -> Result<(), String> {
    let multi = MultiRpcClient::for_testnet(RPC_HOST, BASE_RPC_PORT, VALIDATOR_COUNT);

    let baseline = wait_for_settled_network(&multi).await?;
    log_snapshots("baseline", &baseline);

    for (x, y, r, g, b) in PIXELS {
        let op_start = Instant::now();
        submit_and_verify_pixel(client, SERVICE_ID, x, y, r, g, b, PIXEL_TIMEOUT)
            .await
            .map_err(|e| format!("submit_and_verify_pixel({x},{y}) failed: {e}"))?;

        let snapshots = wait_for_pixel_consensus(&multi, x, y, r, g, b).await?;
        log_snapshots(&format!("after pixel({x},{y})"), &snapshots);
        latencies.push(LatencySample {
            label: format!("consistency pixel({x},{y})"),
            duration: op_start.elapsed(),
        });
    }

    Ok(())
}

async fn wait_for_settled_network(
    multi: &MultiRpcClient,
) -> Result<Vec<ValidatorSnapshot>, String> {
    let deadline = Instant::now() + SETTLE_TIMEOUT;
    let last_err = loop {
        let current_err = match collect_consensus_snapshots(multi).await {
            Ok(snapshots) => {
                let min_head = snapshots.iter().map(|s| s.head_slot).min().unwrap_or(0);
                if min_head < MIN_SETTLED_HEAD {
                    format!("minimum head slot {min_head} is below required {MIN_SETTLED_HEAD}")
                } else if let Err(e) = multi.check_head_proximity(MAX_HEAD_SPREAD).await {
                    e
                } else {
                    return Ok(snapshots);
                }
            }
            Err(e) => e,
        };

        if Instant::now() >= deadline {
            break current_err;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    };

    Err(format!(
        "network consistency did not settle within {:?}: {}",
        SETTLE_TIMEOUT, last_err
    ))
}

async fn wait_for_pixel_consensus(
    multi: &MultiRpcClient,
    x: u8,
    y: u8,
    r: u8,
    g: u8,
    b: u8,
) -> Result<Vec<ValidatorSnapshot>, String> {
    let deadline = Instant::now() + PIXEL_TIMEOUT;
    let last_err = loop {
        let current_err = match collect_consensus_snapshots(multi).await {
            Ok(snapshots) => {
                if let Err(e) = multi.check_head_proximity(MAX_HEAD_SPREAD).await {
                    e
                } else if let Err(e) = check_storage_consensus(multi, x, y, r, g, b).await {
                    e
                } else {
                    return Ok(snapshots);
                }
            }
            Err(e) => e,
        };

        if Instant::now() >= deadline {
            break current_err;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    };

    Err(format!(
        "cross-validator pixel consistency timed out for ({x},{y}) within {:?}: {}",
        PIXEL_TIMEOUT, last_err
    ))
}

async fn collect_consensus_snapshots(
    multi: &MultiRpcClient,
) -> Result<Vec<ValidatorSnapshot>, String> {
    let statuses = multi.get_all_status().await;
    let mut snapshots = Vec::with_capacity(statuses.len());

    for (index, result) in statuses {
        let status = result.map_err(|e| format!("validator {index} unreachable: {e}"))?;
        if status.finalized_hash.is_empty() {
            return Err(format!("validator {index} has no finalized hash yet"));
        }

        let state = multi
            .client(index)
            .get_state_summary(Some(&status.finalized_hash))
            .await
            .map_err(|e| {
                format!(
                    "validator {index} could not fetch finalized state {}: {e}",
                    status.finalized_hash
                )
            })?;

        snapshots.push(ValidatorSnapshot {
            index,
            head_slot: status.head_slot,
            finalized_slot: status.finalized_slot,
            finalized_hash: status.finalized_hash,
            finalized_state_root: state.state_root,
        });
    }

    let reference = snapshots
        .first()
        .ok_or_else(|| "no validator snapshots collected".to_string())?;

    let hash_mismatches: Vec<String> = snapshots
        .iter()
        .filter(|snapshot| snapshot.finalized_hash != reference.finalized_hash)
        .map(|snapshot| {
            format!(
                "v{}={} (slot {})",
                snapshot.index, snapshot.finalized_hash, snapshot.finalized_slot
            )
        })
        .collect();
    if !hash_mismatches.is_empty() {
        return Err(format!(
            "finalized hash divergence: v{}={} (slot {}); {}",
            reference.index,
            reference.finalized_hash,
            reference.finalized_slot,
            hash_mismatches.join(", ")
        ));
    }

    let root_mismatches: Vec<String> = snapshots
        .iter()
        .filter(|snapshot| snapshot.finalized_state_root != reference.finalized_state_root)
        .map(|snapshot| format!("v{}={}", snapshot.index, snapshot.finalized_state_root))
        .collect();
    if !root_mismatches.is_empty() {
        return Err(format!(
            "finalized state root divergence at {}: v{}={}; {}",
            reference.finalized_hash,
            reference.index,
            reference.finalized_state_root,
            root_mismatches.join(", ")
        ));
    }

    Ok(snapshots)
}

async fn check_storage_consensus(
    multi: &MultiRpcClient,
    x: u8,
    y: u8,
    r: u8,
    g: u8,
    b: u8,
) -> Result<(), String> {
    let mut reference_value: Option<String> = None;
    let mut mismatches = Vec::new();

    for index in 0..multi.count() {
        let storage = multi
            .client(index)
            .read_storage(SERVICE_ID, STORAGE_KEY)
            .await
            .map_err(|e| format!("validator {index} read_storage failed: {e}"))?;
        let value = storage
            .value
            .ok_or_else(|| format!("validator {index} returned empty pixels storage"))?;

        if !pixel_matches(&value, x, y, r, g, b) {
            mismatches.push(format!(
                "v{index} missing pixel ({x},{y}) #{r:02x}{g:02x}{b:02x} at slot {}",
                storage.slot
            ));
        }

        if let Some(reference) = &reference_value {
            if &value != reference {
                mismatches.push(format!(
                    "v{index} pixels storage diverged at slot {}",
                    storage.slot
                ));
            }
        } else {
            reference_value = Some(value);
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches.join("; "))
    }
}

fn pixel_matches(value: &str, x: u8, y: u8, r: u8, g: u8, b: u8) -> bool {
    let offset = (y as usize * 100 + x as usize) * 3 * 2;
    if offset + 6 > value.len() {
        return false;
    }
    value[offset..offset + 6] == format!("{r:02x}{g:02x}{b:02x}")
}

fn log_snapshots(label: &str, snapshots: &[ValidatorSnapshot]) {
    info!("validator snapshots ({label})");
    for snapshot in snapshots {
        info!(
            "v{} head={} finalized={} hash={} state_root={}",
            snapshot.index,
            snapshot.head_slot,
            snapshot.finalized_slot,
            short_hex(&snapshot.finalized_hash),
            short_hex(&snapshot.finalized_state_root)
        );
    }
}

fn short_hex(value: &str) -> &str {
    let prefix_len = 16.min(value.len());
    &value[..prefix_len]
}

#[cfg(test)]
mod tests {
    use super::pixel_matches;

    #[test]
    fn pixel_matches_checks_expected_rgb_offset() {
        let mut bytes = vec![0u8; 100 * 100 * 3];
        let offset = (7 * 100 + 5) * 3;
        bytes[offset] = 0x12;
        bytes[offset + 1] = 0x34;
        bytes[offset + 2] = 0x56;
        let storage = hex::encode(bytes);

        assert!(pixel_matches(&storage, 5, 7, 0x12, 0x34, 0x56));
        assert!(!pixel_matches(&storage, 5, 7, 0x12, 0x34, 0x57));
    }
}
