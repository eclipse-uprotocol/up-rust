/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Fixed-layout ABI **profile** of native frame metadata, version 1.
//!
//! MAINTENANCE NOTE (dead-code audits): the consumers of this module are
//! **cross-language** — C/C++ peers reading typed shared-memory headers and
//! future polyglot bindings. Zero references from Rust repositories is the
//! expected steady state and is NOT evidence this module is unused. Do not
//! remove it on the basis of a Rust-side usage census.
//!
//! [`UFrameMetadataAbiV1`] is a *derived representation* of the semantic
//! [`UFrameMetadata`] — not the canonical model, not the default wire form.
//! It exists for the boundaries that genuinely need a fixed, directly
//! readable C/C++/Rust struct: for example an iceoryx2 service whose peers
//! want metadata as a typed user header instead of parsing the canonical
//! field block, or deterministic byte-image test fixtures.
//!
//! Most transports do not need this profile: the selected-wire layer carries
//! variable metadata bytes produced by a metadata codec, and the
//! shared-memory transports (iceoryx2, LoLa) already use a small fixed
//! *placement* header plus that variable block. Use this profile only when
//! both sides explicitly agree on it.
//!
//! Layout rules (the shared-memory contract): `#[repr(C)]`, self-contained
//! (no pointers, no heap, no `Drop`), fixed size, explicit padding only,
//! fixed-width integers and byte arrays only, little-endian in serialized
//! form and native-endian in same-host shared memory. Compile-time
//! assertions at the bottom of this file fail the build if the layout
//! drifts.
//!
//! Conversions to and from [`UFrameMetadata`] are fallible and **never
//! truncate**: a value that exceeds a profile capacity is an error. The
//! per-field capacities are profile policy (documented on each constant),
//! not limits of the semantic model.
//!
//! ## Cross-language reader checklist
//!
//! 1. Match `UFRAME_ABI_TYPE_NAME`, size, alignment, magic, and version before
//!    interpreting a header.
//! 2. Mirror every field with fixed-width integer/byte-array types and C layout;
//!    never substitute native pointer, string, boolean, or enum layouts.
//! 3. Check the presence mask before reading optional fields and reject unknown
//!    mask/flag bits.
//! 4. Treat stored lengths as untrusted and verify each against its documented
//!    capacity before constructing a language-level string/view.
//! 5. Use the fallible conversion/golden byte-image tests as the arbiter. A
//!    value too large for this profile remains valid semantic metadata; choose
//!    the canonical field block instead of truncating it.
//!
//! The normative semantic model and metadata-profile registry live in
//! `up-spec/basics/uframe.adoc`; this Rust struct is one derived ABI profile,
//! not a language-specific conformance requirement.

use core::mem::{align_of, offset_of, size_of};
use std::time::Duration;

use crate::{FrameMessageKind, FramePriority, PayloadEncoding, UCode, UFrameMetadata, UUri, UUID};

// Reuse the canonical presence-bit vocabulary of the field block so the two
// representations stay aligned.
pub use crate::frame::codec::{
    FIELD_COMM_STATUS, FIELD_MASK_V1, FIELD_PAYLOAD_ENCODING, FIELD_PERMISSION_LEVEL, FIELD_REQID,
    FIELD_SINK, FIELD_TOKEN, FIELD_TRACEPARENT, FIELD_TTL,
};

/// Magic bytes at offset 0 of every [`UFrameMetadataAbiV1`]: `"UFA1"`.
pub const UFRAME_ABI_MAGIC: [u8; 4] = *b"UFA1";

/// ABI profile version described by this module.
pub const UFRAME_ABI_VERSION: u8 = 1;

/// Total size of [`UFrameMetadataAbiV1`] in bytes.
pub const UFRAME_ABI_SIZE: usize = 1096;

/// Alignment of [`UFrameMetadataAbiV1`] in bytes.
pub const UFRAME_ABI_ALIGN: usize = 8;

