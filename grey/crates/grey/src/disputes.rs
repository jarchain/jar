//! Block equivocation resolution — node-level stub.
//!
//! # Distinction from `grey-state/src/disputes.rs`
//!
//! `grey-state/src/disputes.rs` handles §10 on-chain work-report verdict
//! processing: verdicts, culprits, faults, offender slashing. It operates on
//! `report_hash` values inside the state transition.
//!
//! This module handles the *finality* side of block-level equivocation: once
//! the network has identified which of two same-slot blocks is the loser,
//! `report_loser` removes it from `GrandpaState` and un-poisons the slot so
//! the surviving fork can become acceptable to GRANDPA again.
//!
//! # TODO(§17)
//!
//! The trigger for `report_loser` will come from the block equivocation
//! reporting protocol (§17). When a validator observes two blocks at the same
//! slot it broadcasts both as evidence. Once a quorum of validators
//! countersigns the evidence, the losing block is identified and
//! `report_loser` is called here. That network protocol is not yet
//! implemented; this module is the stub call site ready to receive it.

use crate::finality::GrandpaState;
use grey_types::Hash;

/// Notify the finality layer that `loser_hash` has been identified as the
/// losing block in a same-slot equivocation and must be removed.
///
/// The caller is responsible for verifying equivocation evidence (quorum of
/// validator countersignatures) before invoking this function.
pub fn report_loser(loser_hash: Hash, grandpa: &mut GrandpaState) {
    grandpa.purge_block(loser_hash);
    tracing::info!(
        "Block equivocation resolved: purged losing block {:?} from finality state",
        loser_hash
    );
}
