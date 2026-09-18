# Use the Transport Layer directly

[`UTransport`](crate::UTransport) is the message-level interface shared by
applications, Communication Layer roles, and transport providers. It supports
outbound messages plus two delivery modes: registered listeners for push and
[`UTransport::receive`](crate::UTransport::receive) for pull. A transport
supports at least one delivery mode and may support both.

## Send a message

```rust
use up_rust::{UMessageBuilder, UPayloadFormat, UStatus, UTransport, UUri};

async fn publish(transport: &dyn UTransport) -> Result<(), UStatus> {
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)
        .expect("valid topic URI");
    let message = UMessageBuilder::publish(topic)
        .build_with_payload("92.5", UPayloadFormat::Text)
        .expect("valid publish message");
    transport.send(message).await
}
```

[`UTransport::send`](crate::UTransport::send) returns the result of that
transport's send operation. Success means that operation reached the checkpoint
defined by the implementation. For a protocol-backed transport, L1 describes
this as handing the message to the underlying protocol's send path. It is not a
portable end-to-end delivery or application-processing acknowledgement. The
example asserts its fixed URI and message inputs so its result exposes the
[`UStatus`](crate::UStatus) returned by `send`.

## Receive by push

Push transports deliver each matching message at least once to registered
[`UListener`](crate::UListener) instances. Registration and unregistration use
listener identity, so retain the same [`Arc`](std::sync::Arc) for both calls:

```rust
use std::sync::Arc;
use up_rust::{UListener, UMessage, UStatus, UTransport, UUri};

struct TempListener;

#[async_trait::async_trait]
impl UListener for TempListener {
    async fn on_receive(&self, message: UMessage) {
        println!("engine temp update: {:?}", message.payload());
    }
}

struct PushRegistration {
    topic: UUri,
    listener: Arc<dyn UListener>,
}

async fn subscribe(transport: &dyn UTransport) -> Result<PushRegistration, UStatus> {
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)
        .expect("valid topic URI");
    let listener: Arc<dyn UListener> = Arc::new(TempListener);

    transport
        .register_listener(&topic, None, listener.clone())
        .await?;

    Ok(PushRegistration { topic, listener })
}

async fn unsubscribe(
    transport: &dyn UTransport,
    registration: PushRegistration,
) -> Result<(), UStatus> {
    transport
        .unregister_listener(&registration.topic, None, registration.listener)
        .await
}
```

[`UTransport::register_listener`](crate::UTransport::register_listener) and
[`UTransport::unregister_listener`](crate::UTransport::unregister_listener)
take the source filter, optional sink filter, and listener. Unregistration must
use the same values that identify the registration, so the returned
`PushRegistration` retains them until `unsubscribe` consumes it.

## Receive by pull

Pull transports return one matching message from
[`UTransport::receive`](crate::UTransport::receive). The method returns
[`UCode::NotFound`](crate::UCode::NotFound) when no matching message is
available. When several messages match, L1 selects the oldest one that has not
expired:

```rust
use up_rust::{UMessage, UStatus, UTransport, UUri};

async fn receive_one(transport: &dyn UTransport) -> Result<UMessage, UStatus> {
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)
        .expect("valid topic URI");
    transport.receive(&topic, None).await
}
```

## Select messages with filters

Push registration and pull receive both take a source filter and an optional
sink filter. [`UUri::any`](crate::UUri::any) matches any source,
[`UUri::any_with_resource_id`](crate::UUri::any_with_resource_id) matches any
source with one resource ID, and [`UUri::matches`](crate::UUri::matches)
evaluates a concrete URI against a wildcard filter.

```rust
use up_rust::UUri;

let any_source = UUri::any();
let topic_at_any_source = UUri::any_with_resource_id(0x8001);
let one_topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)
    .expect("valid topic URI");

assert!(any_source.matches(&one_topic));
assert!(topic_at_any_source.matches(&one_topic));
```

Use an exact [`UUri`](crate::UUri) to select one address. Wildcard filters can
select several sources or resources, subject to the valid source/sink resource
combinations in the L1 Transport Layer specification.

The [transport implementation guide](crate::guide::utransport) points to the
authoritative filter rules, delivery semantics, and protocol-binding
requirements.

See the [trait map](crate::guide::trait_map) for the public API relationships.
