//! Accumulate sub-transition (Section 12 of the Gray Paper).
//!
//! Manages the work-report accumulation queue, dependency resolution,
//! and PVM execution of service Accumulate code (ΨA).

use crate::pvm_backend::{ExitReason, PvmInstance};
use grey_types::config::Config;
use grey_types::work::{WorkReport, WorkResult};
use grey_types::{Gas, Hash, ServiceId, Timeslot};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A queued work report with unfulfilled dependency hashes (eq 12.3).
#[derive(Clone, Debug)]
pub struct ReadyRecord {
    pub report: WorkReport,
    pub dependencies: Vec<Hash>,
}

/// Service account for the accumulate sub-transition.
/// Matches the test vector schema (distinct from the shared grey_types::state::ServiceAccount).
#[derive(Clone, Debug)]
pub struct AccServiceAccount {
    pub version: u8,
    pub code_hash: Hash,
    pub balance: u64,
    pub min_item_gas: Gas,
    pub min_memo_gas: Gas,
    pub bytes: u64,
    pub deposit_offset: u64,
    pub items: u64,
    pub creation_slot: Timeslot,
    pub last_accumulation_slot: Timeslot,
    pub parent_service: ServiceId,
    /// Storage dictionary (key -> value).
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Preimage lookup dictionary (hash -> data).
    pub preimage_lookup: BTreeMap<Hash, Vec<u8>>,
    /// Preimage info/requests ((hash, length) -> timeslots).
    pub preimage_info: BTreeMap<(Hash, u32), Vec<Timeslot>>,
}

/// Privileged service indices (eq 9.9), matching test vector format.
#[derive(Clone, Debug, Default)]
pub struct AccPrivileges {
    pub bless: ServiceId,
    pub assign: Vec<ServiceId>,
    pub designate: ServiceId,
    pub register: ServiceId,
    pub always_acc: Vec<(ServiceId, Gas)>,
}

/// Per-service accumulation statistics.
#[derive(Clone, Debug, Default)]
pub struct AccServiceStats {
    pub provided_count: u32,
    pub provided_size: u64,
    pub refinement_count: u32,
    pub refinement_gas_used: Gas,
    pub imports: u32,
    pub extrinsic_count: u32,
    pub extrinsic_size: u64,
    pub exports: u32,
    pub accumulate_count: u32,
    pub accumulate_gas_used: Gas,
}

/// Accumulate sub-transition state (isolated for testability).
#[derive(Clone, Debug)]
pub struct AccumulateState {
    pub slot: Timeslot,
    pub entropy: Hash,
    /// ω: Ready queue — E slots of queued (report, deps) records.
    pub ready_queue: Vec<Vec<ReadyRecord>>,
    /// ξ: Accumulated history — E slots of work-package hashes.
    pub accumulated: Vec<Vec<Hash>>,
    pub privileges: AccPrivileges,
    pub statistics: Vec<(ServiceId, AccServiceStats)>,
    pub accounts: BTreeMap<ServiceId, AccServiceAccount>,
}

/// Input to the accumulate sub-transition.
pub struct AccumulateInput {
    pub slot: Timeslot,
    pub reports: Vec<WorkReport>,
}

/// Output of the accumulate sub-transition.
#[derive(Debug)]
pub enum AccumulateOutput {
    Ok(Hash),
}

/// Deferred transfer between services (eq 12.16).
#[derive(Clone, Debug)]
pub struct DeferredTransfer {
    pub sender: ServiceId,
    pub destination: ServiceId,
    pub amount: u64,
    pub memo: Vec<u8>,
    pub gas_limit: Gas,
}

/// Output from single-service accumulation (Δ1).
#[derive(Clone, Debug)]
struct ServiceAccResult {
    accounts: BTreeMap<ServiceId, AccServiceAccount>,
    transfers: Vec<DeferredTransfer>,
    output: Option<Hash>,
    gas_used: Gas,
    privileges: AccPrivileges,
}

// ---------------------------------------------------------------------------
// Queue Management (eq 12.1-12.12)
// ---------------------------------------------------------------------------

/// Compute dependency set for a work report (eq 12.6).
/// D(r) = {prerequisites} ∪ K(segment_root_lookup)
fn compute_dependencies(report: &WorkReport) -> Vec<Hash> {
    let mut deps = BTreeSet::new();
    for prereq in &report.context.prerequisites {
        deps.insert(*prereq);
    }
    for (pkg_hash, _root) in &report.segment_root_lookup {
        deps.insert(*pkg_hash);
    }
    deps.into_iter().collect()
}

/// Partition reports into immediate (R!) and queued (RQ) (eq 12.4-12.5).
/// R! = reports with no prerequisites and no segment imports.
/// RQ = reports with dependencies.
fn partition_reports(reports: &[WorkReport]) -> (Vec<WorkReport>, Vec<ReadyRecord>) {
    let mut immediate = Vec::new();
    let mut queued = Vec::new();
    for r in reports {
        let deps = compute_dependencies(r);
        if deps.is_empty() {
            immediate.push(r.clone());
        } else {
            queued.push(ReadyRecord {
                report: r.clone(),
                dependencies: deps,
            });
        }
    }
    (immediate, queued)
}

/// Extract work-package hashes from reports (eq 12.9).
fn package_hashes(reports: &[WorkReport]) -> BTreeSet<Hash> {
    reports.iter().map(|r| r.package_spec.package_hash).collect()
}

