//! Grey conformance testing target.
//!
//! Implements the JAM fuzz protocol v1 over a Unix domain socket.
//! Accepts blocks from a fuzzer, applies state transitions, and returns state roots.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use grey_codec::encode::encode_compact;
use grey_codec::header_codec::{compute_header_hash, compute_unsigned_header_hash};
use grey_codec::DecodeWithConfig;
use grey_merkle::state_serial;
use grey_types::config::Config;
use grey_types::state::State;
use grey_types::Hash;

/// Protocol message discriminants.
const MSG_PEER_INFO: u8 = 0x00;
const MSG_INITIALIZE: u8 = 0x01;
const MSG_STATE_ROOT: u8 = 0x02;
const MSG_IMPORT_BLOCK: u8 = 0x03;
const MSG_GET_STATE: u8 = 0x04;
const MSG_STATE: u8 = 0x05;
const MSG_ERROR: u8 = 0xFF;

/// Feature bits.
const FEATURE_ANCESTRY: u32 = 1;
const FEATURE_FORK: u32 = 2;

/// Tracked state for a particular header hash.
#[derive(Clone)]
struct TrackedState {
    state: State,
    /// Opaque service data KV pairs for re-serialization.
    opaque_data: Vec<([u8; 31], Vec<u8>)>,
}

/// The conformance target server.
struct ConformTarget {
    config: Config,
    /// States indexed by header hash.
    states: HashMap<Hash, TrackedState>,
    /// Ancestry: (timeslot, header_hash) pairs.
    ancestry: Vec<(u32, Hash)>,
}

impl ConformTarget {
    fn new(config: Config) -> Self {
        Self {
            config,
            states: HashMap::new(),
            ancestry: Vec::new(),
        }
    }

