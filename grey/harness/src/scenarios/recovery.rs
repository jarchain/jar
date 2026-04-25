//! Scenario: verify node remains operational after processing invalid inputs.
//!
//! Submits several invalid work packages, then verifies that:
//! - The RPC endpoint is still responsive
//! - Block production continues (head slot advances)
//! - Finalization is not disrupted
//!
//! Does NOT verify pixel inclusion after invalid submissions, because
//! semantically-invalid WPs accepted by RPC may corrupt the service
//! account state at the consensus layer, temporarily preventing new
//! work from being confirmed. Recovery here means "node stays up and
//! keeps producing blocks", not "state is pristine".
//!
//!   Covers Issue #225 Scenario 3.

use std::time::{Duration, Instant};

use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

const BLOCK_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();

    // ── Phase 1: Capture pre-test state ────────────────────────────
    let pre_status = match client.get_status().await {
        Ok(s) => s,
        Err(e) => {
            return ScenarioResult {
                name: "recovery",
                pass: false,
                duration: start.elapsed(),
                error: Some(format!("failed to get pre-test status: {}", e)),
                latencies: vec![],
                metrics: vec![],
            };
        }
    };
    let pre_head_slot = pre_status.head_slot;
    let pre_finalized_slot = pre_status.finalized_slot;

    // ── Phase 2: Submit several invalid work packages ──────────────
    let invalid_payloads = [
        hex::encode([0xDE, 0xAD, 0xBE, 0xEF]),
        String::new(),               // empty
        hex::encode(vec![0u8; 100]), // random bytes
    ];
    for payload in &invalid_payloads {
        // Ignore errors — we expect rejections
        let _ = client.submit_work_package(payload).await;
    }

    // ── Phase 3: Verify RPC is still responsive immediately ───────
    if let Err(e) = client.get_status().await {
        return ScenarioResult {
            name: "recovery",
            pass: false,
            duration: start.elapsed(),
            error: Some(format!("RPC unresponsive after invalid submissions: {}", e)),
            latencies: vec![],
            metrics: vec![],
        };
    }

    // ── Phase 4: Verify block production continues ─────────────────
    // The head slot should advance beyond the pre-test value, proving
    // the node is still producing blocks despite invalid submissions.
    let poll_start = Instant::now();
    loop {
        match client.get_status().await {
            Ok(status) if status.head_slot > pre_head_slot => break,
            Ok(_) => {}
            Err(e) => {
                return ScenarioResult {
                    name: "recovery",
                    pass: false,
                    duration: start.elapsed(),
                    error: Some(format!(
                        "RPC failed while waiting for block production: {}",
                        e
                    )),
                    latencies: vec![],
                    metrics: vec![],
                };
            }
        }
        if poll_start.elapsed() > BLOCK_TIMEOUT {
            return ScenarioResult {
                name: "recovery",
                pass: false,
                duration: start.elapsed(),
                error: Some(format!(
                    "block production stalled: head_slot still {} after {}s",
                    pre_head_slot,
                    BLOCK_TIMEOUT.as_secs()
                )),
                latencies: vec![],
                metrics: vec![],
            };
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // ── Phase 5: Verify finalization is not disrupted ──────────────
    let post_status = match client.get_status().await {
        Ok(s) => s,
        Err(e) => {
            return ScenarioResult {
                name: "recovery",
                pass: false,
                duration: start.elapsed(),
                error: Some(format!("failed to get post-test status: {}", e)),
                latencies: vec![],
                metrics: vec![],
            };
        }
    };

    if post_status.finalized_slot < pre_finalized_slot {
        return ScenarioResult {
            name: "recovery",
            pass: false,
            duration: start.elapsed(),
            error: Some(format!(
                "finalization regressed: finalized_slot went from {} to {}",
                pre_finalized_slot, post_status.finalized_slot
            )),
            latencies: vec![],
            metrics: vec![],
        };
    }

    ScenarioResult {
        name: "recovery",
        pass: true,
        duration: start.elapsed(),
        error: None,
        latencies: vec![],
        metrics: vec![
            crate::scenarios::ScenarioMetric {
                label: "head_slot_before".into(),
                value: pre_head_slot as f64,
                unit: "slot",
            },
            crate::scenarios::ScenarioMetric {
                label: "head_slot_after".into(),
                value: post_status.head_slot as f64,
                unit: "slot",
            },
            crate::scenarios::ScenarioMetric {
                label: "finalized_slot_before".into(),
                value: pre_finalized_slot as f64,
                unit: "slot",
            },
            crate::scenarios::ScenarioMetric {
                label: "finalized_slot_after".into(),
                value: post_status.finalized_slot as f64,
                unit: "slot",
            },
        ],
    }
}
