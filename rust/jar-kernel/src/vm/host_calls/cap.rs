//! Capability-management host calls: derive / call / vault_initialize /
//! create_vault / quota_set.

use javm::kernel::InvocationKernel;

use crate::cap::pinning;
use crate::runtime::Hardware;
use crate::state::cap_registry;
use crate::types::{
    CapId, CapRecord, Capability, Command, KResult, KernelError, KernelRole, ResourceKind,
    SlotContent,
};
use crate::vm::host_abi::*;
use crate::vm::host_calls::read_window;
use crate::vm::{HostCallOutcome, InvocationCtx};

pub fn host_cap_derive<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    // φ[7]=src_frame_slot, φ[8]=dest_frame_slot, φ[9]=narrowing_ptr, φ[10]=narrowing_len,
    // φ[11]=mode (0=Frame, 1=persistent into a CNode-cap-frame-slot), φ[12]=dest_cnode_frame_slot
    let src_slot = vm.active_reg(7) as u8;
    let dst_slot = vm.active_reg(8) as u8;
    let narr_ptr = vm.active_reg(9) as u32;
    let narr_len = vm.active_reg(10) as u32;
    let mode = vm.active_reg(11);
    let dest_cnode_fs = vm.active_reg(12) as u8;

    let src_cap = match ctx.frame.get(src_slot) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let narrowing = match read_window(vm, narr_ptr, narr_len, "cap_derive narrowing") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };
    let src_record = cap_registry::lookup(ctx.state, src_cap)?.clone();
    // Compute the new capability shape (kernel chooses based on src + mode).
    let (new_cap, dest_persistent) = match (&src_record.cap, mode) {
        (Capability::Dispatch { vault_id, .. }, 0) => (
            Capability::DispatchRef {
                vault_id: *vault_id,
            },
            false,
        ),
        (Capability::Dispatch { vault_id, .. }, 1) => {
            let dest_cnode_cap = match ctx.frame.get(dest_cnode_fs) {
                Some(c) => c,
                None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
            };
            let dest_cnode_id = match &cap_registry::lookup(ctx.state, dest_cnode_cap)?.cap {
                Capability::CNode { cnode_id } => *cnode_id,
                _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
            };
            (
                Capability::Dispatch {
                    vault_id: *vault_id,
                    born_in: dest_cnode_id,
                },
                true,
            )
        }
        (Capability::Transact { vault_id, .. }, 0) => (
            Capability::TransactRef {
                vault_id: *vault_id,
            },
            false,
        ),
        (Capability::Transact { vault_id, .. }, 1) => {
            let dest_cnode_cap = match ctx.frame.get(dest_cnode_fs) {
                Some(c) => c,
                None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
            };
            let dest_cnode_id = match &cap_registry::lookup(ctx.state, dest_cnode_cap)?.cap {
                Capability::CNode { cnode_id } => *cnode_id,
                _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
            };
            (
                Capability::Transact {
                    vault_id: *vault_id,
                    born_in: dest_cnode_id,
                },
                true,
            )
        }
        (Capability::DispatchRef { vault_id }, 0) => (
            Capability::DispatchRef {
                vault_id: *vault_id,
            },
            false,
        ),
        (Capability::TransactRef { vault_id }, 0) => (
            Capability::TransactRef {
                vault_id: *vault_id,
            },
            false,
        ),
        (Capability::VaultRef { vault_id, rights }, _) => (
            Capability::VaultRef {
                vault_id: *vault_id,
                rights: *rights,
            },
            mode == 1,
        ),
        _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    match cap_registry::derive(ctx.state, src_cap, new_cap, narrowing, dest_persistent) {
        Ok(new_id) => {
            ctx.frame.set(dst_slot, new_id);
            Ok(HostCallOutcome::Resume(new_id.0, 0))
        }
        Err(KernelError::Pinning(_)) => Ok(HostCallOutcome::Resume(RC_PINNING, 0)),
        Err(e) => Err(e),
    }
}

