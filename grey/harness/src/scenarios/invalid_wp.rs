//! Scenario: submit various invalid work packages and verify error responses.
//!
//! Tests that the node correctly rejects malformed inputs without crashing.

use std::time::Instant;

use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

/// Submit a work package and assert it is rejected.
async fn assert_rejected(
    client: &RpcClient,
    data: &str,
    description: &str,
    start: &Instant,
) -> Option<ScenarioResult> {
    if client.submit_work_package(data).await.is_ok() {
        return Some(ScenarioResult {
            name: "invalid_wp",
            pass: false,
            duration: start.elapsed(),
            error: Some(format!("{} should have been rejected", description)),
            latencies: vec![],
        });
    }
    None
}

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();

    // Test 1: Random bytes (invalid JAM codec)
    let random_hex = hex::encode([0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
    if let Some(r) = assert_rejected(client, &random_hex, "random bytes", &start).await {
        return r;
    }

    // Test 2: Empty work package
    if let Some(r) = assert_rejected(client, "", "empty work package", &start).await {
        return r;
    }

    // Test 3: Invalid hex
    if let Some(r) = assert_rejected(client, "not-hex-data", "invalid hex", &start).await {
        return r;
    }

    // Test 4: Oversized payload (15MB > MAX_WORK_PACKAGE_BLOB_SIZE)
    let oversized = hex::encode(vec![0u8; 15_000_000]);
    if let Some(r) = assert_rejected(client, &oversized, "oversized work package", &start).await {
        return r;
    }

    // Test 5: Verify node is still healthy after all invalid submissions
    if let Err(e) = client.get_status().await {
        return ScenarioResult {
            name: "invalid_wp",
            pass: false,
            duration: start.elapsed(),
            error: Some(format!("node unhealthy after invalid submissions: {}", e)),
            latencies: vec![],
        };
    }

    ScenarioResult {
        name: "invalid_wp",
        pass: true,
        duration: start.elapsed(),
        error: None,
        latencies: vec![],
    }
}
