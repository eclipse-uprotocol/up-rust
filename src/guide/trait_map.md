# The trait map: how the traits fit together

The crate's traits look numerous but form a small system: **four
transport families x four roles, plus three standalone utilities**.
Learn the naming grammar once and every trait's job is readable from its
name:

* **Bare name** (`UTransport`, `UZeroCopyTransport`) — *you call it.*
  The user-facing contract a configured transport hands you.
* **`...Impl` / `...Core`** (`UZeroCopyTransportImpl`,
  `UZeroCopyTransportCore`) — *you implement it,* and only if you are
  writing a transport. `Impl` speaks the library's semantic frame types;
  `Core` receives already-encoded bytes and stays a dumb pipe while
  [`UWireTransport`](crate::UWireTransport) composes wires above it.
* **`...Ext`** (`UZeroCopyTransportExt`) — *free methods.* Blanket
  conveniences you get on every implementation; never implemented by
  hand.
* **`...Listener`** (`UListener`, `UZeroCopyListener`) — *you receive
  through it.* Implement one and register it to be called back.

## The stack in five lines

```text
application            roles (up-L2)  or  UMessage + UTransport (up-L1)
                                 |
transport family       UTransport / owned / zero-copy capability
                                 |
wire composition       UWireTransport = encoded core + wire + codec
                                 |
technology             broker, bus, shared memory
```

## The families

| Family | You implement | You call | You receive with | What flows |
| --- | --- | --- | --- | --- |
| Classic (always available) | [`UTransport`](crate::UTransport) | [`UTransport`](crate::UTransport) | [`UListener`](crate::UListener) | [`UMessage`](crate::UMessage) |
| Owned frames (`owned-frame-transport`) | [`UOwnedTransportImpl`](crate::UOwnedTransportImpl) | [`UOwnedTransport`](crate::UOwnedTransport) | [`UOwnedListener`](crate::UOwnedListener) | the [`Validated`](crate::Validated) frame state |
| Zero-copy (`zero-copy-transport`) | [`UZeroCopyTransportImpl`](crate::UZeroCopyTransportImpl) or [`UZeroCopyTransportCore`](crate::UZeroCopyTransportCore) | [`UZeroCopyTransport`](crate::UZeroCopyTransport) (+ [`UZeroCopyTransportExt`](crate::UZeroCopyTransportExt)) | [`UZeroCopyListener`](crate::UZeroCopyListener) | [`UTxBuffer`](crate::UTxBuffer) out; [`UZeroCopyRxLease`](crate::UZeroCopyRxLease) back |
| Zero-copy, uninitialized loans (same feature) | [`UZeroCopyUninitTransportImpl`](crate::UZeroCopyUninitTransportImpl) | [`UZeroCopyUninitTransport`](crate::UZeroCopyUninitTransport) (+ `Ext`) | (same listener) | [`UUninitTxBuffer`](crate::UUninitTxBuffer) |

The UTransport family is the floor every transport provides. The owned and
zero-copy families are additive capability levels a transport offers
only when its technology honestly supports them — the
[transport tutorial](transports) walks all three with code.

## What flows, in one sentence each

* [`UTxBuffer`](crate::UTxBuffer) — transmit storage a zero-copy
  transport *lends* you; write the payload in place, then commit.
* [`UUninitTxBuffer`](crate::UUninitTxBuffer) — the same loan before
  initialization; the typed init API fills it safely.
* [`UZeroCopyRxLease`](crate::UZeroCopyRxLease) — a received payload you
  read in place; dropping it returns the storage. When the storage is
  contiguous loan-backed memory it also implements
  [`ULoanedContiguousZeroCopyRxFrame`](crate::ULoanedContiguousZeroCopyRxFrame),
  which lets typed payloads be *borrowed* rather than copied out.
* [`UFrameView`](crate::UFrameView) — the neutral read vocabulary every
  received frame speaks, whatever the family: metadata plus ordered
  payload bytes. Owned frames and zero-copy leases both implement it, so
  decoding code is written once.

