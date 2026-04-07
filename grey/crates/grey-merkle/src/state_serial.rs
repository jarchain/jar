//! State serialization T(σ) — Gray Paper eq D.2.
//!
//! Converts between the State struct and a flat mapping of 31-byte keys to
//! variable-length byte values, suitable for Merklization via the binary
//! Patricia Merkle trie.

// Fixed-width encoding helpers (no compact encoding)
fn encode_u32_le(val: u32, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&val.to_le_bytes());
}

fn decode_u32_le_at(data: &[u8], pos: &mut usize) -> Result<u32, ()> {
    if *pos + 4 > data.len() {
        return Err(());
    }
    let val = u32::from_le_bytes(data[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(val)
}

use grey_crypto::blake2b_256;
use grey_types::config::Config;
use grey_types::state::{
    Judgments, PrivilegedServices, RecentBlocks, SafroleState, SealKeySeries, ServiceAccount,
    State, ValidatorStatistics,
};
use grey_types::{Hash, ServiceId};
use std::collections::BTreeMap;

use crate::{state_key_for_service, state_key_from_index};

/// Extract the accumulation root (beefy_root) for a given anchor block from the
/// raw C(3) (recent_blocks) state KV blob. Scans the serialized header entries to
/// find the one matching `anchor_hash` and returns its `accumulation_root` field.
pub fn extract_accumulation_root(
    recent_blocks_blob: &[u8],
    anchor_hash: &Hash,
) -> Result<Option<Hash>, String> {
    let mut pos = 0;
    let header_count = decode_u32_le_at(recent_blocks_blob, &mut pos)
        .map_err(|_| "decode error".to_string())? as usize;

    for _ in 0..header_count {
        // Each header: header_hash(32) + accumulation_root(32) + state_root(32) + packages...
        if pos + 96 > recent_blocks_blob.len() {
            return Err("unexpected end in recent_blocks headers".into());
        }
        let mut header_hash = [0u8; 32];
        header_hash.copy_from_slice(&recent_blocks_blob[pos..pos + 32]);
        let mut acc_root = [0u8; 32];
        acc_root.copy_from_slice(&recent_blocks_blob[pos + 32..pos + 64]);
        // Skip state_root
        pos += 96;

        // Skip reported_packages map
        let pkg_count = decode_u32_le_at(recent_blocks_blob, &mut pos)
            .map_err(|_| "decode error".to_string())? as usize;
        // Each package entry is 64 bytes (two hashes)
        let skip = pkg_count * 64;
        if pos + skip > recent_blocks_blob.len() {
            return Err("unexpected end in reported_packages".into());
        }
        pos += skip;

        if header_hash == anchor_hash.0 {
            return Ok(Some(Hash(acc_root)));
        }
    }
    Ok(None)
}

/// Construct state key C(s, h) where h is an arbitrary byte sequence.
/// The key interleaves E_4(s) and H(h).
fn key_for_service_data(service_id: ServiceId, h: &[u8]) -> [u8; 31] {
    crate::interleave_service_key(service_id, &blake2b_256(h))
}

/// Construct the h argument for storage entries: E_4(2^32-1) ++ k
fn storage_hash_arg(storage_key: &[u8]) -> Vec<u8> {
    let mut h = Vec::with_capacity(4 + storage_key.len());
    h.extend_from_slice(&u32::MAX.to_le_bytes());
    h.extend_from_slice(storage_key);
    h
}

/// Construct the h argument for preimage lookup entries: E_4(2^32-2) ++ hash
fn preimage_hash_arg(hash: &Hash) -> Vec<u8> {
    let mut h = Vec::with_capacity(4 + 32);
    h.extend_from_slice(&(u32::MAX - 1).to_le_bytes());
    h.extend_from_slice(&hash.0);
    h
}

/// Construct the h argument for preimage info entries: E_4(l) ++ hash
fn preimage_info_hash_arg(length: u32, hash: &Hash) -> Vec<u8> {
    let mut h = Vec::with_capacity(4 + 32);
    h.extend_from_slice(&length.to_le_bytes());
    h.extend_from_slice(&hash.0);
    h
}

/// Extract service_id from an opaque service data key C(s, h).
/// The service_id bytes are interleaved at positions 0, 2, 4, 6.
pub fn extract_service_id_from_data_key(key: &[u8; 31]) -> ServiceId {
    u32::from_le_bytes([key[0], key[2], key[4], key[6]])
}

/// Compute the state key for a storage entry: C(s, E_4(2^32-1) ++ k).
pub fn compute_storage_state_key(service_id: ServiceId, storage_key: &[u8]) -> [u8; 31] {
    key_for_service_data(service_id, &storage_hash_arg(storage_key))
}

/// Compute the state key for a preimage lookup entry: C(s, E_4(2^32-2) ++ hash).
pub fn compute_preimage_lookup_state_key(service_id: ServiceId, hash: &Hash) -> [u8; 31] {
    key_for_service_data(service_id, &preimage_hash_arg(hash))
}

/// Compute the state key for a preimage info entry: C(s, E_4(l) ++ hash).
pub fn compute_preimage_info_state_key(
    service_id: ServiceId,
    hash: &Hash,
    length: u32,
) -> [u8; 31] {
    key_for_service_data(service_id, &preimage_info_hash_arg(length, hash))
}

/// Serialize the full state T(σ) into a sorted vector of (key, value) pairs.
pub fn serialize_state(state: &State, config: &Config) -> Vec<([u8; 31], Vec<u8>)> {
    use scale::Encode;
    let _ = config; // Config no longer needed — all sizes encoded in Vecs

    let mut kvs = vec![
        (state_key_from_index(1), state.auth_pool.encode()), // C(1) α
        (state_key_from_index(2), state.auth_queue.encode()), // C(2) ϕ
        (state_key_from_index(3), state.recent_blocks.encode()), // C(3) β
        (state_key_from_index(4), state.safrole.encode()),   // C(4) γ
        (state_key_from_index(5), state.judgments.encode()), // C(5) ψ
        (state_key_from_index(6), serialize_entropy(&state.entropy)), // C(6) η (raw 128B)
        (state_key_from_index(7), state.pending_validators.encode()), // C(7) ι
        (state_key_from_index(8), state.current_validators.encode()), // C(8) κ
        (state_key_from_index(9), state.previous_validators.encode()), // C(9) λ
        (state_key_from_index(10), state.pending_reports.encode()), // C(10) ρ
        (
            state_key_from_index(11),
            state.timeslot.to_le_bytes().to_vec(),
        ), // C(11) τ
        (state_key_from_index(12), state.privileged_services.encode()), // C(12) χ
        (state_key_from_index(13), state.statistics.encode()), // C(13) π
        (state_key_from_index(14), state.accumulation_queue.encode()), // C(14) ω
        (
            state_key_from_index(15),
            state.accumulation_history.encode(),
        ), // C(15) ξ
    ];

    // C(16) → θ accumulation_outputs
    kvs.push((
        state_key_from_index(16),
        serialize_accumulation_outputs(&state.accumulation_outputs),
    ));

    // Service accounts and their data
    for (&service_id, account) in &state.services {
        // C(255, s) → service account metadata
        kvs.push((
            state_key_for_service(255, service_id),
            serialize_service_account_with_id(account, service_id),
        ));

        // C(s, E_4(2^32-1) ++ k) → storage entries
        for (storage_key, value) in &account.storage {
            let h = storage_hash_arg(storage_key);
            kvs.push((key_for_service_data(service_id, &h), value.clone()));
        }

        // C(s, E_4(2^32-2) ++ hash) → preimage lookup
        for (hash, data) in &account.preimage_lookup {
            let h = preimage_hash_arg(hash);
            kvs.push((key_for_service_data(service_id, &h), data.clone()));
        }

        // C(s, E_4(l) ++ hash) → preimage info
        for (&(ref hash, length), timeslots) in &account.preimage_info {
            let h = preimage_info_hash_arg(length, hash);
            let mut val = Vec::new();
            encode_u32_le(timeslots.len() as u32, &mut val);
            for &t in timeslots {
                val.extend_from_slice(&t.to_le_bytes());
            }
            kvs.push((key_for_service_data(service_id, &h), val));
        }
    }

    // Sort by key
    kvs.sort_by(|a, b| a.0.cmp(&b.0));
    kvs
}

/// Serialize state and include additional opaque KV pairs (from deserialization).
/// The opaque entries are service data keys that were passed through unchanged.
pub fn serialize_state_with_opaque(
    state: &State,
    config: &Config,
    opaque: &[([u8; 31], Vec<u8>)],
) -> Vec<([u8; 31], Vec<u8>)> {
    let mut kvs = serialize_state(state, config);
    // Collect state-generated keys for deduplication
    let state_keys: std::collections::HashSet<[u8; 31]> = kvs.iter().map(|(k, _)| *k).collect();
    // Only add opaque entries whose keys don't collide with state entries
    for (k, v) in opaque {
        if !state_keys.contains(k) {
            kvs.push((*k, v.clone()));
        }
    }
    kvs.sort_by(|a, b| a.0.cmp(&b.0));
    kvs
}

// --- Component serializers ---

/// C(6): η entropy — 4 × 32 raw bytes.
fn serialize_entropy(entropy: &[Hash; 4]) -> Vec<u8> {
    scale::Encode::encode(entropy)
}

/// C(16): θ accumulation_outputs — ↕ sorted (E_4(service_id), hash) pairs.
fn serialize_accumulation_outputs(outputs: &[(ServiceId, Hash)]) -> Vec<u8> {
    scale::Encode::encode(outputs)
}

/// E(0, a_c, E_8(a_b, a_g, a_m, a_o, a_f), E_4(a_i, a_r, a_a, a_p))
pub fn serialize_single_service(account: &ServiceAccount) -> Vec<u8> {
    serialize_service_account_with_id(account, 0)
}

fn serialize_service_account_with_id(account: &ServiceAccount, sid: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(89);

    // Compute dependent values i and o from actual storage (GP eq 9.4 / line 1036-1040)
    // a_i = 2·|a_l| + |a_s|
    let computed_i = 2 * account.preimage_info.len() as u32 + account.storage.len() as u32;
    // a_o = Σ_{(h,z) ∈ K(a_l)} (81 + z) + Σ_{(x,y) ∈ a_s} (34 + |y| + |x|)
    let computed_o: u64 = account
        .preimage_info
        .keys()
        .map(|&(_hash, length)| 81u64 + length as u64)
        .sum::<u64>()
        + account
            .storage
            .iter()
            .map(|(k, v)| 34u64 + k.len() as u64 + v.len() as u64)
            .sum::<u64>();

    if computed_i != account.accumulation_counter {
        eprintln!(
            "SERVICE ACCOUNT i mismatch for svc {}: stored={}, computed={} (storage={}, preimage_info={})",
            sid,
            account.accumulation_counter,
            computed_i,
            account.storage.len(),
            account.preimage_info.len()
        );
    }
    if computed_o != account.total_footprint {
        eprintln!(
            "SERVICE ACCOUNT o mismatch for svc {}: stored={}, computed={} (storage entries: {:?})",
            sid,
            account.total_footprint,
            computed_o,
            account
                .storage
                .iter()
                .map(|(k, v)| (k.len(), v.len()))
                .collect::<Vec<_>>()
        );
    }

    // version = 0
    buf.push(0);
    // a_c: code_hash
    buf.extend_from_slice(&account.code_hash.0);
    // E_8 fields: b, g, m, o, f
    buf.extend_from_slice(&account.quota_items.to_le_bytes()); // was: balance
    buf.extend_from_slice(&account.min_accumulate_gas.to_le_bytes());
    buf.extend_from_slice(&account.min_on_transfer_gas.to_le_bytes());
    buf.extend_from_slice(&account.total_footprint.to_le_bytes());
    buf.extend_from_slice(&account.quota_bytes.to_le_bytes()); // was: gratis
    // E_4 fields: i, r, a, p
    buf.extend_from_slice(&account.accumulation_counter.to_le_bytes());
    buf.extend_from_slice(&account.last_accumulation.to_le_bytes());
    buf.extend_from_slice(&account.last_activity.to_le_bytes());
    buf.extend_from_slice(&account.preimage_count.to_le_bytes());

    buf
}

// --- Deserialization ---

/// Deserialize state from key-value pairs (inverse of serialize_state).
///
/// Returns the State and a list of opaque service data KV pairs that cannot
/// be fully deserialized (because the blake2b hash in the key is irreversible).
/// These opaque entries should be passed to `serialize_state_with_opaque` to
/// include them in re-serialization.
#[allow(clippy::type_complexity)]
pub fn deserialize_state(
    kvs: &[([u8; 31], Vec<u8>)],
    config: &Config,
) -> Result<(State, Vec<([u8; 31], Vec<u8>)>), String> {
    let mut state = State {
        auth_pool: vec![Vec::new(); config.core_count as usize],
        recent_blocks: RecentBlocks {
            headers: Vec::new(),
            accumulation_log: Vec::new(),
        },
        accumulation_outputs: Vec::new(),
        safrole: SafroleState {
            pending_keys: Vec::new(),
            ring_root: grey_types::BandersnatchRingRoot::default(),
            seal_key_series: SealKeySeries::Fallback(Vec::new()),
            ticket_accumulator: Vec::new(),
        },
        services: BTreeMap::new(),
        entropy: [Hash::ZERO; 4],
        pending_validators: Vec::new(),
        current_validators: Vec::new(),
        previous_validators: Vec::new(),
        pending_reports: vec![None; config.core_count as usize],
        timeslot: 0,
        auth_queue: vec![vec![Hash::ZERO; config.core_count as usize]; config.auth_queue_size],
        privileged_services: PrivilegedServices::default(),
        judgments: Judgments::default(),
        statistics: ValidatorStatistics::default(),
        accumulation_queue: vec![Vec::new(); config.epoch_length as usize],
        accumulation_history: vec![Vec::new(); config.epoch_length as usize],
    };

    // Collect opaque service data entries (C(s, h) keys).
    // We can't reverse the blake2b hash to determine the original key/hash,
    // so we store these as raw KV pairs and pass them through unchanged.
    let mut opaque_service_data: Vec<([u8; 31], Vec<u8>)> = Vec::new();

    for (key, value) in kvs {
        match classify_key(key) {
            KeyType::Component(idx) => {
                use scale::Decode;
                let decode_err = |name: &str, e| format!("C({idx}) {name}: {e}");
                match idx {
                    1 => {
                        state.auth_pool = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("auth_pool", e))?
                    }
                    2 => {
                        state.auth_queue = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("auth_queue", e))?
                    }
                    3 => {
                        state.recent_blocks = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("recent_blocks", e))?
                    }
                    4 => {
                        state.safrole = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("safrole", e))?
                    }
                    5 => {
                        state.judgments = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("judgments", e))?
                    }
                    6 => state.entropy = deserialize_entropy(value)?,
                    7 => {
                        state.pending_validators = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("pending_validators", e))?
                    }
                    8 => {
                        state.current_validators = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("current_validators", e))?
                    }
                    9 => {
                        state.previous_validators = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("previous_validators", e))?
                    }
                    10 => {
                        state.pending_reports = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("pending_reports", e))?
                    }
                    11 => {
                        if value.len() < 4 {
                            return Err("timeslot too short".into());
                        }
                        state.timeslot =
                            u32::from_le_bytes([value[0], value[1], value[2], value[3]]);
                    }
                    12 => {
                        state.privileged_services = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("privileged", e))?
                    }
                    13 => {
                        state.statistics = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("statistics", e))?
                    }
                    14 => {
                        state.accumulation_queue = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("accum_queue", e))?
                    }
                    15 => {
                        state.accumulation_history = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("accum_history", e))?
                    }
                    16 => {
                        state.accumulation_outputs = Decode::decode(value)
                            .map(|(v, _)| v)
                            .map_err(|e| decode_err("accum_outputs", e))?
                    }
                    _ => {} // unknown component index, ignore
                }
            }
            KeyType::ServiceAccount(service_id) => {
                let account = deserialize_service_account(value)?;
                state.services.insert(service_id, account);
            }
            KeyType::ServiceData => {
                opaque_service_data.push((*key, value.clone()));
            }
        }
    }

    Ok((state, opaque_service_data))
}

