//! Capability-management host calls: derive / vault_initialize /
//! create_vault / quota_set.
//!
//! TEMPORARILY STUBBED — these host calls are slated for retirement in
//! favour of javm-management ecallis (`cap_derive` → javm DOWNGRADE,
//! etc.) on the unified cap-table. They are not exercised by the
//! kernel's smoke fixtures or the 3-node testnet, so stubbing here is
//! invisible to the existing test suite. A future commit reworks each
//! one to the new model or deletes it.
//!
//! `cap_call` already retired — entrypoint invocation goes through
//! plain javm CALL on a Handle / Callable cap-table slot.

use crate::runtime::Hardware;
use crate::types::KResult;
use crate::vm::host_abi::*;
use crate::vm::{HostCallOutcome, InvocationCtx, Vm};

pub fn host_cap_derive<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}

pub fn host_vault_initialize<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}

pub fn host_create_vault<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}

pub fn host_quota_set<H: Hardware>(
    _vm: &mut Vm,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}
