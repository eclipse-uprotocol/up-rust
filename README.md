# Eclipse uProtocol Rust library

This is the [uProtocol v1.6.0-alpha.7 Language Library](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/languages.adoc) for the Rust programming language.

The crate can be used to

* implement uEntities that communicate with each other using the uProtocol [Communication Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l2/api.adoc) over one of the supported transport protocols.
* implement support for an additional transport protocol by means of implementing the [Transport Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/README.adoc).

## Using the Crate
<!--
`uman~up-language-using~1`
Covers:
- req~up-language-documentation~
-->
The crate needs to be added to the `[dependencies]` section of the `Cargo.toml` file:

```toml
[dependencies]
up-rust = { version = "0.9" }
```

Most developers will want to use the Communication Level API and its default implementation which are provided by the `communication` module. Please refer to the [examples](./examples/) for inspiration how to use this crate.

### Choosing Imports

The crate root remains a compatibility import surface, so existing `up_rust::UMessage`, `up_rust::UTransport`, and similar root imports are still valid. New code should choose imports by the layer being used:

* Application and service code should start with the Communication Layer roles in `up_rust::communication`, such as publishers, subscribers, notifiers, and RPC clients/servers.
* Compatibility transport code should use `UTransport`, `UListener`, `UMessage`, `UAttributes`, `UUri`, and `UStatus` directly when it is implementing or adapting the ordinary message transport contract.
* Native-frame, selected-wire, payload-codec, and zero-copy code is advanced transport or wire-representation work. Import those names deliberately when implementing transports, codecs, routing adapters, or loan-backed paths.
* Unsafe stable-payload transmit/init APIs and unchecked constructors are expert surfaces with caller-side safety obligations. They are not the default application path.
* Mocks, in-memory proof transports, vector-backed leases, payload fixtures, and benchmark fixtures are test/proof support surfaces. Prefer them for tests and conformance evidence, not as ordinary production application APIs.

These tiers do not deprecate `UTransport` or ban direct Transport Layer usage.
The current public doors and feature names are authoritative; unpublished
legacy aliases and role-specific feature flags are intentionally not retained.

For direct Transport Layer work, the up-L1 doors are still intentionally easy to find:

| Use case | Public door | Notes |
| --- | --- | --- |
| Compatibility message transport | Root exports such as `up_rust::UTransport`, `up_rust::UListener`, `up_rust::UMessage`, `up_rust::UAttributes`, `up_rust::UUri`, and `up_rust::UStatus` | Direct up-L1 remains a supported compatibility path. |
| Owned native-frame transport | Root exports such as `up_rust::UOwnedTransport`, `up_rust::UOwnedFrame`, `up_rust::Validated`, `up_rust::PreparedOwnedFrame`, and the role facade in `up_rust::communication::owned` | Available with `owned-frame-transport`; validation is represented by the frame typestate and this family does not replace `UTransport`. |
| Zero-copy transport and loans | Root exports such as `up_rust::UZeroCopyTransport`, `up_rust::UZeroCopyUninitTransport`, `up_rust::UTxBuffer`, `up_rust::UUninitTxBuffer`, `up_rust::UZeroCopyRxLease`, and the publish facade in `up_rust::communication::zero_copy` | Loan-backed mechanics remain up-L1 capability. The L2 facade is intentionally narrower than raw transport capability. |
| Selected-wire profiles | The public `up_rust::wire` module plus root exports such as `up_rust::UWire`, `up_rust::UProtocolNativeWire`, `up_rust::ProtobufWire`, `up_rust::WireIdentity`, and selected-wire adapter exports such as `up_rust::UWireTransport` | Selected-wire identity, metadata, and payload-family checks are up-L1 representation/profile semantics. |
| Whole-frame envelopes | The public `up_rust::frame::envelope` module and root exports such as `up_rust::UFrameWireFormat` when `owned-frame-transport` is enabled | Whole-frame `UPFE` serialization is separate from selected-wire `UPWM` metadata prefixes. |
| Payload codecs and stable payload support | The public `up_rust::payload` module plus root exports such as `up_rust::PayloadCodec`, `up_rust::EncodePayload`, `up_rust::DecodePayload`, `up_rust::StablePayload`, and `up_rust::StablePayloadInit` | Safe derive/codegen paths are preferred for applications; manual unsafe slots are expert hatches. |
| Test, fake, and proof support | Feature-gated root exports such as `up_rust::MockTransport`, `up_rust::InMemoryZeroCopyTransport`, vector leases, and payload fixtures | These remain available for tests, benchmarks, and conformance proof, not as production transport evidence by themselves. |

## Building from Source
<!--
`uman~up-language-building~1`
Covers:
- req~up-language-documentation~1
-->

First, the repository needs to be cloned using:

```sh
git clone --recurse-submodules https://github.com/eclipse-uprotocol/up-rust.git
```

The `--recurse-submodules` parameter is important to make sure that the git submodule referring to the uProtocol type definitions is being initialized in the workspace. The proto3 files contained in that submodule define uProtocol's basic types and are being compiled into Rust code as part of the build process.
If the repository has already been cloned without the parameter, the submodule can be initialized manually using `git submodule update --init --recursive`.

The crate can then be built using the [Cargo package manager](https://doc.rust-lang.org/cargo/) from the root folder:
<!--
`impl~use-cargo-build-system~1`
Covers:
- req~up-language-build-sys~1
- req~up-language-build-deps~1
-->

```sh
cargo build
```

The crate has some (optional) _features_ as documented in [lib.rs](src/lib.rs).
The non-default `zero-copy-transport` feature enables the complete zero-copy
family, including typed initialization directly in uninitialized transmit
loans. Add it explicitly to transport or application manifests that use loans,
leases, or zero-copy transport contracts.

VSCode can be instructed to build all features automatically by means of putting the following into `./vscode/settings.json`:

```json
{
  "rust-analyzer.cargo.features": "all"
}
```

### Generating API Documentation

The API documentation can be generated using

```sh
cargo doc --no-deps --all-features --open
```

## License

The crate is published under the terms of the [Apache License 2.0](LICENSE).

## Contributing

Contributions are more than welcome. Please refer to the [Contribution Guide](CONTRIBUTING.md).
