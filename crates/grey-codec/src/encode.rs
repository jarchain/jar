//! Encoding functions (Appendix C of the Gray Paper).

/// Trait for types that can be encoded to the JAM wire format.
pub trait Encode {
    /// Encode this value, appending bytes to the given buffer.
    fn encode_to(&self, buf: &mut Vec<u8>);

    /// Encode this value and return the bytes.
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_to(&mut buf);
        buf
    }
}

/// Encode a variable-length natural number (eq C.1-C.4).
///
/// Used as a length prefix for variable-length sequences.
pub fn encode_natural(value: usize, buf: &mut Vec<u8>) {
    let mut v = value;
    loop {
        if v < 128 {
            buf.push(v as u8);
            break;
        }
        buf.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
}

// Fixed-width little-endian integer encodings (eq C.12).

impl Encode for u8 {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.push(*self);
    }
}

impl Encode for u16 {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_le_bytes());
    }
}

impl Encode for u32 {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_le_bytes());
    }
}

impl Encode for u64 {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_le_bytes());
    }
}

impl Encode for bool {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.push(if *self { 1 } else { 0 });
    }
}

impl Encode for [u8; 32] {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}

impl Encode for [u8; 64] {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}

impl Encode for [u8; 96] {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}

impl Encode for grey_types::Hash {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.0);
    }
}

impl Encode for grey_types::Ed25519PublicKey {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.0);
    }
}

impl Encode for grey_types::BandersnatchPublicKey {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.0);
    }
}

impl Encode for grey_types::BandersnatchSignature {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.0);
    }
}

impl Encode for grey_types::Ed25519Signature {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.0);
    }
}

/// Encode a variable-length sequence with length prefix.
impl<T: Encode> Encode for Vec<T> {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        encode_natural(self.len(), buf);
        for item in self {
            item.encode_to(buf);
        }
    }
}

/// Encode an optional value with a discriminator byte (eq C.5-C.7).
impl<T: Encode> Encode for Option<T> {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        match self {
            None => buf.push(0),
            Some(val) => {
                buf.push(1);
                val.encode_to(buf);
            }
        }
    }
}

// --- Encode impls for tuples ---

impl<A: Encode, B: Encode> Encode for (A, B) {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.0.encode_to(buf);
        self.1.encode_to(buf);
    }
}

// --- Encode impls for protocol types (Appendix C) ---

use grey_types::header::*;
use grey_types::work::*;

impl Encode for RefinementContext {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.anchor.encode_to(buf);
        self.state_root.encode_to(buf);
        self.beefy_root.encode_to(buf);
        self.lookup_anchor.encode_to(buf);
        self.lookup_anchor_timeslot.encode_to(buf);
        self.prerequisites.encode_to(buf);
    }
}

impl Encode for WorkResult {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        match self {
            WorkResult::Ok(data) => {
                buf.push(0);
                data.encode_to(buf);
            }
            WorkResult::OutOfGas => buf.push(1),
            WorkResult::Panic => buf.push(2),
            WorkResult::InvalidExportCount => buf.push(3),
            WorkResult::DigestTooLarge => buf.push(4),
            WorkResult::CodeNotAvailable => buf.push(5),
            WorkResult::CodeTooLarge => buf.push(6),
        }
    }
}

impl Encode for WorkDigest {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.service_id.encode_to(buf);
        self.code_hash.encode_to(buf);
        self.payload_hash.encode_to(buf);
        self.gas_limit.encode_to(buf);
        self.result.encode_to(buf);
        self.gas_used.encode_to(buf);
        self.imports_count.encode_to(buf);
        self.extrinsics_count.encode_to(buf);
        self.extrinsics_size.encode_to(buf);
        self.exports_count.encode_to(buf);
    }
}

impl Encode for ImportSegment {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.hash.encode_to(buf);
        self.index.encode_to(buf);
    }
}

impl Encode for WorkItem {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.service_id.encode_to(buf);
        self.code_hash.encode_to(buf);
        self.gas_limit.encode_to(buf);
        self.accumulate_gas_limit.encode_to(buf);
        self.exports_count.encode_to(buf);
        self.payload.encode_to(buf);
        self.imports.encode_to(buf);
        self.extrinsics.encode_to(buf);
    }
}

impl Encode for AvailabilitySpec {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.package_hash.encode_to(buf);
        self.bundle_length.encode_to(buf);
        self.erasure_root.encode_to(buf);
        self.segment_root.encode_to(buf);
        self.segment_count.encode_to(buf);
    }
}

impl Encode for WorkReport {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.availability.encode_to(buf);
        self.context.encode_to(buf);
        self.core_index.encode_to(buf);
        self.authorizer_hash.encode_to(buf);
        self.authorizer_trace.encode_to(buf);
        self.segment_root_lookup.encode_to(buf);
        self.auth_gas_used.encode_to(buf);
        self.digests.encode_to(buf);
    }
}

/// Encode a BTreeMap as a sorted sequence of key-value pairs (eq C.10).
impl<K: Encode, V: Encode> Encode for std::collections::BTreeMap<K, V> {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        encode_natural(self.len(), buf);
        for (k, v) in self.iter() {
            k.encode_to(buf);
            v.encode_to(buf);
        }
    }
}

impl Encode for WorkPackage {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.authorization_hash.encode_to(buf);
        self.authorization_code.encode_to(buf);
        self.authorization_config.encode_to(buf);
        self.prerequisites.encode_to(buf);
        self.auth_token.encode_to(buf);
        self.items.encode_to(buf);
    }
}

impl Encode for Ticket {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.id.encode_to(buf);
        self.entry_index.encode_to(buf);
    }
}

