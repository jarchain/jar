//! Refine sub-transition (Section 14 of the Gray Paper).
//!
//! Implements the work-package processing pipeline:
//! 1. Ψ_I (is-authorized): verify the package is authorized for its core
//! 2. Ψ_R (refine): execute each work item's refinement code
//! 3. Assemble a WorkReport from the results

use crate::pvm_backend::{ExitReason, PvmInstance};
use grey_types::config::Config;
use grey_types::constants::{GAS_IS_AUTHORIZED, HOST_OOB, HOST_WHAT};
use grey_types::work::*;
use grey_types::{Hash, ServiceId};
use javm::Gas;
use std::collections::BTreeMap;

/// Build an error RefineResult for a work item (non-Ok exit).
fn error_refine_result(item: &WorkItem, result: WorkResult, gas_used: Gas) -> RefineResult {
    RefineResult {
        digest: WorkDigest {
            service_id: item.service_id,
            code_hash: item.code_hash,
            payload_hash: grey_crypto::blake2b_256(&item.payload),
            accumulate_gas: item.accumulate_gas_limit,
            result,
            gas_used,
            imports_count: 0,
            extrinsics_count: 0,
            extrinsics_size: 0,
            exports_count: 0,
        },
        exported_segments: vec![],
    }
}

/// Read output bytes from PVM registers ω[7] (ptr) and ω[8] (len).
fn read_pvm_output(pvm: &PvmInstance) -> Vec<u8> {
    let ptr = pvm.reg(7) as u32;
    let len = pvm.reg(8) as u32;
    if len > 0 {
        pvm.try_read_bytes(ptr, len).unwrap_or_default()
    } else {
        vec![]
    }
}

/// Errors from the refine pipeline.
#[derive(Debug)]
pub enum RefineError {
    /// Service code not found for the given code hash.
    CodeNotFound(Hash),
    /// Authorization failed.
    AuthorizationFailed(String),
    /// PVM initialization failed.
    PvmInitFailed,
}

impl std::fmt::Display for RefineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefineError::CodeNotFound(h) => {
                write!(f, "code not found: 0x{}", hex::encode(&h.0[..8]))
            }
            RefineError::AuthorizationFailed(msg) => write!(f, "authorization failed: {}", msg),
            RefineError::PvmInitFailed => write!(f, "PVM initialization failed"),
        }
    }
}

/// Context for looking up service code and state during refinement.
pub trait RefineContext {
    /// Get the code blob for a service by its code hash.
    fn get_code(&self, code_hash: &Hash) -> Option<Vec<u8>>;

    /// Get a storage value for a service.
    fn get_storage(&self, service_id: ServiceId, key: &[u8]) -> Option<Vec<u8>>;

    /// Get a preimage by its hash.
    fn get_preimage(&self, hash: &Hash) -> Option<Vec<u8>>;
}

/// Simple in-memory refine context for testing.
pub struct SimpleRefineContext {
    pub code_blobs: BTreeMap<Hash, Vec<u8>>,
    pub storage: BTreeMap<(ServiceId, Vec<u8>), Vec<u8>>,
    pub preimages: BTreeMap<Hash, Vec<u8>>,
}

impl RefineContext for SimpleRefineContext {
    fn get_code(&self, code_hash: &Hash) -> Option<Vec<u8>> {
        self.code_blobs.get(code_hash).cloned()
    }
    fn get_storage(&self, service_id: ServiceId, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.get(&(service_id, key.to_vec())).cloned()
    }
    fn get_preimage(&self, hash: &Hash) -> Option<Vec<u8>> {
        self.preimages.get(hash).cloned()
    }
}

