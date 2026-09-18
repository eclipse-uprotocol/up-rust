# Implement UTransport

[`UTransport`](crate::UTransport) adapts [`UMessage`](crate::UMessage) values
and [`UUri`](crate::UUri) filters to a messaging technology. Implement
[`UTransport::send`](crate::UTransport::send) and at least one receive mode:
push through listener registration, pull through
[`UTransport::receive`](crate::UTransport::receive), or both. The default
receive and listener methods return
[`UCode::Unimplemented`](crate::UCode::Unimplemented).

## Start with three contracts

| Source | What it defines |
| --- | --- |
| [`UTransport`](crate::UTransport) | Rust method signatures and default behavior |
| [L1 Transport Layer specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/README.adoc) | Observable operation, validation, authorization, delivery, and error requirements |
| [Zenoh](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/zenoh.adoc), [MQTT 5](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/mqtt_5.adoc), or [SOME/IP](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/someip.adoc) binding | Mapping between uProtocol messages and the selected protocol |

Use all three when implementing and testing a transport. This guide explains
how they fit together; it does not replace the complete L1 specification or the
selected binding.

## See the adapter boundary

A transport owns two directional paths:

```text
outbound: validated UMessage -> encode for binding -> native send path

inbound:  native PDU -> decode and validate for binding -> UMessage
                                                           |-> matching push listeners
                                                           `-> pull receive queue
```

The binding defines the native representation. L1 defines what callers can
observe at the `UTransport` methods.

## Implement outbound send

[`UTransport::send`](crate::UTransport::send) takes one complete `UMessage`.
The implementation should:

1. preserve all attributes and the payload while encoding the selected
   binding;
2. perform the protocol-specific send operation; and
3. convert failures into an appropriate [`UStatus`](crate::UStatus).

Keep attribute validation at the boundary that constructs a `UMessage`.
[`UMessageBuilder`](crate::UMessageBuilder) validates attributes before it
returns a message. Decoders and custom type mappings should likewise reject an
invalid representation instead of producing an invalid `UMessage`. A transport
therefore does not normally need to repeat the same validation in `send`.

The pinned L1 specification still requires
[`UCode::InvalidArgument`](crate::UCode::InvalidArgument) if an invalid message
does reach `send`. Treat that as a fallback for integrations that do not yet
enforce validation when constructing a `UMessage`, not as a reason to revalidate
values from validated constructors.

Successful `send` completion is not a transport-independent delivery receipt.
For protocol-backed transports, L1 says the message has been handed to the
underlying protocol's send path, but the observable checkpoint depends on the
implementation and protocol. For example, one transport might wait for a broker
acknowledgement while an in-process transport might complete after local
dispatch. Document that checkpoint without claiming that a recipient received
or processed the message.

## Build one inbound path

Decode each native protocol data unit according to the selected binding, rebuild
the `UMessage`, and validate its attributes before exposing it through push or
pull. Preserve the message metadata and payload during decoding.

For push, L1 requires invalid inbound data to be discarded. For pull, an
inbound protocol data unit that cannot produce a valid matching `UMessage` is
reported as [`UCode::NotFound`](crate::UCode::NotFound). Keep decoding and
validation shared so both receive modes interpret the binding consistently.

## Implement push delivery

Push delivery uses
[`UTransport::register_listener`](crate::UTransport::register_listener) and
[`UTransport::unregister_listener`](crate::UTransport::unregister_listener).
Both methods identify a registration with the source filter, optional sink
filter, and listener identity:

```rust
use std::sync::Arc;
use up_rust::{UListener, UUri};

struct Registration {
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: Arc<dyn UListener>,
}
```

Validate a filter pair before changing registration state. The SDK helper
implements the shared resource-ID checks from L1:

```rust
use up_rust::{verify_filter_criteria, UStatus, UUri};

fn validate_filters(
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> Result<(), UStatus> {
    verify_filter_criteria(source_filter, sink_filter)
        .map_err(|status| *status)
}
```

A push implementation must make these lifecycle points observable:

* Repeating registration with the same filters and listener has the same effect
  as registering once.
* Registration does not succeed until messages received afterward can reach the
  listener.
* Every valid matching message received after successful registration invokes
  each matching listener at least once.
* Successful unregistration is a barrier: the removed registration receives no
  later messages.
* Configured total and per-filter listener limits fail registration with
  [`UCode::ResourceExhausted`](crate::UCode::ResourceExhausted).

Use listener identity, not merely its concrete type, when comparing
registrations. [`ComparableListener`](crate::ComparableListener) is available
to implementations that need hashable and comparable `Arc<dyn UListener>`
values.

## Implement pull delivery

[`UTransport::receive`](crate::UTransport::receive) selects by the same source
and optional sink filter shape used for push. If multiple unexpired messages
match, return the oldest one. Return
[`UCode::NotFound`](crate::UCode::NotFound) when no valid matching message is
available, and [`UCode::Unimplemented`](crate::UCode::Unimplemented) when the
transport does not support pull.

The implementation decides how to buffer native messages, but it must preserve
the L1 selection and expiry behavior at the method boundary.

## Map errors by operation

These notable L1 outcomes are not an exhaustive list of native failures:

| Method | Required outcome |
| --- | --- |
| `send` | `InvalidArgument` if an invalid message reaches the method; `PermissionDenied` is recommended when unauthorized sending can be determined locally |
| `receive` | `Unimplemented` when pull is unsupported; `NotFound` when no valid matching message is available |
| `register_listener` | `Unimplemented` when push is unsupported; `InvalidArgument` for invalid filters; `ResourceExhausted` at a configured listener limit; `PermissionDenied` is recommended when unauthorized consumption can be determined locally |
| `unregister_listener` | `Unimplemented` when push is unsupported; `InvalidArgument` for invalid filters; `NotFound` when the registration does not exist |

Map other native failures to the closest `UCode`, retain useful diagnostic
detail in `UStatus`, and document any transport-specific outcomes.

## Document transport-specific behavior

L1 and the selected binding remain the conformance authority. A concrete
transport should additionally document behavior that its technology determines:

* **send completion:** which native event allows `send` to return success;
* **reconnect:** whether registrations and buffered messages survive a lost
  connection;
* **ordering:** which messages, if any, are observed in a stable order;
* **backpressure:** whether senders wait, fail, or drop when capacity is
  exhausted; and
* **shutdown:** how pending sends, receives, and listener callbacks finish.

Do not turn those transport-specific choices into general `UTransport`
guarantees.

## Existing implementations

For a minimal in-process push implementation, see
[`LocalTransport`](https://docs.rs/up-rust/latest/up_rust/local_transport/struct.LocalTransport.html).
For protocol-backed designs, the published
[`up-transport-zenoh`](https://crates.io/crates/up-transport-zenoh),
[`up-transport-mqtt5`](https://crates.io/crates/up-transport-mqtt5), and
[`up-transport-vsomeip`](https://crates.io/crates/up-transport-vsomeip) crates
show how concrete technologies implement the interface. Use them as design
references; use L1 and the selected binding as the conformance authority.

Return to the [guide](crate::guide).
