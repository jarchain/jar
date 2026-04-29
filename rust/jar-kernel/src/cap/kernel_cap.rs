//! `KernelCap` — the protocol-cap payload type jar-kernel substitutes
//! into javm's `Cap::Protocol(P)`.
//!
//! Each running VM's javm cap-table is the kernel's per-invocation
//! Frame. Slots hold one of:
//! - `KernelCap::HostCall(u8)` — populated at VM init for the kernel's
//!   host-call selector range (1..=21 today). When the guest does
//!   `ecalli N`, javm yields `KernelResult::ProtocolCall { slot: N }`,
//!   the kernel's `drive_invocation` loop fetches the slot, sees a
//!   `HostCall(N)`, and dispatches the corresponding `HostCall`
//!   handler.
//! - `KernelCap::Cap(Capability)` — kernel cap data placed at a free
//!   cap-table slot (e.g. the per-vault Storage / SnapshotStorage cap
//!   for the running invocation).
//!
//! `KernelCap` is the wrapper that lets these two flavors coexist in
//! one `CapTable<KernelCap>`. As host calls retire to javm-management
//! ecallis, the `HostCall` arm shrinks; eventually we may drop the
//! wrapper entirely and use `P = Capability` directly.
//!
//! The `ProtocolCapT` impl makes javm-side mgmt ecallis (COPY, MOVE,
//! DROP) refuse to mutate slots that hold pinned kernel caps. Host
//! calls bypass these checks because they go through the kernel's
//! `cap_grant` / `cap_move` / `cap_derive` host-call handlers, which
//! enforce their own pinning rules.

use crate::cap::Capability;
use javm::cap::ProtocolCapT;

/// Cap-table slot reserved for the kernel-cap payload at frame init
/// (host-call selector range is 1..=21; slot 32 sits comfortably above
/// it).
pub const KERNEL_CAP_SLOT: u8 = 32;

/// The protocol-cap payload type jar-kernel substitutes into javm's
/// `Cap::Protocol(P)`. See module-level docs.
#[derive(Clone, Debug)]
pub enum KernelCap {
    /// A host-call selector. `ecalli N` on a slot containing
    /// `HostCall(N)` yields `ProtocolCall { slot: N }` to the host.
    HostCall(u8),
    /// A real kernel capability (Storage, SnapshotStorage, etc.) held
    /// in the running VM's cap-table for the duration of the invocation.
    Cap(Capability),
}

impl ProtocolCapT for KernelCap {
    fn is_copyable(&self) -> bool {
        match self {
            // Host-call selectors are stateless ids; copying them is
            // harmless (a guest that copies one just creates another
            // way to invoke the same host call).
            KernelCap::HostCall(_) => true,
            // Kernel caps inherit the pinning rules from `Capability`.
            // Pinned variants (Dispatch / Transact / Schedule) and
            // their refs must not be COPYed by the guest.
            KernelCap::Cap(c) => !c.is_pinned_or_ref(),
        }
    }

    fn is_movable(&self) -> bool {
        // MOVE is a transfer (no aliasing). We allow within a Frame for
        // every payload kind. Persistent placement is gated separately
        // by the kernel's `cap_grant` host call.
        true
    }

    fn is_droppable(&self) -> bool {
        true
    }
}
