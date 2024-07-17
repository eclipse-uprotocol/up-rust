# Eclipse uProtocol Rust library

This is the [uProtocol v1.6.0-alpha.2 Language Library](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/v1.6.0-alpha.2/languages.adoc) for the Rust programming language.

The crate can be used to

* implement uEntities that communicate with each other using the uProtocol [Communication Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l2/api.adoc) over one of the supported transport protocols.
* implement support for an additional transport protocol by means of implementing the [Transport Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l1/README.adoc).

## Building
<!--
`uman~up-language-building~1`
Covers:
- req~up-language-documentation~1
-->

The crate can be built using the [Cargo package manager](https://doc.rust-lang.org/cargo/) from the root folder:
<!--
`impl~use-cargo-build-system~1`
Covers:
- req~up-language-build-sys~1
- req~up-language-build-deps~1
-->

```sh
cargo build
```

The build requires an active connection to the internet because it needs to download and compile [uProtocol protobuf files](https://github.com/eclipse-uprotocol/up-spec/tree/v1.6.0-alpha.2/up-core-api) which define many of the core types used throughout the library.

The crate has some (optional) _features_ as documented in [lib.rs](src/lib.rs).

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

## Using the Crate
<!--
`uman~up-language-using~1`
Covers:
- req~up-language-documentation~
-->
The crate needs to be added to the `[dependencies]` section of the `Cargo.toml` file:

```toml
[dependencies]
up-rust = { version = "0.1" }
```

Most developers will want to use the Communication Level API and its default implementation
which are provided by `communication` module.

## License

The crate is published under the terms of the [Apache License 2.0](LICENSE).

## Contributing

Contributions are more than welcome. Please refer to the [Contribution Guide](CONTRIBUTING.md).
