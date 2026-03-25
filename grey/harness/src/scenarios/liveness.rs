//! Scenario: monitor chain liveness for 5 minutes.

use std::time::{Duration, Instant};

use tracing::info;

use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

const DURATION: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_secs(6);
const MAX_CONSECUTIVE_STALLS: u32 = 10;
const MAX_FINALITY_LAG: u32 = 3;
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
        },
        Err(e) => ScenarioResult {
            name: "liveness",
            pass: false,
            duration: start.elapsed(),
            error: Some(e),
        },
    }
}

async fn run_inner(client: &RpcClient) -> Result<(), String> {
    // Wait for finality to settle after prior scenarios (pixel submissions
    // can leave a large finality backlog).
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

    // Reset timer — monitoring starts after settling.
    let start = Instant::now();
    let mut last_head: i64 = -1;
    let mut consecutive_stalls: u32 = 0;
    let mut max_lag: u32 = 0;
    let mut polls: u32 = 0;

    while start.elapsed() < DURATION {
        let status = client
            .get_status()
            .await
            .map_err(|e| format!("RPC error: {e}"))?;
        polls += 1;

        let lag = status.head_slot.saturating_sub(status.finalized_slot);
        if lag > max_lag {
            max_lag = lag;
        }

        if (status.head_slot as i64) > last_head {
            consecutive_stalls = 0;
            last_head = status.head_slot as i64;
        } else {
            consecutive_stalls += 1;
        }

        if consecutive_stalls > MAX_CONSECUTIVE_STALLS {
            return Err(format!(
                "head stalled for {consecutive_stalls} consecutive polls at slot {last_head}"
            ));
        }

        if lag > MAX_FINALITY_LAG {
            return Err(format!(
                "finality lag {lag} exceeds max {MAX_FINALITY_LAG} (head={}, finalized={})",
                status.head_slot, status.finalized_slot
            ));
        }

        // Log progress every ~30s (5 polls).
        if polls % 5 == 0 {
            info!(
                "{}s: head={} finalized={} lag={lag}",
                start.elapsed().as_secs(),
                status.head_slot,
                status.finalized_slot
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
        "head={} finalized={} lag={final_lag} maxLag={max_lag}",
        final_status.head_slot, final_status.finalized_slot
    );

    Ok(())
}
