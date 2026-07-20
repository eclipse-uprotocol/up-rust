# Writing an application

Applications talk uProtocol through one of **two layers**, and knowing
which one you're on is the whole orientation:

* **[The Communication Layer](crate::guide::applications::communication)**
  (up-L2, feature `communication`) — role traits: `Publisher`,
  `Subscriber`, `Notifier`, `RpcClient`, `RpcServer`. **This is the
  recommended starting point.** Start here unless you have a reason not to.
* **[The Transport Layer](crate::guide::applications::transport)**
  (up-L1, no features) — build [`UMessage`](crate::UMessage)s yourself
  and hand them to a [`UTransport`](crate::UTransport). The floor the
  roles stand on; yours directly when you need minimal dependencies,
  custom filters, or the zero-copy payload path.

Thirty seconds to first message, on the communication layer:

```rust
use std::sync::Arc;
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, UPayloadFormat};
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());
    let me = Arc::new(StaticUriProvider::new("my-vehicle", 0x1_0001, 1)?);

    SimplePublisher::new(transport, me)
        .publish(0x8001, CallOptions::for_publish(None, None, None),
                 Some(UPayload::new("92.5", UPayloadFormat::Text)))
        .await?;
    // Delivered to every subscriber of my-vehicle/0x1_0001/1/0x8001.
    Ok(())
}
```

Everything below applies to both layers.

## Addresses in one box

Every example below uses [`UUri`](crate::UUri) literals; here is what
they mean, once:

```text
UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)
                      authority    entity   ver resource
```

* **authority** — which device/domain the uEntity lives on.
* **entity id** — which uEntity (a u32; deployments assign these).
* **major version** — the uEntity's API major version.
* **resource id** — *what* within the uEntity: `0x0000` is the uEntity
  itself (the address RPC responses return to), low ids (`0x0001..`)
  are RPC **methods**, and ids from **`0x8000` up are topics** a
  uEntity publishes. That is why the publish examples use `0x8001` and
  the RPC endpoint uses `0x00A1`.

## When things fail

Every operation returns a code you can act on. Transport-level errors
arrive as [`UStatus`](crate::UStatus) carrying a [`UCode`](crate::UCode):
`Unavailable` (link down — retry with backoff), `AlreadyExists`
(duplicate listener registration), `InvalidArgument` (a filter or
message the transport cannot express), `PermissionDenied`. The roles
wrap these: publishing fails with `PubSubError`, RPC with
`ServiceInvocationError` — whose `CommStatus` variant means *the remote
service* answered with a failure code, distinct from the transport
failing to deliver. Timeouts surface as `CommStatus(DeadlineExceeded)`
when the `ttl` you passed in `CallOptions` expires.

## Where a real transport comes from

The examples use `LocalTransport` so they can run anywhere. In a
deployment, you construct one of the transport crates at startup —
`up-transport-zenoh-rust`, `up-transport-mqtt5-rust`,
`up-transport-vsomeip-rust`, and so on — configure it, wrap it in an
`Arc`, and hand that same `Arc` to every role. Application code stays
identical: everything above takes `&dyn UTransport` or a generic, which
is the point of the protocol.


For which trait is which, see the [trait map](crate::guide::trait_map).
