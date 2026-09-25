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

//! # Whole-frame envelopes
//!
//! A whole-frame envelope serializes semantic metadata and application payload
//! into one envelope byte value. It is distinct from selected-wire `UPWM`, which
//! prefixes a configured metadata profile while the encoded core carries
//! payload storage separately.
//!
//! ## Walkthrough
//!
//! 1. Start with a validated `UOwnedFrame`.
//! 2. Use `NativeUFrameEnvelope::serialize_frame` for the native frame contract.
//!    Use `ProtobufUMessageFrame` for compatibility with protobuf UMessage peers.
//! 3. Carry the resulting bytes through an ordinary byte channel.
//! 4. Deserialize with the same `UFrameWireFormat`; malformed lengths,
//!    reserved fields, metadata, payload presence, and unsupported identities
//!    fail before a frame is returned.
//! 5. Verify changes with the envelope round-trip, malformed-input, and
//!    projection tests in this module.

use std::{error::Error, fmt::Display};

use bytes::Bytes;

#[cfg(feature = "protobuf-support")]
use crate::{ProtobufMappable, SerializationError, UMessage};
use crate::{UFrameMetadataError, UOwnedFrame};

/// Error type used by whole-frame wire formats.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UFrameWireError {
    /// The encoded bytes do not contain a valid frame or violate native frame invariants.
    InvalidFrame(String),
    /// The selected wire format cannot faithfully represent the frame payload encoding.
    UnsupportedPayloadEncoding(String),
    /// The wire format encoder or decoder failed while converting bytes.
    SerializationError(String),
}

impl UFrameWireError {
    /// Creates an [`UFrameWireError::InvalidFrame`] value.
    #[must_use]
    pub fn invalid_frame(message: impl Into<String>) -> Self {
        Self::InvalidFrame(message.into())
    }

    /// Creates an [`UFrameWireError::UnsupportedPayloadEncoding`] value.
    #[must_use]
    pub fn unsupported_payload_encoding(message: impl Into<String>) -> Self {
        Self::UnsupportedPayloadEncoding(message.into())
    }

    /// Creates an [`UFrameWireError::SerializationError`] value.
    #[must_use]
    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::SerializationError(message.into())
    }
}

impl Display for UFrameWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFrame(message) => write!(f, "invalid frame: {message}"),
            Self::UnsupportedPayloadEncoding(message) => {
                write!(f, "unsupported payload encoding: {message}")
            }
            Self::SerializationError(message) => write!(f, "frame serialization error: {message}"),
        }
    }
}

impl Error for UFrameWireError {}

#[cfg(feature = "protobuf-support")]
impl From<SerializationError> for UFrameWireError {
    fn from(value: SerializationError) -> Self {
        Self::serialization_error(value.to_string())
    }
}

impl From<UFrameMetadataError> for UFrameWireError {
    fn from(value: UFrameMetadataError) -> Self {
        match value {
            other @ (UFrameMetadataError::PayloadWithoutEncoding
            | UFrameMetadataError::EncodingWithoutPayload
            | UFrameMetadataError::FieldNotRepresentable { .. }
            | UFrameMetadataError::InvalidMetadata(_)
            | UFrameMetadataError::MessageBuildError(_)) => Self::invalid_frame(other.to_string()),
        }
    }
}

/// Whole-frame wire format for transporting a complete native uProtocol frame.
///
/// This is distinct from payload codecs, which only transform an application
/// value into frame payload bytes. Implementations must preserve native frame
/// metadata and payload presence. If an envelope cannot represent a
/// [`crate::PayloadEncoding`], it should return
/// [`UFrameWireError::UnsupportedPayloadEncoding`] instead of silently dropping
/// metadata.
pub trait UFrameWireFormat {
    /// Stable implementation name for logs, diagnostics, and configuration.
    fn name() -> &'static str;

    /// Media type of bytes emitted by [`Self::serialize_frame`].
    fn content_type() -> &'static str;

    /// Serializes a complete native frame, including metadata and payload bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame is invalid or cannot be represented by this
    /// wire format.
    fn serialize_frame(frame: &UOwnedFrame) -> Result<Bytes, UFrameWireError>;

    /// Deserializes a complete native frame from this wire format's bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if `src` is malformed or violates native frame invariants.
    fn deserialize_frame(src: &[u8]) -> Result<UOwnedFrame, UFrameWireError>;
}