    fn handle_connection(&mut self, stream: &mut std::os::unix::net::UnixStream) {
        tracing::info!("New connection");

        loop {
            // Read message: 4-byte LE length prefix + body
            let msg = match read_message(stream) {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::info!("Connection closed: {e}");
                    return;
                }
            };

            if msg.is_empty() {
                tracing::warn!("Empty message received");
                return;
            }

            let discriminant = msg[0];
            let body = &msg[1..];

            match discriminant {
                MSG_PEER_INFO => {
                    tracing::info!("Received PeerInfo");
                    let response = self.handle_peer_info(body);
                    if let Err(e) = write_message(stream, &response) {
                        tracing::error!("Failed to send PeerInfo response: {e}");
                        return;
                    }
                }
                MSG_INITIALIZE => {
                    tracing::info!("Received Initialize");
                    let response = self.handle_initialize(body);
                    if let Err(e) = write_message(stream, &response) {
                        tracing::error!("Failed to send Initialize response: {e}");
                        return;
                    }
                }
                MSG_IMPORT_BLOCK => {
                    tracing::info!("Received ImportBlock");
                    let response = self.handle_import_block(body);
                    if let Err(e) = write_message(stream, &response) {
                        tracing::error!("Failed to send ImportBlock response: {e}");
                        return;
                    }
                }
                MSG_GET_STATE => {
                    tracing::info!("Received GetState");
                    let response = self.handle_get_state(body);
                    if let Err(e) = write_message(stream, &response) {
                        tracing::error!("Failed to send GetState response: {e}");
                        return;
                    }
                }
                other => {
                    tracing::warn!("Unknown message discriminant: 0x{other:02x}");
                    return;
                }
            }
        }
    }

    fn handle_peer_info(&self, _body: &[u8]) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.push(MSG_PEER_INFO);

        // fuzz_version: U8
        msg.push(1);

        // fuzz_features: U32 (support ancestry + fork)
        msg.extend_from_slice(&(FEATURE_ANCESTRY | FEATURE_FORK).to_le_bytes());

        // jam_version: Version { major: U8, minor: U8, patch: U8 }
        msg.push(0); // major
        msg.push(7); // minor
        msg.push(2); // patch

        // app_version: Version
        msg.push(0); // major
        msg.push(1); // minor
        msg.push(0); // patch

        // app_name: UTF8String (compact length + bytes)
        let name = b"grey";
        encode_compact(name.len() as u64, &mut msg);
        msg.extend_from_slice(name);

        msg
    }

    fn handle_initialize(&mut self, body: &[u8]) -> Vec<u8> {
        let mut pos = 0;

        // Decode header
        let header = match grey_types::header::Header::decode_with_config(
            &body[pos..],
            &self.config,
        ) {
            Ok((header, consumed)) => {
                pos += consumed;
                header
            }
            Err(e) => {
                tracing::error!("Failed to decode initialize header: {e}");
                return make_error(&format!("header decode: {e}"));
            }
        };

        // Compute header hash
        let header_hash = compute_header_hash(&header);
        tracing::info!("Initialize header hash: {header_hash}");

        // Decode state KV pairs: SEQUENCE OF KeyValue
        let kv_count = match decode_compact_at(body, &mut pos) {
            Ok(n) => n as usize,
            Err(e) => {
                tracing::error!("Failed to decode state KV count: {e}");
                return make_error(&format!("state kv count: {e}"));
            }
        };

        let mut kvs = Vec::with_capacity(kv_count);
        for i in 0..kv_count {
            // key: OCTET STRING (SIZE(31))
            if pos + 31 > body.len() {
                return make_error(&format!("truncated state key at entry {i}"));
            }
            let mut key = [0u8; 31];
            key.copy_from_slice(&body[pos..pos + 31]);
            pos += 31;

            // value: OCTET STRING (compact length + bytes)
            let val_len = match decode_compact_at(body, &mut pos) {
                Ok(n) => n as usize,
                Err(e) => return make_error(&format!("state value length at {i}: {e}")),
            };
            if pos + val_len > body.len() {
                return make_error(&format!("truncated state value at entry {i}"));
            }
            let value = body[pos..pos + val_len].to_vec();
            pos += val_len;

            kvs.push((key, value));
        }

        // Decode ancestry: SEQUENCE OF AncestryItem
        let ancestry_count = match decode_compact_at(body, &mut pos) {
            Ok(n) => n as usize,
            Err(e) => {
                tracing::error!("Failed to decode ancestry count: {e}");
                return make_error(&format!("ancestry count: {e}"));
            }
        };

        self.ancestry.clear();
        for _ in 0..ancestry_count {
            if pos + 36 > body.len() {
                return make_error("truncated ancestry entry");
            }
            let slot =
                u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]);
            pos += 4;
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&body[pos..pos + 32]);
            pos += 32;
            self.ancestry.push((slot, Hash(hash)));
        }

        tracing::info!(
            "Initialize: {} KV pairs, {} ancestry entries",
            kvs.len(),
            self.ancestry.len()
        );

        // Compute state root from raw KV pairs
        let state_root = grey_merkle::compute_state_root_from_kvs(&kvs);
        tracing::info!("Computed state root: {state_root}");

        // Log and verify round-trip for initial KV pairs
        let names = ["","auth_pool","auth_queue","recent_blocks","safrole",
            "judgments","entropy","pend_val","curr_val","prev_val",
            "pend_rpt","timeslot","priv","stats","aq","ah","ao"];
        for (key, value) in &kvs {
            let idx = key[0] as usize;
            let is_component = key[1..].iter().all(|&b| b == 0);
            if is_component && idx <= 16 {
                let name = names.get(idx).unwrap_or(&"?");
                tracing::info!("  Init C({idx}) {name}: {} bytes", value.len());
            }
        }

        // Deserialize state
        match state_serial::deserialize_state(&kvs, &self.config) {
            Ok((state, opaque)) => {
                // Log auth_pool contents
                for (i, pool) in state.auth_pool.iter().enumerate() {
                    let non_zero = pool.iter().filter(|h| **h != Hash::ZERO).count();
                    tracing::info!("  auth_pool[{i}]: {} entries, {} non-zero", pool.len(), non_zero);
                    for (j, h) in pool.iter().enumerate() {
                        tracing::info!("    [{j}]: {h}");
                    }
                }
                // Also log first few auth_queue entries for comparison
                for slot_idx in 0..4.min(state.auth_queue.len()) {
                    for core_idx in 0..state.auth_queue[slot_idx].len() {
                        let h = &state.auth_queue[slot_idx][core_idx];
                        tracing::info!("  auth_queue[{slot_idx}][{core_idx}]: {h}");
                    }
                }
                // Log auth_queue non-zero entries
                let non_zero_q: usize = state.auth_queue.iter()
                    .flat_map(|slot| slot.iter())
                    .filter(|h| **h != Hash::ZERO)
                    .count();
                tracing::info!("  auth_queue: {} slots, {} non-zero entries total", state.auth_queue.len(), non_zero_q);

                // ROUND-TRIP CHECK: re-serialize and compare with original
                let reserialized = state_serial::serialize_state_with_opaque(
                    &state,
                    &self.config,
                    &opaque,
                );
                let reroot = grey_merkle::compute_state_root_from_kvs(&reserialized);
                if reroot != state_root {
                    tracing::error!("ROUND-TRIP MISMATCH! original root={state_root}, reserialized root={reroot}");
                    // Find which components differ
                    for (key, orig_val) in &kvs {
                        let idx = key[0] as usize;
                        let is_component = key[1..].iter().all(|&b| b == 0);
                        let reser_val = reserialized.iter().find(|(k, _)| k == key).map(|(_, v)| v);
                        match reser_val {
                            Some(rv) if rv != orig_val => {
                                let name = if is_component && idx <= 16 { names.get(idx).unwrap_or(&"?") } else { "data" };
                                tracing::error!("  C({idx}) {name}: orig {} bytes, reser {} bytes",
                                    orig_val.len(), rv.len());
                                // Find first diff
                                let min_len = orig_val.len().min(rv.len());
                                for i in 0..min_len {
                                    if orig_val[i] != rv[i] {
                                        fn to_hex(data: &[u8]) -> String {
                                            data.iter().map(|b| format!("{b:02x}")).collect()
                                        }
                                        let s = if i > 8 { i - 8 } else { 0 };
                                        let e = (i + 16).min(min_len);
                                        tracing::error!("    first diff at byte {i}: orig=0x{:02x} reser=0x{:02x}", orig_val[i], rv[i]);
                                        tracing::error!("    orig[{s}..{e}]: {}", to_hex(&orig_val[s..e]));
                                        tracing::error!("    reser[{s}..{e}]: {}", to_hex(&rv[s..e]));
                                        break;
                                    }
                                }
                                if orig_val.len() != rv.len() {
                                    tracing::error!("    LENGTH MISMATCH: {} vs {}", orig_val.len(), rv.len());
                                }
                            }
                            None => tracing::error!("  key missing in reserialized: C({idx})"),
                            _ => {}
                        }
                    }
                    // Check for extra keys in reserialized
                    for (key, _) in &reserialized {
                        if !kvs.iter().any(|(k, _)| k == key) {
                            tracing::error!("  EXTRA key in reserialized: {:?}", key);
                        }
                    }
                } else {
                    tracing::info!("Round-trip OK: root matches");
                }

                self.states.clear();
                let unsigned_hash = compute_unsigned_header_hash(&header);
                let tracked = TrackedState {
                    state,
                    opaque_data: opaque,
                };
                self.states.insert(header_hash, tracked.clone());
                self.states.insert(unsigned_hash, tracked);
            }
            Err(e) => {
                tracing::error!("Failed to deserialize state: {e}");
                return make_error(&format!("state deserialize: {e}"));
            }
        }

        // Return StateRoot
        make_state_root(&state_root)
    }

    fn handle_import_block(&mut self, body: &[u8]) -> Vec<u8> {
        // Decode block
        let block = match grey_types::header::Block::decode_with_config(body, &self.config) {
            Ok((block, consumed)) => {
                // Roundtrip check: re-encode the block and compare
                use grey_codec::Encode;
                let re_encoded = block.encode();
                if re_encoded != body[..consumed] {
                    tracing::warn!(
                        "Block roundtrip MISMATCH! original={} bytes, re-encoded={} bytes",
                        consumed,
                        re_encoded.len()
                    );
                    // Find first difference
                    let min_len = consumed.min(re_encoded.len());
                    for i in 0..min_len {
                        if body[i] != re_encoded[i] {
                            tracing::warn!(
                                "  First diff at byte {i}: original=0x{:02x}, re-encoded=0x{:02x}",
                                body[i],
                                re_encoded[i]
                            );
                            fn to_hex(data: &[u8]) -> String {
                                data.iter().map(|b| format!("{b:02x}")).collect()
                            }
                            let start = if i > 8 { i - 8 } else { 0 };
                            let end = (i + 16).min(min_len);
                            tracing::warn!("  original[{start}..{end}]:     {}", to_hex(&body[start..end]));
                            tracing::warn!("  re-encoded[{start}..{end}]:   {}", to_hex(&re_encoded[start..end]));
                            break;
                        }
                    }
                    // Also check each guarantee's work report
                    for (gi, guarantee) in block.extrinsic.guarantees.iter().enumerate() {
                        let report_encoded = guarantee.report.encode();
                        let hex: String = report_encoded[..32.min(report_encoded.len())]
                            .iter().map(|b| format!("{b:02x}")).collect();
                        tracing::warn!(
                            "  Guarantee {gi} report encoding: {} bytes, first 32: {}",
                            report_encoded.len(),
                            hex
                        );
                    }
                }
                block
            }
            Err(e) => {
                tracing::error!("Failed to decode block: {e}");
                return make_error(&format!("block decode: {e}"));
            }
        };

        let parent_hash = block.header.parent_hash;
        tracing::info!(
            "ImportBlock: timeslot={}, parent={}",
            block.header.timeslot,
            parent_hash
        );

        // Debug: list known states
        for (h, _) in &self.states {
            tracing::debug!("Known state hash: {h}");
        }

        // Look up parent state
        let parent_tracked = match self.states.get(&parent_hash) {
            Some(ts) => ts,
            None => {
                tracing::error!("Parent state not found: {parent_hash}");
                return make_error("parent state not found");
            }
        };

        // Apply state transition
        let parent_opaque = parent_tracked.opaque_data.clone();

        // For the failing block, apply step-by-step to identify the divergent step
        eprintln!("DEBUG: block timeslot = {}", block.header.timeslot);
        let step_by_step = block.header.timeslot == 63;

        match grey_state::transition::apply_with_config(&parent_tracked.state, &block, &self.config, &parent_opaque) {
            Ok((new_state, remaining_opaque)) => {
                // Step-by-step debugging for the failing block
                if step_by_step {
                    self.debug_step_by_step(&parent_tracked.state, &block, &parent_opaque);
                }
                // Compute header hash of the imported block
                let header_hash = compute_header_hash(&block.header);

                // Compute state root
                // Debug: compare structured vs opaque KV entries
                let structured_kvs = state_serial::serialize_state(&new_state, &self.config);
                let structured_keys: std::collections::HashSet<[u8; 31]> =
                    structured_kvs.iter().map(|(k, _)| *k).collect();
                let opaque_superseded = remaining_opaque.iter()
                    .filter(|(k, _)| structured_keys.contains(k))
                    .count();
                let opaque_added: Vec<_> = remaining_opaque.iter()
                    .filter(|(k, _)| !structured_keys.contains(k))
                    .collect();
                tracing::warn!(
                    "  KV merge: {} structured, {} opaque superseded, {} opaque added (zombie)",
                    structured_kvs.len(), opaque_superseded, opaque_added.len()
                );
                for (k, v) in &opaque_added {
                    let key_hex: String = k[..8].iter().map(|b| format!("{b:02x}")).collect();
                    tracing::warn!(
                        "    opaque added: key={key_hex}...: {} bytes",
                        v.len(),
                    );
                }

                let kvs = state_serial::serialize_state_with_opaque(
                    &new_state,
                    &self.config,
                    &remaining_opaque,
                );
                let state_root = grey_merkle::compute_state_root_from_kvs(&kvs);
                tracing::info!("Post-block state root: {state_root}");

                // Diagnostic: root WITHOUT opaque data
                if step_by_step {
                    let kvs_no_opaque = state_serial::serialize_state(&new_state, &self.config);
                    let root_no_opaque = grey_merkle::compute_state_root_from_kvs(&kvs_no_opaque);
                    tracing::warn!("  Root WITHOUT opaque: {root_no_opaque} ({} KVs)", kvs_no_opaque.len());
                    tracing::warn!("  Root WITH opaque:    {state_root} ({} KVs)", kvs.len());

                    // Count how many opaque entries were added (not superseded)
                    let struct_keys: std::collections::HashSet<[u8; 31]> =
                        kvs_no_opaque.iter().map(|(k, _)| *k).collect();
                    let mut opaque_added = 0;
                    let mut opaque_superseded = 0;
                    for (k, _v) in &parent_opaque {
                        if struct_keys.contains(k) {
                            opaque_superseded += 1;
                        } else {
                            opaque_added += 1;
                        }
                    }
                    tracing::warn!("  Opaque: {} total, {} superseded, {} added",
                        parent_opaque.len(), opaque_superseded, opaque_added);

                    // Show all opaque entries that were NOT superseded
                    for (k, v) in &parent_opaque {
                        if !struct_keys.contains(k) {
                            let key_hex: String = k[..8].iter().map(|b| format!("{b:02x}")).collect();
                            tracing::warn!("    zombie opaque: key={key_hex}...: {} bytes", v.len());
                        }
                    }
                }

                // Binary search: which component(s) cause the root mismatch?
                if step_by_step {
                    let expected_root_hex = "b6c72027f5628d674261c017e5c8aa922afb1d9b0c16bb1c0c693c31dcf3ab56";
                    let expected_root = {
                        let mut h = [0u8; 32];
                        for i in 0..32 {
                            h[i] = u8::from_str_radix(&expected_root_hex[i*2..i*2+2], 16).unwrap();
                        }
                        Hash(h)
                    };

                    // Get parent state's KVs
                    let parent_kvs = state_serial::serialize_state_with_opaque(
                        &parent_tracked.state, &self.config, &parent_opaque,
                    );
                    let parent_kv_map: std::collections::HashMap<[u8; 31], Vec<u8>> =
                        parent_kvs.into_iter().collect();

                    // KV diff: show which keys changed, were added, or removed
                    let new_kv_map: std::collections::HashMap<[u8; 31], Vec<u8>> =
                        kvs.iter().map(|(k, v)| (*k, v.clone())).collect();
                    eprintln!("=== KV DIFF parent vs new state (slot {}) ===", block.header.timeslot);
                    let mut all_keys: std::collections::BTreeSet<[u8; 31]> = std::collections::BTreeSet::new();
                    for k in parent_kv_map.keys() { all_keys.insert(*k); }
                    for k in new_kv_map.keys() { all_keys.insert(*k); }
                    let mut changed = 0;
                    let mut added = 0;
                    let mut removed = 0;
                    for k in &all_keys {
                        let key_hex: String = k.iter().map(|b| format!("{b:02x}")).collect();
                        match (parent_kv_map.get(k), new_kv_map.get(k)) {
                            (Some(pv), Some(nv)) if pv != nv => {
                                changed += 1;
                                let pv_hash = grey_crypto::blake2b_256(pv);
                                let nv_hash = grey_crypto::blake2b_256(nv);
                                eprintln!("  CHANGED key={key_hex}: {} -> {} bytes, hash {} -> {}",
                                    pv.len(), nv.len(), pv_hash, nv_hash);
                                // For small values, show hex
                                if nv.len() <= 128 {
                                    let hex: String = nv.iter().map(|b| format!("{b:02x}")).collect();
                                    eprintln!("    new_val: {hex}");
                                }
                                if pv.len() <= 128 {
                                    let hex: String = pv.iter().map(|b| format!("{b:02x}")).collect();
                                    eprintln!("    old_val: {hex}");
                                }
                            }
                            (None, Some(nv)) => {
                                added += 1;
                                eprintln!("  ADDED key={key_hex}: {} bytes", nv.len());
                            }
                            (Some(pv), None) => {
                                removed += 1;
                                eprintln!("  REMOVED key={key_hex}: was {} bytes", pv.len());
                            }
                            _ => {} // unchanged
                        }
                    }
                    eprintln!("  Total: {} changed, {} added, {} removed (out of {} keys)", changed, added, removed, all_keys.len());

                    // Try substituting each component key from parent
                    for comp_idx in 1u8..=16 {
                        let comp_key = {
                            let mut k = [0u8; 31];
                            k[0] = comp_idx;
                            k
                        };
                        if let Some(parent_val) = parent_kv_map.get(&comp_key) {
                            let mut test_kvs = kvs.clone();
                            for (k, v) in test_kvs.iter_mut() {
                                if *k == comp_key {
                                    *v = parent_val.clone();
                                }
                            }
                            let test_root = grey_merkle::compute_state_root_from_kvs(&test_kvs);
                            if test_root == expected_root {
                                eprintln!("  FOUND: substituting C({comp_idx}) from parent gives EXPECTED root!");
                            } else {
                                eprintln!("  sub C({comp_idx}): {test_root}");
                            }
                        }
                    }

                    // Also try substituting C(255) service accounts
                    // Substitute ALL service-related keys from parent
                    {
                        let mut test_kvs = kvs.clone();
                        for (k, v) in test_kvs.iter_mut() {
                            if k[0] == 255 || k[1..] != [0u8; 30] {
                                if let Some(parent_val) = parent_kv_map.get(k) {
                                    *v = parent_val.clone();
                                }
                            }
                        }
                        // Also remove keys that exist in current but not parent for service data
                        test_kvs.retain(|(k, _)| {
                            if (k[0] == 255 && k[1..] != [0u8; 30]) || (k[0] != 255 && k[1..] != [0u8; 30]) {
                                parent_kv_map.contains_key(k)
                            } else {
                                true
                            }
                        });
                        // Add parent keys not in current
                        let current_keys: std::collections::HashSet<[u8; 31]> =
                            test_kvs.iter().map(|(k, _)| *k).collect();
                        for (k, v) in &parent_kv_map {
                            if (k[0] == 255 && k[1..] != [0u8; 30]) || (k[0] != 255 && k[1..] != [0u8; 30]) {
                                if !current_keys.contains(k) {
                                    test_kvs.push((*k, v.clone()));
                                }
                            }
                        }
                        test_kvs.sort_by(|a, b| a.0.cmp(&b.0));
                        let test_root = grey_merkle::compute_state_root_from_kvs(&test_kvs);
                        if test_root == expected_root {
                            eprintln!("  FOUND: substituting ALL service keys from parent gives EXPECTED root!");
                        } else {
                            eprintln!("  sub ALL svc keys: {test_root}");
                        }
                    }

                    // Try combinations of two components
                    let changed_comps: Vec<u8> = vec![3, 6, 10, 11, 13, 14, 15];
                    for i in 0..changed_comps.len() {
                        for j in (i+1)..changed_comps.len() {
                            let ci = changed_comps[i];
                            let cj = changed_comps[j];
                            let key_i = { let mut k = [0u8; 31]; k[0] = ci; k };
                            let key_j = { let mut k = [0u8; 31]; k[0] = cj; k };
                            let mut test_kvs = kvs.clone();
                            for (k, v) in test_kvs.iter_mut() {
                                if *k == key_i {
                                    if let Some(pv) = parent_kv_map.get(&key_i) { *v = pv.clone(); }
                                }
                                if *k == key_j {
                                    if let Some(pv) = parent_kv_map.get(&key_j) { *v = pv.clone(); }
                                }
                            }
                            let test_root = grey_merkle::compute_state_root_from_kvs(&test_kvs);
                            if test_root == expected_root {
                                eprintln!("  FOUND: substituting C({ci})+C({cj}) from parent gives EXPECTED root!");
                            } else {
                                eprintln!("  sub C({ci})+C({cj}): {test_root}");
                            }
                        }
                    }
                }

                // Dump KV pairs to file for debugging
                if block.header.timeslot >= 42 {
                    let path = format!("/tmp/kvs_slot{}.txt", block.header.timeslot);
                    let mut dump = String::new();
                    for (k, v) in &kvs {
                        let key_hex: String = k.iter().map(|b| format!("{b:02x}")).collect();
                        let val_hex: String = v.iter().map(|b| format!("{b:02x}")).collect();
                        dump.push_str(&format!("{key_hex} {val_hex}\n"));
                    }
                    std::fs::write(&path, &dump).ok();
                    tracing::info!("Dumped {} KV pairs to {path}", kvs.len());
                }

                // Dump service 2068330841 account metadata if present
                if let Some(svc) = new_state.services.get(&2068330841) {
                    tracing::warn!(
                        "  svc2068330841: storage={}, preimage_lookup={}, preimage_info={}, balance={}, items={}, bytes={}, a_a={}, a_r={}, a_p={}",
                        svc.storage.len(), svc.preimage_lookup.len(), svc.preimage_info.len(),
                        svc.balance, svc.accumulation_counter, svc.total_footprint,
                        svc.last_activity, svc.last_accumulation, svc.preimage_count
                    );
                    // Show raw serialized bytes for this service
                    let svc_key = state_serial::key_for_service_pub(255, 2068330841);
                    if let Some((_, val)) = kvs.iter().find(|(k, _)| *k == svc_key) {
                        let hex: String = val.iter().map(|b| format!("{b:02x}")).collect();
                        tracing::warn!("    C(255, 2068330841) raw: {hex}");
                    }
                }

                // Log service 0 storage/preimage counts
                if let Some(svc0) = new_state.services.get(&0) {
                    tracing::warn!(
                        "  svc0: storage={}, preimage_lookup={}, preimage_info={}, balance={}, items={}, bytes={}",
                        svc0.storage.len(), svc0.preimage_lookup.len(), svc0.preimage_info.len(),
                        svc0.balance, svc0.accumulation_counter, svc0.total_footprint
                    );
                    for (k, v) in &svc0.storage {
                        let k_hex: String = k.iter().map(|b| format!("{b:02x}")).collect();
                        tracing::warn!("    storage[{k_hex}]: {} bytes", v.len());
                    }
                }

                // Per-component hash logging for all blocks
                for (key, val) in &kvs {
                    let idx = key[0];
                    let is_simple = key[1..] == [0u8; 30];
                    if is_simple {
                        let val_hash = grey_crypto::blake2b_256(val);
                        tracing::info!(
                            "  KV C({idx}): {} bytes, hash={val_hash}",
                            val.len(),
                        );
                    }
                }

                // Targeted debugging for blocks with accumulation
                if block.header.timeslot >= 5 {
                    // Log service account and service data keys
                    for (key, val) in &kvs {
                        let idx = key[0];
                        let is_simple = key[1..] == [0u8; 30];
                        if !is_simple {
                            let key_hex: String = key[..8].iter().map(|b| format!("{b:02x}")).collect();
                            let val_hash = grey_crypto::blake2b_256(val);
                            tracing::info!(
                                "  KV key={key_hex}...: {} bytes, hash={val_hash}",
                                val.len(),
                            );
                        }
                    }

                    // Diagnostic: compute state root without C(16) to check if that's the issue
                    let kvs_no_c16: Vec<_> = kvs.iter()
                        .filter(|(k, _)| !(k[0] == 16 && k[1..] == [0u8; 30]))
                        .cloned()
                        .collect();
                    let root_no_c16 = grey_merkle::compute_state_root_from_kvs(&kvs_no_c16);
                    tracing::info!("  Root without C(16): {root_no_c16}");

                    // Also compute without C(14) and C(15)
                    let kvs_no_accum: Vec<_> = kvs.iter()
                        .filter(|(k, _)| {
                            let idx = k[0];
                            let is_simple = k[1..] == [0u8; 30];
                            !(is_simple && (idx == 14 || idx == 15 || idx == 16))
                        })
                        .cloned()
                        .collect();
                    let root_no_accum = grey_merkle::compute_state_root_from_kvs(&kvs_no_accum);
                    tracing::info!("  Root without C(14,15,16): {root_no_accum}");

                    // Log C(16) raw hex for debugging
                    if let Some((_, val)) = kvs.iter().find(|(k, _)| k[0] == 16 && k[1..] == [0u8; 30]) {
                        let hex: String = val.iter().map(|b| format!("{b:02x}")).collect();
                        tracing::info!("  C(16) raw: {hex}");
                    }

                    // Log number of KV pairs by category
                    let n_components = kvs.iter().filter(|(k, _)| k[1..] == [0u8; 30]).count();
                    let n_svc_accounts = kvs.iter().filter(|(k, _)| k[0] == 255 && k[1..] != [0u8; 30]).count();
                    let n_svc_data = kvs.iter().filter(|(k, _)| k[0] != 255 && k[1..] != [0u8; 30]).count();
                    tracing::info!("  KV breakdown: {} components, {} svc accounts, {} svc data", n_components, n_svc_accounts, n_svc_data);

                    // Root with only components (no service accounts or service data)
                    let kvs_components_only: Vec<_> = kvs.iter()
                        .filter(|(k, _)| k[1..] == [0u8; 30])
                        .cloned()
                        .collect();
                    tracing::info!("  Root components-only: {}", grey_merkle::compute_state_root_from_kvs(&kvs_components_only));
                }

                // Update ancestry
                self.ancestry
                    .push((block.header.timeslot, header_hash));

                // Store new state under both full and unsigned header hashes
                let unsigned_hash = compute_unsigned_header_hash(&block.header);
                tracing::info!("Storing state under header_hash={header_hash} and unsigned_hash={unsigned_hash}");
                let tracked = TrackedState {
                    state: new_state,
                    opaque_data: remaining_opaque,
                };
                self.states.insert(header_hash, tracked.clone());
                self.states.insert(unsigned_hash, tracked);

                make_state_root(&state_root)
            }
            Err(e) => {
                tracing::error!("State transition failed: {e}");
                make_error(&format!("{e}"))
            }
        }
    }

    /// Debug: apply block step-by-step and compute state root after each step.
    fn debug_step_by_step(
        &self,
        state: &State,
        block: &grey_types::header::Block,
        opaque: &[([u8; 31], Vec<u8>)],
    ) {
        use grey_codec::header_codec::compute_header_hash as chh;

        let compute_root = |s: &State| {
            let kvs = state_serial::serialize_state_with_opaque(s, &self.config, opaque);
            grey_merkle::compute_state_root_from_kvs(&kvs)
        };

        let compute_component_hash = |s: &State, comp_idx: u8| {
            let kvs = state_serial::serialize_state_with_opaque(s, &self.config, opaque);
            kvs.iter()
                .find(|(k, _)| k[0] == comp_idx && k[1..] == [0u8; 30])
                .map(|(_, v)| (v.len(), grey_crypto::blake2b_256(v)))
        };

        let header = &block.header;
        let extrinsic = &block.extrinsic;
        let mut s = state.clone();

        tracing::info!("=== STEP-BY-STEP DEBUG for timeslot {} ===", header.timeslot);

        let base_root = compute_root(&s);
        tracing::info!("  Base state root: {base_root}");

        // Step 1: Timekeeping
        let prior_timeslot = s.timeslot;
        s.timeslot = header.timeslot;
        let r1 = compute_root(&s);
        tracing::info!("  After step 1 (timekeeping): {r1}");

        // Step 2: Judgments
        // (just call the public transition but track which component changed)
        // For simplicity, log component hashes for changed components

        // Step 4: Safrole
        grey_state::transition::debug_apply_safrole(&mut s, header, &self.config, prior_timeslot);
        if let Some((len, h)) = compute_component_hash(&s, 6) {
            tracing::info!("  After step 4 (safrole) C(6) eta: {len} bytes, hash={h}");
        }

        // Step 5: Assurances
        let available_reports = grey_state::transition::debug_process_assurances(
            &mut s, &extrinsic.assurances, header.timeslot, &self.config,
        );
        if let Some((len, h)) = compute_component_hash(&s, 10) {
            tracing::info!("  After step 5 (assurances) C(10) rho: {len} bytes, hash={h}");
        }
        tracing::info!("  Available reports from assurances: {}", available_reports.len());

        // Step 6: Guarantees
        let incoming_reports: Vec<&grey_types::work::WorkReport> = extrinsic.guarantees.iter().map(|g| &g.report).collect();
        let _ = grey_state::transition::debug_process_guarantees(
            &mut s, &extrinsic.guarantees, header.timeslot,
        );
        if let Some((len, h)) = compute_component_hash(&s, 10) {
            tracing::info!("  After step 6 (guarantees) C(10) rho: {len} bytes, hash={h}");
        }

        // Step 7: Accumulation
        let (accumulate_root, accumulation_gas_usage, _remaining_opaque) = grey_state::accumulate::run_accumulation(
            &self.config, &mut s, state.timeslot, available_reports.clone(), opaque,
        );
        tracing::info!("  After step 7 (accumulation) accumulate_root={accumulate_root}");
        for comp in [13u8, 14, 15, 16] {
            if let Some((len, h)) = compute_component_hash(&s, comp) {
                tracing::info!("  After step 7 C({comp}): {len} bytes, hash={h}");
            }
        }
        // Also check service account
        {
            let kvs = state_serial::serialize_state_with_opaque(&s, &self.config, opaque);
            for (k, v) in &kvs {
                if k[0] == 255 && k[1..] == [0u8; 30] {
                    let h = grey_crypto::blake2b_256(v);
                    tracing::info!("  After step 7 C(255): {} bytes, hash={h}", v.len());
                } else if k[1..] != [0u8; 30] {
                    let kh: String = k[..8].iter().map(|b| format!("{b:02x}")).collect();
                    let h = grey_crypto::blake2b_256(v);
                    tracing::info!("  After step 7 svc_data key={kh}...: {} bytes, hash={h}", v.len());
                }
            }
        }

        // Step 8: History
        {
            let header_hash = chh(header);
            let work_packages: Vec<(Hash, Hash)> = extrinsic.guarantees.iter().map(|g| {
                (g.report.package_spec.package_hash, g.report.package_spec.exports_root)
            }).collect();
            let input = grey_state::history::HistoryInput {
                header_hash,
                parent_state_root: header.state_root,
                accumulate_root,
                work_packages,
            };
            grey_state::history::update_history(&mut s.recent_blocks, &input);
        }
        if let Some((len, h)) = compute_component_hash(&s, 3) {
            tracing::info!("  After step 8 (history) C(3) beta: {len} bytes, hash={h}");
        }

        // Step 9: Statistics
        grey_state::statistics::update_statistics(
            &self.config, &mut s.statistics, state.timeslot, header.timeslot,
            header.author_index, extrinsic, &incoming_reports,
            &available_reports, &accumulation_gas_usage,
        );
        if let Some((len, h)) = compute_component_hash(&s, 13) {
            tracing::info!("  After step 9 (statistics) C(13) pi: {len} bytes, hash={h}");
        }

        // Step 10: Preimages (no change expected in this block)
        // Step 11: Auth rotation (no change expected)

        let final_root = compute_root(&s);
        tracing::info!("  Final step-by-step root: {final_root}");
        tracing::info!("=== END STEP-BY-STEP ===");
    }

    fn handle_get_state(&self, body: &[u8]) -> Vec<u8> {
        if body.len() < 32 {
            return make_error("get_state: header hash too short");
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&body[..32]);
        let header_hash = Hash(hash);

        tracing::info!("GetState for header: {header_hash}");

        let tracked = match self.states.get(&header_hash) {
            Some(ts) => ts,
            None => {
                return make_error("state not found for header hash");
            }
        };

        // Serialize state as KV pairs
        let kvs = state_serial::serialize_state_with_opaque(
            &tracked.state,
            &self.config,
            &tracked.opaque_data,
        );
        tracing::info!(
            "GetState: {} KV pairs, {} opaque entries, {} services",
            kvs.len(),
            tracked.opaque_data.len(),
            tracked.state.services.len()
        );

        // Encode as State message (disc + compact(kv_count) + KV pairs, no ancestry)
        let mut msg = Vec::new();
        msg.push(MSG_STATE);

        // SEQUENCE OF KeyValue
        encode_compact(kvs.len() as u64, &mut msg);
        for (key, value) in &kvs {
            // key: OCTET STRING (SIZE(31)) — no length prefix (fixed-size)
            msg.extend_from_slice(key);
            // value: OCTET STRING (compact length + bytes)
            encode_compact(value.len() as u64, &mut msg);
            msg.extend_from_slice(value);
        }

        msg
    }
}

