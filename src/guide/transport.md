# The Transport Layer (up-L1)

Sometimes you want the floor itself: no role features, no protobuf, no
tokio beyond your own — just build a [`UMessage`](crate::UMessage) and
hand it to the [`UTransport`](crate::UTransport) your deployment
configured. This is also where the zero-copy payload path lives.

## Publish

```rust,no_run
use up_rust::{UMessageBuilder, UPayloadFormat, UTransport, UUri};

async fn publish(transport: &dyn UTransport) -> Result<(), Box<dyn std::error::Error>> {
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)?;
    let message = UMessageBuilder::publish(topic)
        .build_with_payload("92.5", UPayloadFormat::Text)?;
    transport.send(message).await?;
    Ok(())
}
```

## Receive

```rust,no_run
use std::sync::Arc;
use up_rust::{UListener, UMessage, UTransport, UUri};

struct TempListener;

#[async_trait::async_trait]
impl UListener for TempListener {
    async fn on_receive(&self, message: UMessage) {
        // Prints: engine temp update: Some(b"92.5") for the publish above.
        println!("engine temp update: {:?}", message.payload());
    }
}

async fn subscribe(transport: &dyn UTransport) -> Result<(), Box<dyn std::error::Error>> {
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)?;
    transport.register_listener(&topic, None, Arc::new(TempListener)).await?;
    Ok(())
}
```

## Filters and wildcards

`register_listener` takes a **source filter** (and optionally a sink
filter), and a filter component may be a wildcard: authority `"*"`,
resource id `0xFFFF` (any resource), and the entity-instance wildcard.
So:

```rust
# use up_rust::UUri;
// Everything entity 0x1_0001 publishes, any topic:
let all_topics = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0xFFFF)?;
// One exact topic (the examples above):
let one_topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Exact registrations receive exactly one topic; wildcard registrations
fan in. The matching rules are the same ones a transport itself uses —
the [UTransport tutorial](crate::guide::transports::utransport)'s
loopback shows them from the other side.

## Large payloads without extra copies

If your deployment's transport supports it, you can write big payloads
(camera frames, LiDAR scans, telemetry tables) directly into the
transport's own transmit buffer instead of building them in your memory
and copying them in. That is the *zero-copy* path: ask for a **loan** of
the right size, fill it, send it. The typed helpers on a selected-wire
transport do the bookkeeping; the transport chapter below shows the
mechanics.

## Large payloads: the typed path

When the configured transport supports zero-copy and a wire format was
selected at link construction, the typed helpers write your value
directly into transport storage:

```rust,no_run
# #[cfg(all(feature = "selected-wire-user-api", feature = "zero-copy-transport"))]
# fn main() {
# use up_rust::{PayloadEncoding, UMessageBuilder, UUri, UZeroCopyUninitTransportExt as _};
# #[repr(C)]
# #[derive(Clone, Copy, up_rust::StablePayload, up_rust::ByteBackedStablePayload,
#          up_rust::StablePayloadInit)]
# #[stable_payload(type_name = "guide.demo.LidarScan")]
# struct LidarScan { point_count: u32, first_range_mm: u32 }
# async fn typed_path(
#     transport: &(impl up_rust::UZeroCopyUninitTransport
#           + up_rust::UHasWire<Wire = up_rust::StableContainerWireFormat>
#           + Sync),
# ) -> Result<(), Box<dyn std::error::Error>> {
# let message = UMessageBuilder::publish(
#     UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)?).build()?;
// `metadata` is the frame-level projection of the message attributes you
// would otherwise have built — same addressing, frame form:
let metadata = message.attributes().to_frame_metadata(PayloadEncoding::RAW)?;

transport
    .send_uninit_stable_payload::<LidarScan>(metadata, |context| {
        context.into_init().point_count(1).first_range_mm(12_450).finish()
    })
    .await?;
# Ok(())
# }
# }
# #[cfg(not(all(feature = "selected-wire-user-api", feature = "zero-copy-transport")))]
# fn main() {}
```

**No serialization at all — this is the true zero-copy path.** The
stable layout is written once, in place, directly in transport storage;
the closure's completion proof is what makes it safe. (Wire-encoded
payload types take the codec path instead: one in-place serialization,
still zero intermediate copies — the
[wire chapter](crate::guide::wires) tabulates both.) The mechanics live
in the [zero-copy transport tutorial](crate::guide::transports::zero_copy).


For which trait is which, see the [trait map](crate::guide::trait_map).
