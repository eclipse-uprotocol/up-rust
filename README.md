# Eclipse uProtocol Rust SDK

## Overview

The purpose of this module is to provide language specific code that builds the various data types defined in the [uProtocol Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/tree/main).

The module contains builder methods and validators for all data types used in uProtocol.

The SDKs are then used by the code generators to auto-populate service stubs with generated code that builds uProtocol events. For more information on auto-generating service stubs, please refer to the [uProtocol Main Project](http://github.com/eclipse-uprotocol/uprotocol).

## Getting Started

### Building the SDK

Building the SDK is as simple as running `cargo build` in the project root directory.
Once the SDK is built, you can run the tests with `cargo test`.

__Note:__ the SDK currently builds two protobuf messages from upstream cloudevents and grpc definitions. Specifically, the build process downloads:

- `cloudevents.proto` from the [CNCF CloudEvents specification project](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/cloudevents.proto) and
- `status.proto` from [Google APIs](https://github.com/googleapis/googleapis/blob/master/google/rpc/status.proto).

These are compiled automatically during build, this is controlled by `build.rs`in the project root. The resulting Rust code is made available to the project via `lib.rs` definitions, in `pub::mod::proto`.

__Note:__ the SDK uses non-stable features from the uuid crate, notably version 8 UUIDs. These features are defined in `cargo.toml` (where the uuid crate dependency is declared), and require a compiler flag to be included in the build. This is configured in `.cargo/config.toml`.

### Using the SDK

The SDK is composed of the main packages as shown below:

Package | [uProtocol spec](https://github.com/eclipse-uprotocol/uprotocol-spec) | Purpose
---|---|---
cloudevent | [uProtocol CloudEvents](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/cloudevents.adoc) | Common way to represent uProtocol messages using CloudEvent data model
transport | [uP-L1 Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc) | Interface (and data model) used for bidirectional point-2-point communication between uEs. Interface is to be implemented by the different communication technologies (ex. Binder, MQTT, Zenoh, SOME/IP, DDS, HTTP, etc…​)
uri | [URI, UAthority, UEntity, UResource](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc) | Uniform Resource Identifier (RFC3986) used to address things (devices, software, methods, topics, etc…​) on the network
uuid | [uProtocol UUIDs](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uuid.adoc) | Identifier used to uniquely identify messages that are sent between devices. also includes timestamp for the message
