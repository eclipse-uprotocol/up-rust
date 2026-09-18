# Public trait map

The Transport Layer API is always available. Enable the aggregate
`communication` Cargo feature for the complete Communication Layer role and
implementation map.

## Layers

| Layer | Main API | Implemented by | Use |
| --- | --- | --- | --- |
| Move messages | [`UTransport`](crate::UTransport) | transport implementation | Direct send, push-listener, or pull-receive control |
| Consume push delivery | [`UListener`](crate::UListener) | application or higher layer | Receive messages selected by transport filters |
| Identify the local uEntity | [`LocalUriProvider`](crate::LocalUriProvider) | application or deployment configuration, separately from `UTransport` | Supply source and response addresses |

## Operations

| Task | Primary method |
| --- | --- |
| Send a message | [`UTransport::send`](crate::UTransport::send) |
| Register for push delivery | [`UTransport::register_listener`](crate::UTransport::register_listener) |
| Stop push delivery | [`UTransport::unregister_listener`](crate::UTransport::unregister_listener) |
| Receive one message by pull | [`UTransport::receive`](crate::UTransport::receive) |

## Supporting APIs

* [`UMessageBuilder`](crate::UMessageBuilder) builds messages for direct
  Transport Layer use.
* [`UUri`](crate::UUri) identifies resources and supplies listener filters.
* [`UAttributesValidator`](crate::UAttributesValidator) validates message
  attributes by message kind.
* [`UStatus`](crate::UStatus) reports Transport Layer failures.

## Related guides

Applications: [the application guide](crate::guide::applications).
Transport users: [direct Transport Layer use](crate::guide::applications::transport).
Transport authors: [implementing a transport](crate::guide::utransport).