/// **Legacy compatibility** whole-frame wire format using the generated
/// `UMessage` Protocol Buffers envelope.
///
/// This envelope projects native frames through `UMessage`. Use
/// [`NativeUFrameEnvelope`] when the peer expects the native frame contract.
#[cfg(feature = "protobuf-support")]
#[derive(Debug)]
pub struct ProtobufUMessageFrame;

#[cfg(feature = "protobuf-support")]
impl UFrameWireFormat for ProtobufUMessageFrame {
    fn name() -> &'static str {
        "protobuf-umessage"
    }

    fn content_type() -> &'static str {
        "application/x-uprotocol-umessage+protobuf"
    }

    fn serialize_frame(frame: &UOwnedFrame) -> Result<Bytes, UFrameWireError> {
        let message = crate::frame::metadata::try_project_frame_to_umessage(
            frame.metadata().clone(),
            frame.payload().cloned(),
        )?;
        message
            .write_to_protobuf_bytes()
            .map(Bytes::from)
            .map_err(UFrameWireError::from)
    }

    fn deserialize_frame(src: &[u8]) -> Result<UOwnedFrame, UFrameWireError> {
        let message = UMessage::parse_from_protobuf_bytes(src)?;
        let metadata = crate::frame::metadata::try_project_umessage_to_frame_metadata(&message)?;
        let payload = message.payload().as_deref().map(Bytes::copy_from_slice);
        UOwnedFrame::new(metadata, payload).map_err(UFrameWireError::from)
    }
}

/// Canonical whole-frame envelope carrying native frame metadata and payload
/// bytes with full fidelity.
///
/// The envelope mirrors the physical layout the shared-memory transports
/// already use — a small fixed *placement* header followed by the variable
/// canonical metadata field block, followed by the payload bytes:
///
/// ```text
/// offset  size  field
/// 0       4     magic "UPFE"
/// 4       1     envelope version (1)
/// 5       1     payload presence (0 = absent, 1 = present)
/// 6       2     reserved, MUST be zero
/// 8       4     metadata_len (u32, little-endian)
/// 12      8     payload_len (u64, little-endian; 0 when absent)
/// 20      ...   metadata field block (see [`crate::frame::codec`])
/// 20+m    ...   payload bytes
/// ```
///
/// This is the recommended carrier for native frames over ordinary byte
/// channels (an MQTT payload, a SOME/IP payload, a Zenoh attachment+payload
/// pair collapsed into one buffer, a file, ...). Unlike
/// [`ProtobufUMessageFrame`] it does not require protobuf support.
#[derive(Debug)]
pub struct NativeUFrameEnvelope;

/// Magic bytes of the native whole-frame envelope.
pub const NATIVE_ENVELOPE_MAGIC: [u8; 4] = *b"UPFE";
/// Version of the native whole-frame envelope emitted by this module.
pub const NATIVE_ENVELOPE_VERSION: u8 = 1;
/// Size of the fixed native-envelope placement header in bytes.
pub const NATIVE_ENVELOPE_HEADER_LEN: usize = 20;