/// Semantic type name for cross-language service matching (e.g. iceoryx2
/// `#[type_name(...)]` in Rust and `IOX2_TYPE_NAME` in C++).
pub const UFRAME_ABI_TYPE_NAME: &str = "uprotocol.v2.UFrameMetadataAbiV1";

/// Flag bit: producer stored multi-byte integers little-endian. MUST be set
/// in the serialized form of profile v1.
pub const UFRAME_ABI_FLAG_LITTLE_ENDIAN: u8 = 0b0000_0001;

/// Profile capacity for a UUri authority name (the UUri spec limit).
pub const UFRAME_ABI_AUTHORITY_CAPACITY: usize = 128;
/// Profile capacity for a payload encoding literal id.
pub const UFRAME_ABI_LITERAL_ID_CAPACITY: usize = 64;
/// Profile capacity for a payload encoding content type.
pub const UFRAME_ABI_CONTENT_TYPE_CAPACITY: usize = 100;
/// Profile capacity for a W3C traceparent (version 00 uses 55 characters).
pub const UFRAME_ABI_TRACEPARENT_CAPACITY: usize = 63;
/// Profile capacity for an access token. This is a policy limit of this
/// profile only; conversions fail (never truncate) for larger tokens, which
/// remain fully supported by the semantic model and the canonical field
/// block.
pub const UFRAME_ABI_TOKEN_CAPACITY: usize = 510;

/// Errors returned by ABI profile conversions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UFrameAbiError {
    /// A value exceeds a fixed capacity of this profile.
    CapacityExceeded {
        /// Name of the offending field.
        field: &'static str,
    },
    /// The struct is not a structurally valid profile v1 value.
    InvalidProfile(String),
    /// The metadata violates frame invariants.
    Metadata(crate::UFrameMetadataError),
}

impl std::fmt::Display for UFrameAbiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityExceeded { field } => {
                write!(f, "value of `{field}` exceeds the ABI profile capacity")
            }
            Self::InvalidProfile(message) => write!(f, "invalid ABI profile value: {message}"),
            Self::Metadata(error) => write!(f, "invalid frame metadata: {error}"),
        }
    }
}

impl std::error::Error for UFrameAbiError {}

impl From<crate::UFrameMetadataError> for UFrameAbiError {
    fn from(value: crate::UFrameMetadataError) -> Self {
        Self::Metadata(value)
    }
}

/// UUID as two 64-bit halves. size 16, align 8, no padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UUuidAbi {
    /// Most significant 64 bits of the UUID.
    pub msb: u64,
    /// Least significant 64 bits of the UUID.
    pub lsb: u64,
}

/// Fixed-capacity UUri. size 136, align 4, no padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UUriAbi {
    /// uEntity identifier.
    pub ue_id: u32,
    /// Resource identifier within the uEntity.
    pub resource_id: u16,
    /// uEntity API major version.
    pub ue_version_major: u8,
    /// Length in bytes of the authority name.
    pub authority_name_len: u8,
    /// Authority name bytes (fixed capacity; `authority_name_len` bytes valid).
    pub authority_name: [u8; UFRAME_ABI_AUTHORITY_CAPACITY],
}

impl Default for UUriAbi {
    fn default() -> Self {
        Self {
            ue_id: 0,
            resource_id: 0,
            ue_version_major: 0,
            authority_name_len: 0,
            authority_name: [0; UFRAME_ABI_AUTHORITY_CAPACITY],
        }
    }
}

/// Fixed-capacity open payload encoding. size 172, align 4, no padding.
///
/// Mirrors the three identity components of [`PayloadEncoding`]:
/// `registry_id` (0 when absent — presence tracked via the component flags),
/// a literal id string, and a content type string.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UPayloadEncodingAbi {
    /// Compact registry identifier of the payload encoding.
    pub registry_id: u32,
    /// Length in bytes of the literal encoding id.
    pub literal_id_len: u8,
    /// Length in bytes of the content type.
    pub content_type_len: u8,
    /// bit0: registry_id present; bits 1..=7 MUST be zero.
    pub component_flags: u8,
    /// MUST be zero.
    pub _reserved: u8,
    /// Literal encoding id bytes (fixed capacity; `literal_id_len` bytes valid).
    pub literal_id: [u8; UFRAME_ABI_LITERAL_ID_CAPACITY],
    /// Content type bytes (fixed capacity; `content_type_len` bytes valid).
    pub content_type: [u8; UFRAME_ABI_CONTENT_TYPE_CAPACITY],
}

