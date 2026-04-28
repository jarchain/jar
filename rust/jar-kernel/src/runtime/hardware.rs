//! Hardware abstraction.
//!
//! The kernel takes `&H: Hardware` so it can:
//! - decide proposer-vs-verifier per AttestationCap (`holds_key`)
//! - sign blobs in producer mode (`sign`)
//! - emit Dispatch / BroadcastLite commands to the network (`emit`)
//!
//! σ is **not** owned by Hardware — passed alongside as `&State` /
//! `State`. Off-chain aggregation slots live in `NodeOffchain`, also outside.

use jar_types::{Command, KeyId, Signature};

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq)]
pub enum HwError {
    #[error("hardware does not hold the requested key")]
    KeyAbsent,
    #[error("hardware sign failed: {0}")]
    SignFailure(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TracingEvent {
    InvocationFault { reason: String },
    BlockPanic { reason: String },
}

pub trait Hardware: Send + Sync {
    fn holds_key(&self, key: KeyId) -> bool;

    fn sign(&self, key: KeyId, blob: &[u8]) -> Result<Signature, HwError>;

    fn emit(&self, cmd: Command);

    fn tracing_event(&self, ev: TracingEvent) {
        // Default: drop. Tests / production override.
        let _ = ev;
    }
}
