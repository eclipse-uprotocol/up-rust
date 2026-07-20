# The Communication Layer (up-L2)

The roles hide message building entirely: you say *what* (publish this,
call that) and the layer builds, addresses, and correlates the messages
over whatever transport it was given. Enable `communication`.

## Publishing

The hub page's quickstart is the whole pattern; here it is again with
the pieces named — `LocalTransport` is the in-process transport the
examples use:

```rust
use std::sync::Arc;
use up_rust::local_transport::LocalTransport;
use up_rust::StaticUriProvider;
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};
use up_rust::UPayloadFormat;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());
    let identity = Arc::new(StaticUriProvider::new("my-vehicle", 0x1_0001, 1)?);

    let publisher = SimplePublisher::new(transport, identity);
    let payload = UPayload::new("92.5", UPayloadFormat::Text);
    publisher
        .publish(0x8001, CallOptions::for_publish(None, None, None), Some(payload))
        .await?;
    // Delivered to every subscriber of my-vehicle/0x1_0001/1/0x8001.
    Ok(())
}
```

## Subscribing (and why it needs a service)

Publish, notify, and RPC are pure transport interactions. **Subscription
is different by design**: uProtocol tracks who subscribes to what in a
service — uSubscription — so subscriptions survive reconnects and can be
observed. The [`Subscriber`](crate::communication::Subscriber) role
therefore needs two collaborators: a transport *and* a uSubscription
client.

```rust,no_run
# use std::sync::Arc;
# use up_rust::communication::Subscriber as _;
# use up_rust::{LocalUriProvider, UListener, UMessage, UTransport, UUri};
use up_rust::communication::InMemorySubscriber;
# struct TempListener;
# #[async_trait::async_trait]
# impl UListener for TempListener {
#     async fn on_receive(&self, _message: UMessage) {}
# }
# async fn wire_up<T, P>(
#     transport: Arc<T>,
#     identity: Arc<P>,
#     topic: UUri,
# ) -> Result<(), Box<dyn std::error::Error>>
# where
#     T: UTransport + 'static,
#     P: LocalUriProvider + 'static,
# {

// `new` wires the uSubscription client for you: an RPC client over your
// transport, talking to the uSubscription service in your deployment.
let subscriber = InMemorySubscriber::new(transport, identity).await?;

subscriber.subscribe(&topic, Arc::new(TempListener), None).await?;
# Ok(())
# }
```

A real subscription round-trips through the uSubscription service, so
this compiles against your deployment's transport and RPC client rather
than running standalone; for unit tests, `test-util` provides role
mocks.

## Notifying one receiver

Publish goes to whoever subscribed; a *notification* targets one uEntity.
Same shape, one extra argument:

```rust
use std::sync::Arc;
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, UPayloadFormat, UUri};
use up_rust::communication::{CallOptions, Notifier, SimpleNotifier, UPayload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());
    let identity = Arc::new(StaticUriProvider::new("my-vehicle", 0x1_0001, 1)?);
    let destination = UUri::try_from_parts("my-vehicle", 0x2_0002, 1, 0x0000)?;

    let notifier = SimpleNotifier::new(transport, identity);
    notifier
        .notify(
            0x8002,
            &destination,
            CallOptions::for_notification(None, None, None),
            Some(UPayload::new("door open", UPayloadFormat::Text)),
        )
        .await?;
    // Exactly one receiver: the uEntity at `destination` — not a topic fan-out.
    Ok(())
}
```

## Serving RPC

You expose a resource id and a handler; the server owns listener
registration and response addressing:

```rust
use std::sync::Arc;
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, UAttributes};
use up_rust::communication::{
    InMemoryRpcServer, RequestHandler, RpcServer, ServiceInvocationError, UPayload,
};

struct EchoHandler;

#[async_trait::async_trait]
impl RequestHandler for EchoHandler {
    async fn handle_request(
        &self,
        _resource_id: u16,
        _attributes: &UAttributes,
        request: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        Ok(request) // echo the request payload back
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());
    let service = Arc::new(StaticUriProvider::new("my-vehicle", 0x2_0002, 1)?);

    let server = InMemoryRpcServer::new(transport, service);
    server.register_endpoint(None, 0x00A1, Arc::new(EchoHandler)).await?;
    // The service now answers requests to my-vehicle/0x2_0002/1/0x00A1.
    Ok(())
}
```

Return `Err(ServiceInvocationError::...)` from the handler and the
caller receives it as a `CommStatus` — an application-level failure,
delivered, not a transport failure.

## Calling RPC

The client correlates responses to requests and enforces your deadline.
(The inline echo service here is scaffolding so the round trip can run
and assert; in a deployment it's someone else's uEntity.)

