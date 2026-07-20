/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

/*
 * uProtocol UFrame metadata — fixed-layout ABI profile v1 (C++17).
 *
 * IMPORTANT — this is a *profile*, not the canonical model. See the C
 * header and WIRE-FORMAT.md for the layering rationale: the canonical
 * semantic model is each SDK's owned `UFrameMetadata`, the canonical byte
 * form is the variable-length metadata field block, and this fixed struct
 * exists only for boundaries that explicitly opt into typed fixed-layout
 * metadata interop (e.g. an iceoryx2 user-header profile).
 *
 * The struct is standard-layout and trivially copyable, contains no
 * pointers, heap ownership, or destructors, and is byte-for-byte identical
 * to the C and Rust definitions (enforced below with static_assert).
 *
 * Conversions from a semantic C++ metadata type MUST fail — never truncate
 * — when a value exceeds a profile capacity.
 *
 * Cross-language matching identity (e.g. iceoryx2):
 *   type name  "uprotocol.v2.UFrameMetadataAbiV1"  (UFRAME_ABI_TYPE_NAME)
 *   size       1096
 *   alignment  8
 */

#ifndef UPROTOCOL_UFRAME_METADATA_ABI_V1_HPP
#define UPROTOCOL_UFRAME_METADATA_ABI_V1_HPP

#include <cstddef>
#include <cstdint>
#include <type_traits>

namespace uprotocol::v2 {

inline constexpr char UFRAME_ABI_TYPE_NAME[] = "uprotocol.v2.UFrameMetadataAbiV1";
inline constexpr std::uint8_t UFRAME_ABI_VERSION = 1;
inline constexpr std::size_t UFRAME_ABI_SIZE = 1096;
inline constexpr std::size_t UFRAME_ABI_ALIGN = 8;

inline constexpr std::uint8_t UFRAME_ABI_FLAG_LITTLE_ENDIAN = 0x01;

// presence bits (shared vocabulary with the canonical field block v1)
inline constexpr std::uint32_t UFRAME_FIELD_SINK = 1u << 0;
inline constexpr std::uint32_t UFRAME_FIELD_REQID = 1u << 1;
inline constexpr std::uint32_t UFRAME_FIELD_TTL = 1u << 2;
inline constexpr std::uint32_t UFRAME_FIELD_COMM_STATUS = 1u << 3;
inline constexpr std::uint32_t UFRAME_FIELD_PERMISSION_LEVEL = 1u << 4;
inline constexpr std::uint32_t UFRAME_FIELD_TOKEN = 1u << 5;
inline constexpr std::uint32_t UFRAME_FIELD_TRACEPARENT = 1u << 6;
inline constexpr std::uint32_t UFRAME_FIELD_PAYLOAD_ENCODING = 1u << 7;
inline constexpr std::uint32_t UFRAME_FIELD_MASK_V1 = 0xFF;

// frame message kind wire codes (normative UFrame registry)
enum class UFrameKind : std::uint8_t {
    Publish = 1,
    Request = 2,
    Response = 3,
    Notification = 4,
};

// frame priority wire codes; 0 = absent
enum class UFramePriority : std::uint8_t {
    Absent = 0,
    CS0 = 1,
    CS1 = 2,
    CS2 = 3,
    CS3 = 4,
    CS4 = 5,
    CS5 = 6,
    CS6 = 7,
};

// profile capacities (policy of this profile only)
inline constexpr std::size_t UFRAME_ABI_AUTHORITY_CAPACITY = 128;
inline constexpr std::size_t UFRAME_ABI_LITERAL_ID_CAPACITY = 64;
inline constexpr std::size_t UFRAME_ABI_CONTENT_TYPE_CAPACITY = 100;
inline constexpr std::size_t UFRAME_ABI_TRACEPARENT_CAPACITY = 63;
inline constexpr std::size_t UFRAME_ABI_TOKEN_CAPACITY = 510;

inline constexpr std::uint8_t UFRAME_ABI_ENCODING_HAS_REGISTRY_ID = 0x01;

struct UUuidAbi {
    std::uint64_t msb;
    std::uint64_t lsb;
};

struct UUriAbi {
    std::uint32_t ue_id;
    std::uint16_t resource_id;
    std::uint8_t ue_version_major;
    std::uint8_t authority_name_len;
    std::uint8_t authority_name[UFRAME_ABI_AUTHORITY_CAPACITY]; // UTF-8
};

struct UPayloadEncodingAbi {
    std::uint32_t registry_id;    // valid iff component_flags bit 0
    std::uint8_t literal_id_len;  // 0 = absent
    std::uint8_t content_type_len; // 0 = absent
    std::uint8_t component_flags; // other bits MUST be 0
    std::uint8_t reserved0;       // MUST be 0
    std::uint8_t literal_id[UFRAME_ABI_LITERAL_ID_CAPACITY];     // UTF-8
    std::uint8_t content_type[UFRAME_ABI_CONTENT_TYPE_CAPACITY]; // UTF-8
};

struct UTraceparentAbi {
    std::uint8_t len;
    std::uint8_t bytes[UFRAME_ABI_TRACEPARENT_CAPACITY]; // UTF-8
};

struct UTokenAbi {
    std::uint16_t len;
    std::uint8_t bytes[UFRAME_ABI_TOKEN_CAPACITY]; // UTF-8
};

// Fields guarded by a UFRAME_FIELD_* presence bit are meaningful only when
// that bit is set; producers MUST zero absent fields. payload_size is
// meaningful iff UFRAME_FIELD_PAYLOAD_ENCODING is set and MUST be zero
// otherwise (v1 invariant: payload bytes iff payload encoding).
struct UFrameMetadataAbiV1 {
    // identification / evolution: 0..8
    std::uint8_t magic[4]; // "UFA1"
    std::uint8_t version;
    std::uint8_t flags;
    std::uint16_t metadata_size;

