# The UTransport family

A `UTransport`-family transport implements one trait,
[`UTransport`](crate::UTransport): deliver a [`UMessage`](crate::UMessage),
and keep a registry of listeners with their source/sink filters. Here is
a **complete, working transport** — an in-process loopback — as a
running example:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use up_rust::{
    verify_filter_criteria, ComparableListener, UCode, UListener, UMessage, UStatus, UTransport,
    UUri,
};

struct Registered {
    source: UUri,
    sink: Option<UUri>,
    listener: ComparableListener,
}

#[derive(Default)]
struct MiniTransport {
    listeners: RwLock<Vec<Registered>>,
}

#[async_trait::async_trait]
impl UTransport for MiniTransport {
    async fn send(&self, message: UMessage) -> Result<(), UStatus> {
        // A real transport hands the bytes to its technology here.
        // The loopback "technology" is: match filters, dispatch locally.
        let source = message.attributes().source();
        let sink = message.attributes().sink();
        for r in self.listeners.read().await.iter() {
            let source_ok = r.source.matches(source);
            let sink_ok = match (&r.sink, sink) {
                (Some(pattern), Some(candidate)) => pattern.matches(candidate),
                (None, None) => true,
                _ => false,
            };
            if source_ok && sink_ok {
                r.listener.on_receive(message.clone()).await;
            }
        }
        Ok(())
    }

    async fn register_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        // Reject filter combinations the spec forbids before storing anything.
        verify_filter_criteria(source_filter, sink_filter).map_err(|e| *e)?;
        self.listeners.write().await.push(Registered {
            source: source_filter.to_owned(),
            sink: sink_filter.map(ToOwned::to_owned),
            listener: ComparableListener::new(listener),
        });
        Ok(())
    }

    async fn unregister_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let target = ComparableListener::new(listener);
        let mut listeners = self.listeners.write().await;
        let before = listeners.len();
        listeners.retain(|r| {
            !(r.source == *source_filter
                && r.sink.as_ref() == sink_filter
                && r.listener == target)
        });
        if listeners.len() < before {
            Ok(())
        } else {
            Err(UStatus::fail_with_code(UCode::NotFound, "no such listener"))
        }
    }
}

// Prove it works: register, send, receive.
struct Capture(tokio::sync::mpsc::UnboundedSender<UMessage>);
#[async_trait::async_trait]
impl UListener for Capture {
    async fn on_receive(&self, message: UMessage) {
        let _ = self.0.send(message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use up_rust::{UMessageBuilder, UPayloadFormat};
    let transport = MiniTransport::default();
    let topic = UUri::try_from_parts("demo", 0x1_0001, 1, 0x8001)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    transport.register_listener(&topic, None, Arc::new(Capture(tx))).await?;
    transport
        .send(UMessageBuilder::publish(topic).build_with_payload("42", UPayloadFormat::Text)?)
        .await?;

    let received = rx.recv().await.expect("message delivered");
    assert_eq!(received.payload().unwrap().as_ref(), b"42");
    Ok(())
}
```

The crate's own [`LocalTransport`](crate::local_transport::LocalTransport)
is this same shape grown up: a `HashSet` keyed on
[`ComparableListener`](crate::ComparableListener) so duplicate
registrations are rejected with `AlreadyExists`, and filter matching via
the message's attributes. Read its source when you outgrow the sketch.

What a real technology changes: `send` serializes —
`message.attributes()` to protobuf bytes for the metadata, plus the
payload — and hands both to the broker or bus; the receive side runs the
inverse and rebuilds a `UMessage` before dispatching to listeners.

That is the whole level. Error mapping: use
[`UCode`](crate::UCode) faithfully — `AlreadyExists` for duplicate
registration, `NotFound` for unknown listeners, `InvalidArgument` for
filters your technology cannot express, `Unavailable` when the link is
down. Applications and the roles built above you switch on these codes.

Next levels: [owned frames](crate::guide::transports::owned) and
[zero-copy](crate::guide::transports::zero_copy). Shared obligations:
[the transport hub](crate::guide::transports).