/// `cap_call` — the universal callable-cap exercise.
pub fn host_cap_call<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    let cap_fs = vm.active_reg(7) as u8;
    let args_ptr = vm.active_reg(8) as u32;
    let args_len = vm.active_reg(9) as u32;
    let caps_ptr = vm.active_reg(10) as u32;
    let caps_len = vm.active_reg(11) as u32;

    let cap_id = match ctx.frame.get(cap_fs) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let cap = cap_registry::lookup(ctx.state, cap_id)?.cap.clone();
    let args = match read_window(vm, args_ptr, args_len, "cap_call args") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };
    let caps_bytes = match read_window(vm, caps_ptr, caps_len, "cap_call caps") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };
    let mut arg_caps: Vec<CapId> = Vec::with_capacity(caps_bytes.len());
    for fs in caps_bytes {
        let cid = ctx
            .frame
            .get(fs)
            .ok_or_else(|| KernelError::Internal(format!("cap_call: arg slot {} empty", fs)))?;
        arg_caps.push(cid);
    }

    match cap {
        Capability::VaultRef { rights, .. } if rights.initialize => {
            // Sub-CALL stub. No arg-scan for sub-CALLs.
            Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
        }
        Capability::Dispatch { vault_id, .. } | Capability::DispatchRef { vault_id } => {
            pinning::arg_scan(ctx.state, &arg_caps)?;
            if matches!(ctx.role, KernelRole::AggregateMerge) && vault_id == ctx.current_vault {
                if ctx.slot_emission.is_some() {
                    return Err(KernelError::Internal(
                        "step-3 emitted more than one slot replacement".into(),
                    ));
                }
                *ctx.slot_emission = Some(SlotContent::AggregatedDispatch {
                    payload: args,
                    caps: caps_bytes_to_vec(&arg_caps),
                    attestation_trace: ctx.attestation_trace.clone(),
                    result_trace: ctx.result_trace.clone(),
                });
                return Ok(HostCallOutcome::Resume(RC_OK, 0));
            }
            ctx.commands.push(Command::Dispatch {
                entrypoint: vault_id,
                payload: args,
                caps: caps_bytes_to_vec(&arg_caps),
            });
            Ok(HostCallOutcome::Resume(RC_OK, 0))
        }
        Capability::Transact { vault_id, .. } | Capability::TransactRef { vault_id } => {
            pinning::arg_scan(ctx.state, &arg_caps)?;
            if matches!(ctx.role, KernelRole::AggregateMerge) {
                if ctx.slot_emission.is_some() {
                    return Err(KernelError::Internal(
                        "step-3 emitted more than one slot replacement".into(),
                    ));
                }
                *ctx.slot_emission = Some(SlotContent::AggregatedTransact {
                    target: vault_id,
                    payload: args,
                    caps: caps_bytes_to_vec(&arg_caps),
                    attestation_trace: ctx.attestation_trace.clone(),
                    result_trace: ctx.result_trace.clone(),
                });
                return Ok(HostCallOutcome::Resume(RC_OK, 0));
            }
            ctx.commands.push(Command::Dispatch {
                entrypoint: vault_id,
                payload: args,
                caps: caps_bytes_to_vec(&arg_caps),
            });
            Ok(HostCallOutcome::Resume(RC_OK, 0))
        }
        Capability::Schedule { .. } => Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
        _ => Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    }
}

fn caps_bytes_to_vec(caps: &[CapId]) -> Vec<u8> {
    let mut out = Vec::with_capacity(caps.len() * 8);
    for c in caps {
        out.extend_from_slice(&c.0.to_le_bytes());
    }
    out
}

/// `vault_initialize` — placeholder; real sub-VM scheduling deferred.
pub fn host_vault_initialize<H: Hardware>(
    _vm: &mut InvocationKernel,
    _ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    Ok(HostCallOutcome::Resume(RC_UNIMPLEMENTED, 0))
}

pub fn host_create_vault<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    let res_fs = vm.active_reg(7) as u8;
    let code_hash_ptr = vm.active_reg(8) as u32;
    let dest_fs = vm.active_reg(9) as u8;

    let res_cap_id = match ctx.frame.get(res_fs) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let (quota_items, quota_bytes) = match &cap_registry::lookup(ctx.state, res_cap_id)?.cap {
        Capability::Resource(ResourceKind::CreateVault {
            quota_items,
            quota_bytes,
        }) => (*quota_items, *quota_bytes),
        _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let code_hash_bytes = match read_window(vm, code_hash_ptr, 32, "create_vault code_hash") {
        Ok(b) => b,
        Err(reason) => return Ok(HostCallOutcome::Fault(reason)),
    };
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&code_hash_bytes);
    let code_hash = crate::types::Hash(buf);

    let new_vault_id = ctx.state.next_vault_id();
    let mut vault = crate::types::Vault::new(code_hash);
    vault.quota_items = quota_items;
    vault.quota_bytes = quota_bytes;
    ctx.state
        .vaults
        .insert(new_vault_id, std::sync::Arc::new(vault));

    let cap_id = cap_registry::alloc(
        ctx.state,
        CapRecord {
            cap: Capability::VaultRef {
                vault_id: new_vault_id,
                rights: crate::types::VaultRights::ALL,
            },
            issuer: Some(res_cap_id),
            narrowing: Vec::new(),
        },
    );
    ctx.frame.set(dest_fs, cap_id);
    Ok(HostCallOutcome::Resume(cap_id.0, new_vault_id.0))
}

pub fn host_quota_set<H: Hardware>(
    vm: &mut InvocationKernel,
    ctx: &mut InvocationCtx<'_, H>,
) -> KResult<HostCallOutcome> {
    let res_fs = vm.active_reg(7) as u8;
    let new_items = vm.active_reg(8);
    let new_bytes = vm.active_reg(9);
    let res_cap_id = match ctx.frame.get(res_fs) {
        Some(c) => c,
        None => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let target = match &cap_registry::lookup(ctx.state, res_cap_id)?.cap {
        Capability::Resource(ResourceKind::SetQuota { target }) => *target,
        _ => return Ok(HostCallOutcome::Resume(RC_BAD_CAP, 0)),
    };
    let arc = ctx
        .state
        .vaults
        .get(&target)
        .ok_or(KernelError::VaultNotFound(target))?
        .clone();
    let mut vault: crate::types::Vault = (*arc).clone();
    vault.quota_items = new_items;
    vault.quota_bytes = new_bytes;
    ctx.state.vaults.insert(target, std::sync::Arc::new(vault));
    Ok(HostCallOutcome::Resume(RC_OK, 0))
}
