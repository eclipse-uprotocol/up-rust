# Adding a wire format (and stable payloads)

A wire format teaches uProtocol to carry one payload encoding — CDR,
Arrow IPC, your in-house format — over every capable transport at once.
The whole job is one crate: a marker type, a codec, and identity
constants. In outline:

the runnable demo below is the complete outline — a marker
type carrying identity constants, a codec that measures then writes in
place, and the association binding payload types to that codec.

## Why three traits?

The split is load-bearing, and the demo shows each part earning its
keep:

* [`UWire`](crate::UWire) is the wire's **identity** — the constants two
  peers negotiate on ("are these bytes even mine to decode?"). One
  identity, forever.
* [`PayloadCodec`](crate::PayloadCodec) is one **encoding
  implementation** — a name plus the payload encoding it produces.
* `UWirePayload<T>` is the **binding**: "on this wire, type `T` uses
  that codec." It is separate because one wire identity routinely serves
  many payload types — a `LidarWire` can carry a `PointCloud` through a
  fixed-layout codec and a `StatusBlob` through a raw-bytes codec, one
  negotiated identity, two codecs — which a merged trait cannot express.
  Downstream dispatch also relies on the codec being a nameable type:
  generic wire routing constrains on codec equality, which only works
  when the codec is a type, not a method set folded into the marker.

For the common all-in-one case the marker implements everything and
binds `Codec = Self`, as the demo does —
[`bind_wire_self_codec!`](crate::bind_wire_self_codec) writes those
binding lines for you. (A trait-level default or blanket impl was
considered and rejected: the default form is unstable Rust, and a
blanket impl would forbid, by coherence, a wire binding a different
codec for some payload type — the case this split exists to support.)

Before the checklist, a demo wire **running end to end** — identity and
codec — carrying a `u32` little-endian:

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
    fn payload_encoding() -> PayloadEncoding { PayloadEncoding::RAW }
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

Need a custom metadata profile instead of the native prefix? Construct
directly: `UWireTransport::new(core, wire, my_codec)` — that constructor
*is* the composition; the convenience above is it with the canonical
codec filled in.

One line composes the demo over any encoded core —
`core.into_native_prefix_wire_transport(DemoU32Wire)` — and the
transport then speaks typed `u32` payloads over whatever the core
carries. The [zero-copy tutorial](crate::guide::transports::zero_copy)
compiles that composition, twice, to show the core code never changes
when the wire does. Everything else on this page is the packaging around exactly this:
identity constants that let receivers reject foreign bytes early, and a
codec pair that measures then writes in place.

The steps, in the order that avoids rework:

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
   the canonical field-block profile. Free `with_*_native_prefix` constructors
   are ecosystem conveniences, not requirements on `UWire`.
6. Run wrong-wire, wrong-payload-family, unknown-layout, malformed-envelope,
   exact/short/overlong/oversized-reader, independent-vector, trailing-byte,
   golden, and round-trip tests.

`PayloadFormat` already receives a blanket `PayloadCodec` implementation. Do
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

A wire identity is what lets a receiver reject bytes *before* handing
them to the wrong decoder: the adapter checks the `UPWM` prefix first,
so a CDR payload never reaches an Arrow decoder as garbage-in. That only
works if compact codes never collide — hence uniqueness within each
namespace. The experimental range (`0x8000..=0xFFFE`) exists so teams
can develop wires today without a registry; codes there are
literal-labeled precisely because nothing guarantees two projects chose
differently. When uProtocol operates an identity registry, registered
codes become stable and the experimental range stays what it is: a
sandbox.

## Feature and import doors

Use one role feature rather than relying on feature unification from a larger
workspace:

```text
ordinary selected-wire user: selected-wire-user-api
wire author:                 wire-implementer-api
physical transport author:   transport-implementer-api
typed uninitialized TX user: selected-wire-user-api + zero-copy-transport
communication roles:         communication
```

Each role needs exactly its own feature — depend on `up-rust` with
`default-features = false` and the one feature above.

## Error categories

Public boundaries preserve stable categories even when detailed internal errors
differ:

| Failure | Public status category |
| --- | --- |
| malformed caller input or selected-wire payload | invalid argument |
| unsupported family, wire, profile, or operation | unimplemented |
| unavailable backend, peer, or discovery state | unavailable or not found, as documented by the binding |
| bounded queue, loan, listener, or storage exhaustion | resource exhausted |
| violated lifecycle invariant or unexpected implementation failure | failed precondition or internal |

`UWireError` provides codec diagnostics and currently converts to invalid
argument at generic wire boundaries. A transport must not flatten backend,
exhaustion, or lifecycle statuses into codec errors. Tests inject each public
category at the boundary where it is promised.

## Manual TX: see the zero-copy escape hatch

The ordered contract a manual (untyped/initialized-loan) sender must
uphold lives with the concrete lifecycle it belongs to: the
[zero-copy tutorial's escape hatch](crate::guide::transports::zero_copy).
Prefer the typed helpers; they perform those steps for you.

## The two typed paths (and which one is truly zero-copy)

| path | serializations | intermediate copies | encode step exists? |
| --- | --- | --- | --- |
| codec path (`EncodePayload`) | one | zero | yes — it IS the serialization, written directly into the loaned buffer |
| stable path (`StablePayload` derive) | **zero** | zero | no — the in-memory layout is the wire layout; typed init writes fields in place |

"Zero-copy" on the codec path is about *where* the serialized bytes
land (straight into transport storage, no staging buffer) — a genuine
serialization still happens, once. **The stable path is the true
zero-copy path: no serialization exists at all.** When the payload can
carry a stable layout, prefer it.

## Stable payloads (writing typed data straight into loans)

This section is the layer *beneath* the application-facing typed
helpers — it is for wire and payload authors. Application code never
touches `MaybeUninit`: it writes a closure of named setters ending in
`finish()`, as the
[application transport chapter](crate::guide::applications::transport)
shows. What follows is what the derive generates and the helpers drive.

The whole flow, running — derive, then initialize directly in
uninitialized storage, builder-style, ending in a completion proof:

```rust
use std::mem::MaybeUninit;
use up_rust::StablePayloadInit as _;

#[repr(C)]
#[derive(Clone, Copy, up_rust::StablePayload, up_rust::ByteBackedStablePayload,
         up_rust::StablePayloadInit)]
#[stable_payload(type_name = "guide.demo.Reading")]
struct Reading {
    sensor_id: u16,
    value: u16,
}

let mut storage = [MaybeUninit::<u8>::uninit(); size_of::<Reading>()];
let init = Reading::init_from_uninit_bytes(&mut storage)?;
let _finished = init.sensor_id(7).value(451).finish()?; // finish() is the proof
# Ok::<(), up_rust::payload::UWireError>(())
```

Field setters exist because you derived `StablePayloadInit`; forgetting
a field or calling a setter twice is a *compile* error, not a runtime
one — the typestate pins in the test suite lock that in.

1. Define an explicit `#[repr(C)]` layout with no implicit padding.
2. Derive `StablePayload`, and derive `ByteBackedStablePayload` only when every
   byte pattern is valid for every field.
3. Derive `StablePayloadInit` to obtain the typestate initializer.
4. Initialize every field and explicit padding slot, call `finish()`, and return
   its completion proof from the higher-ranked initialization closure.

The proof constructor is private. Typestate makes incomplete completion
unavailable, and the higher-ranked closure prevents a proof tied to one call
from escaping or being substituted into another. Field writes go through the
library's checked write kernels; you never write unsafe code to implement
a stable payload.

For which trait is which, see the [trait map](trait_map).
