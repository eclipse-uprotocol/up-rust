/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

/*
 * uProtocol UFrame metadata — fixed-layout ABI profile v1 (C11).
 *
 * IMPORTANT — this is a *profile*, not the canonical model.
 *
 *   - The canonical semantic model is `UFrameMetadata` (owned, ergonomic,
 *     variable-length fields) in each language SDK.
 *   - The canonical byte serialization is the variable-length "UFrame
 *     metadata field block v1" (see WIRE-FORMAT.md); transports place those
 *     bytes behind their own fixed placement headers (iceoryx2
 *     UProtocolHeader, LoLa "ULOL" header, Zenoh attachments, ...).
 *   - This fixed struct exists only for boundaries where both sides
 *     explicitly agree to exchange metadata as a directly readable typed
 *     object (e.g. an iceoryx2 user-header profile for C peers, or
 *     deterministic byte-image fixtures).
 *
 * Layout rules: standard-layout, self-contained (no pointers, no heap, no
 * destructors), fixed-width integers and byte arrays only, explicit padding
 * only. Multi-byte integers are little-endian in serialized/shared images
 * (flags bit 0 MUST be set by producers).
 *
 * Conversions from the semantic model MUST fail — never truncate — when a
 * value exceeds a capacity of this profile. Capacities are profile policy,
 * not limits of the semantic model.
 *
 * Cross-language matching identity:
 *   type name  "uprotocol.v2.UFrameMetadataAbiV1"
 *   size       1096
 *   alignment  8
 */

#ifndef UPROTOCOL_UFRAME_METADATA_ABI_V1_H
#define UPROTOCOL_UFRAME_METADATA_ABI_V1_H

#include <stdint.h>
#include <stddef.h>
#include <assert.h>

