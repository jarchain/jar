//! State Merklization, Merkle tries, and Merkle Mountain Ranges (Appendices D & E).
//!
//! Implements:
//! - Binary Patricia Merkle Trie with 64-byte nodes
//! - State key construction C
//! - State serialization T(σ)
//! - Well-balanced binary Merkle tree MB
//! - Constant-depth binary Merkle tree M
//! - Merkle Mountain Ranges and Belts

pub mod mmr;
pub mod state_serial;
pub mod trie;

use grey_types::config::Config;
use grey_types::state::State;
use grey_types::Hash;

/// Compute the state Merklization Mσ(σ) — compose T(σ) with merkle_root.
pub fn compute_state_root(state: &State, config: &Config) -> Hash {
    let kvs = state_serial::serialize_state(state, config);
    compute_state_root_from_kvs(&kvs)
}

/// Compute the state root from pre-serialized KV pairs.
pub fn compute_state_root_from_kvs(kvs: &[([u8; 31], Vec<u8>)]) -> Hash {
    let refs: Vec<(&[u8], &[u8])> = kvs.iter().map(|(k, v)| (k.as_slice(), v.as_slice())).collect();
    trie::merkle_root(&refs)
}

/// GP node function N(v, H) (eq E.1) — returns raw bytes (blob or hash).
///
/// - |v| = 0: H_0 (32 zero bytes)
/// - |v| = 1: v_0 (raw blob, NOT hashed)
/// - |v| > 1: H("node" ⌢ N(left, H) ⌢ N(right, H))
///
/// Note: Reference implementations (Strawberry/Go) use "node" without '$' prefix.
fn merkle_node(leaves: &[&[u8]], hash_fn: fn(&[u8]) -> Hash) -> Vec<u8> {
    match leaves.len() {
        0 => vec![0u8; 32],
        1 => leaves[0].to_vec(),
        n => {
            let mid = (n + 1) / 2; // ceil(n/2)
            let left = merkle_node(&leaves[..mid], hash_fn);
            let right = merkle_node(&leaves[mid..], hash_fn);
            let mut input = Vec::with_capacity(4 + left.len() + right.len());
            input.extend_from_slice(b"node");
            input.extend_from_slice(&left);
            input.extend_from_slice(&right);
            hash_fn(&input).0.to_vec()
        }
    }
}

/// Compute the well-balanced binary Merkle tree root MB (eq E.1).
///
/// - |v| = 1: H(v_0) (hash the single item)
/// - otherwise: N(v, H)
///
/// MB: (⟦B⟧, B → H) → H
pub fn balanced_merkle_root(leaves: &[&[u8]], hash_fn: fn(&[u8]) -> Hash) -> Hash {
    if leaves.len() == 1 {
        return hash_fn(leaves[0]);
    }
    // For 0 or 2+ items, delegate to N
    let result = merkle_node(leaves, hash_fn);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    Hash(hash)
}

/// State-key constructor C (eq D.1).
///
/// Maps state component indices (and optionally service IDs) to 31-byte keys.
pub fn state_key_from_index(index: u8) -> [u8; 31] {
    let mut key = [0u8; 31];
    key[0] = index;
    key
}

/// State-key constructor C for service account components (eq D.1).
pub fn state_key_for_service(index: u8, service_id: u32) -> [u8; 31] {
    let mut key = [0u8; 31];
    let s = service_id.to_le_bytes();
    key[0] = index;
    key[1] = s[0];
    key[2] = 0;
    key[3] = s[1];
    key[4] = 0;
    key[5] = s[2];
    key[6] = 0;
    key[7] = s[3];
    key
}

/// State-key constructor C for service storage items (eq D.1).
pub fn state_key_for_storage(service_id: u32, hash: &Hash) -> [u8; 31] {
    let s = service_id.to_le_bytes();
    let a = grey_crypto::blake2b_256(&hash.0);
    let mut key = [0u8; 31];
    key[0] = s[0];
    key[1] = a.0[0];
    key[2] = s[1];
    key[3] = a.0[1];
    key[4] = s[2];
    key[5] = a.0[2];
    key[6] = s[3];
    key[7] = a.0[3];
    key[8..31].copy_from_slice(&a.0[4..27]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_key_from_index() {
        let key = state_key_from_index(6);
        assert_eq!(key[0], 6);
        assert!(key[1..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_balanced_merkle_root_single() {
        let leaf = b"hello";
        let root = balanced_merkle_root(&[leaf.as_ref()], grey_crypto::blake2b_256);
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_balanced_merkle_root_empty() {
        let root = balanced_merkle_root(&[], grey_crypto::blake2b_256);
        assert_eq!(root, Hash::ZERO);
    }
}
