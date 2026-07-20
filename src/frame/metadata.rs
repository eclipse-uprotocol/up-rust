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

//! Clean semantic native frame metadata.
//!
//! [`UFrameMetadata`] is the canonical metadata model for native uProtocol
//! frames (owned frames, zero-copy frame views, and selected-wire paths).
//! It carries semantic fields directly and contains **no** legacy
//! `UAttributes`, no `UPayloadFormat`, and no protobuf concepts.
//!
//! Legacy compatibility lives at the edges of this module as *fallible
//! projections*:
//!
//! * [`try_project_umessage_to_frame_metadata`] / [`try_project_frame_to_umessage`]
//!   convert between `UMessage` and native frames.
//! * [`try_project_attributes_to_frame_metadata`] /
//!   [`UFrameMetadata::try_project_to_attributes`] convert between
//!   `UAttributes` and native frame metadata.
//!
//! Projections fail — they never truncate or silently drop information —
//! when the target model cannot represent the source (for example a native
//! [`PayloadEncoding`] that has no legacy `UPayloadFormat` equivalent).
//!
//! Fixed-layout representations of this metadata (for C/C++ shared-memory
//! interop) are *derived profiles*, not this type: see the `frame::abi`
//! module. Byte serializations for transports are produced by selected-wire
//! metadata codecs: see the `wire` module.

use std::borrow::Cow;
use std::time::Duration;

use bytes::Bytes;

use crate::{
    UAttributes, UCode, UMessage, UMessageError, UMessageType, UPayloadFormat, UPriority, UUri,
    UUID,
};

// ---------------------------------------------------------------------------
// FrameMessageKind
// ---------------------------------------------------------------------------

/// Semantic kind of a native uProtocol frame.
///
/// This is the native-frame vocabulary. The numeric wire codes used by frame
/// codecs and ABI profiles are defined by [`FrameMessageKind::wire_code`];
/// the mapping to the legacy `UMessageType` is an explicit projection
/// (see [`FrameMessageKind::from_legacy_type`]).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameMessageKind {
    /// An event published to a topic.
    Publish,
    /// A directed notification to one consumer.
    Notification,
    /// An RPC request.
    Request,
    /// An RPC response.
    Response,
}

impl FrameMessageKind {
    /// Gets the normative UFrame wire code of this kind.
    ///
    /// Codes are defined by the UFrame specification: 1 = Publish,
    /// 2 = Request, 3 = Response, 4 = Notification, 0 and 5..=255 reserved.
    /// They deliberately coincide with the legacy protobuf `UMessageType`
    /// numbering so that projections are value-preserving, but the UFrame
    /// registry is normative from here on.
    #[must_use]
    pub fn wire_code(self) -> u8 {
        match self {
            Self::Publish => 1,
            Self::Request => 2,
            Self::Response => 3,
            Self::Notification => 4,
        }
    }

    /// Gets the kind denoted by a UFrame wire code.
    #[must_use]
    pub fn from_wire_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Publish),
            2 => Some(Self::Request),
            3 => Some(Self::Response),
            4 => Some(Self::Notification),
            _ => None,
        }
    }

    /// Projects a legacy `UMessageType` to a frame message kind.
    #[must_use]
    pub fn from_legacy_type(value: UMessageType) -> Self {
        match value {
            UMessageType::Publish => Self::Publish,
            UMessageType::Notification => Self::Notification,
            UMessageType::Request => Self::Request,
            UMessageType::Response => Self::Response,
        }
    }

    /// Projects this frame message kind to the legacy `UMessageType`.
    #[must_use]
    pub fn to_legacy_type(self) -> UMessageType {
        match self {
            Self::Publish => UMessageType::Publish,
            Self::Notification => UMessageType::Notification,
            Self::Request => UMessageType::Request,
            Self::Response => UMessageType::Response,
        }
    }
}

// ---------------------------------------------------------------------------
// FramePriority
// ---------------------------------------------------------------------------

/// Semantic QoS class of a native uProtocol frame.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FramePriority {
    /// Best effort (lowest priority).
    CS0,
    /// Priority class CS1.
    CS1,
    /// Priority class CS2.
    CS2,
    /// Priority class CS3.
    CS3,
    /// Priority class CS4 (streaming).
    CS4,
    /// Priority class CS5 (RPC default).
    CS5,
    /// Priority class CS6 (highest; network control).
    CS6,
}

impl FramePriority {
    /// Gets the normative UFrame wire code of this priority.
    ///
    /// Codes are defined by the UFrame specification: 1..=7 = CS0..=CS6,
    /// 0 = absent (wire/ABI representations only), 8..=255 reserved. They
    /// deliberately coincide with the legacy protobuf `UPriority` numbering.
    #[must_use]
    pub fn wire_code(self) -> u8 {
        match self {
            Self::CS0 => 1,
            Self::CS1 => 2,
            Self::CS2 => 3,
            Self::CS3 => 4,
            Self::CS4 => 5,
            Self::CS5 => 6,
            Self::CS6 => 7,
        }
    }

    /// Gets the priority denoted by a UFrame wire code.
    #[must_use]
    pub fn from_wire_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::CS0),
            2 => Some(Self::CS1),
            3 => Some(Self::CS2),
            4 => Some(Self::CS3),
            5 => Some(Self::CS4),
            6 => Some(Self::CS5),
            7 => Some(Self::CS6),
            _ => None,
        }
    }

    /// Projects a legacy `UPriority` to a frame priority.
    #[must_use]
    pub fn from_legacy_priority(value: UPriority) -> Self {
        match value {
            UPriority::CS0 => Self::CS0,
            UPriority::CS1 => Self::CS1,
            UPriority::CS2 => Self::CS2,
            UPriority::CS3 => Self::CS3,
            UPriority::CS4 => Self::CS4,
            UPriority::CS5 => Self::CS5,
            UPriority::CS6 => Self::CS6,
        }
    }

    /// Projects this frame priority to the legacy `UPriority`.
    #[must_use]
    pub fn to_legacy_priority(self) -> UPriority {
        match self {
            Self::CS0 => UPriority::CS0,
            Self::CS1 => UPriority::CS1,
            Self::CS2 => UPriority::CS2,
            Self::CS3 => UPriority::CS3,
            Self::CS4 => UPriority::CS4,
            Self::CS5 => UPriority::CS5,
            Self::CS6 => UPriority::CS6,
        }
    }
}

// ---------------------------------------------------------------------------
// PayloadEncoding
// ---------------------------------------------------------------------------

/// Open identity of the payload representation carried by a native frame.
///
/// A payload encoding is identified by up to three components:
///
/// * `registry_id`: a numeric id registered with the uProtocol payload
///   encoding registry. Ids `1..=8` are permanently reserved for the
///   encodings historically expressed by the legacy `UPayloadFormat` enum
///   (value-compatible with it), `9..=0x7FFF_FFFF` for future registered
///   encodings, and `0x8000_0000..` for vendor/private use.
/// * `literal_id`: a language-neutral literal identity such as
///   `up.stable-container` or `up.xcdr-v2`.
/// * `content_type`: an RFC 6838 media type, optionally with parameters,
///   such as `application/vnd.uprotocol.xcdr-v2;endianness=little;version=2`.
///
/// At least one component must be present. Unlike the retired closed
/// `UPayloadFormat` enum, new encodings can be introduced without touching
/// any SDK enum: register a literal id (and optionally a numeric id), or
/// use a vendor media type.
///
/// The projection to and from the legacy enum is explicit and fallible:
/// [`PayloadEncoding::try_from_legacy_format`] and
/// [`PayloadEncoding::to_legacy_format`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PayloadEncoding {
    registry_id: Option<u32>,
    literal_id: Option<Cow<'static, str>>,
    content_type: Option<Cow<'static, str>>,
}

