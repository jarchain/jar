//! Header encoding functions E(H) and EU(H) (eq C.22-C.23).
//!
//! Shared between grey-state (for computing header hashes during transitions)
//! and grey (for the conformance binary).

use crate::encode::encode_compact;
use grey_types::header::Header;

/// Encode the full header E(H) = EU(H) ++ HS.
pub fn encode_header(header: &Header) -> Vec<u8> {
    let mut buf = encode_header_unsigned(header);
    // HS: seal (96 bytes)
    buf.extend_from_slice(&header.seal.0);
    buf
}

/// Encode the unsigned portion of a header EU(H) (eq C.23).
///
/// Field order: HP, HR, HX, E4(HT), ¿HE, ¿HW, E2(HI), HV, ↕HO
pub fn encode_header_unsigned(header: &Header) -> Vec<u8> {
    let mut buf = Vec::new();

    // HP: parent_hash (32 bytes)
    buf.extend_from_slice(&header.parent_hash.0);
    // HR: state_root (32 bytes)
    buf.extend_from_slice(&header.state_root.0);
    // HX: extrinsic_hash (32 bytes)
    buf.extend_from_slice(&header.extrinsic_hash.0);
    // E4(HT): timeslot (4 bytes LE)
    buf.extend_from_slice(&header.timeslot.to_le_bytes());

    // ¿HE: epoch_marker (optional/discriminated)
    match &header.epoch_marker {
        None => buf.push(0),
        Some(em) => {
            buf.push(1);
            // entropy: Hash (η₀)
            buf.extend_from_slice(&em.entropy.0);
            // entropy_previous: Hash (η₁)
            buf.extend_from_slice(&em.entropy_previous.0);
            // keys: V × (Bandersnatch(32) + Ed25519(32))
            for (bk, ek) in &em.validators {
                buf.extend_from_slice(&bk.0);
                buf.extend_from_slice(&ek.0);
            }
        }
    }

    // ¿HW: tickets_marker (optional/discriminated)
    match &header.tickets_marker {
        None => buf.push(0),
        Some(tickets) => {
            buf.push(1);
            // E tickets, each = Hash(32) + u8(1) = 33 bytes
            for ticket in tickets {
                buf.extend_from_slice(&ticket.id.0);
                buf.push(ticket.attempt);
            }
        }
    }

    // E2(HI): author_index (2 bytes LE)
    buf.extend_from_slice(&header.author_index.to_le_bytes());

    // HV: vrf_signature (96 bytes)
    buf.extend_from_slice(&header.vrf_signature.0);

    // ↕HO: offenders_marker (compact length + Ed25519 keys)
    encode_compact(header.offenders_marker.len() as u64, &mut buf);
    for key in &header.offenders_marker {
        buf.extend_from_slice(&key.0);
    }

    buf
}

/// Compute header hash H(E(H)) — blake2b-256 of the full header encoding.
///
/// Per GP eq 5.1: HP ≡ H(E(P(H)))
pub fn compute_header_hash(header: &Header) -> grey_types::Hash {
    let encoded = encode_header(header);
    grey_crypto::blake2b_256(&encoded)
}

/// Compute unsigned header hash H(EU(H)) — blake2b-256 of the unsigned header encoding.
///
/// Used by the fuzz-proto GetState lookup.
pub fn compute_unsigned_header_hash(header: &Header) -> grey_types::Hash {
    let encoded = encode_header_unsigned(header);
    grey_crypto::blake2b_256(&encoded)
}
