//! Block / Header / Body shapes.

use std::collections::BTreeMap;

use crate::{
    AttestationEntry, BlockHash, Event, Hash, MerkleProof, ReachEntry, ResultEntry, Slot, VaultId,
};

/// Block header. Minimal shape — chain-specific fields go through
/// `block_validation_cap` rather than the kernel.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Header {
    pub parent: BlockHash,
    pub slot: Slot,
    pub state_root: Hash,
    /// Body root commitment. Chain-defined; the kernel does not verify it.
    pub body_root: Hash,
    /// Author identifier. Opaque to the kernel.
    pub author: [u8; 32],
}

/// Block body. Carries on-chain events plus all sidecar traces.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Body {
    /// Events grouped per Transact entrypoint Vault. Iterated in canonical
    /// order during apply_block's transact phase.
    pub events: BTreeMap<VaultId, Vec<Event>>,
    pub attestation_trace: Vec<AttestationEntry>,
    pub result_trace: Vec<ResultEntry>,
    pub reach_trace: Vec<ReachEntry>,
    pub merkle_traces: Vec<MerkleProof>,
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Block {
    pub header: Header,
    pub body: Body,
}