macro_rules! well_known_encoding {
    ($(#[$attr:meta])* $name:ident, $registry_id:literal, $literal:literal, $content_type:literal) => {
        $(#[$attr])*
        pub const $name: PayloadEncoding = PayloadEncoding {
            registry_id: Some($registry_id),
            literal_id: Some(Cow::Borrowed($literal)),
            content_type: Some(Cow::Borrowed($content_type)),
        };
    };
}

impl PayloadEncoding {
    well_known_encoding!(
        /// Protocol Buffers serialized payload wrapped in `google.protobuf.Any`.
        PROTOBUF_WRAPPED_IN_ANY,
        1,
        "up.protobuf-wrapped-in-any",
        "application/x-protobuf"
    );
    well_known_encoding!(
        /// Protocol Buffers serialized payload.
        PROTOBUF,
        2,
        "up.protobuf",
        "application/protobuf"
    );
    well_known_encoding!(
        /// JSON payload.
        JSON,
        3,
        "up.json",
        "application/json"
    );
    well_known_encoding!(
        /// SOME/IP serialized payload.
        SOMEIP,
        4,
        "up.someip",
        "application/x-someip"
    );
    well_known_encoding!(
        /// SOME/IP TLV serialized payload.
        SOMEIP_TLV,
        5,
        "up.someip-tlv",
        "application/x-someip_tlv"
    );
    well_known_encoding!(
        /// Raw binary payload.
        RAW,
        6,
        "up.raw",
        "application/octet-stream"
    );
    well_known_encoding!(
        /// UTF-8 text payload.
        TEXT,
        7,
        "up.text",
        "text/plain"
    );
    well_known_encoding!(
        /// Shared-memory reference payload.
        SHM,
        8,
        "up.shm",
        "application/x-shm"
    );

    /// All encodings with registry ids permanently reserved for legacy
    /// `UPayloadFormat` compatibility.
    pub const LEGACY_COMPATIBLE: [PayloadEncoding; 8] = [
        Self::PROTOBUF_WRAPPED_IN_ANY,
        Self::PROTOBUF,
        Self::JSON,
        Self::SOMEIP,
        Self::SOMEIP_TLV,
        Self::RAW,
        Self::TEXT,
        Self::SHM,
    ];

    fn legacy_by_registry_id(registry_id: u32) -> Option<Self> {
        match registry_id {
            1 => Some(Self::PROTOBUF_WRAPPED_IN_ANY),
            2 => Some(Self::PROTOBUF),
            3 => Some(Self::JSON),
            4 => Some(Self::SOMEIP),
            5 => Some(Self::SOMEIP_TLV),
            6 => Some(Self::RAW),
            7 => Some(Self::TEXT),
            8 => Some(Self::SHM),
            _ => None,
        }
    }

    fn legacy_by_literal_id(literal_id: &str) -> Option<Self> {
        Self::LEGACY_COMPATIBLE
            .into_iter()
            .find(|encoding| encoding.literal_id() == Some(literal_id))
    }

    fn canonicalize(self) -> Result<Self, UFrameMetadataError> {
        if let Some(registry_id) = self.registry_id {
            if let Some(legacy) = Self::legacy_by_registry_id(registry_id) {
                if self
                    .literal_id()
                    .is_some_and(|literal_id| Some(literal_id) != legacy.literal_id())
                    || self
                        .content_type()
                        .is_some_and(|content_type| Some(content_type) != legacy.content_type())
                {
                    return Err(UFrameMetadataError::InvalidMetadata(format!(
                        "payload encoding registry id {registry_id} conflicts with `{}`",
                        self.describe()
                    )));
                }
                return Ok(legacy);
            }
        }

        if let Some(literal_id) = self.literal_id() {
            if let Some(legacy) = Self::legacy_by_literal_id(literal_id) {
                if self
                    .content_type()
                    .is_some_and(|content_type| Some(content_type) != legacy.content_type())
                {
                    return Err(UFrameMetadataError::InvalidMetadata(format!(
                        "payload encoding literal `{literal_id}` conflicts with content type `{}`",
                        self.content_type().unwrap_or("<absent>")
                    )));
                }
                return Ok(legacy);
            }
        }

        Ok(self)
    }

    /// Creates a validated custom payload encoding from a literal id and a
    /// media type.
    ///
    /// # Errors
    ///
    /// Returns an error when `id` is empty, `content_type` is empty, or
    /// `content_type` is not a valid media type.
    pub fn custom(
        id: impl Into<String>,
        content_type: impl Into<String>,
    ) -> Result<Self, UFrameMetadataError> {
        let encoding = Self {
            registry_id: None,
            literal_id: Some(Cow::Owned(id.into())),
            content_type: Some(Cow::Owned(content_type.into())),
        };
        encoding.validate()?;
        if let Some(literal_id) = encoding.literal_id() {
            if let Some(legacy) = Self::legacy_by_literal_id(literal_id) {
                return Err(UFrameMetadataError::RegisteredPayloadEncodingAlias {
                    literal_id: literal_id.to_string(),
                    registered: legacy.describe(),
                });
            }
        }
        encoding.canonicalize()
    }

    /// Creates a validated payload encoding from explicit identity components.
    ///
    /// # Errors
    ///
    /// Returns an error when all components are absent, when a present
    /// component is empty, or when the content type is not a valid media
    /// type.
    pub fn from_parts(
        registry_id: Option<u32>,
        literal_id: Option<String>,
        content_type: Option<String>,
    ) -> Result<Self, UFrameMetadataError> {
        let encoding = Self {
            registry_id,
            literal_id: literal_id.map(Cow::Owned),
            content_type: content_type.map(Cow::Owned),
        };
        encoding.validate()?;
        encoding.canonicalize()
    }

    /// Returns the registered numeric identity, if present.
    #[must_use]
    pub fn registry_id(&self) -> Option<u32> {
        self.registry_id
    }

    /// Returns the literal identity, if present.
    #[must_use]
    pub fn literal_id(&self) -> Option<&str> {
        self.literal_id.as_deref()
    }

    /// Returns the media type, if present.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Returns the custom encoding identity `(literal_id, content_type)` when
    /// both are present.
    #[must_use]
    pub fn custom_identity(&self) -> Option<(&str, &str)> {
        match (self.literal_id(), self.content_type()) {
            (Some(id), Some(content_type)) => Some((id, content_type)),
            _ => None,
        }
    }

    /// Returns whether this encoding can be decoded by a codec expecting `expected`.
    #[must_use]
    pub fn is_compatible_with(&self, expected: &Self) -> bool {
        self == expected
    }

    /// Projects a legacy `UPayloadFormat` to its reserved registered encoding.
    ///
    /// This is a compatibility projection: the returned encoding is the full
    /// registered identity (numeric id, literal id, and media type) reserved
    /// for the legacy value.
    ///
    /// # Errors
    ///
    /// Returns an error for `UPayloadFormat::Unspecified`, which is not a
    /// concrete payload encoding.
    pub fn try_from_legacy_format(format: UPayloadFormat) -> Result<Self, UFrameMetadataError> {
        match format {
            UPayloadFormat::Unspecified => Err(UFrameMetadataError::UnspecifiedPayloadFormat),
            UPayloadFormat::ProtobufWrappedInAny => Ok(Self::PROTOBUF_WRAPPED_IN_ANY),
            UPayloadFormat::Protobuf => Ok(Self::PROTOBUF),
            UPayloadFormat::Json => Ok(Self::JSON),
            UPayloadFormat::Someip => Ok(Self::SOMEIP),
            UPayloadFormat::SomeipTlv => Ok(Self::SOMEIP_TLV),
            UPayloadFormat::Raw => Ok(Self::RAW),
            UPayloadFormat::Text => Ok(Self::TEXT),
            UPayloadFormat::Shm => Ok(Self::SHM),
        }
    }

    /// Projects this encoding to the legacy `UPayloadFormat`, if it has a
    /// legacy equivalent.
    ///
    /// This is a compatibility projection for `UMessage`-shaped
    /// boundaries. Encodings outside the reserved legacy range return `None`;
    /// callers must treat that as "not representable", never as `Raw`.
    #[must_use]
    pub fn to_legacy_format(&self) -> Option<UPayloadFormat> {
        match self.registry_id {
            Some(1) => Some(UPayloadFormat::ProtobufWrappedInAny),
            Some(2) => Some(UPayloadFormat::Protobuf),
            Some(3) => Some(UPayloadFormat::Json),
            Some(4) => Some(UPayloadFormat::Someip),
            Some(5) => Some(UPayloadFormat::SomeipTlv),
            Some(6) => Some(UPayloadFormat::Raw),
            Some(7) => Some(UPayloadFormat::Text),
            Some(8) => Some(UPayloadFormat::Shm),
            _ => None,
        }
    }

    /// Returns a diagnostic identity string for error messages.
    #[must_use]
    pub fn describe(&self) -> String {
        if let Some(literal) = self.literal_id() {
            return literal.to_string();
        }
        if let Some(id) = self.registry_id {
            return format!("registry:{id}");
        }
        self.content_type().unwrap_or("<empty>").to_string()
    }

    pub(crate) fn validate(&self) -> Result<(), UFrameMetadataError> {
        if self.registry_id.is_none() && self.literal_id.is_none() && self.content_type.is_none() {
            return Err(UFrameMetadataError::EmptyPayloadEncoding);
        }
        if let Some(literal_id) = self.literal_id() {
            if literal_id.is_empty() {
                return Err(UFrameMetadataError::EmptyCustomEncodingId);
            }
        }
        if let Some(content_type) = self.content_type() {
            if content_type.is_empty() {
                return Err(UFrameMetadataError::EmptyCustomEncodingContentType);
            }
            mediatype::MediaType::parse(content_type).map_err(|error| {
                UFrameMetadataError::InvalidCustomEncodingContentType(error.to_string())
            })?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by native frame metadata operations and projections.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UFrameMetadataError {
    /// A payload encoding was constructed without any identity component.
    EmptyPayloadEncoding,
    /// A payload encoding literal id is empty.
    EmptyCustomEncodingId,
    /// A payload encoding content type is empty.
    EmptyCustomEncodingContentType,
    /// A payload encoding content type is not a valid media type.
    InvalidCustomEncodingContentType(String),
    /// `UPayloadFormat::Unspecified` is not a concrete payload encoding.
    UnspecifiedPayloadFormat,
    /// Payload bytes require a payload encoding.
    PayloadWithoutEncoding,
    /// A payload encoding requires payload bytes.
    EncodingWithoutPayload,
    /// A native payload encoding cannot be represented by legacy types.
    EncodingNotRepresentable {
        /// Diagnostic identity of the offending encoding.
        encoding: String,
    },
    /// An open identity used a literal reserved for a registered legacy row.
    RegisteredPayloadEncodingAlias {
        /// Literal identity that aliases a registered legacy row.
        literal_id: String,
        /// Canonical registered spelling.
        registered: String,
    },
    /// A semantic field cannot be represented by legacy types.
    FieldNotRepresentable {
        /// Name of the offending field.
        field: &'static str,
        /// Reason the value is not representable.
        reason: String,
    },
    /// The metadata violates the validity rules for its frame message kind.
    InvalidMetadata(String),
    /// Building a `UMessage` from projected metadata failed.
    MessageBuildError(String),
}

impl std::fmt::Display for UFrameMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPayloadEncoding => {
                f.write_str("payload encoding must have at least one identity component")
            }
            Self::EmptyCustomEncodingId => f.write_str("custom payload encoding id is empty"),
            Self::EmptyCustomEncodingContentType => {
                f.write_str("custom payload encoding content_type is empty")
            }
            Self::InvalidCustomEncodingContentType(error) => f.write_fmt(format_args!(
                "custom payload encoding content_type is invalid: {error}"
            )),
            Self::UnspecifiedPayloadFormat => {
                f.write_str("UPayloadFormat::Unspecified is not a concrete payload encoding")
            }
            Self::PayloadWithoutEncoding => f.write_str("payload bytes require a payload encoding"),
            Self::EncodingWithoutPayload => f.write_str("payload encoding requires payload bytes"),
            Self::EncodingNotRepresentable { encoding } => f.write_fmt(format_args!(
                "payload encoding `{encoding}` cannot be represented by legacy UPayloadFormat"
            )),
            Self::RegisteredPayloadEncodingAlias {
                literal_id,
                registered,
            } => f.write_fmt(format_args!(
                "payload encoding literal `{literal_id}` aliases registered encoding `{registered}`; use the registered spelling"
            )),
            Self::FieldNotRepresentable { field, reason } => f.write_fmt(format_args!(
                "frame metadata field `{field}` cannot be represented by legacy types: {reason}"
            )),
            Self::InvalidMetadata(error) => {
                f.write_fmt(format_args!("invalid frame metadata: {error}"))
            }
            Self::MessageBuildError(error) => f.write_fmt(format_args!(
                "failed to build UMessage from frame metadata: {error}"
            )),
        }
    }
}

impl std::error::Error for UFrameMetadataError {}

impl From<UMessageError> for UFrameMetadataError {
    fn from(value: UMessageError) -> Self {
        Self::MessageBuildError(value.to_string())
    }
}

// ---------------------------------------------------------------------------
// UFrameMetadata
// ---------------------------------------------------------------------------

/// Canonical semantic metadata of a native uProtocol frame.
///
/// This is the model that owned frames (`UOwnedFrame`), zero-copy
/// frame views, and selected-wire metadata codecs reason about. It owns its
/// fields ergonomically (strings, options, [`Duration`]) because it is never
/// reinterpreted as raw bytes across processes or languages; fixed-layout
/// and byte-serialized representations are derived from it.
///
/// The frame's payload presence is a property of the frame carrier
/// (`UOwnedFrame`, `UFrameView`, TX loan specs), not of the
/// metadata; the v1 invariant "payload present if and only if
/// `payload_encoding` is present" is enforced at those boundaries.
#[derive(Clone, Debug, PartialEq)]
pub struct UFrameMetadata {
    kind: FrameMessageKind,
    id: UUID,
    source: UUri,
    sink: Option<UUri>,
    reqid: Option<UUID>,
    priority: Option<FramePriority>,
    ttl: Option<Duration>,
    comm_status: Option<UCode>,
    permission_level: Option<u32>,
    token: Option<String>,
    traceparent: Option<String>,
    payload_encoding: Option<PayloadEncoding>,
}

impl UFrameMetadata {
    /// Starts building metadata for a publish frame on `topic`.
    #[must_use]
    pub fn publish(topic: UUri) -> UFrameMetadataBuilder {
        UFrameMetadataBuilder::new(FrameMessageKind::Publish, topic, None)
    }

    /// Starts building metadata for a notification frame from `origin` to
    /// `destination`.
    #[must_use]
    pub fn notification(origin: UUri, destination: UUri) -> UFrameMetadataBuilder {
        UFrameMetadataBuilder::new(FrameMessageKind::Notification, origin, Some(destination))
    }

    /// Starts building metadata for an RPC request frame invoking `method`,
    /// with responses directed to `reply_to`.
    ///
    /// Request frames require a TTL and default to [`FramePriority::CS4`].
    #[must_use]
    pub fn request(method: UUri, reply_to: UUri, ttl: Duration) -> UFrameMetadataBuilder {
        let mut builder =
            UFrameMetadataBuilder::new(FrameMessageKind::Request, reply_to, Some(method));
        builder.priority = Some(FramePriority::CS4);
        builder.ttl = Some(ttl);
        builder
    }

    /// Starts building metadata for an RPC response frame from `invoked_method`
    /// to `reply_to`, correlated to the request with id `request_id`.
    ///
    /// Response frames default to [`FramePriority::CS4`].
    #[must_use]
    pub fn response(
        invoked_method: UUri,
        reply_to: UUri,
        request_id: UUID,
    ) -> UFrameMetadataBuilder {
        let mut builder =
            UFrameMetadataBuilder::new(FrameMessageKind::Response, invoked_method, Some(reply_to));
        builder.priority = Some(FramePriority::CS4);
        builder.reqid = Some(request_id);
        builder
    }

    /// Assembles metadata from decoded parts without validation.
    ///
    /// Callers (frame codecs, ABI profile conversions) must run
    /// [`UFrameMetadata::validate`] on the result.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_decoded_parts(
        kind: FrameMessageKind,
        id: UUID,
        source: UUri,
        sink: Option<UUri>,
        reqid: Option<UUID>,
        priority: Option<FramePriority>,
        ttl: Option<Duration>,
        comm_status: Option<UCode>,
        permission_level: Option<u32>,
        token: Option<String>,
        traceparent: Option<String>,
        payload_encoding: Option<PayloadEncoding>,
    ) -> Self {
        Self {
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
        }
    }

    /// Returns the frame message kind.
    #[must_use]
    pub fn kind(&self) -> FrameMessageKind {
        self.kind
    }

    /// Returns the unique frame id.
    #[must_use]
    pub fn id(&self) -> &UUID {
        &self.id
    }

    /// Returns the source address.
    #[must_use]
    pub fn source(&self) -> &UUri {
        &self.source
    }

    /// Returns the sink address, if present.
    #[must_use]
    pub fn sink(&self) -> Option<&UUri> {
        self.sink.as_ref()
    }

    /// Returns the correlated request id, if present.
    #[must_use]
    pub fn reqid(&self) -> Option<&UUID> {
        self.reqid.as_ref()
    }

    /// Returns the frame priority, if present.
    #[must_use]
    pub fn priority(&self) -> Option<FramePriority> {
        self.priority
    }

    /// Returns the time-to-live, if present.
    #[must_use]
    pub fn ttl(&self) -> Option<Duration> {
        self.ttl
    }

    /// Returns the communication status, if present.
    #[must_use]
    pub fn comm_status(&self) -> Option<UCode> {
        self.comm_status
    }

    /// Returns the permission level, if present.
    #[must_use]
    pub fn permission_level(&self) -> Option<u32> {
        self.permission_level
    }

    /// Returns the access token, if present.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Returns the W3C traceparent, if present.
    #[must_use]
    pub fn traceparent(&self) -> Option<&str> {
        self.traceparent.as_deref()
    }

    /// Returns the native payload encoding, if one is present.
    #[must_use]
    pub fn payload_encoding(&self) -> Option<&PayloadEncoding> {
        self.payload_encoding.as_ref()
    }

    /// Consumes this metadata and returns its native payload encoding.
    #[must_use]
    pub fn into_payload_encoding(self) -> Option<PayloadEncoding> {
        self.payload_encoding
    }

    /// Returns metadata equal to `self` but carrying `payload_encoding`.
    ///
    /// # Errors
    ///
    /// Returns an error if the encoding or the resulting metadata is invalid.
    pub fn with_payload_encoding(
        mut self,
        payload_encoding: PayloadEncoding,
    ) -> Result<Self, UFrameMetadataError> {
        payload_encoding.validate()?;
        self.payload_encoding = Some(payload_encoding);
        self.validate()?;
        Ok(self)
    }

    /// Returns metadata equal to `self` but without a payload encoding.
    #[must_use]
    pub fn without_payload_encoding(mut self) -> Self {
        self.payload_encoding = None;
        self
    }

    /// Validates this metadata against the rules of its frame message kind.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata violates the rules for its kind, for
    /// example a publish frame carrying a sink, a request frame without TTL,
    /// or an RPC frame with a priority below [`FramePriority::CS4`].
    pub fn validate(&self) -> Result<(), UFrameMetadataError> {
        if let Some(encoding) = &self.payload_encoding {
            encoding.validate()?;
        }
        let mut errors: Vec<String> = Vec::new();
        match self.kind {
            FrameMessageKind::Publish => {
                if let Err(e) = self.source.verify_event() {
                    errors.push(format!("invalid source URI: {e}"));
                }
                if self.sink.is_some() {
                    errors.push("publish frame must not have a sink".to_string());
                }
            }
            FrameMessageKind::Notification => {
                if self.source.is_rpc_response() {
                    errors.push("source must not be an RPC response URI".to_string());
                } else if let Err(e) = self.source.verify_no_wildcards() {
                    errors.push(format!("invalid source URI: {e}"));
                }
                match &self.sink {
                    Some(sink) => {
                        if !sink.is_notification_destination() {
                            errors.push("sink is not a valid notification destination".to_string());
                        } else if let Err(e) = sink.verify_no_wildcards() {
                            errors.push(format!("invalid sink URI: {e}"));
                        }
                    }
                    None => errors.push("notification frame must have a sink".to_string()),
                }
            }
            FrameMessageKind::Request => {
                if let Err(e) = self.source.verify_rpc_response() {
                    errors.push(format!("invalid source URI: {e}"));
                }
                match &self.sink {
                    Some(sink) => {
                        if let Err(e) = sink.verify_rpc_method() {
                            errors.push(format!("invalid sink URI: {e}"));
                        }
                    }
                    None => errors
                        .push("request frame must have a method-to-invoke in the sink".to_string()),
                }
                match self.ttl {
                    Some(ttl) if !ttl.is_zero() => {}
                    Some(_) => {
                        errors.push("request frame TTL must be greater than zero".to_string())
                    }
                    None => errors.push("request frame must have a TTL".to_string()),
                }
                if let Err(e) = self.validate_rpc_priority() {
                    errors.push(e);
                }
            }
            FrameMessageKind::Response => {
                if let Err(e) = self.source.verify_rpc_method() {
                    errors.push(format!("invalid source URI: {e}"));
                }
                match &self.sink {
                    Some(sink) => {
                        if let Err(e) = sink.verify_rpc_response() {
                            errors.push(format!("invalid sink URI: {e}"));
                        }
                    }
                    None => errors.push("response frame must have a sink".to_string()),
                }
                if self.reqid.is_none() {
                    errors.push("response frame must have a request id".to_string());
                }
                if let Err(e) = self.validate_rpc_priority() {
                    errors.push(e);
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(UFrameMetadataError::InvalidMetadata(errors.join("; ")))
        }
    }

    fn validate_rpc_priority(&self) -> Result<(), String> {
        match self.priority {
            Some(priority) if priority >= FramePriority::CS4 => Ok(()),
            Some(_) => Err("RPC frame must have a priority of at least CS4".to_string()),
            None => Err("RPC frame must have a priority".to_string()),
        }
    }

    /// Projects this metadata to legacy `UAttributes`.
    ///
    /// This is a compatibility projection for `UTransport`-family (`UMessage`-shaped)
    /// boundaries and is fallible by design only for fields that cannot be
    /// represented by legacy types, such as a TTL that does not fit the
    /// legacy 32-bit millisecond field. Open payload encodings are represented
    /// by the `UAttributes` payload-encoding identity fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata is invalid or not representable.
    pub fn try_project_to_attributes(&self) -> Result<UAttributes, UFrameMetadataError> {
        let (payload_format, open_encoding) = match &self.payload_encoding {
            None => (Some(UPayloadFormat::Unspecified), None),
            Some(encoding) => match encoding.to_legacy_format() {
                Some(format) => (Some(format), None),
                None => (None, Some(encoding)),
            },
        };
        let mut attributes = self.project_to_attributes_with_payload_format(payload_format)?;
        if let Some(encoding) = open_encoding {
            attributes.payload_encoding_registry_id = encoding.registry_id();
            attributes.payload_encoding = encoding.literal_id().map(str::to_owned);
            attributes.payload_content_type = encoding.content_type().map(str::to_owned);
        }
        Ok(attributes)
    }

    /// Projects this metadata to legacy `UAttributes` with an explicitly
    /// chosen `payload_format`.
    ///
    /// This exists for codecs that carry the open payload encoding in a
    /// separate block next to the serialized attributes (so a non-legacy
    /// encoding does not have to be representable by `UPayloadFormat`).
    pub(crate) fn project_to_attributes_with_payload_format(
        &self,
        payload_format: Option<UPayloadFormat>,
    ) -> Result<UAttributes, UFrameMetadataError> {
        self.validate()?;
        Ok(UAttributes {
            type_: self.kind.to_legacy_type(),
            id: self.id.clone(),
            source: self.source.clone(),
            sink: self.sink.clone(),
            priority: self.priority.map(FramePriority::to_legacy_priority),
            commstatus: self.comm_status,
            ttl: self.ttl.map(project_ttl_to_legacy_millis).transpose()?,
            permission_level: self.permission_level,
            token: self.token.clone(),
            traceparent: self.traceparent.clone(),
            reqid: self.reqid.clone(),
            payload_format,
            payload_encoding_registry_id: None,
            payload_encoding: None,
            payload_content_type: None,
        })
    }
}

fn project_ttl_to_legacy_millis(ttl: Duration) -> Result<u32, UFrameMetadataError> {
    if !ttl.subsec_nanos().is_multiple_of(1_000_000) {
        return Err(UFrameMetadataError::FieldNotRepresentable {
            field: "ttl",
            reason: format!(
                "{ttl:?} has sub-millisecond precision; legacy TTL is in whole milliseconds"
            ),
        });
    }
    u32::try_from(ttl.as_millis()).map_err(|_| UFrameMetadataError::FieldNotRepresentable {
        field: "ttl",
        reason: format!("{ttl:?} exceeds the legacy 32-bit millisecond TTL range"),
    })
}

// ---------------------------------------------------------------------------
// UFrameMetadataBuilder
// ---------------------------------------------------------------------------

/// Builder for native frame metadata.
///
/// Obtained from [`UFrameMetadata::publish`], [`UFrameMetadata::notification`],
/// [`UFrameMetadata::request`], or [`UFrameMetadata::response`]. Native
/// selected-wire code should construct metadata through this builder instead
/// of going through `UMessageBuilder` and projecting.
#[derive(Clone, Debug)]
pub struct UFrameMetadataBuilder {
    kind: FrameMessageKind,
    id: Option<UUID>,
    source: UUri,
    sink: Option<UUri>,
    reqid: Option<UUID>,
    priority: Option<FramePriority>,
    ttl: Option<Duration>,
    comm_status: Option<UCode>,
    permission_level: Option<u32>,
    token: Option<String>,
    traceparent: Option<String>,
    payload_encoding: Option<PayloadEncoding>,
}

impl UFrameMetadataBuilder {
    fn new(kind: FrameMessageKind, source: UUri, sink: Option<UUri>) -> Self {
        Self {
            kind,
            id: None,
            source,
            sink,
            reqid: None,
            priority: None,
            ttl: None,
            comm_status: None,
            permission_level: None,
            token: None,
            traceparent: None,
            payload_encoding: None,
        }
    }

    /// Sets an explicit frame id instead of a generated one.
    #[must_use]
    pub fn with_id(mut self, id: UUID) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the frame priority.
    #[must_use]
    pub fn with_priority(mut self, priority: FramePriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets the time-to-live.
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Sets the communication status.
    #[must_use]
    pub fn with_comm_status(mut self, comm_status: UCode) -> Self {
        self.comm_status = Some(comm_status);
        self
    }

    /// Sets the permission level.
    #[must_use]
    pub fn with_permission_level(mut self, permission_level: u32) -> Self {
        self.permission_level = Some(permission_level);
        self
    }

    /// Sets the access token.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Sets the W3C traceparent.
    #[must_use]
    pub fn with_traceparent(mut self, traceparent: impl Into<String>) -> Self {
        self.traceparent = Some(traceparent.into());
        self
    }

    /// Sets the payload encoding of the frame's payload bytes.
    #[must_use]
    pub fn with_payload_encoding(mut self, payload_encoding: PayloadEncoding) -> Self {
        self.payload_encoding = Some(payload_encoding);
        self
    }

    /// Builds and validates the frame metadata.
    ///
    /// A fresh UUIDv7 frame id is generated unless one was supplied via
    /// [`Self::with_id`].
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata violates the rules of its frame
    /// message kind.
    pub fn build(self) -> Result<UFrameMetadata, UFrameMetadataError> {
        let metadata = UFrameMetadata {
            kind: self.kind,
            id: self.id.unwrap_or_else(UUID::build),
            source: self.source,
            sink: self.sink,
            reqid: self.reqid,
            priority: self.priority,
            ttl: self.ttl,
            comm_status: self.comm_status,
            permission_level: self.permission_level,
            token: self.token,
            traceparent: self.traceparent,
            payload_encoding: self.payload_encoding,
        };
        metadata.validate()?;
        Ok(metadata)
    }
}

// ---------------------------------------------------------------------------
// Legacy projections
// ---------------------------------------------------------------------------

impl crate::UMessage {
    /// Projects this message to frame metadata carrying the given payload
    /// encoding.
    ///
    /// # Errors
    ///
    /// Returns an error if the message attributes and encoding disagree or
    /// the resulting metadata is invalid.
    pub fn to_frame_metadata(
        &self,
        payload_encoding: PayloadEncoding,
    ) -> Result<UFrameMetadata, UFrameMetadataError> {
        if self.payload().is_none() {
            return Err(UFrameMetadataError::EncodingWithoutPayload);
        }
        self.attributes().to_frame_metadata(payload_encoding)
    }

    /// Projects this message to frame metadata for a payload-free message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message carries payload information or the
    /// resulting metadata is invalid.
    pub fn to_frame_metadata_unencoded(&self) -> Result<UFrameMetadata, UFrameMetadataError> {
        if self.payload().is_some() {
            return Err(UFrameMetadataError::PayloadWithoutEncoding);
        }
        self.attributes().to_frame_metadata_unencoded()
    }
}

impl crate::UAttributes {
    /// Projects these attributes to frame metadata carrying the given
    /// payload encoding.
    ///
    /// Convenience for [`try_project_attributes_to_frame_metadata`]; use
    /// [`Self::to_frame_metadata_unencoded`] when the message carries no
    /// payload.
    ///
    /// # Errors
    ///
    /// Returns an error if the attributes and encoding disagree or the
    /// resulting metadata is invalid.
    pub fn to_frame_metadata(
        &self,
        payload_encoding: PayloadEncoding,
    ) -> Result<UFrameMetadata, UFrameMetadataError> {
        try_project_attributes_to_frame_metadata(self, Some(payload_encoding))
    }

    /// Projects these attributes to frame metadata for a payload-free
    /// message.
    ///
    /// # Errors
    ///
    /// Returns an error if the attributes carry payload information or the
    /// resulting metadata is invalid.
    pub fn to_frame_metadata_unencoded(&self) -> Result<UFrameMetadata, UFrameMetadataError> {
        let metadata = try_project_attributes_to_frame_metadata(self, None)?;
        if metadata.payload_encoding().is_some() {
            return Err(UFrameMetadataError::EncodingWithoutPayload);
        }
        Ok(metadata)
    }
}

/// Projects legacy `UAttributes` (plus an optional native payload encoding)
/// into native frame metadata.
///
/// When `payload_encoding` is `None` but the attributes carry a concrete
/// `payload_format`, the reserved registered encoding for that format is
/// used. When both are present they must agree.
///
/// # Errors
///
/// Returns an error if the attributes and encoding disagree or the resulting
/// metadata is invalid.
pub fn try_project_attributes_to_frame_metadata(
    attributes: &UAttributes,
    payload_encoding: Option<PayloadEncoding>,
) -> Result<UFrameMetadata, UFrameMetadataError> {
    let legacy_encoding = match attributes.payload_format() {
        None | Some(UPayloadFormat::Unspecified) => None,
        Some(format) => Some(PayloadEncoding::try_from_legacy_format(format)?),
    };
    let open_present = attributes.open_payload_encoding_parts() != (None, None, None);
    let attributes_encoding = match (legacy_encoding, open_present) {
        (Some(_), true) => {
            return Err(UFrameMetadataError::InvalidMetadata(
                "attributes declare both a concrete payload_format and open payload-encoding fields; exactly one mechanism is allowed".into(),
            ));
        }
        (Some(legacy), false) => Some(legacy),
        (None, true) => {
            let encoding = PayloadEncoding::from_parts(
                attributes.payload_encoding_registry_id,
                attributes.payload_encoding.clone(),
                attributes.payload_content_type.clone(),
            )?;
            if encoding.to_legacy_format().is_some() {
                return Err(UFrameMetadataError::InvalidMetadata(
                    "registered-range payload encodings must use payload_format, not open payload-encoding fields".into(),
                ));
            }
            Some(encoding)
        }
        (None, false) => None,
    };
    let payload_encoding = match (payload_encoding, attributes_encoding) {
        (Some(explicit), Some(from_attributes)) => {
            if explicit != from_attributes {
                return Err(UFrameMetadataError::InvalidMetadata(format!(
                    "attribute payload format `{}` does not match payload encoding `{}`",
                    from_attributes.describe(),
                    explicit.describe()
                )));
            }
            Some(explicit)
        }
        (Some(explicit), None) => Some(explicit),
        (None, from_attributes) => from_attributes,
    };

    let metadata = UFrameMetadata {
        kind: FrameMessageKind::from_legacy_type(attributes.type_),
        id: attributes.id.clone(),
        source: attributes.source.clone(),
        sink: attributes.sink.clone(),
        reqid: attributes.reqid.clone(),
        priority: attributes.priority.map(FramePriority::from_legacy_priority),
        ttl: attributes.ttl.map(u64::from).map(Duration::from_millis),
        comm_status: attributes.commstatus,
        permission_level: attributes.permission_level,
        token: attributes.token.clone(),
        traceparent: attributes.traceparent.clone(),
        payload_encoding,
    };
    metadata.validate()?;
    Ok(metadata)
}

/// Projects a `UMessage` into native frame metadata.
///
/// # Errors
///
/// Returns an error when a message carries payload bytes without a concrete
/// payload encoding, or a concrete payload encoding without payload bytes.
pub fn try_project_umessage_to_frame_metadata(
    message: &UMessage,
) -> Result<UFrameMetadata, UFrameMetadataError> {
    let open_present = message.attributes().open_payload_encoding_parts() != (None, None, None);
    let legacy_present = matches!(
        message.payload_format(),
        Some(format) if format != UPayloadFormat::Unspecified
    );
    match (message.payload().is_some(), legacy_present || open_present) {
        (true, false) => {
            return Err(UFrameMetadataError::PayloadWithoutEncoding);
        }
        (false, true) => {
            return Err(UFrameMetadataError::EncodingWithoutPayload);
        }
        _ => {}
    }
    try_project_attributes_to_frame_metadata(message.attributes(), None)
}

/// Projects native frame metadata and optional payload bytes into a
/// `UMessage`.
///
/// Native payload encodings without a legacy `UPayloadFormat` equivalent are
/// carried by the `UAttributes` open payload-encoding identity fields.
///
/// # Errors
///
/// Returns an error if payload bytes and payload encoding are not both
/// present or both absent, if the encoding is not representable, or if
/// metadata invariants are violated.
pub fn try_project_frame_to_umessage(
    metadata: UFrameMetadata,
    payload: Option<Bytes>,
) -> Result<UMessage, UFrameMetadataError> {
    match (payload.is_some(), metadata.payload_encoding().is_some()) {
        (true, true) | (false, false) => {}
        (true, false) => return Err(UFrameMetadataError::PayloadWithoutEncoding),
        (false, true) => return Err(UFrameMetadataError::EncodingWithoutPayload),
    }
    let attributes = metadata.try_project_to_attributes()?;
    UMessage::new(attributes, payload).map_err(UFrameMetadataError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{UCode, UMessageBuilder, UUri};

    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("failed to create test URI")
    }

    fn method() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x00b1).expect("failed to create method URI")
    }

    fn reply_to() -> UUri {
        UUri::try_from_parts("cloud", 0x10ab, 0x02, 0x0000).expect("failed to create reply URI")
    }

    #[test]
    fn builder_constructs_native_publish_metadata() {
        let metadata = UFrameMetadata::publish(topic())
            .with_priority(FramePriority::CS1)
            .with_traceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
            .with_payload_encoding(PayloadEncoding::JSON)
            .build()
            .expect("metadata");

        assert_eq!(metadata.kind(), FrameMessageKind::Publish);
        assert_eq!(metadata.source(), &topic());
        assert!(metadata.sink().is_none());
        assert_eq!(metadata.priority(), Some(FramePriority::CS1));
        assert_eq!(metadata.payload_encoding(), Some(&PayloadEncoding::JSON));
    }

    #[test]
    fn builder_rejects_publish_with_low_rpc_shape() {
        // request without ttl
        let error = UFrameMetadata::request(method(), reply_to(), Duration::ZERO)
            .build()
            .unwrap_err();
        assert!(matches!(error, UFrameMetadataError::InvalidMetadata(_)));

        // rpc priority below CS4
        let error = UFrameMetadata::request(method(), reply_to(), Duration::from_secs(5))
            .with_priority(FramePriority::CS2)
            .build()
            .unwrap_err();
        assert!(matches!(error, UFrameMetadataError::InvalidMetadata(_)));
    }

    #[test]
    fn builder_constructs_request_and_response_metadata() {
        let request = UFrameMetadata::request(method(), reply_to(), Duration::from_secs(5))
            .build()
            .expect("request metadata");
        assert_eq!(request.kind(), FrameMessageKind::Request);
        assert_eq!(request.priority(), Some(FramePriority::CS4));
        assert_eq!(request.ttl(), Some(Duration::from_secs(5)));

        let response = UFrameMetadata::response(method(), reply_to(), request.id().clone())
            .build()
            .expect("response metadata");
        assert_eq!(response.kind(), FrameMessageKind::Response);
        assert_eq!(response.reqid(), Some(request.id()));
    }

    #[test]
    fn message_without_payload_projects_to_metadata_without_encoding() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");

        let metadata = try_project_umessage_to_frame_metadata(&message).expect("metadata");

        assert!(message.payload().is_none());
        assert!(metadata.payload_encoding().is_none());
        assert_eq!(metadata.id(), message.id());
        assert_eq!(metadata.kind(), FrameMessageKind::Publish);
    }

    #[test]
    fn message_method_without_payload_projects_to_metadata_without_encoding() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");

        let metadata = message.to_frame_metadata_unencoded().expect("metadata");

        assert!(metadata.payload_encoding().is_none());
    }

    #[test]
    fn message_method_with_payload_requires_explicit_encoding() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::from_static(b"payload"), UPayloadFormat::Unspecified)
            .expect("message");

        assert_eq!(
            message.to_frame_metadata_unencoded().unwrap_err(),
            UFrameMetadataError::PayloadWithoutEncoding
        );
        assert_eq!(
            message
                .to_frame_metadata(PayloadEncoding::RAW)
                .expect("explicit encoding")
                .payload_encoding(),
            Some(&PayloadEncoding::RAW)
        );
    }

    #[test]
    fn message_method_rejects_encoding_without_payload() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");

        assert_eq!(
            message.to_frame_metadata(PayloadEncoding::RAW).unwrap_err(),
            UFrameMetadataError::EncodingWithoutPayload
        );
    }

    #[test]
    fn attributes_unencoded_method_rejects_declared_encoding() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::new(), UPayloadFormat::Raw)
            .expect("message");

        assert_eq!(
            message
                .attributes()
                .to_frame_metadata_unencoded()
                .unwrap_err(),
            UFrameMetadataError::EncodingWithoutPayload
        );
    }

    #[test]
    fn empty_present_payload_keeps_encoding() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::new(), UPayloadFormat::Raw)
            .expect("message");

        let metadata = try_project_umessage_to_frame_metadata(&message).expect("metadata");

        assert_eq!(message.payload(), Some(Bytes::new()));
        assert_eq!(metadata.payload_encoding(), Some(&PayloadEncoding::RAW));

        let projected =
            try_project_frame_to_umessage(metadata, Some(Bytes::new())).expect("projected message");
        assert_eq!(projected.payload(), Some(Bytes::new()));
        assert_eq!(projected.payload_format(), Some(UPayloadFormat::Raw));
    }

    #[test]
    fn standard_payload_formats_round_trip() {
        for format in [
            UPayloadFormat::Protobuf,
            UPayloadFormat::ProtobufWrappedInAny,
            UPayloadFormat::Raw,
            UPayloadFormat::Json,
            UPayloadFormat::Text,
            UPayloadFormat::Someip,
            UPayloadFormat::SomeipTlv,
            UPayloadFormat::Shm,
        ] {
            let message = UMessageBuilder::publish(topic())
                .build_with_payload(Bytes::from_static(b"payload"), format)
                .expect("message");
            let metadata = try_project_umessage_to_frame_metadata(&message).expect("metadata");
            let encoding = metadata.payload_encoding().expect("encoding");
            assert_eq!(encoding.to_legacy_format(), Some(format));
            assert!(encoding.registry_id().is_some());
            assert!(encoding.literal_id().is_some());
            assert!(encoding.content_type().is_some());

            let projected =
                try_project_frame_to_umessage(metadata, Some(Bytes::from_static(b"payload")))
                    .expect("projected message");
            assert_eq!(projected.payload(), Some(Bytes::from_static(b"payload")));
            assert_eq!(projected.payload_format(), Some(format));
        }
    }

    #[test]
    fn unspecified_payload_format_with_payload_is_rejected() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::from_static(b"payload"), UPayloadFormat::Unspecified)
            .expect("message");

        let error = try_project_umessage_to_frame_metadata(&message).unwrap_err();

        assert_eq!(error, UFrameMetadataError::PayloadWithoutEncoding);
    }

    #[test]
    fn attribute_projection_rejects_payload_format_mismatch() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::from_static(b"payload"), UPayloadFormat::Raw)
            .expect("message");

        let error = try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(PayloadEncoding::JSON),
        )
        .unwrap_err();

        assert!(matches!(error, UFrameMetadataError::InvalidMetadata(_)));
    }

    #[test]
    fn custom_encoding_projection_to_umessage_carries_open_identity() {
        let metadata = UFrameMetadata::publish(topic())
            .with_payload_encoding(
                PayloadEncoding::custom("up.native", "application/vnd.example.native").unwrap(),
            )
            .build()
            .expect("metadata");

        let message = try_project_frame_to_umessage(metadata, Some(Bytes::from_static(b"payload")))
            .expect("projection");

        assert_eq!(
            message.attributes().open_payload_encoding_parts(),
            (
                None,
                Some("up.native"),
                Some("application/vnd.example.native")
            )
        );
    }

    #[test]
    fn sub_millisecond_ttl_is_rejected_by_legacy_projection_not_truncated() {
        let metadata = UFrameMetadata::request(
            method(),
            reply_to(),
            Duration::from_micros(1_500), // 1.5 ms
        )
        .build()
        .expect("metadata");

        let error = metadata.try_project_to_attributes().unwrap_err();
        assert!(matches!(
            error,
            UFrameMetadataError::FieldNotRepresentable { field: "ttl", .. }
        ));
    }

    #[test]
    fn custom_encoding_is_validated() {
        assert_eq!(
            PayloadEncoding::custom("", "application/vnd.example.native").unwrap_err(),
            UFrameMetadataError::EmptyCustomEncodingId
        );
        assert_eq!(
            PayloadEncoding::custom("native", "").unwrap_err(),
            UFrameMetadataError::EmptyCustomEncodingContentType
        );
        assert!(matches!(
            PayloadEncoding::custom("native", "not a media type"),
            Err(UFrameMetadataError::InvalidCustomEncodingContentType(_))
        ));
        assert_eq!(
            PayloadEncoding::from_parts(None, None, None).unwrap_err(),
            UFrameMetadataError::EmptyPayloadEncoding
        );
        assert!(matches!(
            PayloadEncoding::custom("up.raw", "application/octet-stream"),
            Err(UFrameMetadataError::RegisteredPayloadEncodingAlias { .. })
        ));
    }

    #[test]
    fn registered_literal_decodes_to_canonical_encoding() {
        let encoding = PayloadEncoding::from_parts(
            None,
            Some("up.raw".to_string()),
            Some("application/octet-stream".to_string()),
        )
        .expect("canonicalized");

        assert_eq!(encoding, PayloadEncoding::RAW);
    }

    #[test]
    fn attributes_reject_both_encoding_mechanisms() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload(Bytes::from_static(b"x"), UPayloadFormat::Raw)
            .expect("message");
        let mut attributes = message.attributes().clone();
        attributes.payload_encoding = Some("up.xcdr-v2".to_string());

        let error = try_project_attributes_to_frame_metadata(&attributes, None).unwrap_err();

        assert!(matches!(error, UFrameMetadataError::InvalidMetadata(_)));
    }

    #[test]
    fn attributes_reject_registered_encoding_in_open_fields() {
        let message = UMessageBuilder::publish(topic())
            .build_with_payload_encoding(
                Bytes::from_static(b"x"),
                PayloadEncoding::custom("up.xcdr-v2", "application/vnd.uprotocol.xcdr-v2")
                    .expect("encoding"),
            )
            .expect("message");
        let mut attributes = message.attributes().clone();
        attributes.payload_encoding = Some("up.raw".to_string());
        attributes.payload_content_type = Some("application/octet-stream".to_string());

        let error = try_project_attributes_to_frame_metadata(&attributes, None).unwrap_err();

        assert!(matches!(error, UFrameMetadataError::InvalidMetadata(_)));
    }

    #[test]
    fn legacy_reserved_registry_ids_are_value_compatible() {
        for (encoding, format) in [
            (
                PayloadEncoding::PROTOBUF_WRAPPED_IN_ANY,
                UPayloadFormat::ProtobufWrappedInAny,
            ),
            (PayloadEncoding::PROTOBUF, UPayloadFormat::Protobuf),
            (PayloadEncoding::JSON, UPayloadFormat::Json),
            (PayloadEncoding::SOMEIP, UPayloadFormat::Someip),
            (PayloadEncoding::SOMEIP_TLV, UPayloadFormat::SomeipTlv),
            (PayloadEncoding::RAW, UPayloadFormat::Raw),
            (PayloadEncoding::TEXT, UPayloadFormat::Text),
            (PayloadEncoding::SHM, UPayloadFormat::Shm),
        ] {
            assert_eq!(
                encoding.registry_id(),
                Some(u32::try_from(format.as_i32()).expect("legacy format code is positive"))
            );
            assert_eq!(
                PayloadEncoding::try_from_legacy_format(format).expect("projection"),
                encoding
            );
            assert_eq!(encoding.to_legacy_format(), Some(format));
        }
        assert!(PayloadEncoding::try_from_legacy_format(UPayloadFormat::Unspecified).is_err());
    }

    #[test]
    fn pr328_enum_names_compile() {
        assert_eq!(UCode::InvalidArgument as i32, 3);
        assert_eq!(UCode::Unimplemented as i32, 12);
        assert_eq!(UPayloadFormat::ProtobufWrappedInAny.as_i32(), 1);
        assert_eq!(UPayloadFormat::Someip.as_i32(), 4);
        assert_eq!(UPayloadFormat::SomeipTlv.as_i32(), 5);
    }
}