## The Communication Layer roles (up-L2)

Applications mostly never touch the tables above directly — they use the
role traits. The ready-made implementations below drive a `UTransport`-family
transport. The owned-frame endpoint provides the corresponding owned role
facades; the zero-copy endpoint currently provides publish and lease-preserving
subscribe without pretending its receive lease is a classic `UMessage`
(`communication::owned::Endpoint`, `communication::zero_copy::Endpoint` — see
[the application guide](applications)) (feature
`communication-api` for the traits, `communication` for ready-made
implementations):

| You want to… | Role trait | Ready-made implementation |
| --- | --- | --- |
| Publish to whoever listens | [`Publisher`](crate::communication::Publisher) | [`SimplePublisher`](crate::communication::SimplePublisher) |
| Target one uEntity | [`Notifier`](crate::communication::Notifier) | [`SimpleNotifier`](crate::communication::SimpleNotifier) |
| Consume a topic | [`Subscriber`](crate::communication::Subscriber) | [`InMemorySubscriber`](crate::communication::InMemorySubscriber) (needs a uSubscription client — see [the application tutorial](applications)) |
| Call a service | [`RpcClient`](crate::communication::RpcClient) | [`InMemoryRpcClient`](crate::communication::InMemoryRpcClient) |
| Serve requests | [`RpcServer`](crate::communication::RpcServer) + [`RequestHandler`](crate::communication::RequestHandler) | [`InMemoryRpcServer`](crate::communication::InMemoryRpcServer) |

## The wire author's traits

A wire format is a marker plus a codec (feature `wire-implementer-api`):

| Trait | Job |
| --- | --- |
| [`UWire`](crate::UWire) | The marker: the wire's identity constants |
| [`PayloadCodec`](crate::PayloadCodec) | Names the codec and its payload encoding |
| [`EncodePayload`](crate::EncodePayload)`<T>` | Measure (`payload_layout`) then write `T` in place |
| [`DecodePayload`](crate::DecodePayload)`<T>` | Read `T` from borrowed payload bytes |
| [`ReadDecodePayload`](crate::ReadDecodePayload)`<T>` | Decode from an ordered reader — works on segmented storage without coalescing |
| [`UWirePayload`](crate::UWirePayload)`<T>` | Associates a payload type (and its codec) with the wire |

## The composition structs (not traits, but you'll look for them here)

* [`UWireTransport`](crate::UWireTransport) — wraps an encoded core
  ([`UZeroCopyTransportCore`](crate::UZeroCopyTransportCore) / [`UOwnedTransportCore`](crate::UOwnedTransportCore)) and composes the
  selected wire and metadata codec above it. Concretely: the core moves
  two opaque byte regions; `UWireTransport` owns everything typed above
  them — it encodes metadata through the codec, stamps and checks wire
  identity, sizes loans from `payload_layout`, runs the payload codec in
  place, and rejects foreign bytes before any decoder sees them. This is
  where the families and the wires meet, and it is why the transport
  crate underneath never mentions a codec.
* [`UWireRx`](crate::UWireRx) — the received-payload wrapper (a lease
  for zero-copy, an owned frame for owned) that adds wire-aware typed decoding on
  top of the raw lease.

## The standalone utilities

* [`LocalUriProvider`](crate::LocalUriProvider) — answers "what is *my*
  address?"; roles use it to build source URIs.
* [`ProtobufMappable`](crate::ProtobufMappable) — lets any
  protobuf-generated type ride as a payload implicitly
  (`protobuf-support`).
* [`UAttributesValidator`](crate::UAttributesValidator) — validates
  message attributes per message kind; transports use it at boundaries.

## Where to go

Applications: [the application tutorial](applications). Transport
authors: [the transport tutorial](transports). Wire and payload
authors: [the wire tutorial](wires).
