# Owned frames (experimental)

The owned-frame family (`owned-frame-transport`) exchanges validated metadata
and payload storage as owned values. Selected-wire adapters can apply external
or deployment-specific formats above an encoded transport core.
This is useful when a transport supplies complete buffers, such as DDS samples
or queue entries. The [wire chapter](crate::guide::wires) explains the codecs
and metadata layouts used by those adapters.

You implement `UOwnedTransportImpl`; users call `UOwnedTransport` and
receive through `UOwnedListener`. These interfaces use frames where `UTransport`
uses messages. `UOwnedFrame` defaults to `UOwnedFrame<Validated>`, the state
accepted by the public transport methods.

```rust,no_run
use up_rust::{UOwnedFrame, UOwnedTransportImpl, UStatus};

struct MyTransport { /* connection handle */ }
# struct Backend; impl Backend { fn publish(&self, _m: &up_rust::UFrameMetadata, _p: Option<&bytes::Bytes>) -> Result<(), UStatus> { Ok(()) } }
# impl MyTransport { fn backend(&self) -> Backend { Backend } }

#[async_trait::async_trait]
impl UOwnedTransportImpl for MyTransport {
    async fn send_validated_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus> {
        // `frame` is `UOwnedFrame<Validated>` by type: metadata checked,
        // payload presence consistent — no re-checking here.
        self.backend().publish(frame.metadata(), frame.payload())
    }
    // register/unregister owned listeners: same registry pattern as the
    // UTransport tutorial.
}
```

Frame constructors validate metadata and payload presence before returning:

```rust
use up_rust::{PayloadEncoding, UMessageBuilder, UOwnedFrame, UUri};

let topic = UUri::try_from_parts("demo", 0x1_0001, 1, 0x8001)?;
let message = UMessageBuilder::publish(topic).build()?;

// Frame metadata is a projection of the same attributes a UMessage carries:
let metadata = message.attributes().to_frame_metadata(PayloadEncoding::RAW)?;
let frame = UOwnedFrame::with_payload(metadata, b"reading".to_vec())?;

assert_eq!(frame.payload_bytes(), b"reading");
# Ok::<(), Box<dyn std::error::Error>>(())
```

Raw receive code can construct `UOwnedFrame<`[`Unvalidated`](crate::Unvalidated)`>`.
[`UOwnedFrame::validate`](crate::UOwnedFrame::validate) checks it and either returns
an error or produces `UOwnedFrame<Validated>`. The public listener signature
prevents passing an unvalidated value directly to the callback.

Choose owned frames when your technology hands you complete buffers and you
want wire pluggability plus validation at the public boundary.

For transport code that handles encoded metadata and payload bytes, implement
`UOwnedTransportCore`. `UWireTransport` (from `transport-implementer-api`) applies
the selected codec above it, allowing the same core to support multiple wires.

See the transport implementation guide for shared requirements and
[the trait map](crate::guide::trait_map) for the interfaces.