```rust
use std::sync::Arc;
use up_rust::local_transport::LocalTransport;
use up_rust::{LocalUriProvider, StaticUriProvider, UAttributes, UPayloadFormat};
use up_rust::communication::{
    CallOptions, InMemoryRpcClient, InMemoryRpcServer, RequestHandler, RpcClient, RpcServer,
    ServiceInvocationError, UPayload,
};

# struct EchoHandler;
# #[async_trait::async_trait]
# impl RequestHandler for EchoHandler {
#     async fn handle_request(&self, _r: u16, _a: &UAttributes, req: Option<UPayload>)
#         -> Result<Option<UPayload>, ServiceInvocationError> { Ok(req) }
# }
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());
    let service = Arc::new(StaticUriProvider::new("my-vehicle", 0x2_0002, 1)?);
    # let server = InMemoryRpcServer::new(transport.clone(), service.clone());
    # server.register_endpoint(None, 0x00A1, Arc::new(EchoHandler)).await?;

    let me = Arc::new(StaticUriProvider::new("my-vehicle", 0x3_0003, 1)?);
    let client = InMemoryRpcClient::new(transport, me).await?;

    let response = client
        .invoke_method(
            service.get_resource_uri(0x00A1),
            CallOptions::for_rpc_request(5_000, None, None, None), // 5s deadline
            Some(UPayload::new("ping", UPayloadFormat::Text)),
        )
        .await?;

    assert_eq!(response.unwrap().payload(), "ping");
    Ok(())
}
```

If the deadline passes first, `invoke_method` returns
`CommStatus(DeadlineExceeded)` — see
[when things fail](crate::guide::applications) on the hub.

## The same roles, over the experimental families

The role layer is not UTransport-only. Both experimental transport families
carry it:

### Roles over owned frames

`communication::owned::Endpoint` offers the full role set over any
[`UOwnedTransport`](crate::UOwnedTransport) — same semantics, validated
frames underneath. Runnable, over the in-memory owned transport
(`test-util`):

```rust
use std::sync::Arc;
use up_rust::communication::owned::Endpoint;
use up_rust::communication::{CallOptions, UPayload};
use up_rust::{InMemoryOwnedTransport, StaticUriProvider, UPayloadFormat};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(InMemoryOwnedTransport::default());
    let me = Arc::new(StaticUriProvider::new("my-vehicle", 0x1_0001, 1)?);

    let endpoint = Endpoint::new(transport, me);
    endpoint
        .publisher()
        .publish(0x8001, CallOptions::for_publish(None, None, None),
                 Some(UPayload::new("92.5", UPayloadFormat::Text)))
        .await?;
    // Delivered as a validated owned frame to every matching owned listener.
    Ok(())
}
```

The owned subscriber follows the same rule as the UTransport-family one:
**subscription is a service interaction**, so
`endpoint.subscriber(usubscription_client)` takes a uSubscription client
and informs the service before registering the local listener.

### Typed zero-copy publishing with role ergonomics

Over a selected-wire zero-copy transport,
`communication::zero_copy::Endpoint` gives the publisher role the typed
loan path: `publish_stable` and `publish_uninit_stable` write your value
directly into transport storage, with role-style addressing instead of
manual metadata:

```rust,no_run
# use std::sync::Arc;
# use up_rust::communication::{zero_copy, CallOptions};
# use up_rust::{
#     LocalUriProvider, StableContainerWireFormat, UHasWire, UZeroCopyUninitTransport,
# };
# #[repr(C)]
# #[derive(Clone, Copy, up_rust::StablePayload, up_rust::ByteBackedStablePayload,
#          up_rust::StablePayloadInit)]
# #[stable_payload(type_name = "guide.demo.LidarScan")]
# struct LidarScan { point_count: u32, first_range_mm: u32 }
# async fn publish<T>(wire_transport: Arc<T>, me: Arc<dyn LocalUriProvider>)
# -> Result<(), Box<dyn std::error::Error>>
# where T: UZeroCopyUninitTransport + UHasWire<Wire = StableContainerWireFormat> + Send + Sync + 'static {
let endpoint = zero_copy::Endpoint::new(wire_transport, me);
endpoint
    .publisher()
    .publish_uninit_stable::<LidarScan>(
        0x8003,
        CallOptions::for_publish(None, None, None),
        |context| context.into_init().point_count(1).first_range_mm(12_450).finish(),
    )
    .await?;
# Ok(())
# }
```

The receive direction mirrors the owned subscriber:
`endpoint.subscriber(usubscription)` consults the uSubscription service
first and, on an active or pending subscription, registers a
zero-copy listener — received payloads arrive as typed
[`UWireRx`](crate::UWireRx) leases whose
[`decode_payload`](crate::UWireRx::decode_payload) reads the value in
place.

The mechanics of loans and stable payloads live in the
[Transport Layer page](crate::guide::applications::transport) and the
[zero-copy transport tutorial](crate::guide::transports::zero_copy).

For the typed zero-copy payload path, see
[the Transport Layer page](crate::guide::applications::transport).
