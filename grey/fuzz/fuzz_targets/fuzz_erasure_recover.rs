//! Fuzz target: random bytes into erasure coding recover().
//!
//! Feeds arbitrary chunk data and indices into the Reed-Solomon recovery
//! function. Verifies that no input causes a panic — only Ok or Err.

#![no_main]

use grey_erasure::{ErasureParams, recover};
use libfuzzer_sys::fuzz_target;

const TINY: ErasureParams = ErasureParams::TINY; // 2 data, 6 total

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }

    // Use first byte as original data length (1-255)
    let orig_len = data[0] as usize;
    if orig_len == 0 {
        return;
    }

    // Use second byte to select which 2 shard indices to provide
    let combo = data[1] as usize;
    let n = TINY.total_shards;
    let k = TINY.data_shards;

    // Pick 2 distinct indices from 0..n using combo byte
    let i = combo % n;
    let j = (combo / n + 1) % n;
    let (i, j) = if i == j { (i, (i + 1) % n) } else { (i, j) };

    // Split remaining data into k chunks
    let chunk_data = &data[2..];
    let chunk_len = if chunk_data.is_empty() {
        1
    } else {
        (chunk_data.len() / k).max(1)
    };

    let mut indexed = Vec::with_capacity(k);
    for (idx, shard_idx) in [i, j].iter().enumerate() {
        let start = idx * chunk_len;
        let end = (start + chunk_len).min(chunk_data.len());
        let chunk = if start < chunk_data.len() {
            chunk_data[start..end].to_vec()
        } else {
            vec![0; chunk_len]
        };
        indexed.push((chunk, *shard_idx));
    }

    // This should never panic — only Ok or Err
    let _ = recover(&TINY, &indexed, orig_len);
});
