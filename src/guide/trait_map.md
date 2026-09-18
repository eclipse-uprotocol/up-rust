# Public trait map

* The **Communication Layer** provides the application roles.
* The **Transport Layer** provides direct [`UMessage`](crate::UMessage) send,
  push-listener, and pull-receive operations.
* Transport bindings implement [`UTransport`](crate::UTransport).
* [`LocalUriProvider`](crate::LocalUriProvider) separately supplies the local
  uEntity identity used to address messages.

## Layers

| Layer | Main API | Implemented by | Use |
| --- | --- | --- | --- |
| Application roles | [`Publisher`](crate::communication::Publisher), [`Subscriber`](crate::communication::Subscriber), [`Notifier`](crate::communication::Notifier), [`RpcClient`](crate::communication::RpcClient), [`RpcServer`](crate::communication::RpcServer) | applications or the crate's ready-made implementations | Publish, subscribe, notify, invoke, or serve |
| Move messages | [`UTransport`](crate::UTransport) | implemented by the transport | Direct send, push-listener, or pull-receive control; transport infrastructure |
| Consume push delivery | [`UListener`](crate::UListener) | applications and Communication Layer implementations | Receive messages selected by transport filters |
| Identify the local uEntity | [`LocalUriProvider`](crate::LocalUriProvider) | application or deployment configuration | Supply source and response addresses |

The Communication Layer depends on the Transport Layer. Its role APIs are
generic over [`UTransport`](crate::UTransport), so they can use an in-memory or
protocol-backed implementation.

## Communication Layer implementations

The `communication` Cargo feature enables the ready-made role implementations:

| Role trait | Implementation | Additional collaborator |
| --- | --- | --- |
| [`Publisher`](crate::communication::Publisher) | [`SimplePublisher`](crate::communication::SimplePublisher) | — |
| [`Subscriber`](crate::communication::Subscriber) | [`InMemorySubscriber`](crate::communication::InMemorySubscriber) | uSubscription service |
| [`Notifier`](crate::communication::Notifier) | [`SimpleNotifier`](crate::communication::SimpleNotifier) | — |
| [`RpcClient`](crate::communication::RpcClient) | [`InMemoryRpcClient`](crate::communication::InMemoryRpcClient) | — |
| [`RpcServer`](crate::communication::RpcServer) | [`InMemoryRpcServer`](crate::communication::InMemoryRpcServer) | [`RequestHandler`](crate::communication::RequestHandler) |

## Operations

| Task | Primary method | Collaborator |
| --- | --- | --- |
| Publish an event | [`Publisher::publish`](crate::communication::Publisher::publish) | [`CallOptions`](crate::communication::CallOptions) |
| Subscribe to a topic | [`Subscriber::subscribe`](crate::communication::Subscriber::subscribe) | [`UListener`](crate::UListener) and uSubscription |
| Send a notification | [`Notifier::notify`](crate::communication::Notifier::notify) | [`CallOptions`](crate::communication::CallOptions) |
| Invoke an RPC method | [`RpcClient::invoke_method`](crate::communication::RpcClient::invoke_method) | [`CallOptions`](crate::communication::CallOptions) |
| Serve an RPC method | [`RpcServer::register_endpoint`](crate::communication::RpcServer::register_endpoint) | [`RequestHandler`](crate::communication::RequestHandler) |
| Push a raw message | [`UTransport::send`](crate::UTransport::send) | — |
| Receive raw messages by push | [`UTransport::register_listener`](crate::UTransport::register_listener) | [`UListener`](crate::UListener) |
| Receive one raw message by pull | [`UTransport::receive`](crate::UTransport::receive) | — |

## Supporting APIs

* [`SubscriptionChangeHandler`](crate::communication::SubscriptionChangeHandler)
  receives subscription-state callbacks.
* [`UMessageBuilder`](crate::UMessageBuilder) builds messages for direct
  Transport Layer use.
* [`UAttributesValidator`](crate::UAttributesValidator) validates message
  attributes by message kind.
* [`ProtobufMappable`](crate::ProtobufMappable) integrates protobuf-generated
  payload types when the `protobuf-support` feature is enabled.

## Related guides

Applications: [the application tutorial](crate::guide::applications).
Transport users: [the Transport Layer tutorial](crate::guide::applications::transport).
Transport authors: [the transport implementation tutorial](crate::guide::utransport).