impl Encode for TicketProof {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.entry_index.encode_to(buf);
        self.proof.encode_to(buf);
    }
}

impl Encode for Verdict {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.report_hash.encode_to(buf);
        self.age.encode_to(buf);
        self.judgments.encode_to(buf);
    }
}

impl Encode for Judgment {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.is_valid.encode_to(buf);
        self.validator_index.encode_to(buf);
        self.signature.encode_to(buf);
    }
}

impl Encode for Culprit {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.validator_key.encode_to(buf);
        self.report_hash.encode_to(buf);
        self.signature.encode_to(buf);
    }
}

impl Encode for Fault {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.validator_key.encode_to(buf);
        self.report_hash.encode_to(buf);
        self.is_valid.encode_to(buf);
        self.signature.encode_to(buf);
    }
}

impl Encode for DisputesExtrinsic {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.verdicts.encode_to(buf);
        self.culprits.encode_to(buf);
        self.faults.encode_to(buf);
    }
}

impl Encode for Assurance {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.anchor.encode_to(buf);
        // Bitfield is encoded as raw bytes (ceil(bits/8) bytes)
        encode_bitfield(&self.bitfield, buf);
        self.validator_index.encode_to(buf);
        self.signature.encode_to(buf);
    }
}

/// Encode a fixed-size bitfield as bytes (MSB-first within each byte).
fn encode_bitfield(bits: &[bool], buf: &mut Vec<u8>) {
    let byte_count = (bits.len() + 7) / 8;
    for i in 0..byte_count {
        let mut byte = 0u8;
        for j in 0..8 {
            let bit_idx = i * 8 + j;
            if bit_idx < bits.len() && bits[bit_idx] {
                byte |= 1 << (7 - j);
            }
        }
        buf.push(byte);
    }
}

impl Encode for Guarantee {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.report.encode_to(buf);
        self.timeslot.encode_to(buf);
        self.credentials.encode_to(buf);
    }
}

impl Encode for Extrinsic {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.tickets.encode_to(buf);
        self.disputes.encode_to(buf);
        self.preimages.encode_to(buf);
        self.assurances.encode_to(buf);
        self.guarantees.encode_to(buf);
    }
}

impl Encode for EpochMarker {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.entropy.encode_to(buf);
        self.entropy_previous.encode_to(buf);
        self.validators.encode_to(buf);
    }
}

impl Encode for WinningTicketsMarker {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.tickets.encode_to(buf);
    }
}

impl Encode for Header {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.parent_hash.encode_to(buf);
        self.state_root.encode_to(buf);
        self.extrinsic_hash.encode_to(buf);
        self.timeslot.encode_to(buf);
        self.epoch_marker.encode_to(buf);
        self.winning_tickets_marker.encode_to(buf);
        self.offenders_marker.encode_to(buf);
        self.author_index.encode_to(buf);
        self.vrf_signature.encode_to(buf);
        self.seal.encode_to(buf);
    }
}

impl Encode for Block {
    fn encode_to(&self, buf: &mut Vec<u8>) {
        self.header.encode_to(buf);
        self.extrinsic.encode_to(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_natural_small() {
        let mut buf = Vec::new();
        encode_natural(0, &mut buf);
        assert_eq!(buf, vec![0]);

        let mut buf = Vec::new();
        encode_natural(127, &mut buf);
        assert_eq!(buf, vec![127]);
    }

    #[test]
    fn test_encode_natural_large() {
        let mut buf = Vec::new();
        encode_natural(128, &mut buf);
        assert_eq!(buf, vec![0x80, 0x01]);

        let mut buf = Vec::new();
        encode_natural(300, &mut buf);
        assert_eq!(buf, vec![0xAC, 0x02]);
    }

    #[test]
    fn test_encode_u32_le() {
        let val: u32 = 0x12345678;
        let encoded = val.encode();
        assert_eq!(encoded, vec![0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_encode_hash() {
        let hash = grey_types::Hash([0xAB; 32]);
        let encoded = hash.encode();
        assert_eq!(encoded.len(), 32);
        assert!(encoded.iter().all(|&b| b == 0xAB));
    }

    /// Helper: decode a 0x-prefixed hex string to bytes.
    fn decode_hex(s: &str) -> Vec<u8> {
        hex::decode(s.strip_prefix("0x").unwrap_or(s)).expect("bad hex")
    }

    /// Helper: decode hex string to a Hash.
    fn hash_from_hex(s: &str) -> grey_types::Hash {
        let bytes = decode_hex(s);
        let mut h = [0u8; 32];
        h.copy_from_slice(&bytes);
        grey_types::Hash(h)
    }

    #[test]
    fn test_codec_refine_context() {
        let json: serde_json::Value = serde_json::from_str(
            include_str!("../../../test-vectors/codec/tiny/refine_context.json"),
        )
        .unwrap();
        let expected = include_bytes!("../../../test-vectors/codec/tiny/refine_context.bin");

        let ctx = RefinementContext {
            anchor: hash_from_hex(json["anchor"].as_str().unwrap()),
            state_root: hash_from_hex(json["state_root"].as_str().unwrap()),
            beefy_root: hash_from_hex(json["beefy_root"].as_str().unwrap()),
            lookup_anchor: hash_from_hex(json["lookup_anchor"].as_str().unwrap()),
            lookup_anchor_timeslot: json["lookup_anchor_slot"].as_u64().unwrap() as u32,
            prerequisites: json["prerequisites"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| hash_from_hex(v.as_str().unwrap()))
                .collect(),
        };

        let encoded = ctx.encode();
        assert_eq!(
            encoded,
            expected.as_slice(),
            "refine_context encoding mismatch"
        );
    }
}