#ifdef __cplusplus
extern "C" {
#endif

#define UFRAME_ABI_MAGIC_0 'U'
#define UFRAME_ABI_MAGIC_1 'F'
#define UFRAME_ABI_MAGIC_2 'A'
#define UFRAME_ABI_MAGIC_3 '1'
#define UFRAME_ABI_VERSION 1u
#define UFRAME_ABI_SIZE 1096u
#define UFRAME_ABI_ALIGN 8u
#define UFRAME_ABI_TYPE_NAME "uprotocol.v2.UFrameMetadataAbiV1"

/* flags */
#define UFRAME_ABI_FLAG_LITTLE_ENDIAN 0x01u

/* presence bits (shared vocabulary with the canonical field block v1) */
#define UFRAME_FIELD_SINK             (1u << 0)
#define UFRAME_FIELD_REQID            (1u << 1)
#define UFRAME_FIELD_TTL              (1u << 2)
#define UFRAME_FIELD_COMM_STATUS      (1u << 3)
#define UFRAME_FIELD_PERMISSION_LEVEL (1u << 4)
#define UFRAME_FIELD_TOKEN            (1u << 5)
#define UFRAME_FIELD_TRACEPARENT      (1u << 6)
#define UFRAME_FIELD_PAYLOAD_ENCODING (1u << 7)
#define UFRAME_FIELD_MASK_V1          0xFFu

/* frame message kind wire codes (normative UFrame registry; the numbering
 * deliberately coincides with the legacy protobuf UMessageType projection) */
#define UFRAME_KIND_PUBLISH      1u
#define UFRAME_KIND_REQUEST      2u
#define UFRAME_KIND_RESPONSE     3u
#define UFRAME_KIND_NOTIFICATION 4u

/* frame priority wire codes; 0 = absent */
#define UFRAME_PRIORITY_ABSENT 0u
#define UFRAME_PRIORITY_CS0    1u
#define UFRAME_PRIORITY_CS1    2u
#define UFRAME_PRIORITY_CS2    3u
#define UFRAME_PRIORITY_CS3    4u
#define UFRAME_PRIORITY_CS4    5u
#define UFRAME_PRIORITY_CS5    6u
#define UFRAME_PRIORITY_CS6    7u

/* profile capacities (policy of this profile only) */
#define UFRAME_ABI_AUTHORITY_CAPACITY    128u
#define UFRAME_ABI_LITERAL_ID_CAPACITY    64u
#define UFRAME_ABI_CONTENT_TYPE_CAPACITY 100u
#define UFRAME_ABI_TRACEPARENT_CAPACITY   63u
#define UFRAME_ABI_TOKEN_CAPACITY        510u

/* payload encoding component flags */
#define UFRAME_ABI_ENCODING_HAS_REGISTRY_ID 0x01u

/* Payload encoding registry ids 1..=8 are permanently reserved and
 * value-compatible with the legacy UPayloadFormat enum:
 *   1 protobuf-wrapped-in-any, 2 protobuf, 3 json, 4 someip, 5 someip-tlv,
 *   6 raw, 7 text, 8 shm.
 * 9..=0x7FFFFFFF: future registered encodings. 0x80000000..: vendor use. */

typedef struct uframe_uuid_abi {
    uint64_t msb;
    uint64_t lsb;
} uframe_uuid_abi; /* size 16, align 8 */

typedef struct uframe_uuri_abi {
    uint32_t ue_id;
    uint16_t resource_id;
    uint8_t ue_version_major;
    uint8_t authority_name_len;
    uint8_t authority_name[UFRAME_ABI_AUTHORITY_CAPACITY]; /* UTF-8, not NUL-terminated */
} uframe_uuri_abi; /* size 136, align 4 */

typedef struct uframe_payload_encoding_abi {
    uint32_t registry_id;      /* valid iff component_flags bit 0 */
    uint8_t literal_id_len;    /* 0 = absent */
    uint8_t content_type_len;  /* 0 = absent */
    uint8_t component_flags;   /* UFRAME_ABI_ENCODING_*; other bits MUST be 0 */
    uint8_t reserved0;         /* MUST be 0 */
    uint8_t literal_id[UFRAME_ABI_LITERAL_ID_CAPACITY];     /* UTF-8 */
    uint8_t content_type[UFRAME_ABI_CONTENT_TYPE_CAPACITY]; /* UTF-8 */
} uframe_payload_encoding_abi; /* size 172, align 4 */

typedef struct uframe_traceparent_abi {
    uint8_t len;
    uint8_t bytes[UFRAME_ABI_TRACEPARENT_CAPACITY]; /* UTF-8 */
} uframe_traceparent_abi; /* size 64, align 1 */

typedef struct uframe_token_abi {
    uint16_t len;
    uint8_t bytes[UFRAME_ABI_TOKEN_CAPACITY]; /* UTF-8 */
} uframe_token_abi; /* size 512, align 2 */

/*
 * Fields guarded by a UFRAME_FIELD_* presence bit are meaningful only when
 * that bit is set; producers MUST zero absent fields. payload_size is
 * meaningful iff UFRAME_FIELD_PAYLOAD_ENCODING is set (v1 invariant: a
 * frame carries payload bytes exactly when it declares a payload encoding)
 * and MUST be zero otherwise.
 */
typedef struct uframe_metadata_abi_v1 {
    /* identification / evolution: 0..8 */
    uint8_t magic[4];      /* "UFA1" */
    uint8_t version;       /* UFRAME_ABI_VERSION */
    uint8_t flags;         /* UFRAME_ABI_FLAG_* */
    uint16_t metadata_size; /* UFRAME_ABI_SIZE in v1 */

    /* presence & small scalars: 8..16 */
    uint32_t presence;     /* UFRAME_FIELD_* bits */
    uint8_t kind;          /* UFRAME_KIND_* */
    uint8_t priority;      /* UFRAME_PRIORITY_* */
    uint8_t comm_status;   /* UCode value; iff FIELD_COMM_STATUS */
    uint8_t reserved0;     /* MUST be 0 */

    /* scalars: 16..32 */
    uint64_t ttl_ns;           /* iff FIELD_TTL */
    uint32_t permission_level; /* iff FIELD_PERMISSION_LEVEL */
    uint32_t reserved1;        /* MUST be 0 */

    /* identifiers: 32..64 */
    uframe_uuid_abi id;
    uframe_uuid_abi reqid; /* iff FIELD_REQID */

    /* payload description: 64..244 */
    uint64_t payload_size;                        /* iff FIELD_PAYLOAD_ENCODING */
    uframe_payload_encoding_abi payload_encoding; /* iff FIELD_PAYLOAD_ENCODING */

    /* addressing: 244..516 */
    uframe_uuri_abi source;
    uframe_uuri_abi sink; /* iff FIELD_SINK */

    /* tracing / auth: 516..1092 */
    uframe_traceparent_abi traceparent; /* iff FIELD_TRACEPARENT */
    uframe_token_abi token;             /* iff FIELD_TOKEN */

    /* explicit tail padding: 1092..1096 */
    uint8_t reserved_tail[4]; /* MUST be 0 */
} uframe_metadata_abi_v1;

/* ---- compile-time layout conformance (normative for profile v1) ---- */

static_assert(sizeof(uframe_uuid_abi) == 16, "uframe_uuid_abi size");
static_assert(_Alignof(uframe_uuid_abi) == 8, "uframe_uuid_abi align");
static_assert(sizeof(uframe_uuri_abi) == 136, "uframe_uuri_abi size");
static_assert(_Alignof(uframe_uuri_abi) == 4, "uframe_uuri_abi align");
static_assert(sizeof(uframe_payload_encoding_abi) == 172, "encoding size");
static_assert(_Alignof(uframe_payload_encoding_abi) == 4, "encoding align");
static_assert(sizeof(uframe_traceparent_abi) == 64, "traceparent size");
static_assert(sizeof(uframe_token_abi) == 512, "token size");

static_assert(sizeof(uframe_metadata_abi_v1) == UFRAME_ABI_SIZE, "profile size");
static_assert(_Alignof(uframe_metadata_abi_v1) == UFRAME_ABI_ALIGN, "profile align");

static_assert(offsetof(uframe_metadata_abi_v1, magic) == 0, "magic offset");
static_assert(offsetof(uframe_metadata_abi_v1, version) == 4, "version offset");
static_assert(offsetof(uframe_metadata_abi_v1, flags) == 5, "flags offset");
static_assert(offsetof(uframe_metadata_abi_v1, metadata_size) == 6, "metadata_size offset");
static_assert(offsetof(uframe_metadata_abi_v1, presence) == 8, "presence offset");
static_assert(offsetof(uframe_metadata_abi_v1, kind) == 12, "kind offset");
static_assert(offsetof(uframe_metadata_abi_v1, priority) == 13, "priority offset");
static_assert(offsetof(uframe_metadata_abi_v1, comm_status) == 14, "comm_status offset");
static_assert(offsetof(uframe_metadata_abi_v1, reserved0) == 15, "reserved0 offset");
static_assert(offsetof(uframe_metadata_abi_v1, ttl_ns) == 16, "ttl_ns offset");
static_assert(offsetof(uframe_metadata_abi_v1, permission_level) == 24, "permission_level offset");
static_assert(offsetof(uframe_metadata_abi_v1, reserved1) == 28, "reserved1 offset");
static_assert(offsetof(uframe_metadata_abi_v1, id) == 32, "id offset");
static_assert(offsetof(uframe_metadata_abi_v1, reqid) == 48, "reqid offset");
static_assert(offsetof(uframe_metadata_abi_v1, payload_size) == 64, "payload_size offset");
static_assert(offsetof(uframe_metadata_abi_v1, payload_encoding) == 72, "payload_encoding offset");
static_assert(offsetof(uframe_metadata_abi_v1, source) == 244, "source offset");
static_assert(offsetof(uframe_metadata_abi_v1, sink) == 380, "sink offset");
static_assert(offsetof(uframe_metadata_abi_v1, traceparent) == 516, "traceparent offset");
static_assert(offsetof(uframe_metadata_abi_v1, token) == 580, "token offset");
static_assert(offsetof(uframe_metadata_abi_v1, reserved_tail) == 1092, "reserved_tail offset");

static_assert(offsetof(uframe_payload_encoding_abi, registry_id) == 0, "enc registry_id offset");
static_assert(offsetof(uframe_payload_encoding_abi, literal_id_len) == 4, "enc literal_id_len offset");
static_assert(offsetof(uframe_payload_encoding_abi, content_type_len) == 5, "enc content_type_len offset");
static_assert(offsetof(uframe_payload_encoding_abi, component_flags) == 6, "enc component_flags offset");
static_assert(offsetof(uframe_payload_encoding_abi, reserved0) == 7, "enc reserved0 offset");
static_assert(offsetof(uframe_payload_encoding_abi, literal_id) == 8, "enc literal_id offset");
static_assert(offsetof(uframe_payload_encoding_abi, content_type) == 72, "enc content_type offset");

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* UPROTOCOL_UFRAME_METADATA_ABI_V1_H */
