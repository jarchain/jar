//! Sidecar trace types: attestation_trace, result_trace, reach_trace,
//! merkle_traces.
//!
//! These are produced by the proposer during apply_block and consumed
//! position-by-position by verifiers. The kernel enforces strict-equality
//! and exhaustion at apply_block end.

use crate::{Hash, KeyId, Signature, VaultId};

/// One signature recorded by an `attest()` call.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct AttestationEntry {
    pub key: KeyId,
    pub blob_hash: Hash,
    pub signature: Signature,
}

impl AttestationEntry {
    /// Returns true if this slot has not yet been filled (Sealing reserved).
    pub fn is_reserved(&self) -> bool {
        self.signature == Signature::default()
    }
}

/// One canonical computation output recorded by a `result_equal()` call.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct ResultEntry {
    pub blob: Vec<u8>,
}

/// Reach: which Vaults were initialized during one top-level invocation.
/// Strict-equality checked in verifier mode.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct ReachEntry {
    pub entrypoint: VaultId,
    pub event_idx: u32,
    pub vaults: Vec<VaultId>,
}

/// One storage_read proof. Used by light-clients to verify
/// `block_validation_cap` and `block_finalization_cap` runs against
/// `prior_block.header.state_root`.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct MerkleProof {
    pub vault: VaultId,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    /// Stub for now; populated by the future Merkle-trie implementation.
    pub proof_path: Vec<Hash>,
}
