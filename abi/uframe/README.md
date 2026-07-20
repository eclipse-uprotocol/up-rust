# uframe-abi (revision 2)

Cross-language artifacts for native UFrame metadata, revised per the
reframing audits: the fixed-layout struct is now an **opt-in derived
profile**, and the primary cross-language contract is the **variable-length
metadata field block**.

## Contents

| path | what | verified with |
|---|---|---|
| `WIRE-FORMAT.md` | **Normative** byte contracts: metadata field block v1, `UPFE` whole-frame envelope, kind/priority/presence/encoding registries | round-trip unit tests in `up-rust` (`frame::codec`, `frame::envelope`) |
| `rust/uframe_metadata_abi_v1.rs` | standalone `no_std` Rust reference definition of the fixed ABI profile | `rustc 1.89 --edition 2021 -D warnings` + `const` layout asserts |
| `c/uframe_metadata_abi_v1.h` (+ `check_layout.c`) | C11 definition | `gcc -std=c11 -Wall -Wextra -Werror -pedantic` + `static_assert` |
| `cpp/uframe_metadata_abi_v1.hpp` (+ `check_layout.cpp`) | C++17 definition | `g++ -std=c++17 -Wall -Wextra -Werror -pedantic` + `static_assert` |

All three profile definitions assert the same sizes, alignments, and field
offsets at compile time; a drift in any language fails its build.

## What changed from revision 1

Revision 1 proposed one fixed 1024-byte `UFrameMetadata` as the semantic
model, the ABI, and the canonical wire image. Per the audits, those are now
three separate things:

1. **Semantic model** — `up_rust::UFrameMetadata` (owned, ergonomic,
   no fixed capacities). Not in this folder; it lives in each SDK.
2. **Canonical bytes** — the field block in `WIRE-FORMAT.md`. Variable
   length, presence-driven. This is what selected-wire codecs emit, what
   sits behind iceoryx2's `UProtocolHeader` and LoLa's `ULOL` placement
   headers, and what the `UPFE` envelope carries over classic channels.
   *This is the contract C/C++ peers of those transports parse.*
3. **Fixed ABI profile** — `UFrameMetadataAbiV1` (this folder,
   size 1096, align 8, magic `"UFA1"`, type name
   `uprotocol.v2.UFrameMetadataAbiV1`). For boundaries where both sides
   explicitly agree to exchange a directly readable typed struct, e.g. an
   iceoryx2 user-header profile for C consumers or deterministic test
   fixtures. Its per-field capacities (authority 128, literal id 64,
   content type 100, traceparent 63, token 510) are **profile policy**;
   conversions from the semantic model fail — never truncate — on
   overflow, and larger values remain fully supported by the semantic
   model and the field block.

Other revisions:

* Magic changed `"UPF1"` → `"UFA1"` and the total size changed
  1024 → 1096, because the payload-encoding member now mirrors the open
  three-component encoding identity (registry id + literal id + content
  type) instead of a closed `(u16 id, content type)` pair.
* Kind/priority/presence vocabularies are now normatively defined by the
  UFrame registry (`WIRE-FORMAT.md` §3) with explicit projection tables to
  the legacy protobuf enums; the value coincidence is a documented choice,
  not an inheritance.
* TTL is carried as `u64` nanoseconds (matching the semantic
  `Duration`); the legacy 32-bit-millisecond form is a fallible
  projection.
* The presence-bit vocabulary is shared verbatim between the field block
  and the ABI profile, so a fixed-profile consumer and a field-block
  consumer reason about the same bits.