/// Component flag: `registry_id` carries a value.
pub const UFRAME_ABI_ENCODING_HAS_REGISTRY_ID: u8 = 0b0000_0001;

impl Default for UPayloadEncodingAbi {
    fn default() -> Self {
        Self {
            registry_id: 0,
            literal_id_len: 0,
            content_type_len: 0,
            component_flags: 0,
            _reserved: 0,
            literal_id: [0; UFRAME_ABI_LITERAL_ID_CAPACITY],
            content_type: [0; UFRAME_ABI_CONTENT_TYPE_CAPACITY],
        }
    }
}

/// Fixed-capacity traceparent. size 64, align 1, no padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UTraceparentAbi {
    /// Length in bytes of the traceparent value.
    pub len: u8,
    /// Traceparent bytes (fixed capacity; `len` bytes valid).
    pub bytes: [u8; UFRAME_ABI_TRACEPARENT_CAPACITY],
}

impl Default for UTraceparentAbi {
    fn default() -> Self {
        Self {
            len: 0,
            bytes: [0; UFRAME_ABI_TRACEPARENT_CAPACITY],
        }
    }
}

/// Fixed-capacity access token. size 512, align 2, no padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UTokenAbi {
    /// Length in bytes of the token value.
    pub len: u16,
    /// Token bytes (fixed capacity; `len` bytes valid).
    pub bytes: [u8; UFRAME_ABI_TOKEN_CAPACITY],
}

impl Default for UTokenAbi {
    fn default() -> Self {
        Self {
            len: 0,
            bytes: [0; UFRAME_ABI_TOKEN_CAPACITY],
        }
    }
}

/// Fixed-layout ABI profile v1 of native frame metadata.
///
/// size 1096, align 8, no implicit padding. A field guarded by a `FIELD_*`
/// presence bit is meaningful only when that bit is set; producers MUST
/// zero absent fields. `payload_size` is meaningful if and only if
/// [`FIELD_PAYLOAD_ENCODING`] is set (the v1 invariant: a frame carries
/// payload bytes exactly when it declares a payload encoding).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UFrameMetadataAbiV1 {
    // identification / evolution block: 0..8
    /// MUST equal [`UFRAME_ABI_MAGIC`].
    pub magic: [u8; 4],
    /// MUST equal [`UFRAME_ABI_VERSION`].
    pub version: u8,
    /// See `UFRAME_ABI_FLAG_*`.
    pub flags: u8,
    /// MUST equal [`UFRAME_ABI_SIZE`] in v1; consumers use this to skip
    /// trailing fields added by newer minor revisions.
    pub metadata_size: u16,

    // presence & small scalars: 8..16
    /// Bitwise OR of `FIELD_*` presence bits (shared with the canonical
    /// field block).
    pub presence: u32,
    /// [`FrameMessageKind`] wire code (1..=4).
    pub kind: u8,
    /// [`FramePriority`] wire code; 0 = absent.
    pub priority: u8,
    /// `UCode` value; valid iff [`FIELD_COMM_STATUS`].
    pub comm_status: u8,
    /// MUST be zero.
    pub _reserved0: u8,

    // scalars: 16..32
    /// Time-to-live in nanoseconds; valid iff [`FIELD_TTL`].
    pub ttl_ns: u64,
    /// Valid iff [`FIELD_PERMISSION_LEVEL`].
    pub permission_level: u32,
    /// MUST be zero.
    pub _reserved1: u32,

    // identifiers: 32..64
    /// Message id as a fixed-layout UUID.
    pub id: UUuidAbi,
    /// Valid iff [`FIELD_REQID`].
    pub reqid: UUuidAbi,

    // payload description: 64..244
    /// Number of payload bytes belonging to the frame; valid iff
    /// [`FIELD_PAYLOAD_ENCODING`], MUST be zero otherwise.
    pub payload_size: u64,
    /// Valid iff [`FIELD_PAYLOAD_ENCODING`].
    pub payload_encoding: UPayloadEncodingAbi,

    // addressing: 244..516
    /// Source address as a fixed-layout URI.
    pub source: UUriAbi,
    /// Valid iff [`FIELD_SINK`].
    pub sink: UUriAbi,

    // tracing / auth: 516..1092
    /// Valid iff [`FIELD_TRACEPARENT`].
    pub traceparent: UTraceparentAbi,
    /// Valid iff [`FIELD_TOKEN`].
    pub token: UTokenAbi,

    // explicit tail padding: 1092..1096
    /// MUST be zero.
    pub _reserved_tail: [u8; 4],
}

