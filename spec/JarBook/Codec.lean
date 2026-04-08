import VersoManual
import Jar.Codec
import Jar.Codec.Jar1

open Verso.Genre Manual
open Jar.Codec

set_option verso.docstring.allowMissing true

#doc (Manual) "Serialization Codec" =>

Binary encoding of protocol types for hashing and network transmission (GP Appendix C).
All encodings are little-endian.

jar1 uses a distinct codec (`Codec.Jar1`) that replaces JAM's variable-length natural
encoding with fixed u32 little-endian count prefixes throughout. This simplifies parsing
and produces deterministic-length encodings for all collection types.

# Primitive Encoders

{docstring Jar.Codec.encodeFixedNat}

{docstring Jar.Codec.decodeFixedNat}

{docstring Jar.Codec.encodeNat}

{docstring Jar.Codec.encodeOption}

{docstring Jar.Codec.encodeLengthPrefixed}

{docstring Jar.Codec.encodeBits}

# Work Types

{docstring Jar.Codec.encodeWorkResult}

{docstring Jar.Codec.encodeAvailSpec}

{docstring Jar.Codec.encodeRefinementContext}

{docstring Jar.Codec.encodeWorkDigest}

{docstring Jar.Codec.encodeWorkReport}

# Extrinsic Encoders

{docstring Jar.Codec.encodeTicket}

{docstring Jar.Codec.encodeTicketProof}

{docstring Jar.Codec.encodeAssurance}

{docstring Jar.Codec.encodeGuarantee}

{docstring Jar.Codec.encodeDisputes}

{docstring Jar.Codec.encodePreimages}

# Block Encoding

{docstring Jar.Codec.encodeEpochMarker}

{docstring Jar.Codec.encodeUnsignedHeader}

{docstring Jar.Codec.encodeHeader}

{docstring Jar.Codec.encodeExtrinsic}

{docstring Jar.Codec.encodeBlock}

# Decoder Monad

Decoding is the inverse of encoding: binary bytes back to structured types.
The decoder uses a monadic combinator approach — a `Decoder α` consumes bytes
from a `DecodeState` and returns `Option (α × DecodeState)`.

{docstring Jar.Codec.DecodeState}

{docstring Jar.Codec.Decoder}

{docstring Jar.Codec.Decoder.run}

{docstring Jar.Codec.Decoder.bind}

{docstring Jar.Codec.Decoder.readBytes}

# Primitive Decoders

{docstring Jar.Codec.Decoder.decodeArray}

{docstring Jar.Codec.Decoder.decodeFixedNat}

{docstring Jar.Codec.Decoder.decodeNat}

{docstring Jar.Codec.Decoder.decodeOption}

{docstring Jar.Codec.Decoder.decodeLengthPrefixed}

{docstring Jar.Codec.Decoder.decodeBits}

# Block Decoding

{docstring Jar.Codec.Decoder.decodeHeader}

{docstring Jar.Codec.Decoder.decodeExtrinsic}

{docstring Jar.Codec.Decoder.decodeBlock}

# jar1 Codec

The jar1 variant uses `Codec.Jar1` — all collection counts and byte lengths
are encoded as u32 LE (4 bytes), replacing JAM's variable-length natural encoding.
Numeric fields (service IDs, gas values, etc.) use fixed-width LE encoding throughout.

{docstring Jar.Codec.Jar1.encodeCountPrefixed}

{docstring Jar.Codec.Jar1.encodeLengthPrefixed}

{docstring Jar.Codec.Jar1.encodeWorkReport}

{docstring Jar.Codec.Jar1.encodeBlock}