/// Run the Is-Authorized invocation Ψ_I (GP eq B.1-B.2).
///
/// Entry point: PC=0
/// Arguments: authorization ++ work_package_encoding
/// Returns: (auth_output, gas_used) on success
pub fn invoke_is_authorized(
    _config: &Config,
    code_blob: &[u8],
    authorization: &[u8],
    work_package_encoding: &[u8],
    gas_limit: Gas,
) -> Result<(Vec<u8>, Gas), RefineError> {
    // Build arguments: authorization ++ work_package_encoding
    let mut args = Vec::with_capacity(authorization.len() + work_package_encoding.len());
    args.extend_from_slice(authorization);
    args.extend_from_slice(work_package_encoding);

    let mut pvm =
        PvmInstance::initialize(code_blob, &args, gas_limit).ok_or(RefineError::PvmInitFailed)?;

    // Entry point for is-authorized: PC=0 (default after initialize)
    let initial_gas = pvm.gas();

    loop {
        let exit = pvm.run();
        match exit {
            ExitReason::Halt => {
                let gas_used = initial_gas - pvm.gas();
                let output = read_pvm_output(&pvm);
                tracing::debug!(
                    "is_authorized HALT: gas_used={}, output_len={}",
                    gas_used,
                    output.len()
                );
                return Ok((output, gas_used));
            }
            ExitReason::Panic => {
                return Err(RefineError::AuthorizationFailed("PVM panic".into()));
            }
            ExitReason::OutOfGas => {
                return Err(RefineError::AuthorizationFailed("out of gas".into()));
            }
            ExitReason::PageFault(addr) => {
                return Err(RefineError::AuthorizationFailed(format!(
                    "page fault at 0x{:08x}",
                    addr
                )));
            }
            ExitReason::HostCall(id) => {
                if !handle_is_authorized_host_call(id, &mut pvm) {
                    if pvm.gas() == 0 {
                        return Err(RefineError::AuthorizationFailed("out of gas".into()));
                    } else {
                        return Err(RefineError::AuthorizationFailed("page fault".into()));
                    }
                }
            }
        }
    }
}

/// Result of a single refine invocation: the work digest plus exported segments.
pub struct RefineResult {
    pub digest: WorkDigest,
    pub exported_segments: Vec<Vec<u8>>,
}

/// Run the Refine invocation Ψ_R for a single work item (GP eq B.3-B.5).
///
/// Entry point: PC=0
/// Arguments: payload
/// Returns: RefineResult with work digest and exported segments
pub fn invoke_refine(
    _config: &Config,
    code_blob: &[u8],
    item: &WorkItem,
    export_offset: u16,
    import_data: &[Vec<u8>],
) -> RefineResult {
    let mut pvm = match PvmInstance::initialize(code_blob, &item.payload, item.gas_limit) {
        Some(p) => p,
        None => return error_refine_result(item, WorkResult::BadCode, 0),
    };

    // Entry point for refine: PC=0 (default)
    let initial_gas = pvm.gas();
    let mut exported_segments: Vec<Vec<u8>> = Vec::new();

    loop {
        let exit = pvm.run();
        match exit {
            ExitReason::Halt => {
                let gas_used = initial_gas - pvm.gas();
                let output = read_pvm_output(&pvm);
                let exports_count = exported_segments.len() as u16;
                let result = if item.exports_count != exports_count && item.exports_count > 0 {
                    WorkResult::BadExports
                } else {
                    WorkResult::Ok(output)
                };
                tracing::debug!(
                    "refine HALT: service={}, gas_used={}, exports={}",
                    item.service_id,
                    gas_used,
                    exports_count
                );
                return RefineResult {
                    digest: WorkDigest {
                        service_id: item.service_id,
                        code_hash: item.code_hash,
                        payload_hash: grey_crypto::blake2b_256(&item.payload),
                        accumulate_gas: item.accumulate_gas_limit,
                        result,
                        gas_used,
                        imports_count: item.imports.len() as u16,
                        extrinsics_count: item.extrinsics.len() as u16,
                        extrinsics_size: 0,
                        exports_count,
                    },
                    exported_segments,
                };
            }
            ExitReason::Panic => {
                let gas_used = initial_gas - pvm.gas();
                tracing::debug!(
                    "refine PANIC: service={}, gas_used={}",
                    item.service_id,
                    gas_used
                );
                return error_refine_result(item, WorkResult::Panic, gas_used);
            }
            ExitReason::OutOfGas => {
                tracing::debug!("refine OOG: service={}", item.service_id);
                return error_refine_result(item, WorkResult::OutOfGas, initial_gas);
            }
            ExitReason::PageFault(_addr) => {
                let gas_used = initial_gas - pvm.gas();
                return error_refine_result(item, WorkResult::Panic, gas_used);
            }
            ExitReason::HostCall(id) => {
                let mut ctx = RefineHostContext {
                    item,
                    exported_segments: &mut exported_segments,
                    export_offset,
                    import_data,
                };
                if !handle_refine_host_call(id, &mut pvm, &mut ctx) {
                    // Host call signaled OOG or page fault
                    if pvm.gas() == 0 {
                        return error_refine_result(item, WorkResult::OutOfGas, initial_gas);
                    } else {
                        let gas_used = initial_gas - pvm.gas();
                        return error_refine_result(item, WorkResult::Panic, gas_used);
                    }
                }
            }
        }
    }
}