impl Default for UFrameMetadataAbiV1 {
    fn default() -> Self {
        Self {
            magic: UFRAME_ABI_MAGIC,
            version: UFRAME_ABI_VERSION,
            flags: UFRAME_ABI_FLAG_LITTLE_ENDIAN,
            metadata_size: UFRAME_ABI_SIZE as u16,
            presence: 0,
            kind: 0,
            priority: 0,
            comm_status: 0,
            _reserved0: 0,
            ttl_ns: 0,
            permission_level: 0,
            _reserved1: 0,
            id: UUuidAbi::default(),
            reqid: UUuidAbi::default(),
            payload_size: 0,
            payload_encoding: UPayloadEncodingAbi::default(),
            source: UUriAbi::default(),
            sink: UUriAbi::default(),
            traceparent: UTraceparentAbi::default(),
            token: UTokenAbi::default(),
            _reserved_tail: [0; 4],
        }
    }
}

fn fill_str(
    target: &mut [u8],
    target_len_max: usize,
    value: &str,
    field: &'static str,
) -> Result<usize, UFrameAbiError> {
    let bytes = value.as_bytes();
    if bytes.len() > target_len_max {
        return Err(UFrameAbiError::CapacityExceeded { field });
    }
    let target = target
        .get_mut(..bytes.len())
        .ok_or(UFrameAbiError::CapacityExceeded { field })?;
    target.copy_from_slice(bytes);
    Ok(bytes.len())
}

fn read_str<'a>(
    bytes: &'a [u8],
    len: usize,
    capacity: usize,
    field: &'static str,
) -> Result<&'a str, UFrameAbiError> {
    if len > capacity {
        return Err(UFrameAbiError::InvalidProfile(format!(
            "`{field}` length {len} exceeds capacity {capacity}"
        )));
    }
    let bytes = bytes.get(..len).ok_or_else(|| {
        UFrameAbiError::InvalidProfile(format!(
            "`{field}` length {len} exceeds available bytes {}",
            bytes.len()
        ))
    })?;
    core::str::from_utf8(bytes)
        .map_err(|error| UFrameAbiError::InvalidProfile(format!("`{field}` is not UTF-8: {error}")))
}

fn uuri_to_abi(uri: &UUri, field: &'static str) -> Result<UUriAbi, UFrameAbiError> {
    let mut abi = UUriAbi {
        ue_id: (u32::from(uri.uentity_instance_id()) << 16) | u32::from(uri.uentity_type_id()),
        resource_id: uri.resource_id(),
        ue_version_major: uri.uentity_major_version(),
        ..Default::default()
    };
    let len = fill_str(
        &mut abi.authority_name,
        UFRAME_ABI_AUTHORITY_CAPACITY,
        uri.authority_name(),
        field,
    )?;
    abi.authority_name_len = len as u8;
    Ok(abi)
}

fn uuri_from_abi(abi: &UUriAbi, field: &'static str) -> Result<UUri, UFrameAbiError> {
    let authority = read_str(
        &abi.authority_name,
        usize::from(abi.authority_name_len),
        UFRAME_ABI_AUTHORITY_CAPACITY,
        field,
    )?;
    UUri::try_from_parts(authority, abi.ue_id, abi.ue_version_major, abi.resource_id)
        .map_err(|error| UFrameAbiError::InvalidProfile(format!("invalid `{field}`: {error}")))
}

