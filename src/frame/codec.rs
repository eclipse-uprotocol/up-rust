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

//! Canonical byte codec for native frame metadata: the *UFrame metadata
//! field block*, version 1.
//!
//! This is the language-neutral serialization of [`UFrameMetadata`] used by
//! the canonical selected-wire metadata codec and the native whole-frame
//! envelope. Transports place these bytes wherever their physical layout
//! wants them (a Zenoh attachment, the variable metadata prefix behind
//! iceoryx2's placement header, the metadata block behind LoLa's `ULOL`
//! header, ...); the block itself is deliberately variable-length and
//! presence-driven so that absent fields cost zero bytes.
//!
//! Layout (all multi-byte integers little-endian):
//!
//! ```text
//! u8   block_version        = 1
//! u8   kind                 FrameMessageKind wire code (1..=4)
//! u8   priority             0 = absent, 1..=7 = CS0..=CS6
//! u8   reserved             = 0
//! u32  presence             FIELD_* bitmask; unknown bits MUST be zero
//! u64  id.msb
//! u64  id.lsb
//! [FIELD_REQID]             u64 msb, u64 lsb
//! [FIELD_TTL]               u64 ttl in nanoseconds
//! [FIELD_COMM_STATUS]       i32 UCode value
//! [FIELD_PERMISSION_LEVEL]  u32
//! source UUri block         u32 ue_id, u16 resource_id, u8 ue_version_major,
//!                           u8 authority_len, authority bytes (UTF-8)
//! [FIELD_SINK]              UUri block
//! [FIELD_TOKEN]             u16 len, token bytes (UTF-8)
//! [FIELD_TRACEPARENT]       u8 len, traceparent bytes (UTF-8)
//! [FIELD_PAYLOAD_ENCODING]  u8 component bitmask (ENCODING_HAS_*), then in order:
//!                           [u32 registry_id] [u16 len + literal id bytes]
//!                           [u16 len + content type bytes]
//! ```
//!
//! Values that do not fit a length field are rejected at encode time —
//! encoding never truncates.
//!
//! ## Walkthrough
//!
//! 1. Construct and validate semantic [`UFrameMetadata`].
//! 2. Call [`encode_frame_metadata_fields`] once; transports carry the returned
//!    block as opaque bytes in their binding-specific placement.
//! 3. On receive, call [`decode_frame_metadata_fields`]. Version, reserved bits,
//!    lengths, UTF-8, payload identity, and metadata invariants are checked
//!    before a semantic frame is exposed.
//! 4. Pin changes with round trips, malformed/truncated inputs, unknown-bit
//!    rejection, and golden vectors in `wire_metadata_conformance` and
//!    `wire_metadata_golden`.
//!
//! This is a metadata profile, not a whole-frame envelope and not an
//! application payload codec. `up-spec/basics/uframe.adoc` defines the profile
//! registry; `up-spec/up-l1/transport_families.adoc` defines selected-wire
//! identity and rejection behavior.

use std::time::Duration;

use crate::{
    FrameMessageKind, FramePriority, PayloadEncoding, UCode, UFrameMetadata, UFrameMetadataError,
    UUri, UUID,
};

/// Version of the field block emitted by [`encode_frame_metadata_fields`].
pub const FRAME_FIELDS_VERSION: u8 = 1;

/// Presence bit: the frame has a sink URI.
pub const FIELD_SINK: u32 = 1 << 0;
/// Presence bit: the frame has a correlated request id.
pub const FIELD_REQID: u32 = 1 << 1;
/// Presence bit: the frame has a time-to-live.
pub const FIELD_TTL: u32 = 1 << 2;
/// Presence bit: the frame has a communication status.
pub const FIELD_COMM_STATUS: u32 = 1 << 3;
/// Presence bit: the frame has a permission level.
pub const FIELD_PERMISSION_LEVEL: u32 = 1 << 4;
/// Presence bit: the frame has an access token.
pub const FIELD_TOKEN: u32 = 1 << 5;
/// Presence bit: the frame has a W3C traceparent.
pub const FIELD_TRACEPARENT: u32 = 1 << 6;
/// Presence bit: the frame has a payload encoding.
pub const FIELD_PAYLOAD_ENCODING: u32 = 1 << 7;
/// All presence bits defined by field block version 1.
pub const FIELD_MASK_V1: u32 = (1 << 8) - 1;