/// Look up a preimage (e.g., code blob) for a specific service from opaque KV data.
/// This computes the expected key C(service_id, E_4(2^32-2) ++ hash) and searches
/// the opaque data for a matching entry.
pub fn lookup_preimage_in_opaque(
    service_id: ServiceId,
    hash: &Hash,
    opaque_data: &[([u8; 31], Vec<u8>)],
) -> Option<Vec<u8>> {
    let h = preimage_hash_arg(hash);
    let expected_key = key_for_service_data(service_id, &h);
    opaque_data
        .iter()
        .find(|(k, _)| *k == expected_key)
        .map(|(_, v)| v.clone())
}

/// Classify a 31-byte state key.
enum KeyType {
    /// C(i) — state component index.
    Component(u8),
    /// C(255, s) — service account metadata.
    ServiceAccount(ServiceId),
    /// C(s, h) — service data (storage/preimage).
    ServiceData,
}

/// Classify a key based on its structure.
/// C(i): key = [i, 0, 0, ...]
/// C(255, s): key = [255, s0, 0, s1, 0, s2, 0, s3, 0, 0, ...]
/// C(s, h): key = [s0, a0, s1, a1, s2, a2, s3, a3, ...]  (interleaved)
fn classify_key(key: &[u8; 31]) -> KeyType {
    // C(i) keys have index > 0 and index < 255, with remaining bytes = 0
    // C(255, s) keys have key[0] = 255, key[2] = 0, key[4] = 0, key[6] = 0
    // C(s, h) keys have key[0] = s0 which could overlap with C(i)
    //
    // The distinguishing factor: C(i) has key[1..] all zeros.
    // C(255, s) has key[0] = 255 and key[2], key[4], key[6] = 0.
    // C(s, h) has non-zero bytes in odd positions (from hash).

    if key[0] == 255 {
        // Could be C(255, s) — check if positions 2, 4, 6 are zero (C(i,s) format)
        if key[2] == 0 && key[4] == 0 && key[6] == 0 && key[8..].iter().all(|&b| b == 0) {
            let service_id = u32::from_le_bytes([key[1], key[3], key[5], key[7]]);
            return KeyType::ServiceAccount(service_id);
        }
    }

    // Check if this is C(i) — index + all zeros
    if key[1..].iter().all(|&b| b == 0) {
        return KeyType::Component(key[0]);
    }

    // Check for C(i, s) pattern: key[2], key[4], key[6] = 0, rest = 0
    if key[0] >= 1
        && key[0] <= 16
        && key[2] == 0
        && key[4] == 0
        && key[6] == 0
        && key[8..].iter().all(|&b| b == 0)
    {
        return KeyType::Component(key[0]);
    }

    // Otherwise it's a service data key C(s, h)
    KeyType::ServiceData
}