impl UFrameMetadataAbiV1 {
    /// Converts semantic frame metadata (plus the frame's payload size, when
    /// it carries a payload) into the fixed ABI profile.
    ///
    /// `payload_size` must be `Some` exactly when the metadata has a payload
    /// encoding, mirroring the frame-level invariant.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata is invalid, payload presence is
    /// inconsistent, or a value exceeds a profile capacity. Conversion never
    /// truncates.
    pub fn try_from_metadata(
        metadata: &UFrameMetadata,
        payload_size: Option<u64>,
    ) -> Result<Self, UFrameAbiError> {
        metadata.validate()?;
        match (
            payload_size.is_some(),
            metadata.payload_encoding().is_some(),
        ) {
            (true, true) | (false, false) => {}
            (true, false) => {
                return Err(UFrameAbiError::Metadata(
                    crate::UFrameMetadataError::PayloadWithoutEncoding,
                ))
            }
            (false, true) => {
                return Err(UFrameAbiError::Metadata(
                    crate::UFrameMetadataError::EncodingWithoutPayload,
                ))
            }
        }

        let (msb, lsb) = metadata.id().as_u64_pair();
        let mut abi = Self {
            kind: metadata.kind().wire_code(),
            priority: metadata.priority().map_or(0, FramePriority::wire_code),
            id: UUuidAbi { msb, lsb },
            source: uuri_to_abi(metadata.source(), "source.authority_name")?,
            ..Self::default()
        };

        if let Some(sink) = metadata.sink() {
            abi.sink = uuri_to_abi(sink, "sink.authority_name")?;
            abi.presence |= FIELD_SINK;
        }
        if let Some(reqid) = metadata.reqid() {
            let (msb, lsb) = reqid.as_u64_pair();
            abi.reqid = UUuidAbi { msb, lsb };
            abi.presence |= FIELD_REQID;
        }
        if let Some(ttl) = metadata.ttl() {
            abi.ttl_ns = u64::try_from(ttl.as_nanos())
                .map_err(|_| UFrameAbiError::CapacityExceeded { field: "ttl" })?;
            abi.presence |= FIELD_TTL;
        }
        if let Some(comm_status) = metadata.comm_status() {
            abi.comm_status = u8::try_from(comm_status.value()).map_err(|_| {
                UFrameAbiError::CapacityExceeded {
                    field: "comm_status",
                }
            })?;
            abi.presence |= FIELD_COMM_STATUS;
        }
        if let Some(permission_level) = metadata.permission_level() {
            abi.permission_level = permission_level;
            abi.presence |= FIELD_PERMISSION_LEVEL;
        }
        if let Some(token) = metadata.token() {
            let len = fill_str(
                &mut abi.token.bytes,
                UFRAME_ABI_TOKEN_CAPACITY,
                token,
                "token",
            )?;
            abi.token.len = len as u16;
            abi.presence |= FIELD_TOKEN;
        }
        if let Some(traceparent) = metadata.traceparent() {
            let len = fill_str(
                &mut abi.traceparent.bytes,
                UFRAME_ABI_TRACEPARENT_CAPACITY,
                traceparent,
                "traceparent",
            )?;
            abi.traceparent.len = len as u8;
            abi.presence |= FIELD_TRACEPARENT;
        }
        if let Some(encoding) = metadata.payload_encoding() {
            if let Some(registry_id) = encoding.registry_id() {
                abi.payload_encoding.registry_id = registry_id;
                abi.payload_encoding.component_flags |= UFRAME_ABI_ENCODING_HAS_REGISTRY_ID;
            }
            if let Some(literal_id) = encoding.literal_id() {
                let len = fill_str(
                    &mut abi.payload_encoding.literal_id,
                    UFRAME_ABI_LITERAL_ID_CAPACITY,
                    literal_id,
                    "payload_encoding.literal_id",
                )?;
                abi.payload_encoding.literal_id_len = len as u8;
            }
            if let Some(content_type) = encoding.content_type() {
                let len = fill_str(
                    &mut abi.payload_encoding.content_type,
                    UFRAME_ABI_CONTENT_TYPE_CAPACITY,
                    content_type,
                    "payload_encoding.content_type",
                )?;
                abi.payload_encoding.content_type_len = len as u8;
            }
            abi.payload_size = payload_size.unwrap_or_default();
            abi.presence |= FIELD_PAYLOAD_ENCODING;
        }
        Ok(abi)
    }