/// Queue editing function E (eq 12.7).
/// Removes entries whose report package hash is in `accumulated_set`,
/// and removes fulfilled dependencies from remaining entries.
fn edit_queue(queue: &[ReadyRecord], accumulated_set: &BTreeSet<Hash>) -> Vec<ReadyRecord> {
    queue
        .iter()
        .filter(|rr| !accumulated_set.contains(&rr.report.package_spec.package_hash))
        .map(|rr| ReadyRecord {
            report: rr.report.clone(),
            dependencies: rr
                .dependencies
                .iter()
                .filter(|d| !accumulated_set.contains(d))
                .cloned()
                .collect(),
        })
        .collect()
}

/// Priority queue resolution Q (eq 12.8).
/// Recursively finds reports with zero remaining dependencies.
fn resolve_queue(queue: &[ReadyRecord]) -> Vec<WorkReport> {
    // Find reports with empty dependency set
    let ready: Vec<WorkReport> = queue
        .iter()
        .filter(|rr| rr.dependencies.is_empty())
        .map(|rr| rr.report.clone())
        .collect();

    if ready.is_empty() {
        return vec![];
    }

    // Remove ready reports and edit remaining
    let ready_hashes = package_hashes(&ready);
    let remaining = edit_queue(queue, &ready_hashes);

    // Recursively resolve
    let mut result = ready;
    result.extend(resolve_queue(&remaining));
    result
}

/// Compute R* with newly queued reports included (eq 12.10-12.12).
fn compute_accumulatable_with_new(
    immediate: &[WorkReport],
    ready_queue: &[Vec<ReadyRecord>],
    new_queued: &[ReadyRecord],
    epoch_length: usize,
    slot_index: usize,
) -> Vec<WorkReport> {
    let mut all_queued: Vec<ReadyRecord> = Vec::new();

    // Rotate: start from slot_index, wrap around
    for i in 0..epoch_length {
        let idx = (slot_index + i) % epoch_length;
        if idx < ready_queue.len() {
            all_queued.extend(ready_queue[idx].iter().cloned());
        }
    }

    // Add new queued reports
    all_queued.extend(new_queued.iter().cloned());

    // Edit queue with immediate report hashes
    let immediate_hashes = package_hashes(immediate);
    let edited = edit_queue(&all_queued, &immediate_hashes);

    let mut result = immediate.to_vec();
    result.extend(resolve_queue(&edited));
    result
}

// ---------------------------------------------------------------------------
// PVM Accumulation (ΨA, Appendix B.4)
// ---------------------------------------------------------------------------

/// Accumulation context L (eq B.7-B.8).
#[derive(Clone, Debug)]
struct AccContext {
    service_id: ServiceId,
    accounts: BTreeMap<ServiceId, AccServiceAccount>,
    next_service_id: ServiceId,
    transfers: Vec<DeferredTransfer>,
    output: Option<Hash>,
    preimage_provisions: Vec<(ServiceId, Vec<u8>)>,
    privileges: AccPrivileges,
}

/// Run PVM accumulation for a single service (Δ1, eq 12.24).
fn accumulate_single_service(
    accounts: &BTreeMap<ServiceId, AccServiceAccount>,
    transfers: &[DeferredTransfer],
    reports: &[WorkReport],
    privileges: &AccPrivileges,
    service_id: ServiceId,
    timeslot: Timeslot,
    entropy: &Hash,
    fetch_ctx: &FetchContext,
) -> ServiceAccResult {
    let account = match accounts.get(&service_id) {
        Some(a) => a,
        None => {
            return ServiceAccResult {
                accounts: accounts.clone(),
                transfers: vec![],
                output: None,
                gas_used: 0,
                privileges: privileges.clone(),
            };
        }
    };

    // Compute gas budget: free_gas + transfer_gas + operand_gas
    let free_gas: Gas = privileges
        .always_acc
        .iter()
        .find(|(s, _)| *s == service_id)
        .map(|(_, g)| *g)
        .unwrap_or(0);

    let transfer_gas: Gas = transfers
        .iter()
        .filter(|t| t.destination == service_id)
        .map(|t| t.gas_limit)
        .sum();

    let operand_gas: Gas = reports
        .iter()
        .flat_map(|r| r.results.iter())
        .filter(|d| d.service_id == service_id)
        .map(|d| d.accumulate_gas)
        .sum();

    let total_gas = free_gas
        .saturating_add(transfer_gas)
        .saturating_add(operand_gas);

    if total_gas == 0 && transfers.iter().all(|t| t.destination != service_id) {
        return ServiceAccResult {
            accounts: accounts.clone(),
            transfers: vec![],
            output: None,
            gas_used: 0,
            privileges: privileges.clone(),
        };
    }

    // Look up code blob from preimage_lookup using code_hash
    let code_blob = account.preimage_lookup.get(&account.code_hash).cloned();

    if code_blob.is_none() {
        // No code available: no-op
        return ServiceAccResult {
            accounts: accounts.clone(),
            transfers: vec![],
            output: None,
            gas_used: 0,
            privileges: privileges.clone(),
        };
    }
    let code_blob = code_blob.unwrap();

    // Initialize accumulation context (regular dimension x)
    // Credit incoming transfers to balance first (eq B.9)
    let mut initial_accounts = accounts.clone();
    let transfer_balance: u64 = transfers
        .iter()
        .filter(|t| t.destination == service_id)
        .map(|t| t.amount)
        .sum();
    if let Some(acc) = initial_accounts.get_mut(&service_id) {
        acc.balance = acc.balance.saturating_add(transfer_balance);
    }

    // Compute next available service ID (eq B.10)
    let max_existing = initial_accounts.keys().max().copied().unwrap_or(0);
    let hash_input = encode_new_service_hash(service_id, entropy, timeslot);
    let hash_bytes = grey_crypto::blake2b_256(&hash_input);
    let _hash_val = u32::from_le_bytes([hash_bytes.0[0], hash_bytes.0[1], hash_bytes.0[2], hash_bytes.0[3]]);
    // Simplified: next_service_id computation
    let next_service_id = max_existing.saturating_add(1).max(256);

    let regular = AccContext {
        service_id,
        accounts: initial_accounts.clone(),
        next_service_id,
        transfers: vec![],
        output: None,
        preimage_provisions: vec![],
        privileges: privileges.clone(),
    };
    let exceptional = regular.clone();

    // Count items for this service (transfers to + work digests for)
    let transfer_count = transfers.iter().filter(|t| t.destination == service_id).count();
    let work_count: usize = reports.iter().flat_map(|r| &r.results).filter(|d| d.service_id == service_id).count();
    let item_count = (transfer_count + work_count) as u32;

    // Encode minimal argument blob: varint(timeslot, service_id, item_count)
    let args = encode_accumulate_args(timeslot, service_id, item_count);

    // Build per-service fetch context with encoded items
    let items_blob = build_items_blob(transfers, service_id, reports);
    // Build individual items for fetch mode 15
    let mut individual_items: Vec<Vec<u8>> = Vec::new();
    for t in transfers.iter().filter(|t| t.destination == service_id) {
        let mut item = vec![1u8]; // transfer discriminator
        item.extend(encode_transfer(t));
        individual_items.push(item);
    }
    for report in reports {
        for digest in &report.results {
            if digest.service_id == service_id {
                let mut item = vec![0u8]; // operand discriminator
                item.extend(encode_operand(report, digest));
                individual_items.push(item);
            }
        }
    }

    let service_fetch_ctx = FetchContext {
        config_blob: fetch_ctx.config_blob.clone(),
        entropy: fetch_ctx.entropy,
        items_blob,
        items: individual_items,
    };

    // Run PVM
    let (final_context, gas_used) =
        run_accumulate_pvm(&code_blob, total_gas, &args, regular, exceptional, timeslot, entropy, &service_fetch_ctx);

    ServiceAccResult {
        accounts: final_context.accounts,
        transfers: final_context.transfers,
        output: final_context.output,
        gas_used,
        privileges: final_context.privileges,
    }
}