#[cfg(test)]
mod projection_round_trip_properties {
    use proptest::prelude::*;

    use crate::{PayloadEncoding, UMessageBuilder, UPayloadFormat, UUri};

    proptest! {
        /// Metadata projection round-trip: a publish message built from any
        /// valid parts projects to frame metadata whose canonical field-block
        /// encoding decodes back to equal metadata. Hand-written cases pin the
        /// known edges; this pins the space between them.
        #[test]
        fn projected_metadata_survives_field_block_round_trip(
            authority in "[a-z][a-z0-9]{0,11}",
            ue_id in 1u32..0xFFFF,
            version in 1u8..=0xFE,
            resource in 0x8000u16..0xFFFE,
            payload in proptest::collection::vec(any::<u8>(), 0..64),
        ) {
            let topic = UUri::try_from_parts(&authority, ue_id, version, resource)
                .expect("parts chosen from the valid range");
            let message = if payload.is_empty() {
                UMessageBuilder::publish(topic).build().expect("valid publish message")
            } else {
                UMessageBuilder::publish(topic)
                    .build_with_payload(payload, UPayloadFormat::Raw)
                    .expect("valid publish message with payload")
            };

            let metadata = if message.payload().is_some() {
                message.to_frame_metadata(PayloadEncoding::RAW)
            } else {
                message.to_frame_metadata_unencoded()
            }
            .expect("valid message must project");

            let encoded = crate::frame::codec::encode_frame_metadata_fields(&metadata)
                .expect("projected metadata must encode");
            let decoded = crate::frame::codec::decode_frame_metadata_fields(&encoded)
                .expect("encoded metadata must decode");

            prop_assert_eq!(metadata, decoded);
        }
    }
}
