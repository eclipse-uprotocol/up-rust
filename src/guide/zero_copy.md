# Level 3: zero-copy (experimental)

The zero-copy family (`zero-copy-transport`) is for technologies that
can lend transmit storage and
deliver received payloads in place. You have two ways to plug in, named
by the trait you implement:

* Implement **`UZeroCopyTransportImpl`** when your API can speak the
  library's frame types directly (`UFrameMetadata` in, leases out). You
  own more; you depend on more.
* Implement **`UZeroCopyTransportCore`** when your technology should stay
  a dumb pipe: you receive *already-encoded* metadata bytes and payload
  storage, and move them. The library's `UWireTransport` layers every
  wire format and codec on top — you never see them. **This is the
  recommended default**, and it is why adding a wire format never
  requires touching a transport (the `N + M` property: N transports and
  M wires, not N x M integrations).

A transport may offer both entry points if it genuinely supports both;
just say in your crate docs which one a given type is.

## The zero-copy lifecycle, concretely

Transmit is a loan lifecycle: size it, stamp the encoding, borrow TX
storage, write in place, then **commit** — committing a loan means
completing it by sending (`send_zero_copy`); an uncommitted loan is
simply dropped and its storage returns. Never commit after a failed
step: a missing encoding, an undersized or misaligned buffer, or a
codec error means the loan is dropped, not sent.

The same lifecycle, runnable, with raw bytes over the in-memory zero-copy
transport (features `zero-copy-transport` + `test-util`):

```rust
use up_rust::{
    InMemoryZeroCopyTransport, PayloadEncoding, UFrameView, UMessageBuilder, UTxBuffer,
    UTxLoanSpec, UUri, UZeroCopyTransport,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = InMemoryZeroCopyTransport::default();
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)?;

    // Frame metadata comes from the same attributes a UMessage carries.
    let message = UMessageBuilder::publish(topic.clone()).build()?;
    let metadata = message.attributes().to_frame_metadata(PayloadEncoding::RAW)?;

    // The abort rule, demonstrated: an invalid layout fails *before* any
    // storage is loaned — alignment must be a power of two.
    assert!(UTxLoanSpec::payload(metadata.clone(), 4, 3).is_err());

    let payload = b"lidar scan bytes";
    let spec = UTxLoanSpec::payload(metadata, payload.len(), 1)?;   // size + alignment
    let mut buffer = transport.loan_tx(spec).await?;                // borrow TX storage
    buffer.payload_mut().copy_from_slice(payload);                  // write in place
    transport.send_zero_copy(buffer).await?;                        // commit

    // Receive side: an immutable lease over the same storage.
    let lease = transport.receive_zero_copy(&topic, None).await?;
    assert_eq!(lease.try_contiguous_payload(), Some(payload.as_slice()));
    Ok(())
}
```

Receive: your transport delivers payloads behind an immutable **lease**;
the application reads (or decodes) in place, and dropping the lease
returns the storage. The generic layer — not your transport — owns wire
identity checks, decoding, and validation.

The requirement-level contract behind all three levels is
[transport families](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/transport_families.adoc).


## The encoded core, sketched

The recommended entry point deserves its shape on the page. A core moves
two byte regions and never interprets either:

```rust,no_run
use up_rust::{PreparedTxLoanSpec, UStatus, UZeroCopyTransportCore};
# use up_rust::{UEncodedRxFrame, UVecTxBuffer};

struct MyCore { /* shared-memory handle */ }

#[async_trait::async_trait]
impl UZeroCopyTransportCore for MyCore {
    type Tx = UVecTxBuffer;      // your loan type: implements UTxBuffer
    type Rx = MyRxFrame;         // your lease type: implements UEncodedRxFrame
#    // (UVecTxBuffer is the crate's heap-backed loan; a real shared-memory
#    // transport supplies its own. MyRxFrame is sketched hidden below.)

    async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
        // Allocate (or borrow from shared memory) storage sized and aligned
        // per the spec: encoded metadata region + payload region.
        unimplemented!("allocate storage per {spec:?}")
    }

    async fn send_prepared_zero_copy(&self, _buffer: Self::Tx) -> Result<(), UStatus> {
        // The library already wrote encoded metadata and payload into the
        // loan. Publish the storage; do not parse it.
        unimplemented!("publish the storage")
    }

    // receive_encoded_zero_copy / register_encoded_zero_copy_listener:
    // deliver encoded regions as Self::Rx leases; same registry pattern
    // as the UTransport tutorial.
}
# struct MyRxFrame;
# impl UEncodedRxFrame for MyRxFrame {
#     type PayloadReader<'a> = std::io::Cursor<&'a [u8]>;
#     type PayloadSlices<'a> = std::iter::Once<&'a [u8]>;
#     fn encoded_metadata(&self) -> &[u8] { unimplemented!() }
#     fn payload_len(&self) -> usize { unimplemented!() }
#     fn payload_reader(&self) -> Self::PayloadReader<'_> { unimplemented!() }
#     fn payload_slices(&self) -> Self::PayloadSlices<'_> { unimplemented!() }
# }
```

Wrap it once and every wire format, metadata validation, identity
rejection, and typed initialization arrive composed on top — and the
wrap line is the *only* place the wire is named. Swapping wires never
touches the core:

```rust,no_run
# use up_rust::{StableContainerWireFormat, UProtocolNativeWire, UWithNativePrefixWire as _, UZeroCopyTransportCore};
# fn compose(core_a: impl UZeroCopyTransportCore, core_b: impl UZeroCopyTransportCore) {
// The same core, composed under two different wires — core code identical:
let stable_links = core_a.into_native_prefix_wire_transport(StableContainerWireFormat);
let native_links = core_b.into_native_prefix_wire_transport(UProtocolNativeWire);
# let _ = (stable_links, native_links);
# }
```

Your transport crate never sees a codec; N transports and M wires stay
`N + M` integrations, not `N x M` — every pairing above compiles from
the same two crates.

## Escape hatch: manual loans

You almost never need this — the typed helpers perform exactly these
steps — but a low-level sender working with raw bytes or an initialized
loan must uphold the same ordered contract:

1. Compute `C::payload_layout(&value)` and retain its exact
   length/alignment.
2. Stamp metadata with
   `metadata.with_payload_encoding(C::payload_encoding())?`.
3. Create `UTxLoanSpec::payload(metadata, layout.len(), layout.align())?`.
4. Await `transport.loan_tx(spec)`.
5. Call `verify_tx_buffer_payload_layout` before exposing storage to the
   codec.
6. Encode exactly into `buffer.payload_mut()` and propagate the codec
   error.
7. Commit — send the loaned buffer — only through
   `transport.send_zero_copy(buffer).await`.

Do not commit after a missing encoding, undersized buffer, alignment
mismatch, partial initialization, or failed codec write. Writing
directly into a loan avoids an intermediate owned payload, but it does
not by itself prove the codec performs no copies or that the route is
end-to-end no-copy — for the path with no serialization at all, use
stable payloads (the true zero-copy path; see the
[wire chapter](crate::guide::wires)'s two-path table).

Shared obligations: [the transport hub](crate::guide::transports). The
wire-generic manual TX sequence lives with the
[wire tutorial](crate::guide::wires); the runnable lifecycle above is
its concrete instance.