/// Encode arguments for ΨA invocation (Gray Paper eq B.9).
/// Format: varint(timeslot) ⌢ varint(service_id) ⌢ varint(item_count)
/// Items are accessed via fetch host call, NOT the argument blob.
fn encode_accumulate_args(
    timeslot: Timeslot,
    service_id: ServiceId,
    item_count: u32,
) -> Vec<u8> {
    let mut args = Vec::new();
    grey_codec::encode::encode_natural(timeslot as usize, &mut args);
    grey_codec::encode::encode_natural(service_id as usize, &mut args);
    grey_codec::encode::encode_natural(item_count as usize, &mut args);
    args
}

/// Encode a single work-item operand (type U, eq C.29).
/// EU(x) ≡ E(xp, xe, xa, xy, xg, O(xl), ↕xt)
fn encode_operand(
    report: &WorkReport,
    digest: &grey_types::work::WorkDigest,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&report.package_spec.package_hash.0); // p: 32 bytes
    buf.extend_from_slice(&report.package_spec.exports_root.0); // e: 32 bytes
    buf.extend_from_slice(&report.authorizer_hash.0);           // a: 32 bytes
    buf.extend_from_slice(&digest.payload_hash.0);              // y: 32 bytes
    grey_codec::encode::encode_natural(digest.accumulate_gas as usize, &mut buf); // g: varint
    // O(xl) - result encoding (eq C.34)
    match &digest.result {
        WorkResult::Ok(data) => {
            buf.push(0); // success discriminator
            grey_codec::encode::encode_natural(data.len(), &mut buf); // length prefix
            buf.extend_from_slice(data);
        }
        _ => {
            buf.push(2); // panic discriminator
        }
    }
    // ↕xt - length-prefixed authorizer trace
    grey_codec::encode::encode_natural(report.auth_output.len(), &mut buf);
    buf.extend_from_slice(&report.auth_output);
    buf
}

/// Encode a single deferred transfer (type X, eq C.31).
/// EX(x) ≡ E(E4(xs), E4(xd), E8(xa), xm, E8(xg))
fn encode_transfer(t: &DeferredTransfer) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&t.sender.to_le_bytes());      // E4(sender)
    buf.extend_from_slice(&t.destination.to_le_bytes());  // E4(dest)
    buf.extend_from_slice(&t.amount.to_le_bytes());       // E8(amount)
    // Memo: fixed 128 bytes (padded with zeros)
    let mut memo = [0u8; 128];
    let copy_len = t.memo.len().min(128);
    memo[..copy_len].copy_from_slice(&t.memo[..copy_len]);
    buf.extend_from_slice(&memo);                          // memo: 128 bytes
    buf.extend_from_slice(&t.gas_limit.to_le_bytes());    // E8(gas_limit)
    buf
}

/// Build encoded items list for fetch (eq C.33).
/// Items are discriminated: 0x00 + EU(operand) or 0x01 + EX(transfer).
/// Order: transfers first (iT), then operands (iU).
fn build_items_blob(
    transfers: &[DeferredTransfer],
    service_id: ServiceId,
    reports: &[WorkReport],
) -> Vec<u8> {
    let mut items: Vec<Vec<u8>> = Vec::new();
    // iT: transfers to this service
    for t in transfers.iter().filter(|t| t.destination == service_id) {
        let mut item = vec![1u8]; // transfer discriminator
        item.extend(encode_transfer(t));
        items.push(item);
    }
    // iU: work-item operands for this service
    for report in reports {
        for digest in &report.results {
            if digest.service_id == service_id {
                let mut item = vec![0u8]; // operand discriminator
                item.extend(encode_operand(report, digest));
                items.push(item);
            }
        }
    }
    // Encode as length-prefixed sequence: varint(count) + item_0 + item_1 + ...
    let mut blob = Vec::new();
    grey_codec::encode::encode_natural(items.len(), &mut blob);
    for item in &items {
        blob.extend(item);
    }
    blob
}

