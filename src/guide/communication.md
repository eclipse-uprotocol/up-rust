# Communication Layer roles

Use five Communication Layer role traits across four messaging patterns:

* publish events;
* subscribe to events;
* send one-way notifications; and
* invoke or serve RPC methods.

Enable the Cargo features used by the examples with:

```bash
cargo add up-rust --features communication,util
```

The `communication` feature enables the role APIs. The `util` feature provides
the in-memory transport used by the examples.

## Publish events

[`Publisher`](crate::communication::Publisher) creates publish messages from a
resource ID, call options, and an optional payload. The publishing example uses
this in-process transport:

```rust
use std::sync::Arc;
use up_rust::communication::{CallOptions, PubSubError, Publisher, SimplePublisher, UPayload};
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, PayloadEncoding};

#[tokio::main]
async fn main() -> Result<(), PubSubError> {
    let transport = Arc::new(LocalTransport::default());
    let identity = Arc::new(
        StaticUriProvider::new("my-vehicle", 0x1_0001, 1)
            .expect("valid publisher identity"),
    );

    let publisher = SimplePublisher::new(transport, identity);
    let payload = UPayload::new("92.5", PayloadEncoding::TEXT);
    publisher
        .publish(
            0x8001,
            CallOptions::for_publish(None, None, None),
            Some(payload),
        )
        .await?;
    Ok(())
}
```

[`UTransport::send`](crate::UTransport::send) dispatches the message to matching
listeners when implemented by
[`LocalTransport`](crate::local_transport::LocalTransport). A deployed transport
maps the same Transport Layer operation to its protocol.

## Subscribe to events

[`Subscriber`](crate::communication::Subscriber) registers listeners for a
topic pattern. [`InMemorySubscriber`](crate::communication::InMemorySubscriber)
implements [`Subscriber::subscribe`](crate::communication::Subscriber::subscribe)
using the uSubscription service to create a subscription record containing a
topic and subscriber URI. The service reports subscription state changes to the
subscriber URI. The in-memory subscriber's local listener registrations remain
process-local, so this example is `no_run` unless the configured transport can
reach a uSubscription service.

```rust,no_run
# use std::sync::Arc;
# use up_rust::communication::{RegistrationError, Subscriber as _};
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
# ) -> Result<(), RegistrationError>
# where
#     T: UTransport + 'static,
#     P: LocalUriProvider + 'static,
# {
let subscriber = InMemorySubscriber::new(transport, identity).await?;
subscriber
    .subscribe(&topic, Arc::new(TempListener), None)
    .await?;
# Ok(())
# }
```

Both [`InMemorySubscriber::new`](crate::communication::InMemorySubscriber::new)
and [`Subscriber::subscribe`](crate::communication::Subscriber::subscribe)
return [`RegistrationError`](crate::communication::RegistrationError).

## Send a notification

[`Notifier`](crate::communication::Notifier) sends a one-way message to one
uEntity rather than publishing to a topic's subscribers:

```rust
use std::sync::Arc;
use up_rust::communication::{
    CallOptions, NotificationError, Notifier, SimpleNotifier, UPayload,
};
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, PayloadEncoding, UUri};

#[tokio::main]
async fn main() -> Result<(), NotificationError> {
    let transport = Arc::new(LocalTransport::default());
    let identity = Arc::new(
        StaticUriProvider::new("my-vehicle", 0x1_0001, 1)
            .expect("valid notifier identity"),
    );
    let destination = UUri::try_from_parts("my-vehicle", 0x2_0002, 1, 0x0000)
        .expect("valid notification destination");

    let notifier = SimpleNotifier::new(transport, identity);
    notifier
        .notify(
            0x8002,
            &destination,
            CallOptions::for_notification(None, None, None),
            Some(UPayload::new("door open", PayloadEncoding::TEXT)),
        )
        .await?;
    Ok(())
}
```

## Serve RPC requests

[`RpcServer`](crate::communication::RpcServer) registers a
[`RequestHandler`](crate::communication::RequestHandler) for a resource ID. The
server builds and addresses each response. This `no_run` example remains alive
after registration so the server and transport can continue receiving requests;
a deployed service would await its shutdown signal instead:

```rust,no_run
use std::sync::Arc;
use up_rust::communication::{
    InMemoryRpcServer, RegistrationError, RequestHandler, RpcServer,
    ServiceInvocationError, UPayload,
};
use up_rust::local_transport::LocalTransport;
use up_rust::{StaticUriProvider, UAttributes};

struct EchoHandler;

#[async_trait::async_trait]
impl RequestHandler for EchoHandler {
    async fn handle_request(
        &self,
        _resource_id: u16,
        _attributes: &UAttributes,
        request: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        Ok(request)
    }
}

#[tokio::main]
async fn main() -> Result<(), RegistrationError> {
    let transport = Arc::new(LocalTransport::default());
    let service = Arc::new(
        StaticUriProvider::new("my-vehicle", 0x2_0002, 1)
            .expect("valid RPC service identity"),
    );

    let server = InMemoryRpcServer::new(transport, service);
    server
        .register_endpoint(None, 0x00A1, Arc::new(EchoHandler))
        .await?;

#   // tarpaulin does not respect the "no_run" property on the doc test
#   // so we explicitly exclude the following statement from being compiled
#   // because otherwise tarpaulin would run into a timeout waiting for the
#   // doc test to finish
#   #[cfg(not(tarpaulin))]
    std::future::pending::<()>().await;
    Ok(())
}
```

[`RpcServer::register_endpoint`](crate::communication::RpcServer::register_endpoint)
completes after installing the handler. Keep the server and transport alive
until the service shuts down.

## Invoke an RPC method

[`RpcClient`](crate::communication::RpcClient) correlates the response with its
request and enforces the request TTL. This example registers an echo endpoint in
the same process; a deployed client calls the target uEntity through its
configured transport.

```rust
use std::sync::Arc;
use up_rust::communication::{
    CallOptions, InMemoryRpcClient, InMemoryRpcServer, RequestHandler, RpcClient, RpcServer,
    ServiceInvocationError, UPayload,
};
use up_rust::local_transport::LocalTransport;
use up_rust::{LocalUriProvider, StaticUriProvider, UAttributes, PayloadEncoding};

# struct EchoHandler;
# #[async_trait::async_trait]
# impl RequestHandler for EchoHandler {
#     async fn handle_request(&self, _r: u16, _a: &UAttributes, req: Option<UPayload>)
#         -> Result<Option<UPayload>, ServiceInvocationError> { Ok(req) }
# }
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());
    let service = Arc::new(
        StaticUriProvider::new("my-vehicle", 0x2_0002, 1)
            .expect("valid RPC service identity"),
    );
    let server = InMemoryRpcServer::new(transport.clone(), service.clone());
    server
        .register_endpoint(None, 0x00A1, Arc::new(EchoHandler))
        .await?;

    let identity = Arc::new(
        StaticUriProvider::new("my-vehicle", 0x3_0003, 1)
            .expect("valid RPC client identity"),
    );
    let client = InMemoryRpcClient::new(transport, identity).await?;

    let response = client
        .invoke_method(
            service.get_resource_uri(0x00A1),
            CallOptions::for_rpc_request(5_000, None, None, None),
            Some(UPayload::new("ping", PayloadEncoding::TEXT)),
        )
        .await?;

    assert_eq!(
        response.expect("echo response contains a payload").payload(),
        "ping"
    );
    Ok(())
}
```

This combined example uses `Box<dyn Error>` because RPC setup returns
[`RegistrationError`](crate::communication::RegistrationError), while method
invocation returns
[`ServiceInvocationError`](crate::communication::ServiceInvocationError).

## Handle role errors

The Communication Layer reports role-specific errors:

* [`RegistrationError`](crate::communication::RegistrationError) — a listener,
  subscriber, RPC client, or RPC endpoint could not be registered or
  unregistered;
* [`PubSubError::InvalidArgument`](crate::communication::PubSubError::InvalidArgument),
  [`NotificationError::InvalidArgument`](crate::communication::NotificationError::InvalidArgument),
  and [`ServiceInvocationError::InvalidArgument`](crate::communication::ServiceInvocationError::InvalidArgument)
  — URI, priority, TTL, or role input is invalid;
* [`PubSubError::PublishError`](crate::communication::PubSubError::PublishError)
  and [`NotificationError::NotifyError`](crate::communication::NotificationError::NotifyError)
  — the publish or notification transport operation returned a
  [`UStatus`](crate::UStatus);
* [`ServiceInvocationError::DeadlineExceeded`](crate::communication::ServiceInvocationError::DeadlineExceeded)
  — the RPC client did not receive a matching response before the TTL elapsed;
* other [`ServiceInvocationError`](crate::communication::ServiceInvocationError)
  variants — an RPC transport operation or response produced the corresponding
  status. A status without a dedicated variant becomes
  [`ServiceInvocationError::RpcError`](crate::communication::ServiceInvocationError::RpcError).

Correct invalid input, retry failures according to the application's policy,
and handle RPC response errors according to their
[`ServiceInvocationError`](crate::communication::ServiceInvocationError)
variant.

## Next steps

Use [the Transport Layer](crate::guide::applications::transport) directly for
exact [`UMessage`](crate::UMessage) control, custom listener filters, or
transport infrastructure. See
[application error handling](crate::guide::applications) and the
[trait map](crate::guide::trait_map) for the public API relationships.
