//! Scenario: monitor chain liveness for 50 blocks.
//!
//! Instead of wall-clock duration (which is slow with real testnet and
//! trivially fast with seq-testnet), we monitor block-count progress.
//! The test passes if 50 new blocks are produced with acceptable finality lag.

use std::time::{Duration, Instant};

use tracing::info;

use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

/// Number of new blocks to observe.
const TARGET_BLOCKS: u32 = 50;
/// Poll interval.
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// Give up if no progress after this long.
const TIMEOUT: Duration = Duration::from_secs(300);
/// Max allowed finality lag (head - finalized).
const MAX_FINALITY_LAG: u32 = 5;
/// Max consecutive polls where lag exceeds threshold before failing.
const MAX_CONSECUTIVE_LAG_SPIKES: u32 = 5;
/// Max consecutive polls with no head progress before failing.
const MAX_CONSECUTIVE_STALLS: u32 = 20;
/// Wait for finality to settle before starting.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();

    let result = run_inner(client).await;

    match result {
        Ok(()) => ScenarioResult {
            name: "liveness",
            pass: true,
            duration: start.elapsed(),
            error: None,
            latencies: vec![],
        },
        Err(e) => ScenarioResult {
            name: "liveness",
            pass: false,
            duration: start.elapsed(),
            error: Some(e),
            latencies: vec![],
        },
    }
}

async fn run_inner(client: &RpcClient) -> Result<(), String> {
    // Wait for finality to settle after prior scenarios.
    let settle_deadline = Instant::now() + SETTLE_TIMEOUT;
    loop {
        let status = client
            .get_status()
            .await
            .map_err(|e| format!("RPC error: {e}"))?;
        let lag = status.head_slot.saturating_sub(status.finalized_slot);
        if lag <= MAX_FINALITY_LAG {
            info!(
                "finality settled (head={}, finalized={}, lag={lag})",
                status.head_slot, status.finalized_slot
            );
            break;
        }
        if Instant::now() > settle_deadline {
            return Err(format!(
                "finality did not settle within {:?} (head={}, finalized={}, lag={lag})",
                SETTLE_TIMEOUT, status.head_slot, status.finalized_slot
            ));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    // Record starting head slot.
    let initial_status = client
        .get_status()
        .await
        .map_err(|e| format!("RPC error: {e}"))?;
    let start_head = initial_status.head_slot;

    let start = Instant::now();
    let mut last_head = start_head;
    let mut consecutive_stalls: u32 = 0;
    let mut consecutive_lag_spikes: u32 = 0;
    let mut max_lag: u32 = 0;
    let mut blocks_observed: u32 = 0;

    while blocks_observed < TARGET_BLOCKS {
        if start.elapsed() > TIMEOUT {
            return Err(format!(
                "timeout after {:?}: only observed {blocks_observed}/{TARGET_BLOCKS} blocks",
                TIMEOUT
            ));
        }

        let status = client
            .get_status()
            .await
            .map_err(|e| format!("RPC error: {e}"))?;

        let lag = status.head_slot.saturating_sub(status.finalized_slot);
        if lag > max_lag {
            max_lag = lag;
        }

        // Check progress
        if status.head_slot > last_head {
            let new_blocks = status.head_slot - last_head;
            blocks_observed += new_blocks;
            consecutive_stalls = 0;
            last_head = status.head_slot;
        } else {
            consecutive_stalls += 1;
        }

        if consecutive_stalls > MAX_CONSECUTIVE_STALLS {
            return Err(format!(
                "head stalled for {consecutive_stalls} consecutive polls at slot {last_head}"
            ));
        }

        // Check finality lag
        if lag > MAX_FINALITY_LAG {
            consecutive_lag_spikes += 1;
            if consecutive_lag_spikes > MAX_CONSECUTIVE_LAG_SPIKES {
                return Err(format!(
                    "finality lag {lag} exceeded max {MAX_FINALITY_LAG} for {consecutive_lag_spikes} consecutive polls (head={}, finalized={})",
                    status.head_slot, status.finalized_slot
                ));
            }
        } else {
            consecutive_lag_spikes = 0;
        }

        // Log progress every 10 blocks
        if blocks_observed > 0 && blocks_observed.is_multiple_of(10) {
            info!(
                "{blocks_observed}/{TARGET_BLOCKS} blocks: head={} finalized={} lag={lag}",
                status.head_slot, status.finalized_slot
            );
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }

    let final_status = client
        .get_status()
        .await
        .map_err(|e| format!("RPC error: {e}"))?;
    let final_lag = final_status
        .head_slot
        .saturating_sub(final_status.finalized_slot);
    info!(
        "done: {blocks_observed} blocks, head={} finalized={} lag={final_lag} maxLag={max_lag}",
        final_status.head_slot, final_status.finalized_slot
    );

    Ok(())
}
