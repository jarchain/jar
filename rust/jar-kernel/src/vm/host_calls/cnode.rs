//! CNode-mutation host calls: grant / revoke / move.
//!
//! TEMPORARILY STUBBED — these host calls are slated for retirement in
//! favour of javm-management ecallis on the unified cap-table (see
//! `KernelCap` design notes). They are not exercised by the kernel's
//! smoke fixtures or the 3-node testnet today, so stubbing them here
//! is invisible to the existing test suite. A future commit will
//! either rewrite them against the new cap-table model or delete them
//! outright.

use crate::runtime::Hardware;
use crate::types::KResult;
use crate::vm::host_abi::*;
use crate::vm::{HostCallOutcome, InvocationCtx, Vm};

pub fn host_cnode_grant<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}

pub fn host_cnode_revoke<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}

pub fn host_cnode_move<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}