/// Process a complete work package: authorize, then refine each item.
///
/// This is the main entry point for the refine pipeline (GP eq 14.12).
pub fn process_work_package(
    config: &Config,
    package: &WorkPackage,
    ctx: &dyn RefineContext,
    core_index: u16,
) -> Result<WorkReport, RefineError> {
    // 1. Look up the authorizer code
    let auth_code = ctx
        .get_code(&package.auth_code_hash)
        .ok_or(RefineError::CodeNotFound(package.auth_code_hash))?;

    // 2. Run is-authorized (Ψ_I)
    // For now, use a simple encoding of the work package
    let wp_encoding = encode_work_package_simple(package);
    let (auth_output, auth_gas_used) = invoke_is_authorized(
        config,
        &auth_code,
        &package.authorization,
        &wp_encoding,
        GAS_IS_AUTHORIZED,
    )?;

    let authorizer_hash = grey_crypto::blake2b_256(&auth_code);

    // 3. Refine each work item (Ψ_R)
    let mut results = Vec::with_capacity(package.items.len());
    let mut all_exported_segments: Vec<Vec<u8>> = Vec::new();
    let mut export_offset: u16 = 0;
    for item in &package.items {
        let item_code = ctx
            .get_code(&item.code_hash)
            .ok_or(RefineError::CodeNotFound(item.code_hash))?;

        // Resolve import segment data (if available via context)
        // TODO: resolve import segments from availability store
        let import_data: Vec<Vec<u8>> = Vec::new();

        let refine_result = invoke_refine(config, &item_code, item, export_offset, &import_data);
        export_offset += refine_result.exported_segments.len() as u16;
        all_exported_segments.extend(refine_result.exported_segments);
        results.push(refine_result.digest);
    }

    // 4. Compute package hash
    let package_hash = grey_crypto::blake2b_256(&wp_encoding);

    // 5. Compute exports_root using constant-depth Merkle tree (eq E.4)
    let exports_root = if all_exported_segments.is_empty() {
        Hash::ZERO
    } else {
        let segment_refs: Vec<&[u8]> = all_exported_segments.iter().map(|s| s.as_slice()).collect();
        grey_merkle::constant_depth_merkle_root(&segment_refs, grey_crypto::blake2b_256)
    };

    // 6. Assemble work report
    let report = WorkReport {
        package_spec: AvailabilitySpec {
            package_hash,
            bundle_length: wp_encoding.len() as u32,
            erasure_root: Hash::ZERO, // Computed by guarantor after erasure coding
            exports_root,
            exports_count: results.iter().map(|r| r.exports_count).sum(),
        },
        context: package.context.clone(),
        core_index,
        authorizer_hash,
        auth_gas_used,
        auth_output,
        segment_root_lookup: BTreeMap::new(),
        results,
    };

    tracing::info!(
        "Refined work package: hash=0x{}, core={}, items={}, auth_gas={}",
        hex::encode(&package_hash.0[..8]),
        core_index,
        report.results.len(),
        auth_gas_used
    );

    Ok(report)
}

/// Handle host calls available in Ψ_I (is-authorized).
/// Only gas(0) and grow_heap(1) are available.
fn handle_is_authorized_host_call(id: u32, pvm: &mut PvmInstance) -> bool {
    let host_gas_cost: u64 = 10;
    if pvm.gas() < host_gas_cost {
        pvm.set_gas(0);
        return false;
    }
    pvm.set_gas(pvm.gas() - host_gas_cost);

    match id {
        0 => {
            pvm.set_reg(7, pvm.gas());
            true
        }
        1 => refine_grow_heap(pvm),
        _ => {
            tracing::trace!(id, "unsupported host call in is-authorized context");
            pvm.set_reg(7, HOST_WHAT);
            true
        }
    }
}

/// Context for refine host calls — provides access to work item data.
struct RefineHostContext<'a> {
    item: &'a WorkItem,
    exported_segments: &'a mut Vec<Vec<u8>>,
    export_offset: u16,
    /// Resolved import segment data (populated by caller if available).
    import_data: &'a [Vec<u8>],
}

