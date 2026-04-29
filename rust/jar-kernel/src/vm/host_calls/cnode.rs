//! CNode-mutation host calls: grant / revoke / move.

use javm::kernel::InvocationKernel;

use crate::runtime::Hardware;
use crate::state::{cap_registry, cnode};
use crate::types::{CNodeId, Capability, KResult, KernelError};
use crate::vm::host_abi::*;
use crate::vm::{HostCallOutcome, InvocationCtx};

pub fn host_cnode_grant<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    // φ[7]=src_frame_slot, φ[8]=dest_cnode_frame_slot, φ[9]=dest_cnode_slot
    let src_slot = vm.active_reg(7) as u8;
    let dest_cnode_slot = vm.active_reg(8) as u8;
    let dest_slot = vm.active_reg(9) as u8;
    let src_cap = match ctx.frame.get(src_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let dest_cnode_cap = match ctx.frame.get(dest_cnode_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let dest_cnode_id = match &cap_registry::lookup(ctx.state, dest_cnode_cap)?.cap {
        Capability::CNode { cnode_id } => *cnode_id,
        _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    match cnode::cnode_grant(ctx.state, src_cap, dest_cnode_id, dest_slot) {
        Ok(_) => Ok(HostCallOutcome::Resume(RC_OK, 0)),
        Err(KernelError::Pinning(_)) => Ok(HostCallOutcome::Resume(RC_PINNING, 0)),
        Err(e) => Err(e),
    }
}

pub fn host_cnode_revoke<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    let cnode_frame_slot = vm.active_reg(7) as u8;
    let cnode_slot = vm.active_reg(8) as u8;
    let cnode_cap = match ctx.frame.get(cnode_frame_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let cnode_id = match &cap_registry::lookup(ctx.state, cnode_cap)?.cap {
        Capability::CNode { cnode_id } => *cnode_id,
        _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    cnode::cnode_revoke(ctx.state, cnode_id, cnode_slot)?;
    Ok(HostCallOutcome::Resume(RC_OK, 0))
}

pub fn host_cnode_move<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    // φ[7]=src_cnode_frame_slot, φ[8]=src_slot, φ[9]=dest_cnode_frame_slot, φ[10]=dest_slot
    let src_cn_fs = vm.active_reg(7) as u8;
    let src_slot = vm.active_reg(8) as u8;
    let dst_cn_fs = vm.active_reg(9) as u8;
    let dst_slot = vm.active_reg(10) as u8;
    let resolve = |state: &crate::types::State, fs: u8| -> KResult<CNodeId> {
        let cap = ctx
            .frame
            .get(fs)
            .ok_or_else(|| KernelError::Internal(format!("frame slot {} empty", fs)))?;
        match &cap_registry::lookup(state, cap)?.cap {
            Capability::CNode { cnode_id } => Ok(*cnode_id),
            _ => Err(KernelError::Internal("expected CNode cap".into())),
        }
    };
    let src_cn = resolve(ctx.state, src_cn_fs)?;
    let dst_cn = resolve(ctx.state, dst_cn_fs)?;
    match cnode::cnode_move(ctx.state, src_cn, src_slot, dst_cn, dst_slot) {
        Ok(_) => Ok(HostCallOutcome::Resume(RC_OK, 0)),
        Err(KernelError::Pinning(_)) => Ok(HostCallOutcome::Resume(RC_PINNING, 0)),
        Err(e) => Err(e),
    }
}