const ENCODING_HAS_REGISTRY_ID: u8 = 1 << 0;
const ENCODING_HAS_LITERAL_ID: u8 = 1 << 1;
const ENCODING_HAS_CONTENT_TYPE: u8 = 1 << 2;
const ENCODING_MASK_V1: u8 = (1 << 3) - 1;

/// Errors returned by the frame metadata field block codec.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UFrameFieldsError {
    /// The input bytes are not a well-formed field block.
    Malformed(String),
    /// A value does not fit its field block length field.
    ValueTooLong {
        /// Name of the offending field.
        field: &'static str,
    },
    /// The (decoded or to-be-encoded) metadata violates frame invariants.
    Metadata(UFrameMetadataError),
}

impl std::fmt::Display for UFrameFieldsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(message) => write!(f, "malformed frame metadata fields: {message}"),
            Self::ValueTooLong { field } => {
                write!(
                    f,
                    "frame metadata field `{field}` does not fit the field block"
                )
            }
            Self::Metadata(error) => write!(f, "invalid frame metadata: {error}"),
        }
    }
}

impl std::error::Error for UFrameFieldsError {}

impl From<UFrameMetadataError> for UFrameFieldsError {
    fn from(value: UFrameMetadataError) -> Self {
        Self::Metadata(value)
    }
}

/// Encodes validated frame metadata into its canonical field block bytes.
///
/// # Errors
///
/// Returns an error if the metadata is invalid or a value does not fit its
/// length field. Encoding never truncates.
pub fn encode_frame_metadata_fields(
    metadata: &UFrameMetadata,
) -> Result<Vec<u8>, UFrameFieldsError> {
    metadata.validate()?;

    let mut presence = 0_u32;
    if metadata.sink().is_some() {
        presence |= FIELD_SINK;
    }
    if metadata.reqid().is_some() {
        presence |= FIELD_REQID;
    }
    if metadata.ttl().is_some() {
        presence |= FIELD_TTL;
    }
    if metadata.comm_status().is_some() {
        presence |= FIELD_COMM_STATUS;
    }
    if metadata.permission_level().is_some() {
        presence |= FIELD_PERMISSION_LEVEL;
    }
    if metadata.token().is_some() {
        presence |= FIELD_TOKEN;
    }
    if metadata.traceparent().is_some() {
        presence |= FIELD_TRACEPARENT;
    }
    if metadata.payload_encoding().is_some() {
        presence |= FIELD_PAYLOAD_ENCODING;
    }

    let mut out = Vec::with_capacity(64);
    out.push(FRAME_FIELDS_VERSION);
    out.push(metadata.kind().wire_code());
    out.push(metadata.priority().map_or(0, FramePriority::wire_code));
    out.push(0); // reserved
    out.extend_from_slice(&presence.to_le_bytes());
    write_uuid(&mut out, metadata.id());
    if let Some(reqid) = metadata.reqid() {
        write_uuid(&mut out, reqid);
    }
    if let Some(ttl) = metadata.ttl() {
        let nanos = u64::try_from(ttl.as_nanos())
            .map_err(|_| UFrameFieldsError::ValueTooLong { field: "ttl" })?;
        out.extend_from_slice(&nanos.to_le_bytes());
    }
    if let Some(comm_status) = metadata.comm_status() {
        out.extend_from_slice(&comm_status.value().to_le_bytes());
    }
    if let Some(permission_level) = metadata.permission_level() {
        out.extend_from_slice(&permission_level.to_le_bytes());
    }
    write_uuri(&mut out, metadata.source())?;
    if let Some(sink) = metadata.sink() {
        write_uuri(&mut out, sink)?;
    }
    if let Some(token) = metadata.token() {
        let len = u16::try_from(token.len())
            .map_err(|_| UFrameFieldsError::ValueTooLong { field: "token" })?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(token.as_bytes());
    }
    if let Some(traceparent) = metadata.traceparent() {
        let len = u8::try_from(traceparent.len()).map_err(|_| UFrameFieldsError::ValueTooLong {
            field: "traceparent",
        })?;
        out.push(len);
        out.extend_from_slice(traceparent.as_bytes());
    }
    if let Some(encoding) = metadata.payload_encoding() {
        write_payload_encoding(&mut out, encoding)?;
    }
    Ok(out)
}