/// Handle host calls available during refinement (Ψ_R).
///
/// JAR v0.8.0 numbering: 0=gas, 1=grow_heap, 2=fetch, 3=(reserved),
/// 4=export, 5=machine, 6..=10=(reserved for peek/poke/pages/invoke/expunge).
///
/// Returns false if the PVM should stop (OOG or page fault).
fn handle_refine_host_call(
    id: u32,
    pvm: &mut PvmInstance,
    ctx: &mut RefineHostContext<'_>,
) -> bool {
    // Host-call gas cost: all host calls cost g=10 (charged upfront).
    let host_gas_cost: u64 = 10;
    if pvm.gas() < host_gas_cost {
        pvm.set_gas(0);
        return false;
    }
    pvm.set_gas(pvm.gas() - host_gas_cost);

    match id {
        0 => {
            // gas(): return remaining gas in φ[7].
            pvm.set_reg(7, pvm.gas());
            true
        }
        1 => {
            // grow_heap(): expand writable memory pages.
            // φ[7] = desired page count. Returns previous page count.
            refine_grow_heap(pvm)
        }
        2 => {
            // fetch(): read work-item context data (GP §14, Ω_Y for refine).
            // φ[7]=buf_ptr, φ[8]=offset, φ[9]=max_len, φ[10]=mode
            refine_fetch(pvm, ctx)
        }
        4 => {
            // export(): append a WG-byte segment to exports.
            // φ[7] = pointer to segment data in memory.
            // Returns: φ[7] = global export index, or HOST_OOB.
            refine_export(pvm, ctx)
        }
        5 => {
            // machine(): return machine/service info.
            // φ[7]=mode. Mode 0 = service_id, mode 1 = code_hash.
            refine_machine(pvm, ctx)
        }
        _ => {
            // Unimplemented: historical_lookup(3), peek(6), poke(7),
            // pages(8), invoke(9), expunge(10).
            tracing::trace!(id, "unimplemented refine host call");
            pvm.set_reg(7, HOST_WHAT);
            true
        }
    }
}

/// grow_heap for refine context (identical to accumulate grow_heap).
fn refine_grow_heap(pvm: &mut PvmInstance) -> bool {
    let desired = pvm.reg(7);
    let ps = javm::PVM_PAGE_SIZE;
    let current_pages = (pvm.heap_top() as u64).div_ceil(ps as u64);
    if desired <= current_pages || desired > (1u64 << 32) / ps as u64 {
        pvm.set_reg(7, current_pages);
        return true;
    }
    let new_pages = desired - current_pages;
    let extra_gas = new_pages * 10;
    if pvm.gas() < extra_gas {
        pvm.set_gas(0);
        return false;
    }
    pvm.set_gas(pvm.gas() - extra_gas);
    let old_top = pvm.heap_top();
    let new_top = (desired as u32) * ps;
    pvm.map_pages_rw(old_top / ps, desired as u32);
    pvm.set_heap_top(new_top);
    pvm.set_reg(7, current_pages);
    true
}

/// fetch for refine context (GP §14, Ω_Y adapted for refinement).
///
/// Available modes in refine context:
///   0 = protocol configuration blob
///   2 = work item payload (y)
///   3 = import segment at index φ[11]
///   4 = extrinsic data at index φ[11]
/// Other modes return NONE (u64::MAX).
fn refine_fetch(pvm: &mut PvmInstance, ctx: &RefineHostContext<'_>) -> bool {
    let buf_ptr = pvm.reg(7) as u32;
    let offset = pvm.reg(8);
    let max_len = pvm.reg(9);
    let mode = pvm.reg(10);
    let sub1 = pvm.reg(11) as usize;

    let data: Option<&[u8]> = match mode {
        2 => {
            // Payload (y)
            Some(&ctx.item.payload)
        }
        3 => {
            // Import segment data at index φ[11]
            ctx.import_data.get(sub1).map(|v| v.as_slice())
        }
        4 => {
            // Extrinsic: return the hash at index φ[11]
            ctx.item
                .extrinsics
                .get(sub1)
                .map(|(hash, _)| hash.0.as_slice())
        }
        _ => None,
    };

    let data = match data {
        Some(d) => d,
        None => {
            pvm.set_reg(7, u64::MAX); // NONE
            return true;
        }
    };

    let data_len = data.len() as u64;
    let f = offset.min(data_len);
    let l = max_len.min(data_len - f);

    if l > 0 {
        let src = &data[f as usize..(f + l) as usize];
        if pvm.try_write_bytes(buf_ptr, src).is_none() {
            return false; // page fault → PANIC
        }
    }

    pvm.set_reg(7, data_len);
    true
}

/// export for refine context: append a WG-byte segment to exports.
fn refine_export(pvm: &mut PvmInstance, ctx: &mut RefineHostContext<'_>) -> bool {
    let ptr = pvm.reg(7) as u32;
    let segment_size = grey_types::constants::SEGMENT_SIZE;
    match pvm.try_read_bytes(ptr, segment_size) {
        Some(data) => {
            let index = ctx.export_offset as u64 + ctx.exported_segments.len() as u64;
            ctx.exported_segments.push(data);
            pvm.set_reg(7, index);
        }
        None => {
            pvm.set_reg(7, HOST_OOB);
        }
    }
    true
}

