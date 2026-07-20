/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! uProtocol UFrame metadata — fixed-layout ABI profile v1 (standalone Rust
//! reference definition, no dependencies).
//!
//! IMPORTANT — this is a *profile*, not the canonical model. The canonical
//! semantic model is `up_rust::UFrameMetadata`; the canonical byte
//! serialization is the variable-length metadata field block (see
//! WIRE-FORMAT.md and `up_rust::frame::codec`); this fixed struct exists for
//! boundaries that explicitly opt into typed fixed-layout metadata interop.
//! The `up-rust` crate ships this same layout with fallible conversions as
//! `up_rust::frame::abi::UFrameMetadataAbiV1`.
//!
//! Layout rules: `#[repr(C)]`, self-contained (no pointers, heap, or
//! `Drop`), fixed-width integers and byte arrays only, explicit padding
//! only, little-endian serialized images (flags bit 0). Conversions from
//! the semantic model MUST fail — never truncate — on capacity overflow.
//!
//! Cross-language matching identity: type name
//! `uprotocol.v2.UFrameMetadataAbiV1`, size 1096, alignment 8.

#![no_std]
#![allow(dead_code)]

pub const UFRAME_ABI_MAGIC: [u8; 4] = *b"UFA1";
pub const UFRAME_ABI_VERSION: u8 = 1;
pub const UFRAME_ABI_SIZE: usize = 1096;
pub const UFRAME_ABI_ALIGN: usize = 8;
pub const UFRAME_ABI_TYPE_NAME: &str = "uprotocol.v2.UFrameMetadataAbiV1";

pub const UFRAME_ABI_FLAG_LITTLE_ENDIAN: u8 = 0x01;

// presence bits (shared vocabulary with the canonical field block v1)
pub const UFRAME_FIELD_SINK: u32 = 1 << 0;
pub const UFRAME_FIELD_REQID: u32 = 1 << 1;
pub const UFRAME_FIELD_TTL: u32 = 1 << 2;
pub const UFRAME_FIELD_COMM_STATUS: u32 = 1 << 3;
pub const UFRAME_FIELD_PERMISSION_LEVEL: u32 = 1 << 4;
pub const UFRAME_FIELD_TOKEN: u32 = 1 << 5;
pub const UFRAME_FIELD_TRACEPARENT: u32 = 1 << 6;
pub const UFRAME_FIELD_PAYLOAD_ENCODING: u32 = 1 << 7;
pub const UFRAME_FIELD_MASK_V1: u32 = 0xFF;

// frame message kind wire codes (normative UFrame registry)
pub const UFRAME_KIND_PUBLISH: u8 = 1;
pub const UFRAME_KIND_REQUEST: u8 = 2;
pub const UFRAME_KIND_RESPONSE: u8 = 3;
pub const UFRAME_KIND_NOTIFICATION: u8 = 4;

// frame priority wire codes; 0 = absent
pub const UFRAME_PRIORITY_ABSENT: u8 = 0;
pub const UFRAME_PRIORITY_CS0: u8 = 1;
pub const UFRAME_PRIORITY_CS6: u8 = 7;

// profile capacities (policy of this profile only)
pub const UFRAME_ABI_AUTHORITY_CAPACITY: usize = 128;
pub const UFRAME_ABI_LITERAL_ID_CAPACITY: usize = 64;
pub const UFRAME_ABI_CONTENT_TYPE_CAPACITY: usize = 100;
pub const UFRAME_ABI_TRACEPARENT_CAPACITY: usize = 63;
pub const UFRAME_ABI_TOKEN_CAPACITY: usize = 510;

pub const UFRAME_ABI_ENCODING_HAS_REGISTRY_ID: u8 = 0x01;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UUuidAbi {
    pub msb: u64,
    pub lsb: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UUriAbi {
    pub ue_id: u32,
    pub resource_id: u16,
    pub ue_version_major: u8,
    pub authority_name_len: u8,
    pub authority_name: [u8; UFRAME_ABI_AUTHORITY_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UPayloadEncodingAbi {
    pub registry_id: u32,
    pub literal_id_len: u8,
    pub content_type_len: u8,
    pub component_flags: u8,
    pub reserved0: u8,
    pub literal_id: [u8; UFRAME_ABI_LITERAL_ID_CAPACITY],
    pub content_type: [u8; UFRAME_ABI_CONTENT_TYPE_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UTraceparentAbi {
    pub len: u8,
    pub bytes: [u8; UFRAME_ABI_TRACEPARENT_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UTokenAbi {
    pub len: u16,
    pub bytes: [u8; UFRAME_ABI_TOKEN_CAPACITY],
}

/// Fields guarded by a `UFRAME_FIELD_*` presence bit are meaningful only
/// when that bit is set; producers MUST zero absent fields. `payload_size`
/// is meaningful iff `UFRAME_FIELD_PAYLOAD_ENCODING` is set and MUST be
/// zero otherwise (v1 invariant: payload bytes iff payload encoding).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UFrameMetadataAbiV1 {
    // identification / evolution: 0..8
    pub magic: [u8; 4],
    pub version: u8,
    pub flags: u8,
    pub metadata_size: u16,

    // presence & small scalars: 8..16
    pub presence: u32,
    pub kind: u8,
    pub priority: u8,
    pub comm_status: u8,
    pub reserved0: u8,

    // scalars: 16..32
    pub ttl_ns: u64,
    pub permission_level: u32,
    pub reserved1: u32,

    // identifiers: 32..64
    pub id: UUuidAbi,
    pub reqid: UUuidAbi,

    // payload description: 64..244
    pub payload_size: u64,
    pub payload_encoding: UPayloadEncodingAbi,

    // addressing: 244..516
    pub source: UUriAbi,
    pub sink: UUriAbi,

    // tracing / auth: 516..1092
    pub traceparent: UTraceparentAbi,
    pub token: UTokenAbi,

    // explicit tail padding: 1092..1096
    pub reserved_tail: [u8; 4],
}

// ---- compile-time layout conformance (normative for profile v1) ----

const _: () = {
    use core::mem::{align_of, offset_of, size_of};

    assert!(size_of::<UUuidAbi>() == 16 && align_of::<UUuidAbi>() == 8);
    assert!(size_of::<UUriAbi>() == 136 && align_of::<UUriAbi>() == 4);
    assert!(size_of::<UPayloadEncodingAbi>() == 172 && align_of::<UPayloadEncodingAbi>() == 4);
    assert!(size_of::<UTraceparentAbi>() == 64 && align_of::<UTraceparentAbi>() == 1);
    assert!(size_of::<UTokenAbi>() == 512 && align_of::<UTokenAbi>() == 2);

    assert!(size_of::<UFrameMetadataAbiV1>() == UFRAME_ABI_SIZE);
    assert!(align_of::<UFrameMetadataAbiV1>() == UFRAME_ABI_ALIGN);

    assert!(offset_of!(UFrameMetadataAbiV1, magic) == 0);
    assert!(offset_of!(UFrameMetadataAbiV1, version) == 4);
    assert!(offset_of!(UFrameMetadataAbiV1, flags) == 5);
    assert!(offset_of!(UFrameMetadataAbiV1, metadata_size) == 6);
    assert!(offset_of!(UFrameMetadataAbiV1, presence) == 8);
    assert!(offset_of!(UFrameMetadataAbiV1, kind) == 12);
    assert!(offset_of!(UFrameMetadataAbiV1, priority) == 13);
    assert!(offset_of!(UFrameMetadataAbiV1, comm_status) == 14);
    assert!(offset_of!(UFrameMetadataAbiV1, reserved0) == 15);
    assert!(offset_of!(UFrameMetadataAbiV1, ttl_ns) == 16);
    assert!(offset_of!(UFrameMetadataAbiV1, permission_level) == 24);
    assert!(offset_of!(UFrameMetadataAbiV1, reserved1) == 28);
    assert!(offset_of!(UFrameMetadataAbiV1, id) == 32);
    assert!(offset_of!(UFrameMetadataAbiV1, reqid) == 48);
    assert!(offset_of!(UFrameMetadataAbiV1, payload_size) == 64);
    assert!(offset_of!(UFrameMetadataAbiV1, payload_encoding) == 72);
    assert!(offset_of!(UFrameMetadataAbiV1, source) == 244);
    assert!(offset_of!(UFrameMetadataAbiV1, sink) == 380);
    assert!(offset_of!(UFrameMetadataAbiV1, traceparent) == 516);
    assert!(offset_of!(UFrameMetadataAbiV1, token) == 580);
    assert!(offset_of!(UFrameMetadataAbiV1, reserved_tail) == 1092);

    assert!(offset_of!(UPayloadEncodingAbi, registry_id) == 0);
    assert!(offset_of!(UPayloadEncodingAbi, literal_id) == 8);
    assert!(offset_of!(UPayloadEncodingAbi, content_type) == 72);
};