/// Encode hash input for new service ID computation.
fn encode_new_service_hash(service_id: ServiceId, entropy: &Hash, timeslot: Timeslot) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&service_id.to_le_bytes());
    buf.extend_from_slice(&entropy.0);
    buf.extend_from_slice(&timeslot.to_le_bytes());
    buf
}

/// Data available to the fetch host call during accumulation.
struct FetchContext {
    /// Protocol configuration blob (mode 0).
    config_blob: Vec<u8>,
    /// Entropy hash η'_0 (mode 1).
    entropy: Hash,
    /// Encoded items blob for modes 14/15.
    items_blob: Vec<u8>,
    /// Individual encoded items (discriminated).
    items: Vec<Vec<u8>>,
}

/// Run PVM accumulation with host-call loop.
fn run_accumulate_pvm(
    code_blob: &[u8],
    gas: Gas,
    args: &[u8],
    mut regular: AccContext,
    mut exceptional: AccContext,
    timeslot: Timeslot,
    entropy: &Hash,
    fetch_ctx: &FetchContext,
) -> (AccContext, Gas) {
    // Initialize PVM
    let mut pvm = match PvmInstance::initialize(code_blob, args, gas) {
        Some(p) => p,
        None => return (exceptional, 0),
    };

    // Set entry point: ΨM(c, 5, ...) starts at instruction counter 5 for accumulate
    pvm.set_pc(5);

    let initial_gas = pvm.gas();

    loop {
        let exit_reason = pvm.run();
        match exit_reason {
            ExitReason::Halt => {
                let gas_used = initial_gas - pvm.gas();
                return (regular, gas_used);
            }
            ExitReason::Panic | ExitReason::OutOfGas | ExitReason::PageFault(_) => {
                let gas_used = initial_gas - pvm.gas();
                return (exceptional, gas_used);
            }
            ExitReason::HostCall(id) => {
                let ok = handle_host_call(
                    id,
                    &mut pvm,
                    &mut regular,
                    &mut exceptional,
                    timeslot,
                    entropy,
                    fetch_ctx,
                );
                if !ok {
                    let gas_used = initial_gas - pvm.gas();
                    return (exceptional, gas_used);
                }
            }
        }
    }
}

/// Handle a host call from the PVM during accumulation.
/// Returns true to continue, false to abort.
fn handle_host_call(
    id: u32,
    pvm: &mut PvmInstance,
    regular: &mut AccContext,
    exceptional: &mut AccContext,
    timeslot: Timeslot,
    _entropy: &Hash,
    fetch_ctx: &FetchContext,
) -> bool {
    // Host-call gas cost (GP Appendix B, eq B.15): ϱ′ ≡ ϱ − g
    //
    // Per the GP spec (A.36), when the PVM exits on ecalli the machine state
    // is "prior to this instruction" — ecalli's instruction gas (1) is NOT
    // applied.  The host-call gas g covers the full cost.
    //
    // With basic-block gas metering, ecalli's cost (1) is already charged as
    // part of the block.  We therefore refund 1 gas so the net charge matches
    // the spec: total = g (not 1 + g).
    let host_gas_cost: u64 = match id {
        100 => 0,  // log: g=0 (JIP-1, per accumulate test vector README)
        20 => 10,  // transfer: base cost 10, gas_limit charged on success only
        _ => 10,
    };

    // Refund ecalli instruction cost (1) that was included in the block gas
    // charge, since the spec says the exit state is prior to ecalli.
    pvm.set_gas(pvm.gas() + 1);

    if pvm.gas() < host_gas_cost {
        return false;
    }
    pvm.set_gas(pvm.gas() - host_gas_cost);

    match id {
        0 => host_gas(pvm, regular),
        1 => host_fetch(pvm, fetch_ctx),
        3 => host_read(pvm, regular),
        4 => host_write(pvm, regular),
        5 => host_info(pvm, regular),
        17 => host_checkpoint(pvm, regular, exceptional),
        20 => host_transfer(pvm, regular),
        21 => host_eject(pvm, regular, timeslot),
        25 => host_yield(pvm, regular),
        100 => {
            // log host call (JIP-1): always returns WHAT in φ'7
            pvm.set_reg(7, u64::MAX);
            true
        }
        _ => {
            // Unknown host call: set WHAT in register 7
            pvm.set_reg(7, u64::MAX);
            true
        }
    }
}

/// gas (id=0): Return remaining gas in φ[7].
fn host_gas(pvm: &mut PvmInstance, _ctx: &mut AccContext) -> bool {
    pvm.set_reg(7, pvm.gas());
    true
}