/// machine for refine context: return machine/service info.
fn refine_machine(pvm: &mut PvmInstance, ctx: &RefineHostContext<'_>) -> bool {
    let mode = pvm.reg(7);
    match mode {
        0 => {
            // Service ID
            pvm.set_reg(7, ctx.item.service_id as u64);
        }
        1 => {
            // Code hash: write to buffer at φ[8], return 32
            let buf_ptr = pvm.reg(8) as u32;
            if pvm.try_write_bytes(buf_ptr, &ctx.item.code_hash.0).is_none() {
                return false;
            }
            pvm.set_reg(7, 32);
        }
        _ => {
            pvm.set_reg(7, HOST_WHAT);
        }
    }
    true
}

/// Simple work-package encoding for hashing and authorization.
fn encode_work_package_simple(pkg: &WorkPackage) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&pkg.auth_code_host.to_le_bytes());
    buf.extend_from_slice(&pkg.auth_code_hash.0);
    buf.extend_from_slice(&pkg.authorization);
    for item in &pkg.items {
        buf.extend_from_slice(&item.service_id.to_le_bytes());
        buf.extend_from_slice(&item.code_hash.0);
        buf.extend_from_slice(&item.gas_limit.to_le_bytes());
        buf.extend_from_slice(&item.payload);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub blob for structural tests (empty — PVM init will fail gracefully).
    /// PVM-based refine tests use the block trace vectors in stf_blocks instead.
    #[allow(dead_code)]
    fn make_echo_blob() -> Vec<u8> {
        vec![]
    }

    #[test]
    fn test_work_digest_fields() {
        // Verify WorkDigest construction
        let digest = WorkDigest {
            service_id: 42,
            code_hash: Hash::ZERO,
            payload_hash: Hash::ZERO,
            accumulate_gas: 1_000_000,
            result: WorkResult::Ok(vec![1, 2, 3]),
            gas_used: 500,
            imports_count: 0,
            extrinsics_count: 0,
            extrinsics_size: 0,
            exports_count: 0,
        };
        assert_eq!(digest.service_id, 42);
        assert_eq!(digest.gas_used, 500);
        match &digest.result {
            WorkResult::Ok(data) => assert_eq!(data, &[1, 2, 3]),
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn test_encode_work_package() {
        let pkg = WorkPackage {
            auth_code_host: 1,
            auth_code_hash: Hash::ZERO,
            context: RefinementContext {
                anchor: Hash::ZERO,
                state_root: Hash::ZERO,
                beefy_root: Hash::ZERO,
                lookup_anchor: Hash::ZERO,
                lookup_anchor_timeslot: 0,
                prerequisites: vec![],
            },
            authorization: vec![0xAB, 0xCD],
            authorizer_config: vec![],
            items: vec![WorkItem {
                service_id: 42,
                code_hash: Hash([1u8; 32]),
                gas_limit: 1000,
                accumulate_gas_limit: 500,
                exports_count: 0,
                payload: vec![10, 20, 30],
                imports: vec![],
                extrinsics: vec![],
            }],
        };

        let encoded = encode_work_package_simple(&pkg);
        assert!(!encoded.is_empty());
        // Verify deterministic
        let encoded2 = encode_work_package_simple(&pkg);
        assert_eq!(encoded, encoded2);
    }

    #[test]
    fn test_process_work_package_code_not_found() {
        let config = Config::tiny();
        let ctx = SimpleRefineContext {
            code_blobs: BTreeMap::new(),
            storage: BTreeMap::new(),
            preimages: BTreeMap::new(),
        };

        let pkg = WorkPackage {
            auth_code_host: 1,
            auth_code_hash: Hash([99u8; 32]),
            context: RefinementContext {
                anchor: Hash::ZERO,
                state_root: Hash::ZERO,
                beefy_root: Hash::ZERO,
                lookup_anchor: Hash::ZERO,
                lookup_anchor_timeslot: 0,
                prerequisites: vec![],
            },
            authorization: vec![],
            authorizer_config: vec![],
            items: vec![],
        };

        let result = process_work_package(&config, &pkg, &ctx, 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            RefineError::CodeNotFound(h) => assert_eq!(h.0, [99u8; 32]),
            other => panic!("expected CodeNotFound, got: {}", other),
        }
    }
}
