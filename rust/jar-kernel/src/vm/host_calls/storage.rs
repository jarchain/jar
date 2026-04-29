//! Storage host calls: read / write / delete against a `Storage` or
//! `SnapshotStorage` cap held in the invocation Frame.

use javm::kernel::InvocationKernel;

use crate::runtime::Hardware;
use crate::state::storage;
use crate::types::{KResult, KernelError};
use crate::vm::host_abi::*;
use crate::vm::host_calls::{read_window, write_window};
use crate::vm::{HostCallOutcome, InvocationCtx};

pub fn host_storage_read<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    // φ[7] = frame_slot for Storage / SnapshotStorage cap
    // φ[8] = key_ptr, φ[9] = key_len, φ[10] = out_ptr, φ[11] = out_max
    let frame_slot = vm.active_reg(7) as u8;
    let key_ptr = vm.active_reg(8) as u32;
    let key_len = vm.active_reg(9) as u32;
    let out_ptr = vm.active_reg(10) as u32;
    let out_max = vm.active_reg(11) as u32;

    let cap_id = match ctx.frame.get(frame_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let key = match read_window(vm, key_ptr, key_len, "storage_read key") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };
    match storage::storage_read(ctx.state, cap_id, &key)? {
        Some(value) => {
            let to_write = value.len().min(out_max as usize);
            if to_write > 0
                && let Err(reason) =
                    write_window(vm, out_ptr, &value[..to_write], "storage_read out")
            {
                return Ok(HostCallOutcome::Fault(reason));
            }
            Ok(HostCallOutcome::Resume(value.len() as u64, 0))
        }
        None => Ok(HostCallOutcome::Resume(RC_NONE, 0)),
    }
}

pub fn host_storage_write<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    let frame_slot = vm.active_reg(7) as u8;
    let key_ptr = vm.active_reg(8) as u32;
    let key_len = vm.active_reg(9) as u32;
    let val_ptr = vm.active_reg(10) as u32;
    let val_len = vm.active_reg(11) as u32;

    let cap_id = match ctx.frame.get(frame_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let key = match read_window(vm, key_ptr, key_len, "storage_write key") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };
    let val = match read_window(vm, val_ptr, val_len, "storage_write val") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };

    match storage::storage_write(ctx.state, cap_id, &key, &val) {
        Ok(()) => Ok(HostCallOutcome::Resume(RC_OK, 0)),
        Err(KernelError::ReadOnly(_)) => Ok(HostCallOutcome::Resume(RC_READONLY, 0)),
        Err(KernelError::QuotaExceeded { .. }) => Ok(HostCallOutcome::Resume(RC_QUOTA, 0)),
        Err(e) => Err(e),
    }
}

pub fn host_storage_delete<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    let frame_slot = vm.active_reg(7) as u8;
    let key_ptr = vm.active_reg(8) as u32;
    let key_len = vm.active_reg(9) as u32;

    let cap_id = match ctx.frame.get(frame_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let key = match read_window(vm, key_ptr, key_len, "storage_delete key") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };

    match storage::storage_delete(ctx.state, cap_id, &key) {
        Ok(()) => Ok(HostCallOutcome::Resume(RC_OK, 0)),
        Err(KernelError::ReadOnly(_)) => Ok(HostCallOutcome::Resume(RC_READONLY, 0)),
        Err(e) => Err(e),
    }
}