/// Decodes and validates frame metadata from its canonical field block bytes.
///
/// The entire input must be consumed; trailing bytes are rejected.
///
/// # Errors
///
/// Returns an error if the bytes are malformed, use an unsupported block
/// version, contain unknown presence bits, or decode into invalid metadata.
pub fn decode_frame_metadata_fields(src: &[u8]) -> Result<UFrameMetadata, UFrameFieldsError> {
    let mut reader = FieldsReader { src, pos: 0 };

    let version = reader.u8()?;
    if version != FRAME_FIELDS_VERSION {
        return Err(UFrameFieldsError::Malformed(format!(
            "unsupported field block version {version}"
        )));
    }
    let kind = FrameMessageKind::from_wire_code(reader.u8()?).ok_or_else(|| {
        UFrameFieldsError::Malformed("unknown frame message kind code".to_string())
    })?;
    let priority = match reader.u8()? {
        0 => None,
        code => Some(FramePriority::from_wire_code(code).ok_or_else(|| {
            UFrameFieldsError::Malformed(format!("unknown frame priority code {code}"))
        })?),
    };
    let reserved = reader.u8()?;
    if reserved != 0 {
        return Err(UFrameFieldsError::Malformed(
            "reserved byte must be zero".to_string(),
        ));
    }
    let presence = reader.u32()?;
    if presence & !FIELD_MASK_V1 != 0 {
        return Err(UFrameFieldsError::Malformed(format!(
            "unknown presence bits {:#010x}",
            presence & !FIELD_MASK_V1
        )));
    }

    let id = reader.uuid()?;
    let reqid = if presence & FIELD_REQID != 0 {
        Some(reader.uuid()?)
    } else {
        None
    };
    let ttl = if presence & FIELD_TTL != 0 {
        Some(Duration::from_nanos(reader.u64()?))
    } else {
        None
    };
    let comm_status = if presence & FIELD_COMM_STATUS != 0 {
        let raw = reader.i32()?;
        Some(UCode::try_from_i32(raw).map_err(|_| {
            UFrameFieldsError::Malformed(format!("unknown communication status code {raw}"))
        })?)
    } else {
        None
    };
    let permission_level = if presence & FIELD_PERMISSION_LEVEL != 0 {
        Some(reader.u32()?)
    } else {
        None
    };
    let source = reader.uuri()?;
    let sink = if presence & FIELD_SINK != 0 {
        Some(reader.uuri()?)
    } else {
        None
    };
    let token = if presence & FIELD_TOKEN != 0 {
        let len = usize::from(reader.u16()?);
        Some(reader.utf8(len, "token")?)
    } else {
        None
    };
    let traceparent = if presence & FIELD_TRACEPARENT != 0 {
        let len = usize::from(reader.u8()?);
        Some(reader.utf8(len, "traceparent")?)
    } else {
        None
    };
    let payload_encoding = if presence & FIELD_PAYLOAD_ENCODING != 0 {
        Some(reader.payload_encoding()?)
    } else {
        None
    };
    reader.finish()?;

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
    Ok(metadata)
}

fn write_uuid(out: &mut Vec<u8>, uuid: &UUID) {
    let (msb, lsb) = uuid.as_u64_pair();
    out.extend_from_slice(&msb.to_le_bytes());
    out.extend_from_slice(&lsb.to_le_bytes());
}

