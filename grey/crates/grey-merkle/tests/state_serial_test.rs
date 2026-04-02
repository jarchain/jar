//! Tests for state serialization T(σ) roundtrip.
//!
//! Creates a genesis state with Config::full(), serializes to KVs,
//! deserializes back, and verifies the roundtrip.

use grey_merkle::compute_state_root_from_kvs;
use grey_merkle::state_serial::{deserialize_state, serialize_state_with_opaque};
use grey_types::config::Config;

/// Create a minimal genesis state for testing.
fn make_test_genesis() -> grey_types::state::State {
    let config = Config::full();
    let (state, _secrets) = grey_consensus::genesis::create_genesis(&config);
    state
}

#[test]
fn test_serialize_roundtrip() {
    let config = Config::full();
    let state = make_test_genesis();

    // Serialize to KVs
    let kvs = serialize_state_with_opaque(&state, &config, &[]);

    // Must have entries
    assert!(!kvs.is_empty(), "serialized state should not be empty");

    // Deserialize back
    let (state2, _opaque) = deserialize_state(&kvs, &config).expect("deserialization failed");

    // Check basic properties
    assert_eq!(state2.timeslot, state.timeslot);
    assert_eq!(
        state2.pending_validators.len(),
        state.pending_validators.len()
    );
    assert_eq!(
        state2.current_validators.len(),
        state.current_validators.len()
    );
    assert_eq!(
        state2.previous_validators.len(),
        state.previous_validators.len()
    );
    assert_eq!(state2.pending_reports.len(), state.pending_reports.len());
    assert_eq!(state2.judgments.good.len(), state.judgments.good.len());
    assert_eq!(state2.judgments.bad.len(), state.judgments.bad.len());
    assert_eq!(
        state2.privileged_services.manager,
        state.privileged_services.manager
    );
}

#[test]
fn test_serialize_roundtrip_values() {
    let config = Config::full();
    let state = make_test_genesis();

    // Serialize → KVs
    let kvs1 = serialize_state_with_opaque(&state, &config, &[]);

    // Deserialize → State
    let (state2, opaque) = deserialize_state(&kvs1, &config).expect("deserialization failed");

    // Re-serialize → KVs2
    let kvs2 = serialize_state_with_opaque(&state2, &config, &opaque);

    // KV counts must match
    assert_eq!(
        kvs1.len(),
        kvs2.len(),
        "KV count mismatch: first={} second={}",
        kvs1.len(),
        kvs2.len()
    );

    // Each KV pair must match
    for (i, ((k1, v1), (k2, v2))) in kvs1.iter().zip(kvs2.iter()).enumerate() {
        assert_eq!(
            k1,
            k2,
            "Key mismatch at entry {i}: {} vs {}",
            hex::encode(k1),
            hex::encode(k2)
        );
        assert_eq!(
            v1,
            v2,
            "Value mismatch at entry {i} (key {}): {} bytes vs {} bytes",
            hex::encode(k1),
            v1.len(),
            v2.len()
        );
    }
}

#[test]
fn test_state_root_deterministic() {
    let config = Config::full();
    let state = make_test_genesis();

    let kvs = serialize_state_with_opaque(&state, &config, &[]);
    let root1 = compute_state_root_from_kvs(&kvs);
    let root2 = compute_state_root_from_kvs(&kvs);

    assert_eq!(
        root1,
        root2,
        "State root should be deterministic: {} vs {}",
        hex::encode(root1.0),
        hex::encode(root2.0)
    );
}
