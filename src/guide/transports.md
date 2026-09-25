# Implementing a transport

A transport carries messages using a broker, bus, shared memory, or another
technology. These guides cover the message and owned-frame interfaces:

* **[The `UTransport` family](crate::guide::utransport)** —
  implement [`UTransport`](crate::UTransport): send messages, register
  listeners. This is the standard uP-L1 message API. Its metadata encoding is
  defined by the transport binding.
* **[Owned frames](crate::guide::owned)**
  (`owned-frame-transport`, experimental) — exchange whole validated
  frames as owned values and use selected-wire adapters for external or
  deployment-specific wire encodings, registered identities, and
  native-prefix metadata, none of which the `UTransport` path offers.

Whatever the family, the shared obligations below apply.

## Declare supported transport families

A transport declares `UTransport` and owned-frame support independently, and
downstream routers and tests dispatch on those declarations. Declare only what
the implementation supports. Unsupported families fail explicitly rather
than hanging or degrading to a different family.

## Payload and routing integrity

For `UTransport`-family and owned carriage, preserve three payload
states: absent, present with zero bytes, and present with nonempty
bytes. Presence is not inferred from length.

Encoded metadata and payload regions are opaque to the encoded-core
[`UOwnedTransportCore`](crate::UOwnedTransportCore): a core moves those bytes and never interprets
them. If the underlying protocol or broker mirrors routing fields in
its own native headers, classify those fields explicitly. Required mirrors must
agree with decoded metadata. Documented untrusted routing hints may select
physical queues or candidate listeners, but cannot override decoded source/sink
metadata or satisfy the public listener filter on their own. The shared wire
adapter performs metadata validation and final filtering before public delivery.
Never derive routing or wire compatibility from application payload bytes.

## Listener lifecycle and readiness

Received leases and transmit loans own their backing lifetime. Native mappings,
proxies, subscriptions or allocators required by an outstanding object must remain
alive until that object is released, even if a listener is unregistered or its
transport handle is dropped. Unregister stops future delivery; it does not revoke
frames already handed to the application. Verify this with real backend tests that
retain metadata/payload access and original addresses across teardown, including
uninitialized TX loans. A heap-backed fixture alone cannot prove native ownership.

Registration and removal use the complete filter and listener identity.
The loopback example in the UTransport tutorial includes a listener registry.

Registration completes only when the binding can state what readiness
means for that technology. Discovery-backed transports should expose a
bounded peer or subscription readiness result, not use duplicate
application sends as a readiness protocol. An absent peer returns a
bounded, observable status.

Local registration and remote discovery are different milestones. A binding which
accepts registration before a peer exists must document that state and how the
first sample is retained or how bounded data-ready discovery is observed. A success
marker in an application must not claim a stronger state than the binding provides.

Listener dispatch preserves the binding's documented ordering and is
bounded by an explicit queue, worker, or backpressure policy.
Successful unregister is a quiescence boundary: a callback already
running may complete if the binding says so, but no later callback may
begin. Cancellation wakes blocking receive/poll operations, workers
expose health, shutdown joins them, and native entities are deleted
deterministically so a transport can be recreated in the same domain.

The opt-in `util` feature provides `ListenerAdmission` for bindings whose policy
allows an entered callback to finish. It closes admission at the actual first
poll, supports self-unregister, and leaves queueing and worker teardown with the
binding. It is not a substitute for retaining native loan owners.
The admission boundary is not a callback-draining guarantee: `stop` can return
while a previously entered callback is pending. A binding that promises completed
callbacks or joined workers must provide that additional teardown policy.

Polling is an acceptable native fallback when its wait is wakeable or
bounded, each iteration bounds take and dispatch work, failures become
health/status signals, and drop cannot leave an unjoined worker.

## TTL when crossing frame and classic boundaries

Frame metadata stores an optional `Duration` in nanoseconds. Rust `UAttributes`
uses optional whole milliseconds, while protobuf's TTL field is a scalar `u32`.
The SDK performs these explicit projections:

| Frame TTL | Rust attribute TTL | Protobuf TTL | Reverse frame projection |
| --- | --- | --- | --- |
| Absent | `None` | `0` | Absent |
| Present `Duration::ZERO` | `None` | `0` | Absent (canonical no-expiry form) |
| Positive exact milliseconds within `u32` | `Some(milliseconds)` | That positive value | The same duration |
| Positive fractional milliseconds or overflow | Projection error | No successful projection | Not applicable |

Absent and zero both mean no expiration for messages that permit it. This is
many-to-one canonicalization: explicit native zero-presence does not survive a
classic/protobuf round trip. Requests require a positive TTL; zero/absence does
not make a request non-expiring. Native field-block decode retains explicit zero
before any classic projection, as pinned by `wire_metadata_golden`.

Bindings must preserve original producer lifetime where required by their mapping;
remaining broker lifetime is a separate quantity. In particular, MQTT interval
zero is not the uProtocol no-expiry value.

## Validation

Run conformance checks for each supported transport family. This crate tests
owned-frame lifecycle, validation, filtering and selected-wire identity behavior.
Backend tests must verify native ownership and readiness. Integration tests check
cross-transport routing against the declared capabilities.

For which trait is which, see the [trait map](crate::guide::trait_map).
