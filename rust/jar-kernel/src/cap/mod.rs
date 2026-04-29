//! Capability helpers: pinning rules + attestation dispatch.
//!
//! The `Capability` enum + variant-data structs live in `crate::types::cap`
//! today; commit 3 lifts each variant into its own struct under this
//! module.

pub mod attest;
pub mod pinning;
