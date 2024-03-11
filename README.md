# Eclipse uProtocol Rust library

## Overview

This library implements the [uProtocol Language Specific Library Requirements](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/languages.adoc) for Rust defined in [uProtocol Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/tree/main). The library is organized into packages that are described below. Each package contains a README file that describes the purpose of the package and how to use it.

The module contains the factory methods, serializers, and validators for all data types defined in the specifications, and any data models that either haven't or couldn't be defined in up-core-api yet.

## Getting Started

### Working with the library

Building the library is as simple as running `cargo build` in the project root directory. The build pulls [uprotocol protobuf definitions](https://github.com/eclipse-uprotocol/up-core-api) that constitute the core types used in the library. All of these are compiled automatically by `build.rs`. The resulting Rust code is made available to the project via the `up_core_api::` internal module.

__Note:__ the library uses non-stable features from the uuid crate, notably version 8 UUIDs. These features are defined in `cargo.toml` (where the uuid crate dependency is declared), and require a compiler flag to be included in the build. This is configured in `.cargo/config.toml`.

### Optional features

The up-rust crate contains some optional features, as per the crate documentation (refer to src/lib.rs). One of these features is the usage of protobuf message definitions from the cloudevents project. Specifically, with this feature enabled the build process downloads:

- `cloudevents.proto` from the [CNCF CloudEvents specification project](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/cloudevents.proto)

To tell VSCode to just build all crate features automatically, place the following in `./vscode/settings.json`:

```json
{
    "rust-analyzer.cargo.features": "all"
}
```

For more details on Rust (cargo) features, please refer to [The Cargo Book](https://doc.rust-lang.org/cargo/reference/features.html).

### Using the library

Sub-modules of this crate, as well as optional features, are listed in the crate documentation in `src/lib.rs`.
