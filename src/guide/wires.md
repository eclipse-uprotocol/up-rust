# Adding a wire format

A selected wire defines format identities, payload codecs, and a metadata layout.
Transport adapters use these definitions to encode and validate frames.
The example below defines the identities and a codec for a little-endian `u32`.

## Why three traits?

The traits have separate responsibilities:

* `UWire` declares wire and payload-family identities, the metadata layout,
  and the format version used for compatibility checks.
* [`PayloadCodec`](crate::PayloadCodec) identifies the encoding produced by
  a codec implementation.
* `UWirePayload<T>` selects the codec for payload type `T`. One wire can use
  different codecs for different payload types. The associated codec type also
  lets generic code require matching codecs when necessary.

A marker can implement the codec itself and select `Codec = Self` through
`UWirePayload<T>`. The [`bind_wire_self_codec!`](crate::bind_wire_self_codec)
macro generates that association. A blanket implementation would prevent a
wire from selecting a separate codec for another payload type.

This example encodes and decodes a `u32` in memory:

```rust
# #[cfg(feature = "wire-implementer-api")]
# fn main() -> Result<(), up_rust::payload::UWireError> {
use up_rust::payload::codec::{DecodePayload, EncodePayload, PayloadCodec, PayloadLayout};
use up_rust::payload::UWireError;
use up_rust::{PayloadEncoding, UWire, WireIdentity};

struct DemoU32Wire;

impl UWire for DemoU32Wire {
    // Experimental compact codes (0x8000..=0xFFFE): literal-labeled, unregistered.
    const WIRE_ID: WireIdentity = WireIdentity::new("guide.demo.u32-le", 0x8042);
    const PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("guide.demo.u32", 0x8042);
    const METADATA_LAYOUT_ID: WireIdentity =
        WireIdentity::new("uprotocol.native-prefix.v1", 0x0001);
    const FORMAT_VERSION: u16 = 1;
}

impl PayloadCodec for DemoU32Wire {
    fn codec_name() -> &'static str { "demo-u32-le" }
    fn payload_identity(_profile: Option<&up_rust::NativeProfileAgreement>) -> Result<up_rust::PayloadIdentity, UWireError> {
        Ok(up_rust::PayloadIdentity::Fixed(PayloadEncoding::RAW))
    }
}

impl EncodePayload<u32> for DemoU32Wire {
    fn payload_layout(_value: &u32) -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(4, 4) // fixed size, four-byte alignment
    }
    fn encode_payload(value: &u32, dst: &mut [u8]) -> Result<(), UWireError> {
        dst.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

impl<'a> DecodePayload<'a, u32> for DemoU32Wire {
    fn decode_payload(src: &'a [u8]) -> Result<u32, UWireError> {
        let bytes: [u8; 4] = src.try_into().map_err(|_| {
            UWireError::invalid_payload("demo-u32-le payload must be exactly 4 bytes")
        })?;
        Ok(u32::from_le_bytes(bytes))
    }
}

// Round-trip: measure, encode in place, decode.
let value: u32 = 0xDEAD_BEEF;
let layout = DemoU32Wire::payload_layout(&value)?;
let mut buf = vec![0u8; layout.len()];
DemoU32Wire::encode_payload(&value, &mut buf)?;
assert_eq!(DemoU32Wire::decode_payload(&buf)?, value);
assert_eq!(DemoU32Wire::WIRE_ID.compact_id(), 0x8042); // experimental range
# Ok::<(), up_rust::payload::UWireError>(())
# }
# #[cfg(not(feature = "wire-implementer-api"))]
# fn main() {}
```

Implementation steps:

1. Write down the governing representation/version, supported subset, byte
   order, framing markers, options, padding, canonical-byte and complete-input
   rules before implementing the codec.
2. Implement `PayloadCodec`, `EncodePayload`, `DecodePayload`, and, when ordered
   reader input is supported, `ReadDecodePayload`.
3. Define selected-wire and payload-family identities. Compact codes are unique
   within their own namespace. Codes in the experimental range must stay
   literal-labeled and must not be described as registered or stable.
4. Implement `UWire` on a marker and associate each payload type through
   `UWirePayload<T>`. The associated codec can be a separate type; it is not
   required to be the marker.
5. Choose a documented metadata profile. `NativePrefixFrameMetadataCodec` is
   the canonical field-block profile.
6. Run wrong-wire, wrong-payload-family, unknown-layout, malformed-envelope,
   exact/short/overlong/oversized-reader, independent-vector, trailing-byte,
   golden, and round-trip tests.

`PayloadCodecIdentity` already receives a blanket `PayloadCodec` implementation. Do
not add a second `PayloadCodec` implementation for the same format marker; Rust
will correctly reject the overlap with E0119. Put encode/decode capability on
the existing marker or use a distinct codec type and select it through
`UWirePayload<T>`.

The external XCDRv2, Arrow, and OMGIDL crates demonstrate different profiles
without adding transport-specific codec code. Their implementation details do
not override the trait contracts. In particular, a `ReadDecodePayload`
implementation allocates only after applying a finite limit, consumes exactly
the declared length, distinguishes early EOF, probes one additional byte to
reject overlong input, and returns errors rather than panicking.

`payload_layout` is a measurement operation. A variable-length codec may need a
probe serialization or traversal and may do equivalent work again during
`encode_payload`. Benchmark these as separate phases; never label layout probing
as zero-cost.

## Why identities are strict

The adapter checks the `UPWM` prefix before decoding. Compact codes must be
unique within their namespace so that incompatible wire formats can be
distinguished. Experimental codes (`0x8000..=0xFFFE`) have names but no global
assignment guarantee; participating implementations must agree on their meaning.
These selected-wire identities are separate from payload-encoding IDs.

Enable `wire-implementer-api` for the complete public authoring surface instead
of relying on feature unification from a larger workspace.
