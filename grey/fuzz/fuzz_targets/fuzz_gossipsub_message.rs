//! Fuzz target: random bytes into gossipsub message decode.
//!
//! Simulates receiving arbitrary bytes on each gossipsub topic
//! (blocks, finality, guarantees, assurances, announcements,
//! tickets, equivocation) and verifies that decoding never panics.
//!
//! This covers the decode path that network message handlers invoke
//! when receiving gossipsub messages from peers.

#![no_main]

use grey_codec::Decode;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Each gossipsub topic carries a different message type.
    // Fuzz all of them to ensure no panic on arbitrary input.

    // Topic: blocks — Block
    let _ = grey_types::header::Block::decode(data);

    // Topic: finality — FinalityVotes (encoded as Vec<Ed25519Signature>)
    let _ = Vec::<grey_types::Ed25519Signature>::decode(data);

    // Topic: guarantees — Guarantee
    let _ = grey_types::header::Guarantee::decode(data);

    // Topic: assurances — Assurance
    let _ = grey_types::header::Assurance::decode(data);

    // Topic: tickets — Ticket
    let _ = grey_types::header::Ticket::decode(data);

    // Topic: equivocation — EquivocationEvidence
    let _ = grey_types::header::EquivocationEvidence::decode(data);

    // Topic: announcements — (BandersnatchPublicKey, Ed25519PublicKey) pair
    let _: Result<(grey_types::BandersnatchPublicKey, grey_types::Ed25519PublicKey), _> =
        (|| {
            let key1 = grey_types::BandersnatchPublicKey::decode(data)?;
            let key2 = grey_types::Ed25519PublicKey::decode(data)?;
            Ok((key1, key2))
        })();
});
