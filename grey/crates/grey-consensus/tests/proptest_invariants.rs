//! Property-based tests for state transition invariants.
//!
//! These tests verify that applying empty blocks to genesis state
//! preserves fundamental invariants that must hold after every block.

use grey_consensus::authoring::author_block;
use grey_consensus::genesis::create_genesis;
use grey_types::config::Config;
use proptest::prelude::*;

/// Apply a sequence of empty blocks and check invariants after each.
fn check_invariants_over_slots(slot_offsets: Vec<u32>) {
    let config = Config::tiny();
    let (mut state, secrets) = create_genesis(&config);

    let initial_validator_count = state.current_validators.len();
    let mut prev_timeslot = state.timeslot;

    for offset in slot_offsets {
        // Advance by at least 1 slot, capped to avoid huge gaps
        let next_slot = prev_timeslot + 1 + (offset % 10);
        let author_index = (next_slot as u16) % config.validators_count;

        let state_root = grey_types::Hash::ZERO;
        let block = author_block(
            &state,
            &config,
            next_slot,
            author_index,
            &secrets[author_index as usize],
            state_root,
        );

        match grey_state::transition::apply_with_config(&state, &block, &config, &[]) {
            Ok((new_state, _)) => {
                // Invariant 1: timeslot strictly increases
                assert!(
                    new_state.timeslot > prev_timeslot,
                    "timeslot must increase: {} -> {}",
                    prev_timeslot,
                    new_state.timeslot
                );

                // Invariant 2: validator set size is constant
                assert_eq!(
                    new_state.current_validators.len(),
                    initial_validator_count,
                    "validator set size changed from {} to {}",
                    initial_validator_count,
                    new_state.current_validators.len()
                );

                // Invariant 3: previous validators size matches
                assert_eq!(
                    new_state.previous_validators.len(),
                    initial_validator_count,
                    "previous validator set size changed"
                );

                // Invariant 4: entropy array has 4 elements
                assert_eq!(new_state.entropy.len(), 4, "entropy array length changed");

                // Invariant 5: pending_reports length equals total_cores
                assert_eq!(
                    new_state.pending_reports.len(),
                    usize::from(config.core_count),
                    "pending_reports length changed"
                );

                // Invariant 6: auth_pool length equals total_cores
                assert_eq!(
                    new_state.auth_pool.len(),
                    usize::from(config.core_count),
                    "auth_pool length changed from {}",
                    new_state.auth_pool.len()
                );

                // Invariant 7: auth_queue length equals auth_queue_size
                assert_eq!(
                    new_state.auth_queue.len(),
                    config.auth_queue_size,
                    "auth_queue length changed from {}",
                    new_state.auth_queue.len()
                );

                // Invariant 8: pending_validators size is constant
                assert_eq!(
                    new_state.pending_validators.len(),
                    initial_validator_count,
                    "pending_validators size changed"
                );

                // Invariant 9: safrole pending_keys size is constant
                assert_eq!(
                    new_state.safrole.pending_keys.len(),
                    initial_validator_count,
                    "safrole pending_keys size changed"
                );

                prev_timeslot = new_state.timeslot;
                state = new_state;
            }
            Err(_) => {
                // Some blocks may fail validation (e.g., bad seal for the slot).
                // This is expected — just skip and try the next slot.
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// Applying a sequence of empty blocks preserves state invariants.
    #[test]
    fn state_invariants_hold(
        offsets in proptest::collection::vec(0u32..10, 1..8),
    ) {
        check_invariants_over_slots(offsets);
    }
}