/// Read a framed message (4-byte LE length + body).
fn read_message(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| format!("read length: {e}"))?;
    let len = u32::from_le_bytes(len_buf) as usize;

    if len == 0 {
        return Ok(Vec::new());
    }

    let mut body = vec![0u8; len];
    stream
        .read_exact(&mut body)
        .map_err(|e| format!("read body ({len} bytes): {e}"))?;
    Ok(body)
}

/// Write a framed message (4-byte LE length + body).
fn write_message(stream: &mut impl Write, data: &[u8]) -> Result<(), String> {
    let len = data.len() as u32;
    stream
        .write_all(&len.to_le_bytes())
        .map_err(|e| format!("write length: {e}"))?;
    stream
        .write_all(data)
        .map_err(|e| format!("write body: {e}"))?;
    stream.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

/// Create a StateRoot response message.
fn make_state_root(root: &Hash) -> Vec<u8> {
    let mut msg = Vec::with_capacity(33);
    msg.push(MSG_STATE_ROOT);
    msg.extend_from_slice(&root.0);
    msg
}

/// Create an Error response message.
fn make_error(text: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(MSG_ERROR);
    encode_compact(text.len() as u64, &mut msg);
    msg.extend_from_slice(text.as_bytes());
    msg
}

/// Decode a compact natural from a byte slice at the given position.
fn decode_compact_at(data: &[u8], pos: &mut usize) -> Result<u64, String> {
    if *pos >= data.len() {
        return Err("unexpected end of data".into());
    }
    let first = data[*pos];
    *pos += 1;

    if first < 128 {
        return Ok(first as u64);
    }

    let leading_ones = first.leading_ones() as usize;

    if leading_ones == 8 {
        if *pos + 8 > data.len() {
            return Err("unexpected end of data in compact".into());
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[*pos..*pos + 8]);
        *pos += 8;
        return Ok(u64::from_le_bytes(bytes));
    }

    let extra_bytes = leading_ones;
    if *pos + extra_bytes > data.len() {
        return Err("unexpected end of data in compact".into());
    }

    let mask = (1u8 << (8 - leading_ones - 1)) - 1;
    let first_value_bits = first & mask;

    let mut value: u64 = 0;
    for i in 0..extra_bytes {
        value |= (data[*pos + i] as u64) << (i * 8);
    }
    *pos += extra_bytes;

    value |= (first_value_bits as u64) << (extra_bytes * 8);

    Ok(value)
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse socket path from args or use default
    let socket_path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/jam_target.sock".into())
        .into();

    // Select config
    let config = match std::env::var("JAM_CONSTANTS").as_deref() {
        Ok("full") => {
            tracing::info!("Using full configuration");
            Config::full()
        }
        _ => {
            tracing::info!("Using tiny configuration (set JAM_CONSTANTS=full for full)");
            Config::tiny()
        }
    };

    // Clean up stale socket
    let _ = std::fs::remove_file(&socket_path);

    // Bind socket
    let listener = UnixListener::bind(&socket_path).unwrap_or_else(|e| {
        eprintln!("Failed to bind socket at {}: {e}", socket_path.display());
        std::process::exit(1);
    });

    tracing::info!("Listening on {}", socket_path.display());

    let mut target = ConformTarget::new(config);

    // Accept connections
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                target.handle_connection(&mut stream);
            }
            Err(e) => {
                tracing::error!("Accept error: {e}");
            }
        }
    }
}
