# Owned frames (experimental)

The owned-frame family (`owned-frame-transport`) exchanges whole
**validated frames as owned values** — and, the headline it earns over
the `UTransport` family: owned frames ride the **selected-wire system**.
Custom wire encodings, negotiated wire identities, native-prefix
metadata — the machinery the [wire chapter](crate::guide::wires)
describes — compose over owned carriage, while the `UTransport` path's
encoding stays whatever the transport does internally. Pick this family
when you want either of two things: pluggable wire formats, or
validation done once at the boundary when your technology naturally
hands over complete buffers (a DDS sample, a queue element).

You implement `UOwnedTransportImpl`; users call `UOwnedTransport` and
receive through `UOwnedListener`. The shape mirrors the UTransport
family with frames in place of messages — and validity is in the type:
`UOwnedFrame` written plain means `UOwnedFrame<Validated>`, the only
state a transport ever sees.

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

Building a frame is one step — constructors validate, so what you get
back is already the state dispatch requires:

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

The other state exists for the receive side: bytes decoded off the wire
become `UOwnedFrame<`[`Unvalidated`](crate::Unvalidated)`>`, and the
only way forward is [`UOwnedFrame::validate`](crate::UOwnedFrame::validate), which transitions to
[`Validated`](crate::Validated). The state transition *is* the
validation; an invalid frame reaching a listener is a compile error,
not a runtime drop.

**When to choose owned over zero-copy:** your technology hands you
complete, already-copied buffers anyway, so loan mechanics buy nothing —
but wire pluggability and boundary validation still do.

If your technology should instead stay ignorant of frames entirely and
carry pre-encoded bytes, implement `UOwnedTransportCore` and let
[`UWireTransport`](crate::UWireTransport) compose the encoding above you
— the same composition story as the
[zero-copy core](crate::guide::transports::zero_copy), and the wire
chapter's [composition section](crate::guide::wires) shows the payoff:
the transport code is identical for every wire.

Shared obligations: [the transport hub](crate::guide::transports). Which
trait is which: [the trait map](crate::guide::trait_map).
