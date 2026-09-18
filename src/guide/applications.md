# Build an application

The application-facing APIs are:

* **[The Communication Layer](crate::guide::applications::communication)**
  provides five role traits across four messaging patterns:
  [`Publisher`](crate::communication::Publisher),
  [`Subscriber`](crate::communication::Subscriber),
  [`Notifier`](crate::communication::Notifier), and
  [`RpcClient`](crate::communication::RpcClient) with
  [`RpcServer`](crate::communication::RpcServer).
* **[The Transport Layer](crate::guide::applications::transport)** provides
  direct message-level control and is the interface implemented by transport
  providers.

```rust
use std::sync::Arc;
use up_rust::communication::{CallOptions, PubSubError, Publisher, SimplePublisher, UPayload};
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, UPayloadFormat};

#[tokio::main]
async fn main() -> Result<(), PubSubError> {
    let transport = Arc::new(LocalTransport::default());
    let identity = Arc::new(
        StaticUriProvider::new("my-vehicle", 0x1_0001, 1)
            .expect("valid publisher identity"),
    );

    SimplePublisher::new(transport, identity)
        .publish(
            0x8001,
            CallOptions::for_publish(None, None, None),
            Some(UPayload::new("92.5", UPayloadFormat::Text)),
        )
        .await?;
    Ok(())
}
```

## Choose an application role

| Requirement | API | Additional dependency |
| --- | --- | --- |
| One-to-many events | [`Publisher`](crate::communication::Publisher) | — |
| Receive published events | [`Subscriber`](crate::communication::Subscriber) | [`InMemorySubscriber`](crate::communication::InMemorySubscriber) uses uSubscription for bookkeeping and status |
| One-way delivery to one uEntity | [`Notifier`](crate::communication::Notifier) | — |
| Request/response | [`RpcClient`](crate::communication::RpcClient) and [`RpcServer`](crate::communication::RpcServer) | — |
| Direct message send or receive | [`UTransport`](crate::UTransport) | — |

## Address messages with UUri

[`UUri`](crate::UUri) addresses identify an authority, uEntity, major version,
and resource:

```text
UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)
                      authority    entity   ver resource
```

* **authority** identifies the device or domain.
* **entity id** identifies the uEntity.
* **major version** identifies the uEntity API version.
* **resource id** identifies a resource within the uEntity. `0x0000` addresses
  the uEntity, `0x0001..=0x7FFF` identify RPC methods,
  `0x8000..=0xFFFE` identify topics, and `0xFFFF` is the resource wildcard.

## Share a transport

The examples use [`LocalTransport`](crate::local_transport::LocalTransport), the
crate's in-process push transport. A deployment can instead construct a
transport such as
[`up-transport-zenoh`](https://crates.io/crates/up-transport-zenoh),
[`up-transport-mqtt5`](https://crates.io/crates/up-transport-mqtt5), or
[`up-transport-vsomeip`](https://crates.io/crates/up-transport-vsomeip).

Publishers, notifiers, subscribers, RPC clients, and RPC servers can share the
same [`UTransport`](crate::UTransport) instance through
[`Arc`](std::sync::Arc) clones. The role implementations are generic over
`UTransport`, so application code does not need to name the concrete transport
type:

```rust
# use std::sync::Arc;
# use up_rust::local_transport::LocalTransport;
let transport = Arc::new(LocalTransport::default());
let publisher_transport = transport.clone();
let rpc_transport = transport.clone();
# let _ = (publisher_transport, rpc_transport);
```

See the [trait map](crate::guide::trait_map) for the public API relationships.

## Handle failures by API

* [`UStatus`](crate::UStatus) is returned by Transport Layer operations. For
  example, [`UTransport::send`](crate::UTransport::send) can reject an invalid
  message with [`UCode::InvalidArgument`](crate::UCode::InvalidArgument) or
  report an unavailable transport with
  [`UCode::Unavailable`](crate::UCode::Unavailable).
* [`RegistrationError`](crate::communication::RegistrationError) is returned
  when a Communication Layer listener, subscriber, RPC client, or RPC endpoint
  cannot be registered or unregistered.
* [`PubSubError`](crate::communication::PubSubError),
  [`NotificationError`](crate::communication::NotificationError), and
  [`ServiceInvocationError`](crate::communication::ServiceInvocationError) are
  returned by Communication Layer role operations. The publish and notification
  transport-failure variants wrap a [`UStatus`](crate::UStatus), while RPC
  transport and response statuses are converted into `ServiceInvocationError`
  variants. For example,
  [`ServiceInvocationError::DeadlineExceeded`](crate::communication::ServiceInvocationError::DeadlineExceeded)
  reports an RPC timeout.

For RPC, both a transport `UStatus` and a response's non-OK communication status
use the same [`ServiceInvocationError`](crate::communication::ServiceInvocationError)
mapping. A client-side timeout returns
[`ServiceInvocationError::DeadlineExceeded`](crate::communication::ServiceInvocationError::DeadlineExceeded).

## Next steps

* [Communication Layer roles](crate::guide::applications::communication)
* [Direct Transport Layer use](crate::guide::applications::transport)
* [Public trait relationships](crate::guide::trait_map)
