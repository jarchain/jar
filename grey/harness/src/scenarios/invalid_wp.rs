//! Scenario: submit various invalid work packages and verify error responses.
//!
//! Tests that the node correctly rejects malformed inputs without crashing.
//! Covers both structurally invalid (bad encoding) and semantically invalid
//! (valid encoding but wrong field values) work packages.

use std::time::Instant;

use grey_codec::Encode;
use grey_types::Hash;
use grey_types::work::{RefinementContext, WorkPackage};

use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

const NAME: &str = "invalid_wp";

/// Submit a work package and assert it is rejected.
async fn assert_rejected(
    client: &RpcClient,
    data: &str,
    description: &str,
    start: &Instant,
) -> Option<ScenarioResult> {
    if client.submit_work_package(data).await.is_ok() {
        return Some(ScenarioResult {
            name: NAME,
            pass: false,
            duration: start.elapsed(),
            error: Some(format!("{} should have been rejected", description)),
            latencies: vec![],
        });
    }
    None
}

/// Submit an encoded work package and verify the node doesn't crash.
///
/// Semantically invalid but structurally valid work packages are accepted
/// at the RPC layer (validation happens later in the guarantor pipeline).
/// We verify the submission doesn't cause a panic or hang, and the node
/// stays healthy afterward.
async fn assert_wp_accepted_gracefully(
    client: &RpcClient,
    wp: &WorkPackage,
    description: &str,
    start: &Instant,
) -> Option<ScenarioResult> {
    let encoded = hex::encode(wp.encode());
    match client.submit_work_package(&encoded).await {
        Ok(_) => {
            // Accepted — expected for structurally valid WPs.
            // Verify node is still responsive.
            if let Err(e) = client.get_status().await {
                return Some(ScenarioResult {
                    name: NAME,
                    pass: false,
                    duration: start.elapsed(),
                    error: Some(format!(
                        "node unhealthy after accepting {}: {}",
                        description, e
                    )),
                    latencies: vec![],
                });
            }
            None
        }
        Err(_) => {
            // Also fine — the node rejected it at the RPC layer.
            None
        }
    }
}

/// Build a dummy refinement context with zero hashes.
fn dummy_context() -> RefinementContext {
    RefinementContext {
        anchor: Hash::ZERO,
        state_root: Hash::ZERO,
        beefy_root: Hash::ZERO,
        lookup_anchor: Hash::ZERO,
        lookup_anchor_timeslot: 0,
        prerequisites: vec![],
    }
}

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();

    // === Structurally invalid (bad encoding) ===

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

    // === Semantically invalid (valid encoding, wrong fields) ===
    //
    // Structurally valid but semantically invalid WPs pass RPC validation
    // and enter the guarantor pipeline. Submitting many of them can clog
    // the pipeline and interfere with subsequent scenarios (e.g., recovery).
    //
    // We only test empty-items WPs here — they are lightweight and unlikely
    // to cause pipeline congestion. Tests for invalid service ID, wrong code
    // hash, and wrong context should be unit tests against the RPC handler,
    // not integration tests against a live node.

    // Test 5: Empty work items (valid structure, zero items)
    let empty_items = WorkPackage {
        auth_code_host: 0,
        auth_code_hash: Hash::ZERO,
        context: dummy_context(),
        authorization: vec![],
        authorizer_config: vec![],
        items: vec![],
    };
    if let Some(r) =
        assert_wp_accepted_gracefully(client, &empty_items, "empty work items", &start).await
    {
        return r;
    }

    // === Health check after all negative tests ===

    if let Err(e) = client.get_status().await {
        return ScenarioResult {
            name: NAME,
            pass: false,
            duration: start.elapsed(),
            error: Some(format!("node unhealthy after invalid submissions: {}", e)),
            latencies: vec![],
        };
    }

    ScenarioResult {
        name: NAME,
        pass: true,
        duration: start.elapsed(),
        error: None,
        latencies: vec![],
    }
}
