# uProtocol CloudEvents

## Overview

[uProtocol CloudEvents](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/cloudevents.adoc) is a common message envelope that could be used to carry way to represent uProtocol transport layer information `UUri` (source), `UPayload`, and `UAttributes`. `CloudEvents` are used by a number of Device-2-Cloud and Cloud-2-Device based transports such as MQTT and HTTP, however it could also be used by any transport (ex. Binder).

NOTE: CloudEvent is not, nor was not, meant to be _the only_ message format used below or above the transport layer.

### CloudEventBuilder

Builder for various types of CloudEvents for uProtocol (publish, notification, request, response)

## Examples

The SDK contains comprehensive tests, the best place to look at how all the APIs are used.

- [ucloudeventbuilder.rs](src/cloudevent/builder/ucloudeventbuilder.rs)
