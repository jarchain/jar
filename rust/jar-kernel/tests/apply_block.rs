//! End-to-end apply_block tests using a minimal genesis (no PVM blob yet —
//! Transact entrypoints run a smoke VM that halts immediately).

use std::sync::Arc;

use jar_kernel::genesis::GenesisBuilder;
use jar_kernel::runtime::{InMemoryBus, InMemoryHardware};
use jar_kernel::{BlockOutcome, apply_block};
use jar_types::{BlockHash, Body, Hash, Header, Slot};

fn build_genesis() -> jar_types::State {
    GenesisBuilder::default().build().expect("genesis ok").state
}

fn no_op_hardware() -> InMemoryHardware {
    let bus = InMemoryBus::new();
    InMemoryHardware::new(bus)
}

#[test]
fn apply_block_accepts_a_minimal_block() {
    let state = build_genesis();
    let parent = BlockHash::ZERO;
    let header = Header {
        parent,
        slot: Slot(1),
        ..Default::default()
    };
    let body = Body::default();
    let hw = no_op_hardware();
    let out = apply_block(&state, parent, &header, &body, &hw).unwrap();
    assert!(matches!(out.block_outcome, BlockOutcome::Accepted));
    assert_eq!(out.state_next.bookkeeping.slot, Slot(1));
}

#[test]
fn apply_block_rejects_non_monotonic_slot() {
    let state = build_genesis();
    let parent = BlockHash::ZERO;
    let header = Header {
        parent,
        slot: Slot(0), // not strictly greater than σ.bookkeeping.slot=0
        ..Default::default()
    };
    let body = Body::default();
    let hw = no_op_hardware();
    let out = apply_block(&state, parent, &header, &body, &hw).unwrap();
    match out.block_outcome {
        BlockOutcome::Panicked(reason) => {
            assert!(
                reason.contains("slot non-monotone"),
                "unexpected: {}",
                reason
            );
        }
        other => panic!("expected Panicked, got {:?}", other),
    }
}

#[test]
fn apply_block_rejects_wrong_parent_hash() {
    let state = build_genesis();
    let parent_actual = BlockHash::ZERO;
    let parent_claimed = Hash([7u8; 32]);
    let header = Header {
        parent: parent_claimed,
        slot: Slot(1),
        ..Default::default()
    };
    let body = Body::default();
    let hw = no_op_hardware();
    let out = apply_block(&state, parent_actual, &header, &body, &hw).unwrap();
    match out.block_outcome {
        BlockOutcome::Panicked(reason) => {
            assert!(reason.contains("parent hash"), "unexpected: {}", reason);
        }
        other => panic!("expected Panicked, got {:?}", other),
    }
}

#[test]
fn state_root_changes_after_block() {
    let state = build_genesis();
    let pre_root = jar_kernel::state_root(&state);
    let header = Header {
        parent: BlockHash::ZERO,
        slot: Slot(1),
        ..Default::default()
    };
    let body = Body::default();
    let hw = no_op_hardware();
    let out = apply_block(&state, BlockHash::ZERO, &header, &body, &hw).unwrap();
    let post_root = jar_kernel::state_root(&out.state_next);
    assert_ne!(
        pre_root, post_root,
        "state root must change once slot advances"
    );
}

// Keep Arc import alive (used implicitly by State).
#[allow(dead_code)]
fn _retain(_: Arc<()>) {}