// --- Component deserializers ---

fn read_hash(data: &[u8], pos: &mut usize) -> Result<Hash, String> {
    if *pos + 32 > data.len() {
        return Err("unexpected end reading hash".into());
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&data[*pos..*pos + 32]);
    *pos += 32;
    Ok(Hash(h))
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, String> {
    if *pos + 4 > data.len() {
        return Err("unexpected end reading u32".into());
    }
    let v = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

fn read_u64(data: &[u8], pos: &mut usize) -> Result<u64, String> {
    if *pos + 8 > data.len() {
        return Err("unexpected end reading u64".into());
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[*pos..*pos + 8]);
    *pos += 8;
    Ok(u64::from_le_bytes(bytes))
}

fn deserialize_entropy(data: &[u8]) -> Result<[Hash; 4], String> {
    if data.len() < 128 {
        return Err("entropy data too short".into());
    }
    let mut entropy = [Hash::ZERO; 4];
    for i in 0..4 {
        entropy[i].0.copy_from_slice(&data[i * 32..(i + 1) * 32]);
    }
    Ok(entropy)
}

fn deserialize_service_account(data: &[u8]) -> Result<ServiceAccount, String> {
    let mut pos = 0;

    if pos >= data.len() {
        return Err("service account data empty".into());
    }
    let _version = data[pos];
    pos += 1;

    let code_hash = read_hash(data, &mut pos)?;
    let quota_items = read_u64(data, &mut pos)?; // was: balance
    let min_accumulate_gas = read_u64(data, &mut pos)?;
    let min_on_transfer_gas = read_u64(data, &mut pos)?;
    let total_footprint = read_u64(data, &mut pos)?;
    let quota_bytes = read_u64(data, &mut pos)?; // was: free_storage_offset
    let accumulation_counter = read_u32(data, &mut pos)?;
    let last_accumulation = read_u32(data, &mut pos)?;
    let last_activity = read_u32(data, &mut pos)?;
    let preimage_count = read_u32(data, &mut pos)?;

    Ok(ServiceAccount {
        code_hash,
        quota_items,
        min_accumulate_gas,
        min_on_transfer_gas,
        storage: BTreeMap::new(),
        preimage_lookup: BTreeMap::new(),
        preimage_info: BTreeMap::new(),
        total_footprint,
        quota_bytes,
        accumulation_counter,
        last_accumulation,
        last_activity,
        preimage_count,
    })
}
