//! Fuzz target: random bytes into state KV decoding logic.
//!
//! Re-implements grey_store::decode_state_kvs logic to verify that
//! parsing arbitrary bytes as state KV pairs never panics — only
//! produces Some or None. This mirrors the exact store decoder.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Mirrors grey_store::decode_state_kvs — parses (31-byte key, variable-length value) pairs.
/// Returns None on any malformed input, never panics.
fn decode_state_kvs_fuzz(data: &[u8]) -> Option<Vec<([u8; 31], Vec<u8>)>> {
    if data.len() < 4 {
        return None;
    }
    let count = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
    let mut pos = 4;
    // Same OOM guard as the real implementation
    if count > (data.len() - pos) / 35 {
        return None;
    }
    let mut kvs = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 31 + 4 > data.len() {
            return None;
        }
        let mut key = [0u8; 31];
        key.copy_from_slice(&data[pos..pos + 31]);
        pos += 31;
        let vlen = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;
        if pos + vlen > data.len() {
            return None;
        }
        kvs.push((key, data[pos..pos + vlen].to_vec()));
        pos += vlen;
    }
    Some(kvs)
}

/// Encode KV pairs back — for roundtrip verification.
fn encode_state_kvs(kvs: &[([u8; 31], Vec<u8>)]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(kvs.len() as u32).to_le_bytes());
    for (key, val) in kvs {
        buf.extend_from_slice(key);
        buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
        buf.extend_from_slice(val);
    }
    buf
}

fuzz_target!(|data: &[u8]| {
    if let Some(kvs) = decode_state_kvs_fuzz(data) {
        // Roundtrip: encode the decoded KVs and decode again
        let encoded = encode_state_kvs(&kvs);
        let re_decoded = decode_state_kvs_fuzz(&encoded);
        assert_eq!(re_decoded, Some(kvs), "roundtrip failed");
    }
    // If decode returns None, that's fine — malformed input
});