fn write_uuri(out: &mut Vec<u8>, uri: &UUri) -> Result<(), UFrameFieldsError> {
    let ue_id = (u32::from(uri.uentity_instance_id()) << 16) | u32::from(uri.uentity_type_id());
    out.extend_from_slice(&ue_id.to_le_bytes());
    out.extend_from_slice(&uri.resource_id().to_le_bytes());
    out.push(uri.uentity_major_version());
    let authority = uri.authority_name();
    let len = u8::try_from(authority.len()).map_err(|_| UFrameFieldsError::ValueTooLong {
        field: "authority_name",
    })?;
    out.push(len);
    out.extend_from_slice(authority.as_bytes());
    Ok(())
}

fn write_payload_encoding(
    out: &mut Vec<u8>,
    encoding: &PayloadEncoding,
) -> Result<(), UFrameFieldsError> {
    let mut components = 0_u8;
    if encoding.registry_id().is_some() {
        components |= ENCODING_HAS_REGISTRY_ID;
    }
    if encoding.literal_id().is_some() {
        components |= ENCODING_HAS_LITERAL_ID;
    }
    if encoding.content_type().is_some() {
        components |= ENCODING_HAS_CONTENT_TYPE;
    }
    out.push(components);
    if let Some(registry_id) = encoding.registry_id() {
        out.extend_from_slice(&registry_id.to_le_bytes());
    }
    if let Some(literal_id) = encoding.literal_id() {
        let len = u16::try_from(literal_id.len()).map_err(|_| UFrameFieldsError::ValueTooLong {
            field: "payload_encoding.literal_id",
        })?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(literal_id.as_bytes());
    }
    if let Some(content_type) = encoding.content_type() {
        let len =
            u16::try_from(content_type.len()).map_err(|_| UFrameFieldsError::ValueTooLong {
                field: "payload_encoding.content_type",
            })?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(content_type.as_bytes());
    }
    Ok(())
}

struct FieldsReader<'a> {
    src: &'a [u8],
    pos: usize,
}