impl UFrameWireFormat for NativeUFrameEnvelope {
    fn name() -> &'static str {
        "native-uframe-envelope"
    }

    fn content_type() -> &'static str {
        "application/vnd.uprotocol.uframe;version=1"
    }

    fn serialize_frame(frame: &UOwnedFrame) -> Result<Bytes, UFrameWireError> {
        // `UOwnedFrame<Validated>` by type: validity is guaranteed by construction.
        let metadata = crate::frame::codec::encode_frame_metadata_fields(frame.metadata())
            .map_err(|error| UFrameWireError::invalid_frame(error.to_string()))?;
        let metadata_len = u32::try_from(metadata.len()).map_err(|_| {
            UFrameWireError::invalid_frame("metadata exceeds the u32 envelope limit")
        })?;
        let payload = frame.payload().map(bytes::Bytes::as_ref);
        let payload_len = payload.map_or(0_u64, |payload| payload.len() as u64);

        let mut out = Vec::with_capacity(
            NATIVE_ENVELOPE_HEADER_LEN + metadata.len() + payload.map_or(0, <[u8]>::len),
        );
        out.extend_from_slice(&NATIVE_ENVELOPE_MAGIC);
        out.push(NATIVE_ENVELOPE_VERSION);
        out.push(u8::from(payload.is_some()));
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&metadata_len.to_le_bytes());
        out.extend_from_slice(&payload_len.to_le_bytes());
        out.extend_from_slice(&metadata);
        if let Some(payload) = payload {
            out.extend_from_slice(payload);
        }
        Ok(Bytes::from(out))
    }

    fn deserialize_frame(src: &[u8]) -> Result<UOwnedFrame, UFrameWireError> {
        let header = src
            .get(..NATIVE_ENVELOPE_HEADER_LEN)
            .ok_or_else(|| UFrameWireError::invalid_frame("input shorter than envelope header"))?;
        let magic = header
            .get(..4)
            .ok_or_else(|| UFrameWireError::invalid_frame("input shorter than envelope magic"))?;
        if magic != NATIVE_ENVELOPE_MAGIC {
            return Err(UFrameWireError::invalid_frame("wrong envelope magic"));
        }
        let version = *header
            .get(4)
            .ok_or_else(|| UFrameWireError::invalid_frame("input shorter than envelope version"))?;
        if version != NATIVE_ENVELOPE_VERSION {
            return Err(UFrameWireError::invalid_frame(format!(
                "unsupported envelope version {}",
                version
            )));
        }
        let payload_marker = *header.get(5).ok_or_else(|| {
            UFrameWireError::invalid_frame("input shorter than payload presence marker")
        })?;
        let payload_present = match payload_marker {
            0 => false,
            1 => true,
            other => {
                return Err(UFrameWireError::invalid_frame(format!(
                    "invalid payload presence marker {other}"
                )))
            }
        };
        let reserved = header.get(6..8).ok_or_else(|| {
            UFrameWireError::invalid_frame("input shorter than reserved envelope bytes")
        })?;
        if reserved != [0, 0] {
            return Err(UFrameWireError::invalid_frame(
                "reserved envelope bytes must be zero",
            ));
        }
        let metadata_len_bytes = header
            .get(8..12)
            .ok_or_else(|| UFrameWireError::invalid_frame("input shorter than metadata length"))?;
        let metadata_len =
            u32::from_le_bytes(metadata_len_bytes.try_into().expect("4 bytes")) as usize;
        let payload_len_bytes = header
            .get(12..20)
            .ok_or_else(|| UFrameWireError::invalid_frame("input shorter than payload length"))?;
        let payload_len = u64::from_le_bytes(payload_len_bytes.try_into().expect("8 bytes"));

        let body = src
            .get(NATIVE_ENVELOPE_HEADER_LEN..)
            .ok_or_else(|| UFrameWireError::invalid_frame("input shorter than envelope header"))?;
        let metadata_bytes = body.get(..metadata_len).ok_or_else(|| {
            UFrameWireError::invalid_frame("input shorter than declared metadata length")
        })?;
        let payload_bytes = body.get(metadata_len..).ok_or_else(|| {
            UFrameWireError::invalid_frame("input shorter than declared metadata length")
        })?;
        if payload_bytes.len() as u64 != payload_len || (!payload_present && payload_len != 0) {
            return Err(UFrameWireError::invalid_frame(
                "payload length disagrees with envelope header",
            ));
        }

        let metadata = crate::frame::codec::decode_frame_metadata_fields(metadata_bytes)
            .map_err(|error| UFrameWireError::invalid_frame(error.to_string()))?;
        let payload = payload_present.then(|| Bytes::copy_from_slice(payload_bytes));
        UOwnedFrame::new(metadata, payload).map_err(UFrameWireError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "protobuf-support")]
    use crate::payload::codec::RawBytes;
    use crate::UMessageBuilder;
    use crate::{PayloadEncoding, UFrameMetadata, UUri};

    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic")
    }

    fn metadata_with_raw_encoding() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::new(), PayloadEncoding::RAW)
            .expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(RawBytes::encoding()),
        )
        .expect("metadata")
    }

    #[test]
    fn protobuf_umessage_frame_round_trips_raw_payload() {
        let frame = UOwnedFrame::with_payload(
            metadata_with_raw_encoding(),
            Bytes::from_static(b"raw payload"),
        )
        .expect("frame");

        let encoded = ProtobufUMessageFrame::serialize_frame(&frame).expect("serialize frame");
        let decoded =
            ProtobufUMessageFrame::deserialize_frame(&encoded).expect("deserialize frame");

        assert_eq!(decoded, frame);
    }

    #[test]
    fn protobuf_umessage_frame_round_trips_private_use_payload_encoding() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let encoding = PayloadEncoding::from_id(0x1000_0BEE).expect("private-use id");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(encoding),
        )
        .expect("metadata");
        let frame = UOwnedFrame::with_payload(metadata, Bytes::from_static(b"native payload"))
            .expect("frame");

        let encoded = ProtobufUMessageFrame::serialize_frame(&frame).expect("serialize frame");
        let decoded =
            ProtobufUMessageFrame::deserialize_frame(&encoded).expect("deserialize frame");

        assert_eq!(decoded.metadata().payload_encoding(), Some(&encoding));
        assert_eq!(
            decoded.payload(),
            Some(&Bytes::from_static(b"native payload"))
        );
    }

    #[test]
    fn protobuf_umessage_frame_rejects_invalid_metadata() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let metadata = crate::frame::metadata::try_project_umessage_to_frame_metadata(&message)
            .expect("metadata");
        // payload bytes without a payload encoding violate the frame invariant
        let frame = UOwnedFrame::with_payload_unchecked(metadata, Bytes::from_static(b"payload"));

        // An invalid frame cannot reach serialize_frame; the rejection lives at
        // the typestate transition:
        let error = frame.validate().unwrap_err();

        assert!(matches!(error, UFrameMetadataError::PayloadWithoutEncoding));
    }

    #[test]
    fn protobuf_umessage_frame_round_trips_unspecified_absent_payload() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            None,
        )
        .expect("metadata");
        let frame = UOwnedFrame::without_payload(metadata).expect("frame");

        let encoded = ProtobufUMessageFrame::serialize_frame(&frame).expect("serialize frame");
        let decoded =
            ProtobufUMessageFrame::deserialize_frame(&encoded).expect("deserialize frame");

        assert_eq!(decoded.metadata().payload_encoding(), None);
        assert!(!decoded.has_payload());
    }

    #[test]
    fn native_envelope_round_trips_private_use_encoding_frame() {
        let metadata = UFrameMetadata::publish(topic())
            .with_payload_encoding(PayloadEncoding::from_id(0x1000_0BEE).expect("private-use id"))
            .build()
            .expect("metadata");
        let frame = UOwnedFrame::with_payload(metadata, Bytes::from_static(b"native payload"))
            .expect("frame");

        let encoded = NativeUFrameEnvelope::serialize_frame(&frame).expect("serialize frame");
        let decoded = NativeUFrameEnvelope::deserialize_frame(&encoded).expect("deserialize");

        assert_eq!(decoded, frame);
    }

    #[test]
    fn native_envelope_preserves_present_empty_payload() {
        let metadata = UFrameMetadata::publish(topic())
            .with_payload_encoding(PayloadEncoding::RAW)
            .build()
            .expect("metadata");
        let frame = UOwnedFrame::with_payload(metadata, Bytes::new()).expect("frame");

        let decoded = NativeUFrameEnvelope::deserialize_frame(
            &NativeUFrameEnvelope::serialize_frame(&frame).expect("serialize"),
        )
        .expect("deserialize");

        assert!(decoded.has_payload());
        assert_eq!(decoded.payload(), Some(&Bytes::new()));
    }

    #[test]
    fn native_envelope_round_trips_absent_payload() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let frame = UOwnedFrame::without_payload(metadata).expect("frame");

        let decoded = NativeUFrameEnvelope::deserialize_frame(
            &NativeUFrameEnvelope::serialize_frame(&frame).expect("serialize"),
        )
        .expect("deserialize");

        assert!(!decoded.has_payload());
        assert_eq!(decoded, frame);
    }

    #[test]
    fn native_envelope_rejects_corrupted_input() {
        let metadata = UFrameMetadata::publish(topic()).build().expect("metadata");
        let frame = UOwnedFrame::without_payload(metadata).expect("frame");
        let encoded = NativeUFrameEnvelope::serialize_frame(&frame).expect("serialize");

        let mut bad = encoded.to_vec();
        *bad.first_mut().expect("magic byte") = b'X';
        assert!(NativeUFrameEnvelope::deserialize_frame(&bad).is_err());

        let mut bad = encoded.to_vec();
        bad.push(0);
        assert!(NativeUFrameEnvelope::deserialize_frame(&bad).is_err());
    }
}
