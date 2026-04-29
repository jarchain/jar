//! Capabilities — variant structs, the `Capability` enum, and shared
//! helpers (pinning rules + attestation dispatch).

pub mod attest;
pub mod capability;
pub mod pinning;

pub use capability::*;