    /// Converts this profile value back into semantic frame metadata plus the
    /// frame's payload size (when the frame carries a payload).
    ///
    /// # Errors
    ///
    /// Returns an error if the profile value is structurally invalid or
    /// decodes into invalid metadata.
    pub fn try_to_metadata(&self) -> Result<(UFrameMetadata, Option<u64>), UFrameAbiError> {
        if self.magic != UFRAME_ABI_MAGIC {
            return Err(UFrameAbiError::InvalidProfile("wrong magic".to_string()));
        }
        if self.version != UFRAME_ABI_VERSION {
            return Err(UFrameAbiError::InvalidProfile(format!(
                "unsupported profile version {}",
                self.version
            )));
        }
        if usize::from(self.metadata_size) < UFRAME_ABI_SIZE {
            return Err(UFrameAbiError::InvalidProfile(
                "declared metadata_size is smaller than profile v1".to_string(),
            ));
        }
        if self.presence & !FIELD_MASK_V1 != 0 {
            return Err(UFrameAbiError::InvalidProfile(
                "unknown presence bits".to_string(),
            ));
        }
        if self._reserved0 != 0 || self._reserved1 != 0 || self._reserved_tail != [0; 4] {
            return Err(UFrameAbiError::InvalidProfile(
                "reserved fields must be zero".to_string(),
            ));
        }

        let kind = FrameMessageKind::from_wire_code(self.kind).ok_or_else(|| {
            UFrameAbiError::InvalidProfile(format!("unknown kind code {}", self.kind))
        })?;
        let priority = match self.priority {
            0 => None,
            code => Some(FramePriority::from_wire_code(code).ok_or_else(|| {
                UFrameAbiError::InvalidProfile(format!("unknown priority code {code}"))
            })?),
        };
        let id = UUID::from_u64_pair(self.id.msb, self.id.lsb)
            .map_err(|error| UFrameAbiError::InvalidProfile(format!("invalid id: {error}")))?;
        let source = uuri_from_abi(&self.source, "source")?;
        let sink = if self.presence & FIELD_SINK != 0 {
            Some(uuri_from_abi(&self.sink, "sink")?)
        } else {
            None
        };
        let reqid = if self.presence & FIELD_REQID != 0 {
            Some(
                UUID::from_u64_pair(self.reqid.msb, self.reqid.lsb).map_err(|error| {
                    UFrameAbiError::InvalidProfile(format!("invalid reqid: {error}"))
                })?,
            )
        } else {
            None
        };
        let ttl = (self.presence & FIELD_TTL != 0).then(|| Duration::from_nanos(self.ttl_ns));
        let comm_status = if self.presence & FIELD_COMM_STATUS != 0 {
            Some(
                UCode::try_from_i32(i32::from(self.comm_status)).map_err(|_| {
                    UFrameAbiError::InvalidProfile(format!(
                        "unknown communication status code {}",
                        self.comm_status
                    ))
                })?,
            )
        } else {
            None
        };
        let permission_level =
            (self.presence & FIELD_PERMISSION_LEVEL != 0).then_some(self.permission_level);
        let token = if self.presence & FIELD_TOKEN != 0 {
            Some(
                read_str(
                    &self.token.bytes,
                    usize::from(self.token.len),
                    UFRAME_ABI_TOKEN_CAPACITY,
                    "token",
                )?
                .to_owned(),
            )
        } else {
            None
        };
        let traceparent = if self.presence & FIELD_TRACEPARENT != 0 {
            Some(
                read_str(
                    &self.traceparent.bytes,
                    usize::from(self.traceparent.len),
                    UFRAME_ABI_TRACEPARENT_CAPACITY,
                    "traceparent",
                )?
                .to_owned(),
            )
        } else {
            None
        };
        let (payload_encoding, payload_size) = if self.presence & FIELD_PAYLOAD_ENCODING != 0 {
            let enc = &self.payload_encoding;
            if enc.component_flags & !UFRAME_ABI_ENCODING_HAS_REGISTRY_ID != 0 || enc._reserved != 0
            {
                return Err(UFrameAbiError::InvalidProfile(
                    "unknown payload encoding component flags".to_string(),
                ));
            }
            let registry_id = (enc.component_flags & UFRAME_ABI_ENCODING_HAS_REGISTRY_ID != 0)
                .then_some(enc.registry_id);
            let literal_id = if enc.literal_id_len > 0 {
                Some(
                    read_str(
                        &enc.literal_id,
                        usize::from(enc.literal_id_len),
                        UFRAME_ABI_LITERAL_ID_CAPACITY,
                        "payload_encoding.literal_id",
                    )?
                    .to_owned(),
                )
            } else {
                None
            };
            let content_type = if enc.content_type_len > 0 {
                Some(
                    read_str(
                        &enc.content_type,
                        usize::from(enc.content_type_len),
                        UFRAME_ABI_CONTENT_TYPE_CAPACITY,
                        "payload_encoding.content_type",
                    )?
                    .to_owned(),
                )
            } else {
                None
            };
            (
                Some(PayloadEncoding::from_parts(
                    registry_id,
                    literal_id,
                    content_type,
                )?),
                Some(self.payload_size),
            )
        } else {
            if self.payload_size != 0 {
                return Err(UFrameAbiError::InvalidProfile(
                    "payload_size must be zero without a payload encoding".to_string(),
                ));
            }
            (None, None)
        };

        let metadata = UFrameMetadata::from_decoded_parts(
            kind,
            id,
            source,
            sink,
            reqid,
            priority,
            ttl,
            comm_status,
            permission_level,
            token,
            traceparent,
            payload_encoding,
        );
        metadata.validate()?;
        Ok((metadata, payload_size))
    }
}

