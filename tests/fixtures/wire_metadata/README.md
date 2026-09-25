# Wire Metadata Fixtures

This directory is the USR-02 golden vector register location for native-prefix metadata.

`tests/wire_metadata_conformance.rs` generates and validates the first-wave vectors through public `up-rust` APIs:

- no-payload metadata
- standard-payload metadata
- private-use-payload metadata
- wrong magic
- unknown metadata layout id
- unsupported version
- wrong selected wire id
- unknown selected wire id
- payload-family mismatch
- malformed length
- reserved-zero payload encoding
- trailing bytes

The vectors are intentionally constructed by the public encoder and mutated by tests so the tests remain coupled to the frozen byte layout rather than to private helpers.
