# Eclipse uProtocol Rust library

## Overview

This library implements the [uProtocol Language Specific Library Requirements](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/languages.adoc) for Rust defined in [uProtocol Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/tree/main). The library is organized into packages that are described below. Each package contains a README file that describes the purpose of the package and how to use it.

The module contains the factory methods, serializers, and validators for all data types defined in the specifications, and any data models that either haven't or couldn't be defined in up-core-api yet.

## Getting Started

### Building the library

Building the library is as simple as running `cargo build` in the project root directory. You can run the tests with `cargo test`.

__Note:__ the library current uses protobuf message definitions from the cloudevents project. Specifically, the build process downloads:

- `cloudevents.proto` from the [CNCF CloudEvents specification project](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/cloudevents.proto)

Additionally, the build pulls [uprotocol protobuf definitions](https://github.com/eclipse-uprotocol/up-core-api) that constitute the core types used in the library. All of these are compiled automatically during build by `build.rs`in the project root. The resulting Rust code is made available to the project via `lib.rs` definitions, in `pub::mod::proto`.

__Note:__ the library uses non-stable features from the uuid crate, notably version 8 UUIDs. These features are defined in `cargo.toml` (where the uuid crate dependency is declared), and require a compiler flag to be included in the build. This is configured in `.cargo/config.toml`.

### Using the library

The library contains the following modules:

Package | [uProtocol spec](https://github.com/eclipse-uprotocol/uprotocol-spec) | Purpose
---|---|---
cloudevent | [uProtocol CloudEvents](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/cloudevents.adoc) | Common way to represent uProtocol messages using CloudEvent data model
proto | | Various augmentations and convenience functions on generated protobuf code, like Display trait implementations or various conversions
transport | [uP-L1 Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc) | Interface (and data model) used for bidirectional point-2-point communication between uEs. Interface is to be implemented by the different communication technologies (ex. Binder, MQTT, Zenoh, SOME/IP, DDS, HTTP, etc…​)
uri | [URI, UAthority, UEntity, UResource](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc) | Uniform Resource Identifier (RFC3986) used to address things (devices, software, methods, topics, etc…​) on the network
uuid | [uProtocol UUIDs](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uuid.adoc) | Identifier used to uniquely identify messages that are sent between devices. also includes timestamp for the message
types | | Types commonly used across the SDK