// ---------------------------------------------------------------------------
// Compile-time layout conformance checks (normative for profile v1)
// ---------------------------------------------------------------------------

const _: () = {
    assert!(size_of::<UUuidAbi>() == 16);
    assert!(align_of::<UUuidAbi>() == 8);
    assert!(size_of::<UUriAbi>() == 136);
    assert!(align_of::<UUriAbi>() == 4);
    assert!(size_of::<UPayloadEncodingAbi>() == 172);
    assert!(align_of::<UPayloadEncodingAbi>() == 4);
    assert!(size_of::<UTraceparentAbi>() == 64);
    assert!(align_of::<UTraceparentAbi>() == 1);
    assert!(size_of::<UTokenAbi>() == 512);
    assert!(align_of::<UTokenAbi>() == 2);

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
    assert!(offset_of!(UFrameMetadataAbiV1, _reserved0) == 15);
    assert!(offset_of!(UFrameMetadataAbiV1, ttl_ns) == 16);
    assert!(offset_of!(UFrameMetadataAbiV1, permission_level) == 24);
    assert!(offset_of!(UFrameMetadataAbiV1, _reserved1) == 28);
    assert!(offset_of!(UFrameMetadataAbiV1, id) == 32);
    assert!(offset_of!(UFrameMetadataAbiV1, reqid) == 48);
    assert!(offset_of!(UFrameMetadataAbiV1, payload_size) == 64);
    assert!(offset_of!(UFrameMetadataAbiV1, payload_encoding) == 72);
    assert!(offset_of!(UFrameMetadataAbiV1, source) == 244);
    assert!(offset_of!(UFrameMetadataAbiV1, sink) == 380);
    assert!(offset_of!(UFrameMetadataAbiV1, traceparent) == 516);
    assert!(offset_of!(UFrameMetadataAbiV1, token) == 580);
    assert!(offset_of!(UFrameMetadataAbiV1, _reserved_tail) == 1092);

    assert!(offset_of!(UPayloadEncodingAbi, registry_id) == 0);
    assert!(offset_of!(UPayloadEncodingAbi, literal_id_len) == 4);
    assert!(offset_of!(UPayloadEncodingAbi, content_type_len) == 5);
    assert!(offset_of!(UPayloadEncodingAbi, component_flags) == 6);
    assert!(offset_of!(UPayloadEncodingAbi, _reserved) == 7);
    assert!(offset_of!(UPayloadEncodingAbi, literal_id) == 8);
    assert!(offset_of!(UPayloadEncodingAbi, content_type) == 72);
};

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic")
    }

    fn method() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x00b1).expect("method")
    }

    fn reply_to() -> UUri {
        UUri::try_from_parts("cloud", 0x10ab, 0x02, 0x0000).expect("reply-to")
    }

    #[test]
    fn minimal_publish_metadata_round_trips_through_profile() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let abi = UFrameMetadataAbiV1::try_from_metadata(&metadata, None).expect("profile");
        let (decoded, payload_size) = abi.try_to_metadata().expect("metadata");
        assert_eq!(decoded, metadata);
        assert_eq!(payload_size, None);
    }

    #[test]
    fn fully_populated_request_round_trips_through_profile() {
        let metadata = UFrameMetadata::request(method(), reply_to(), Duration::from_millis(250))
            .with_priority(FramePriority::CS6)
            .with_comm_status(UCode::Ok)
            .with_permission_level(3)
            .with_token("token-value")
            .with_traceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
            .with_payload_encoding(
                PayloadEncoding::custom("up.xcdr-v2", "application/vnd.uprotocol.xcdr-v2")
                    .expect("encoding"),
            )
            .build()
            .expect("metadata");

        let abi = UFrameMetadataAbiV1::try_from_metadata(&metadata, Some(64)).expect("profile");
        let (decoded, payload_size) = abi.try_to_metadata().expect("metadata");
        assert_eq!(decoded, metadata);
        assert_eq!(payload_size, Some(64));
    }

    #[test]
    fn oversized_token_fails_instead_of_truncating() {
        let metadata = UFrameMetadata::publish(topic())
            .with_token("t".repeat(UFRAME_ABI_TOKEN_CAPACITY + 1))
            .build()
            .expect("metadata");
        assert_eq!(
            UFrameMetadataAbiV1::try_from_metadata(&metadata, None).unwrap_err(),
            UFrameAbiError::CapacityExceeded { field: "token" }
        );
    }

    #[test]
    fn payload_size_and_encoding_presence_must_agree() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        assert!(matches!(
            UFrameMetadataAbiV1::try_from_metadata(&metadata, Some(8)).unwrap_err(),
            UFrameAbiError::Metadata(crate::UFrameMetadataError::PayloadWithoutEncoding)
        ));

        let metadata = UFrameMetadata::publish(topic())
            .with_payload_encoding(PayloadEncoding::RAW)
            .build()
            .expect("metadata");
        assert!(matches!(
            UFrameMetadataAbiV1::try_from_metadata(&metadata, None).unwrap_err(),
            UFrameAbiError::Metadata(crate::UFrameMetadataError::EncodingWithoutPayload)
        ));
    }

    #[test]
    fn structural_validation_rejects_corrupted_profiles() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let abi = UFrameMetadataAbiV1::try_from_metadata(&metadata, None).expect("profile");

        let mut bad = abi;
        bad.magic = *b"XXXX";
        assert!(bad.try_to_metadata().is_err());

        let mut bad = abi;
        bad.presence |= 1 << 30;
        assert!(bad.try_to_metadata().is_err());

        let mut bad = abi;
        bad._reserved_tail = [1; 4];
        assert!(bad.try_to_metadata().is_err());
    }
}