/// fetch (id=1): Read protocol/context data (ΩY).
/// φ[7]=buffer_ptr, φ[8]=offset, φ[9]=max_len, φ[10]=mode, φ[11]=sub1, φ[12]=sub2
/// Returns: φ'[7] = |v| (total data length) or NONE (u64::MAX).
fn host_fetch(pvm: &mut PvmInstance, fetch_ctx: &FetchContext) -> bool {
    let buf_ptr = pvm.reg(7) as u32;
    let offset = pvm.reg(8);
    let max_len = pvm.reg(9);
    let mode = pvm.reg(10);
    let sub1 = pvm.reg(11) as usize;

    // Select data based on mode (accumulate context: modes 0, 1, 14, 15)
    let owned_data: Option<Vec<u8>>;
    let data: Option<&[u8]> = match mode {
        0 => Some(&fetch_ctx.config_blob),        // Protocol configuration
        1 => Some(&fetch_ctx.entropy.0),           // Entropy η'_0
        14 => Some(&fetch_ctx.items_blob),         // All items encoded
        15 => {                                     // Single item at index φ[11]
            if sub1 < fetch_ctx.items.len() {
                owned_data = Some(fetch_ctx.items[sub1].clone());
                owned_data.as_deref()
            } else {
                None
            }
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

    // Write data[f..f+l] to memory at buf_ptr
    if l > 0 {
        let src = &data[f as usize..(f + l) as usize];
        pvm.write_bytes(buf_ptr, src);
    }

    // Return total length of the data
    pvm.set_reg(7, data_len);
    true
}

/// read (id=3): Read from service storage.
/// φ[7] = service_id (or if ≥ 2^32, defaults to current service s),
/// φ[8] = key_ptr, φ[9] = key_len,
/// φ[10] = output_ptr, φ[11] = output_max_len
/// Returns: φ[7] = value_len or NONE
fn host_read(pvm: &mut PvmInstance, ctx: &mut AccContext) -> bool {
    // GP eq B.14: s* = s if φ₇ = NONE, else φ₇
    let service_id = if pvm.reg(7) == u64::MAX {
        ctx.service_id
    } else if pvm.reg(7) <= u32::MAX as u64 {
        pvm.reg(7) as ServiceId
    } else {
        pvm.set_reg(7, u64::MAX); // NONE
        return true;
    };
    let key_ptr = pvm.reg(8) as u32;
    let key_len = pvm.reg(9) as u32;
    let out_ptr = pvm.reg(10) as u32;
    let offset = pvm.reg(11);
    let max_len = pvm.reg(12);

    let key = pvm.read_bytes(key_ptr, key_len);

    if let Some(account) = ctx.accounts.get(&service_id) {
        if let Some(value) = account.storage.get(&key) {
            let v_len = value.len() as u64;
            let f = offset.min(v_len) as usize;
            let l = max_len.min(v_len - f as u64) as usize;
            if l > 0 {
                pvm.write_bytes(out_ptr, &value[f..f + l]);
            }
            pvm.set_reg(7, v_len);
        } else {
            pvm.set_reg(7, u64::MAX); // NONE
        }
    } else {
        pvm.set_reg(7, u64::MAX); // NONE
    }

    true
}

/// write (id=4): Write to current service's storage.
/// φ[7] = key_ptr, φ[8] = key_len, φ[9] = value_ptr, φ[10] = value_len
/// Returns: φ[7] = OK(0) or error
fn host_write(pvm: &mut PvmInstance, ctx: &mut AccContext) -> bool {
    const FULL: u64 = u64::MAX - 4;

    let key_ptr = pvm.reg(7) as u32;
    let key_len = pvm.reg(8) as u32;
    let value_ptr = pvm.reg(9) as u32;
    let value_len = pvm.reg(10) as u32;

    let key = pvm.read_bytes(key_ptr, key_len);
    let value = pvm.read_bytes(value_ptr, value_len);

    if let Some(account) = ctx.accounts.get_mut(&ctx.service_id) {
        let old_len: u64 = account
            .storage
            .get(&key)
            .map(|v| v.len() as u64)
            .unwrap_or(u64::MAX);

        let old_size: u64 = account
            .storage
            .get(&key)
            .map(|v| (34 + key.len() + v.len()) as u64)
            .unwrap_or(0);

        let new_bytes;
        let new_items;
        if value_len == 0 {
            if account.storage.contains_key(&key) {
                new_bytes = account.bytes.saturating_sub(old_size);
                new_items = account.items.saturating_sub(1);
            } else {
                new_bytes = account.bytes;
                new_items = account.items;
            }
        } else {
            let new_size = (34 + key.len() + value.len()) as u64;
            let was_new = !account.storage.contains_key(&key);
            new_bytes = account.bytes.saturating_sub(old_size).saturating_add(new_size);
            new_items = if was_new { account.items + 1 } else { account.items };
        }

        let threshold = new_items as u64 * grey_types::constants::BALANCE_PER_ITEM
            + new_bytes * grey_types::constants::BALANCE_PER_OCTET;
        if threshold > account.balance {
            pvm.set_reg(7, FULL);
            return true;
        }

        if value_len == 0 {
            if account.storage.remove(&key).is_some() {
                account.bytes = new_bytes;
                account.items = new_items;
            }
        } else {
            account.storage.insert(key, value);
            account.bytes = new_bytes;
            account.items = new_items;
        }
        pvm.set_reg(7, old_len);
    } else {
        pvm.set_reg(7, u64::MAX);
    }

    true
}

/// info (id=5): Get service account info.
/// φ[7] = service_id (or if ≥ 2^32, defaults to current service s)
/// φ[8] = output_ptr
/// Returns φ[7] = OK(0) or error, writes info to memory
fn host_info(pvm: &mut PvmInstance, ctx: &mut AccContext) -> bool {
    let service_id = if pvm.reg(7) == u64::MAX {
        ctx.service_id
    } else if pvm.reg(7) <= u32::MAX as u64 {
        pvm.reg(7) as ServiceId
    } else {
        pvm.set_reg(7, u64::MAX);
        return true;
    };
    let out_ptr = pvm.reg(8) as u32;

    if let Some(account) = ctx.accounts.get(&service_id) {
        // code_hash (32 bytes) + balance (8) + min_item_gas (8) + min_memo_gas (8)
        let mut buf = [0u8; 56];
        buf[..32].copy_from_slice(&account.code_hash.0);
        buf[32..40].copy_from_slice(&account.balance.to_le_bytes());
        buf[40..48].copy_from_slice(&account.min_item_gas.to_le_bytes());
        buf[48..56].copy_from_slice(&account.min_memo_gas.to_le_bytes());
        pvm.write_bytes(out_ptr, &buf);
        pvm.set_reg(7, 0); // OK
    } else {
        pvm.set_reg(7, u64::MAX); // NONE
    }

    true
}

/// checkpoint (id=17): Save rollback point. y ← x.
fn host_checkpoint(
    pvm: &mut PvmInstance,
    regular: &mut AccContext,
    exceptional: &mut AccContext,
) -> bool {
    *exceptional = regular.clone();
    pvm.set_reg(7, pvm.gas());
    true
}

/// transfer (id=20): Queue a deferred balance transfer (GP eq B.19-B.20).
/// φ[7] = dest, φ[8] = amount, φ[9] = gas_limit, φ[10] = memo_ptr
/// Memo is always exactly W_T (128) bytes read from memory at φ[10].
/// Returns: OK, WHO (dest unknown), LOW (gas < min), CASH (insufficient balance)
fn host_transfer(pvm: &mut PvmInstance, ctx: &mut AccContext) -> bool {
    const MEMO_SIZE: u32 = 128; // W_T
    const WHO: u64 = u64::MAX - 3;
    const LOW: u64 = u64::MAX - 7;
    const CASH: u64 = u64::MAX - 6;

    let dest = pvm.reg(7) as ServiceId;
    let amount = pvm.reg(8);
    let gas_limit = pvm.reg(9);
    let memo_ptr = pvm.reg(10) as u32;

    let memo = pvm.read_bytes(memo_ptr, MEMO_SIZE);

    if !ctx.accounts.contains_key(&dest) {
        pvm.set_reg(7, WHO);
        return true;
    }

    if let Some(dest_acc) = ctx.accounts.get(&dest) {
        if gas_limit < dest_acc.min_memo_gas {
            pvm.set_reg(7, LOW);
            return true;
        }
    }

    if let Some(account) = ctx.accounts.get(&ctx.service_id) {
        if account.balance < amount {
            pvm.set_reg(7, CASH);
            return true;
        }
    }

    if pvm.gas() < gas_limit {
        pvm.set_gas(0);
        return false;
    }
    pvm.set_gas(pvm.gas() - gas_limit);

    if let Some(account) = ctx.accounts.get_mut(&ctx.service_id) {
        account.balance -= amount;
    }

    ctx.transfers.push(DeferredTransfer {
        sender: ctx.service_id,
        destination: dest,
        amount,
        memo,
        gas_limit,
    });

    pvm.set_reg(7, 0); // OK
    true
}

/// eject (id=21): Eject a service (GP eq ΩJ).
/// φ[7] = target service to eject (d), φ[8] = hash_ptr (o)
/// On success: removes target, transfers its balance to caller.
fn host_eject(pvm: &mut PvmInstance, ctx: &mut AccContext, _timeslot: Timeslot) -> bool {
    const WHO: u64 = u64::MAX - 3;

    let target = pvm.reg(7) as ServiceId;

    if target == ctx.service_id {
        pvm.set_reg(7, WHO);
        return true;
    }

    if let Some(ejected) = ctx.accounts.remove(&target) {
        if let Some(self_acc) = ctx.accounts.get_mut(&ctx.service_id) {
            self_acc.balance = self_acc.balance.saturating_add(ejected.balance);
        }
        pvm.set_reg(7, 0); // OK
    } else {
        pvm.set_reg(7, WHO);
    }

    true
}

/// yield (id=25): Set accumulation output hash.
/// φ[7] = hash_ptr (pointer to 32-byte hash in memory)
fn host_yield(pvm: &mut PvmInstance, ctx: &mut AccContext) -> bool {
    let hash_ptr = pvm.reg(7) as u32;

    let data = pvm.read_bytes(hash_ptr, 32);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&data);

    ctx.output = Some(Hash(hash));
    pvm.set_reg(7, 0); // OK
    true
}

// ---------------------------------------------------------------------------
// Accumulation Pipeline (Δ+, Δ*, Δ1)
// ---------------------------------------------------------------------------

/// Batch accumulation Δ* (eq 12.19).
/// All reports in the batch are processed together — each involved service
/// receives ALL items from ALL reports in a single PVM invocation.
fn accumulate_batch(
    accounts: &BTreeMap<ServiceId, AccServiceAccount>,
    transfers: &[DeferredTransfer],
    reports: &[WorkReport],
    privileges: &AccPrivileges,
    timeslot: Timeslot,
    entropy: &Hash,
    fetch_ctx: &FetchContext,
) -> (
    BTreeMap<ServiceId, AccServiceAccount>,
    Vec<DeferredTransfer>,
    Vec<(ServiceId, Hash)>,
    Vec<(ServiceId, Gas)>,
    AccPrivileges,
) {
    // Collect all involved service IDs across all reports
    let mut involved = BTreeSet::new();
    for report in reports {
        for digest in &report.results {
            involved.insert(digest.service_id);
        }
    }
    for (sid, _) in &privileges.always_acc {
        involved.insert(*sid);
    }
    for t in transfers {
        involved.insert(t.destination);
    }

    let mut current_accounts = accounts.clone();
    let mut all_transfers = Vec::new();
    let mut outputs = Vec::new();
    let mut gas_usage = Vec::new();
    let mut current_privileges = privileges.clone();

    for &sid in &involved {
        let result = accumulate_single_service(
            &current_accounts,
            transfers,
            reports,
            &current_privileges,
            sid,
            timeslot,
            entropy,
            fetch_ctx,
        );

        current_accounts = result.accounts;
        all_transfers.extend(result.transfers);
        gas_usage.push((sid, result.gas_used));
        current_privileges = result.privileges;

        if let Some(output) = result.output {
            outputs.push((sid, output));
        }
    }

    (
        current_accounts,
        all_transfers,
        outputs,
        gas_usage,
        current_privileges,
    )
}

/// Outer accumulation Δ+ (eq 12.18).
fn accumulate_all(
    gas_budget: Gas,
    transfers: Vec<DeferredTransfer>,
    reports: &[WorkReport],
    accounts: &BTreeMap<ServiceId, AccServiceAccount>,
    privileges: &AccPrivileges,
    timeslot: Timeslot,
    entropy: &Hash,
    fetch_ctx: &FetchContext,
) -> (
    usize,
    BTreeMap<ServiceId, AccServiceAccount>,
    Vec<(ServiceId, Hash)>,
    Vec<(ServiceId, Gas)>,
    AccPrivileges,
) {
    if reports.is_empty() {
        return (0, accounts.clone(), vec![], vec![], privileges.clone());
    }

    // Find max reports that fit in gas budget
    let mut gas_sum: Gas = 0;
    let mut max_reports = 0;
    for report in reports {
        let report_gas: Gas = report.results.iter().map(|d| d.accumulate_gas).sum();
        if gas_sum.saturating_add(report_gas) > gas_budget {
            break;
        }
        gas_sum = gas_sum.saturating_add(report_gas);
        max_reports += 1;
    }

    if max_reports == 0 {
        return (0, accounts.clone(), vec![], vec![], privileges.clone());
    }

    // Process this batch
    let batch_reports = &reports[..max_reports];
    let (new_accounts, new_transfers, outputs, gas_usage, new_privileges) =
        accumulate_batch(accounts, &transfers, batch_reports, privileges, timeslot, entropy, fetch_ctx);

    let batch_gas_used: Gas = gas_usage.iter().map(|(_, g)| *g).sum();
    let remaining_gas = gas_budget.saturating_sub(batch_gas_used);

    // Process remaining reports recursively
    if max_reports < reports.len() {
        let (more_count, final_accounts, more_outputs, more_gas, final_privileges) =
            accumulate_all(
                remaining_gas,
                new_transfers,
                &reports[max_reports..],
                &new_accounts,
                &new_privileges,
                timeslot,
                entropy,
                fetch_ctx,
            );

        let mut all_outputs = outputs;
        all_outputs.extend(more_outputs);
        let mut all_gas = gas_usage;
        all_gas.extend(more_gas);

        (
            max_reports + more_count,
            final_accounts,
            all_outputs,
            all_gas,
            final_privileges,
        )
    } else {
        (max_reports, new_accounts, outputs, gas_usage, new_privileges)
    }
}

// ---------------------------------------------------------------------------
// Top-Level Processing Function
// ---------------------------------------------------------------------------

/// Process the accumulate sub-transition.
pub fn process_accumulate(
    config: &Config,
    state: &mut AccumulateState,
    input: &AccumulateInput,
) -> AccumulateOutput {
    let epoch_length = config.epoch_length as usize;
    let slot_index = input.slot as usize % epoch_length;

    // Step 1: Partition input reports into immediate and queued
    let (immediate, new_queued) = partition_reports(&input.reports);
    // Step 2: Compute R* (all accumulatable reports)
    let accumulatable = compute_accumulatable_with_new(
        &immediate,
        &state.ready_queue,
        &new_queued,
        epoch_length,
        slot_index,
    );

    // Step 3: Compute gas budget (eq 12.25)
    let always_gas: Gas = state.privileges.always_acc.iter().map(|(_, g)| *g).sum();
    let gas_budget = (config.gas_total_accumulation + always_gas)
        .max(config.gas_total_accumulation);

    // Build shared fetch context (items are per-service, built in accumulate_single_service)
    let fetch_ctx = FetchContext {
        config_blob: config.encode_config_blob(),
        entropy: state.entropy,
        items_blob: vec![],
        items: vec![],
    };

    // Step 4: Run accumulation pipeline (Δ+)
    let (n, new_accounts, outputs, gas_usage, new_privileges) = accumulate_all(
        gas_budget,
        vec![],
        &accumulatable,
        &state.accounts,
        &state.privileges,
        input.slot,
        &state.entropy,
        &fetch_ctx,
    );

    // Step 5: Update service accounts
    state.accounts = new_accounts;

    // Step 6: Update last_accumulation_slot for all accumulated services
    for (sid, _) in &gas_usage {
        if let Some(account) = state.accounts.get_mut(sid) {
            account.last_accumulation_slot = input.slot;
        }
    }

    // Step 7: Update statistics
    update_statistics(&mut state.statistics, &gas_usage, &accumulatable, n);

    // Step 8: Update accumulated history (eq 12.32)
    // Shift: drop oldest, add new slot at end
    shift_accumulated(
        &mut state.accumulated,
        &accumulatable,
        n,
        epoch_length,
    );

    // Step 9: Update ready queue (eq 12.34)
    let accumulated_hashes: BTreeSet<Hash> = state
        .accumulated
        .last()
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();

    update_ready_queue(
        &mut state.ready_queue,
        &new_queued,
        &accumulated_hashes,
        epoch_length,
        state.slot,
        input.slot,
    );

    // Step 10: Update privileges
    state.privileges = new_privileges;

    // Step 11: Update slot
    state.slot = input.slot;

    // Step 12: Compute output hash (Keccak Merkle root of outputs)
    let output_hash = compute_output_hash(&outputs);
    AccumulateOutput::Ok(output_hash)
}

/// Shift accumulated history (eq 12.32).
/// Always shifts left by 1, dropping the oldest entry and recording new hashes at [E-1].
fn shift_accumulated(
    accumulated: &mut Vec<Vec<Hash>>,
    accumulatable: &[WorkReport],
    n: usize,
    epoch_length: usize,
) {
    // Shift left by 1
    if !accumulated.is_empty() {
        accumulated.remove(0);
    }
    accumulated.push(vec![]);

    // Ensure correct length
    while accumulated.len() < epoch_length {
        accumulated.push(vec![]);
    }

    // Record accumulated package hashes in the last slot (sorted)
    let last_idx = epoch_length - 1;
    let mut hashes: Vec<Hash> = accumulatable[..n]
        .iter()
        .map(|r| r.package_spec.package_hash)
        .collect();
    hashes.sort();
    accumulated[last_idx] = hashes;
}

/// Update ready queue after accumulation (eq 12.34).
/// The ready queue is a circular buffer indexed by slot % E.
/// All positions for skipped+current slots are cleared.
/// Position m (current slot) receives new queued entries.
/// Other surviving positions are edited to remove fulfilled dependencies.
fn update_ready_queue(
    ready_queue: &mut Vec<Vec<ReadyRecord>>,
    new_queued: &[ReadyRecord],
    accumulated_hashes: &BTreeSet<Hash>,
    epoch_length: usize,
    prev_slot: Timeslot,
    current_slot: Timeslot,
) {
    // Ensure correct length
    while ready_queue.len() < epoch_length {
        ready_queue.push(vec![]);
    }

    // Clear positions for all slots from prev_slot+1 to current_slot
    let slots_advanced = if current_slot > prev_slot {
        (current_slot - prev_slot) as usize
    } else {
        1
    };

    for offset in 0..slots_advanced.min(epoch_length) {
        let slot = prev_slot as usize + 1 + offset;
        let pos = slot % epoch_length;
        ready_queue[pos] = vec![];
    }

    // Edit surviving slots: remove fulfilled dependencies and accumulated reports
    for slot in ready_queue.iter_mut() {
        *slot = edit_queue(slot, accumulated_hashes);
    }

    // Insert newly queued reports at current position m
    let m = current_slot as usize % epoch_length;
    let edited_new = edit_queue(new_queued, accumulated_hashes);
    ready_queue[m].extend(edited_new);
}

/// Update per-service statistics.
fn update_statistics(
    stats: &mut Vec<(ServiceId, AccServiceStats)>,
    gas_usage: &[(ServiceId, Gas)],
    accumulatable: &[WorkReport],
    n: usize,
) {
    // Collect refinement statistics from reports
    let reports = &accumulatable[..n];
    let mut stat_map: BTreeMap<ServiceId, AccServiceStats> = BTreeMap::new();

    for report in reports {
        for digest in &report.results {
            let entry = stat_map.entry(digest.service_id).or_default();
            entry.refinement_count += 1;
            entry.refinement_gas_used += digest.gas_used;
            entry.imports += digest.imports_count as u32;
            entry.extrinsic_count += digest.extrinsics_count as u32;
            entry.extrinsic_size += digest.extrinsics_size as u64;
            entry.exports += digest.exports_count as u32;
        }
    }

    // Add accumulation gas usage.
    // accumulate_count = number of REPORTS that involve each service (not PVM invocations).
    for (sid, gas) in gas_usage {
        let entry = stat_map.entry(*sid).or_default();
        // Count how many reports have results for this service
        let report_count = reports
            .iter()
            .filter(|r| r.results.iter().any(|d| d.service_id == *sid))
            .count() as u32;
        entry.accumulate_count += report_count.max(1); // at least 1 if accumulated
        entry.accumulate_gas_used += *gas;
    }

    *stats = stat_map.into_iter().collect();
}

/// Compute the accumulate output hash (M_K over per-service yields, eq 12.17).
///
/// Each service that calls yield produces a (service_id, output_hash) pair.
/// The output commitment is the balanced Keccak-256 Merkle root (M_K) over the
/// list of encoded pairs `E4(service_id) ⌢ output_hash`, sorted by service_id.
fn compute_output_hash(outputs: &[(ServiceId, Hash)]) -> Hash {
    if outputs.is_empty() {
        return Hash([0u8; 32]);
    }
    // Encode each (service_id, yield_hash) pair as 36 bytes
    let mut leaves: Vec<Vec<u8>> = outputs
        .iter()
        .map(|(sid, hash)| {
            let mut leaf = Vec::with_capacity(36);
            leaf.extend_from_slice(&sid.to_le_bytes());
            leaf.extend_from_slice(&hash.0);
            leaf
        })
        .collect();
    // Sort by service_id (already encoded LE, so sorting by first 4 bytes = sorting by sid)
    leaves.sort();
    // Balanced Keccak-256 Merkle tree M_K (eq E.4)
    keccak_merkle_root(leaves)
}

/// Balanced Keccak-256 Merkle tree M_K(L) (eq E.4).
fn keccak_merkle_root(leaves: Vec<Vec<u8>>) -> Hash {
    match leaves.len() {
        0 => Hash([0u8; 32]),
        1 => grey_crypto::keccak_256(&leaves[0]),
        n => {
            let mid = (n + 1) / 2;
            let left = keccak_merkle_root(leaves[..mid].to_vec());
            let right = keccak_merkle_root(leaves[mid..].to_vec());
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&left.0);
            combined[32..].copy_from_slice(&right.0);
            grey_crypto::keccak_256(&combined)
        }
    }
}
