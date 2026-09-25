# uframe-abi (revision 2)

Cross-language artifacts for native UFrame metadata, revised per the
reframing audits: the fixed-layout struct is now an **opt-in derived
profile**, and the primary cross-language contract is the **variable-length
metadata field block**.

## Contents

| path | what | verified with |
|---|---|---|
| `WIRE-FORMAT.md` | Metadata field block and `UPFE` whole-frame envelope byte contracts | Rust round-trip and golden tests |
| `rust/uframe_metadata_abi_v1.rs` | standalone `no_std` Rust reference definition of the fixed ABI profile | `rustc 1.95 --edition 2021 -D warnings` + `const` layout asserts |
| `c/uframe_metadata_abi_v1.h` (+ `check_layout.c`) | C11 definition | `gcc -std=c11 -Wall -Wextra -Werror -pedantic` + `static_assert` |
| `cpp/uframe_metadata_abi_v1.hpp` (+ `check_layout.cpp`) | C++17 definition | `g++ -std=c++17 -Wall -Wextra -Werror -pedantic` + `static_assert` |

The crate, standalone Rust, C and C++ definitions assert the same sizes,
alignments, and field offsets at compile time; a drift in any view fails its
build.

## What changed from revision 1

Revision 1 proposed one fixed 1024-byte `UFrameMetadata` as the semantic
model, the ABI, and the canonical wire image. Per the audits, those are now
three separate things:

1. **Semantic model** — `up_rust::UFrameMetadata` (owned, ergonomic,
   no fixed capacities). Not in this folder; it lives in each SDK.
2. **Canonical bytes** — the variable-length, presence-driven field block
   implemented by `up_rust::frame::codec`. This is the contract C/C++ peers
   of native transports parse.
3. **Fixed ABI profile** — `UFrameMetadataAbiV1` (this folder,
   size 928, align 8, magic `"UFA1"`, type name
   `uprotocol.v2.UFrameMetadataAbiV1`). For boundaries where both sides
   explicitly agree to exchange a directly readable typed struct, e.g. an
   iceoryx2 user-header profile for C consumers or deterministic test
   fixtures. Its per-field capacities (authority 128, traceparent 63, token
   510) are **profile policy**;
   conversions from the semantic model fail — never truncate — on
   overflow, and larger values remain fully supported by the semantic
   model and the field block.

Other revisions:

* Magic changed `"UPF1"` to `"UFA1"`; the profile is 928 bytes and carries
  one `u32` payload-encoding registry identifier.
* Kind/priority/presence vocabularies are defined by this experimental native
  profile with explicit projection tables to the protobuf enums; the value
  coincidence is a documented choice, not an inheritance.
* Payload identifiers are structurally 16-bit values stored in a four-byte
  slot. Presence is independent of zero, including a present empty payload
  with contract-defined encoding. Allocation/decoder policy is not inferred
  by the field codec.
* TTL is carried as `u64` nanoseconds (matching the semantic
  `Duration`); the legacy 32-bit-millisecond form is a fallible
  projection.
* The presence-bit vocabulary is shared verbatim between the field block
  and the ABI profile, so a fixed-profile consumer and a field-block
  consumer reason about the same bits.

## TTL projection and canonicalization

| Native TTL | Rust UAttributes TTL | Protobuf scalar TTL | Reverse native TTL |
| --- | --- | --- | --- |
| Absent | None | 0 | Absent |
| Present zero nanoseconds | None | 0 | Absent |
| Positive exact milliseconds fitting u32 | Some(milliseconds) | Same value | Same duration |
| Positive fractional milliseconds or overflow | Error | No projection | Not applicable |

The native profiles retain a present zero TTL; classic projection intentionally
canonicalizes it to no expiry and loses its separate presence. Requests still
require positive TTL. The explicit-zero golden in `tests/golden/wire-metadata/`
pins the native presence bit and zero bytes, while the projection test pins the
different classic/protobuf canonical form. Existing wire and ABI layouts do not
change to implement this rule.