    // presence & small scalars: 8..16
    std::uint32_t presence;
    std::uint8_t kind;
    std::uint8_t priority;
    std::uint8_t comm_status;
    std::uint8_t reserved0;

    // scalars: 16..32
    std::uint64_t ttl_ns;
    std::uint32_t permission_level;
    std::uint32_t reserved1;

    // identifiers: 32..64
    UUuidAbi id;
    UUuidAbi reqid;

    // payload description: 64..244
    std::uint64_t payload_size;
    UPayloadEncodingAbi payload_encoding;

    // addressing: 244..516
    UUriAbi source;
    UUriAbi sink;

    // tracing / auth: 516..1092
    UTraceparentAbi traceparent;
    UTokenAbi token;

    // explicit tail padding: 1092..1096
    std::uint8_t reserved_tail[4];
};

// ---- compile-time layout conformance (normative for profile v1) ----

static_assert(std::is_standard_layout_v<UFrameMetadataAbiV1>);
static_assert(std::is_trivially_copyable_v<UFrameMetadataAbiV1>);

static_assert(sizeof(UUuidAbi) == 16 && alignof(UUuidAbi) == 8);
static_assert(sizeof(UUriAbi) == 136 && alignof(UUriAbi) == 4);
static_assert(sizeof(UPayloadEncodingAbi) == 172 && alignof(UPayloadEncodingAbi) == 4);
static_assert(sizeof(UTraceparentAbi) == 64 && alignof(UTraceparentAbi) == 1);
static_assert(sizeof(UTokenAbi) == 512 && alignof(UTokenAbi) == 2);

static_assert(sizeof(UFrameMetadataAbiV1) == UFRAME_ABI_SIZE);
static_assert(alignof(UFrameMetadataAbiV1) == UFRAME_ABI_ALIGN);

static_assert(offsetof(UFrameMetadataAbiV1, magic) == 0);
static_assert(offsetof(UFrameMetadataAbiV1, version) == 4);
static_assert(offsetof(UFrameMetadataAbiV1, flags) == 5);
static_assert(offsetof(UFrameMetadataAbiV1, metadata_size) == 6);
static_assert(offsetof(UFrameMetadataAbiV1, presence) == 8);
static_assert(offsetof(UFrameMetadataAbiV1, kind) == 12);
static_assert(offsetof(UFrameMetadataAbiV1, priority) == 13);
static_assert(offsetof(UFrameMetadataAbiV1, comm_status) == 14);
static_assert(offsetof(UFrameMetadataAbiV1, reserved0) == 15);
static_assert(offsetof(UFrameMetadataAbiV1, ttl_ns) == 16);
static_assert(offsetof(UFrameMetadataAbiV1, permission_level) == 24);
static_assert(offsetof(UFrameMetadataAbiV1, reserved1) == 28);
static_assert(offsetof(UFrameMetadataAbiV1, id) == 32);
static_assert(offsetof(UFrameMetadataAbiV1, reqid) == 48);
static_assert(offsetof(UFrameMetadataAbiV1, payload_size) == 64);
static_assert(offsetof(UFrameMetadataAbiV1, payload_encoding) == 72);
static_assert(offsetof(UFrameMetadataAbiV1, source) == 244);
static_assert(offsetof(UFrameMetadataAbiV1, sink) == 380);
static_assert(offsetof(UFrameMetadataAbiV1, traceparent) == 516);
static_assert(offsetof(UFrameMetadataAbiV1, token) == 580);
static_assert(offsetof(UFrameMetadataAbiV1, reserved_tail) == 1092);

static_assert(offsetof(UPayloadEncodingAbi, registry_id) == 0);
static_assert(offsetof(UPayloadEncodingAbi, literal_id) == 8);
static_assert(offsetof(UPayloadEncodingAbi, content_type) == 72);

} // namespace uprotocol::v2

#endif // UPROTOCOL_UFRAME_METADATA_ABI_V1_HPP