impl FieldsReader<'_> {
    fn take(&mut self, len: usize) -> Result<&[u8], UFrameFieldsError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| UFrameFieldsError::Malformed("length overflow".to_string()))?;
        let chunk = self.src.get(self.pos..end).ok_or_else(|| {
            UFrameFieldsError::Malformed(format!(
                "needed {len} byte(s) at offset {}, input has {} byte(s)",
                self.pos,
                self.src.len()
            ))
        })?;
        self.pos = end;
        Ok(chunk)
    }

    fn finish(&self) -> Result<(), UFrameFieldsError> {
        if self.pos == self.src.len() {
            Ok(())
        } else {
            Err(UFrameFieldsError::Malformed(format!(
                "{} trailing byte(s)",
                self.src.len() - self.pos
            )))
        }
    }

    fn u8(&mut self) -> Result<u8, UFrameFieldsError> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| UFrameFieldsError::Malformed("missing byte".to_string()))
    }

    fn u16(&mut self) -> Result<u16, UFrameFieldsError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ))
    }

    fn u32(&mut self) -> Result<u32, UFrameFieldsError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn i32(&mut self) -> Result<i32, UFrameFieldsError> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn u64(&mut self) -> Result<u64, UFrameFieldsError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }

    fn utf8(&mut self, len: usize, field: &'static str) -> Result<String, UFrameFieldsError> {
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(ToOwned::to_owned)
            .map_err(|error| {
                UFrameFieldsError::Malformed(format!("field `{field}` is not UTF-8: {error}"))
            })
    }

    fn uuid(&mut self) -> Result<UUID, UFrameFieldsError> {
        let msb = self.u64()?;
        let lsb = self.u64()?;
        UUID::from_u64_pair(msb, lsb)
            .map_err(|error| UFrameFieldsError::Malformed(format!("invalid UUID: {error}")))
    }

    fn uuri(&mut self) -> Result<UUri, UFrameFieldsError> {
        let ue_id = self.u32()?;
        let resource_id = self.u16()?;
        let ue_version_major = self.u8()?;
        let authority_len = usize::from(self.u8()?);
        let authority = self.utf8(authority_len, "authority_name")?;
        UUri::try_from_parts(&authority, ue_id, ue_version_major, resource_id)
            .map_err(|error| UFrameFieldsError::Malformed(format!("invalid UUri: {error}")))
    }

    fn payload_encoding(&mut self) -> Result<PayloadEncoding, UFrameFieldsError> {
        let components = self.u8()?;
        if components & !ENCODING_MASK_V1 != 0 {
            return Err(UFrameFieldsError::Malformed(format!(
                "unknown payload encoding component bits {:#04x}",
                components & !ENCODING_MASK_V1
            )));
        }
        let registry_id = if components & ENCODING_HAS_REGISTRY_ID != 0 {
            Some(self.u32()?)
        } else {
            None
        };
        let literal_id = if components & ENCODING_HAS_LITERAL_ID != 0 {
            let len = usize::from(self.u16()?);
            Some(self.utf8(len, "payload_encoding.literal_id")?)
        } else {
            None
        };
        let content_type = if components & ENCODING_HAS_CONTENT_TYPE != 0 {
            let len = usize::from(self.u16()?);
            Some(self.utf8(len, "payload_encoding.content_type")?)
        } else {
            None
        };
        PayloadEncoding::from_parts(registry_id, literal_id, content_type)
            .map_err(UFrameFieldsError::from)
    }
}

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
    fn minimal_publish_metadata_round_trips() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let bytes = encode_frame_metadata_fields(&metadata).expect("encode");
        let decoded = decode_frame_metadata_fields(&bytes).expect("decode");
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn fully_populated_request_metadata_round_trips() {
        let metadata = UFrameMetadata::request(method(), reply_to(), Duration::from_millis(250))
            .with_priority(FramePriority::CS5)
            .with_comm_status(UCode::Ok)
            .with_permission_level(4)
            .with_token("bearer-token")
            .with_traceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
            .with_payload_encoding(
                PayloadEncoding::custom("up.xcdr-v2", "application/vnd.uprotocol.xcdr-v2")
                    .expect("encoding"),
            )
            .build()
            .expect("metadata");

        let bytes = encode_frame_metadata_fields(&metadata).expect("encode");
        let decoded = decode_frame_metadata_fields(&bytes).expect("decode");
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn registered_encoding_round_trips_all_components() {
        let metadata = UFrameMetadata::publish(topic())
            .with_payload_encoding(PayloadEncoding::PROTOBUF)
            .build()
            .expect("metadata");
        let decoded =
            decode_frame_metadata_fields(&encode_frame_metadata_fields(&metadata).expect("encode"))
                .expect("decode");
        assert_eq!(decoded.payload_encoding(), Some(&PayloadEncoding::PROTOBUF));
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let mut bytes = encode_frame_metadata_fields(&metadata).expect("encode");
        bytes.push(0);
        assert!(matches!(
            decode_frame_metadata_fields(&bytes),
            Err(UFrameFieldsError::Malformed(_))
        ));
    }

    #[test]
    fn unknown_presence_bits_are_rejected() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let mut bytes = encode_frame_metadata_fields(&metadata).expect("encode");
        *bytes.get_mut(5).expect("presence byte") |= 0x01_u8; // undefined presence bit 8
        assert!(matches!(
            decode_frame_metadata_fields(&bytes),
            Err(UFrameFieldsError::Malformed(_))
        ));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let mut bytes = encode_frame_metadata_fields(&metadata).expect("encode");
        *bytes.first_mut().expect("version byte") = 2;
        assert!(matches!(
            decode_frame_metadata_fields(&bytes),
            Err(UFrameFieldsError::Malformed(_))
        ));
    }

    #[test]
    fn oversized_traceparent_fails_instead_of_truncating() {
        let metadata = UFrameMetadata::publish(topic())
            .with_traceparent("t".repeat(256))
            .build()
            .expect("metadata");
        assert_eq!(
            encode_frame_metadata_fields(&metadata).unwrap_err(),
            UFrameFieldsError::ValueTooLong {
                field: "traceparent"
            }
        );
    }
}
