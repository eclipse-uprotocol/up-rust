# UFrame Metadata Byte Contracts (v1)

This document is the language-neutral, normative definition of the two byte
contracts every UFrame implementation must be able to parse, plus the
payload-encoding registry. Multi-byte integers are **little-endian**
throughout. Values that do not fit a length field are **encode-time errors**
— implementations never truncate.

The layering these contracts live in:

```text
UFrameMetadata (semantic model, per-language, owned/ergonomic)
  └─ metadata field block v1 (this doc, §1) ── canonical byte serialization
       ├─ carried behind native-prefix framing by selected-wire codecs
       ├─ carried behind iceoryx2's UProtocolHeader placement header
       ├─ carried behind LoLa's "ULOL" placement header
       ├─ carried as an opaque Zenoh attachment
       └─ carried inside the UPFE whole-frame envelope (§2) on classic channels
  └─ UFrameMetadataAbiV1 (fixed profile, see headers in this folder) —
       opt-in typed interop only; NOT the default carrier
```

## 1. Metadata field block, version 1

The canonical serialization of frame metadata. Variable-length and
presence-driven: absent optional fields cost zero bytes.

```text
offset  size  field
0       1     block_version        = 1
1       1     kind                 frame message kind wire code (§3.1)
2       1     priority             0 = absent, 1..=7 = CS0..=CS6 (§3.2)
3       1     reserved             MUST be 0
4       4     presence             u32 bitmask (§3.3); unknown bits MUST be 0
8       8     id.msb               u64
16      8     id.lsb               u64
```

Then, in this exact order, each section present only when its presence bit
is set:

```text
[FIELD_REQID]             u64 msb, u64 lsb
[FIELD_TTL]               u64 ttl in nanoseconds
[FIELD_COMM_STATUS]       i32 UCode value
[FIELD_PERMISSION_LEVEL]  u32
source UUri block         (always present; §1.1)
[FIELD_SINK]              UUri block
[FIELD_TOKEN]             u16 len, `len` UTF-8 bytes
[FIELD_TRACEPARENT]       u8 len, `len` UTF-8 bytes
[FIELD_PAYLOAD_ENCODING]  payload encoding block (§1.2)
```

Decoders MUST consume the entire input; trailing bytes are an error.
Decoders MUST reject unknown `block_version`, unknown presence bits, a
nonzero reserved byte, unknown kind/priority codes, and non-UTF-8 strings.
Decoded metadata MUST additionally satisfy the per-kind validity rules of
the semantic model (e.g. a request has a sink, a TTL > 0, and priority ≥
CS4).

### 1.1 UUri block

```text
u32  ue_id             (uE instance id << 16) | uE type id
u16  resource_id
u8   ue_version_major
u8   authority_name_len
...  authority_name    UTF-8, `authority_name_len` bytes (≤ 128 per UUri spec)
```

### 1.2 Payload encoding block

An open identity with up to three components; at least one MUST be present.

```text
u8   component_flags   bit0 registry_id, bit1 literal_id, bit2 content_type;
                       other bits MUST be 0
[bit0] u32 registry_id
[bit1] u16 len, `len` UTF-8 bytes (literal id, e.g. "up.xcdr-v2")
[bit2] u16 len, `len` UTF-8 bytes (RFC 6838 media type, may carry parameters)
```

## 2. Native whole-frame envelope ("UPFE"), version 1

The lossless carrier for a complete native frame over classic byte channels
(an MQTT payload, a SOME/IP payload, a file, a test fixture). It mirrors the
physical pattern the shared-memory transports already use: a small fixed
*placement* header, then the variable metadata field block, then payload.

```text
offset  size  field
0       4     magic "UPFE"
4       1     envelope version = 1
5       1     payload presence: 0 = absent, 1 = present
6       2     reserved, MUST be 0
8       4     metadata_len  (u32)
12      8     payload_len   (u64; MUST be 0 when payload absent)
20      m     metadata field block (§1), m = metadata_len
20+m    p     payload bytes, p = payload_len
```

Content type: `application/vnd.uprotocol.uframe;version=1`.

Decoders MUST verify that the input length equals `20 + metadata_len +
payload_len`, that presence and `payload_len` agree, and that the decoded
frame satisfies the v1 frame invariant: payload bytes are present **iff**
the metadata declares a payload encoding (a present *empty* payload — the
`presence=1, payload_len=0` case — is distinct from an absent payload).

## 3. Registries

### 3.1 Frame message kind wire codes

Defined by the UFrame specification. The numbering deliberately coincides
with the legacy protobuf `UMessageType` values so the projection is
value-preserving; the UFrame registry below is normative from here on.

| code | kind         | legacy `UMessageType` projection |
|------|--------------|----------------------------------|
| 0    | reserved (invalid) | —                          |
| 1    | Publish      | `UMESSAGE_TYPE_PUBLISH`          |
| 2    | Request      | `UMESSAGE_TYPE_REQUEST`          |
| 3    | Response     | `UMESSAGE_TYPE_RESPONSE`         |
| 4    | Notification | `UMESSAGE_TYPE_NOTIFICATION`     |
| 5..=255 | reserved  | —                                |

### 3.2 Frame priority wire codes

| code | priority | legacy `UPriority` projection |
|------|----------|-------------------------------|
| 0    | absent (wire/ABI representations only) | — |
| 1..=7 | CS0..=CS6 | `UPRIORITY_CS0..=CS6`       |
| 8..=255 | reserved | —                          |

### 3.3 Presence bits (shared by field block and ABI profile)

| bit | name                     |
|-----|--------------------------|
| 0   | FIELD_SINK               |
| 1   | FIELD_REQID              |
| 2   | FIELD_TTL                |
| 3   | FIELD_COMM_STATUS        |
| 4   | FIELD_PERMISSION_LEVEL   |
| 5   | FIELD_TOKEN              |
| 6   | FIELD_TRACEPARENT        |
| 7   | FIELD_PAYLOAD_ENCODING   |
| 8..=31 | reserved (MUST be 0 in v1) |

### 3.4 Payload encoding registry ids

| id | literal id | media type | legacy `UPayloadFormat` |
|----|------------|------------|--------------------------|
| 0  | reserved (never a valid id) | — | (`UNSPECIFIED` has **no** encoding; absence of the block expresses it) |
| 1  | `up.protobuf-wrapped-in-any` | `application/x-protobuf` | `PROTOBUF_WRAPPED_IN_ANY` |
| 2  | `up.protobuf` | `application/protobuf` | `PROTOBUF` |
| 3  | `up.json` | `application/json` | `JSON` |
| 4  | `up.someip` | `application/x-someip` | `SOMEIP` |
| 5  | `up.someip-tlv` | `application/x-someip_tlv` | `SOMEIP_TLV` |
| 6  | `up.raw` | `application/octet-stream` | `RAW` |
| 7  | `up.text` | `text/plain` | `TEXT` |
| 8  | `up.shm` | `application/x-shm` | `SHM` |
| 9..=0x7FFF_FFFF | future registered encodings | | — |
| 0x8000_0000.. | vendor / private use | | — |

Ids 1..=8 are **permanently reserved** and value-compatible with the legacy
enum. Encodings such as `up.stable-container` and `up.xcdr-v2` are literal-id
identities today and may additionally register numeric ids from the open
range without changing any SDK enum.
