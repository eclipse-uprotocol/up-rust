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

#![cfg_attr(not(feature = "wire-implementer-api"), allow(dead_code))]

//! Selected-wire identities, payload codecs, and metadata profiles.
//!
//! Wire authors define a [`UWire`] marker and stable [`WireIdentity`], associate
//! payload types through [`UWirePayload`], and implement the codec traits the
//! payload family requires. Physical carriage does not belong here. The
//! adapter in `wire_transport` composes a wire and metadata codec over any
//! compatible encoded core.
//!
//! Receive checks identity before profile decoding. A mismatch is an explicit
//! unknown-identity error, never fallback to another decoder
//! (`req~selected-wire-explicit-rejection~1`). The conformance and golden tests
//! under `tests/wire_metadata_*` plus external `up-wire-xcdrv2-rust` are the
//! implementation references.

#[cfg(any(feature = "protobuf-support", feature = "up-core-api"))]
use std::io::Read;
#[cfg(feature = "selected-wire-codec-core")]
use std::{error::Error, fmt::Display};

#[cfg(feature = "protobuf-support")]
use crate::payload::codec::{PayloadFormat, PayloadLayout, ProtobufPayload};
#[cfg(feature = "protobuf-support")]
use crate::payload::UWireError;
use crate::payload::{
    codec::{DecodePayload, EncodePayload, PayloadCodec, ReadDecodePayload},
    stable::{StableContainerPayload, StablePayload},
};
#[cfg(any(feature = "protobuf-support", feature = "up-core-api"))]
use crate::PayloadEncoding;
#[cfg(feature = "protobuf-support")]
use crate::ProtobufMappable;
#[cfg(feature = "selected-wire-codec-core")]
use crate::{UCode, UFrameMetadata, UFrameMetadataError, UStatus};

#[cfg(feature = "selected-wire-codec-core")]
const NATIVE_PREFIX_MAGIC: &[u8; 4] = b"UPWM";
#[cfg(feature = "selected-wire-codec-core")]
const ID_REF_COMPACT: u8 = 0x00;
#[cfg(feature = "selected-wire-codec-core")]
const ID_REF_LITERAL: u8 = 0x01;

/// First supported native-prefix metadata byte-format version.
pub const FORMAT_VERSION: u16 = 1;

/// Compact ID reserved for invalid-ID fixtures.
pub const INVALID_ID_FIXTURE_COMPACT_ID: u16 = 0xFFFF;

/// First compact ID reserved for local or experimental future tools.
pub const LOCAL_EXPERIMENTAL_COMPACT_ID_START: u16 = 0x8000;

/// Last compact ID reserved for local or experimental future tools.
pub const LOCAL_EXPERIMENTAL_COMPACT_ID_END: u16 = 0xFFFE;

/// Identity for the first-wave uProtocol native selected wire.
pub const UPROTOCOL_NATIVE_WIRE_ID: WireIdentity =
    WireIdentity::new("org.eclipse.uprotocol.wire.native", 0x0001);

/// Payload-family identity for explicit native payload bytes.
pub const NATIVE_EXPLICIT_PAYLOAD_FAMILY_ID: WireIdentity =
    WireIdentity::new("native-explicit", 0x0001);

/// Identity for the first-wave Protocol Buffers selected wire.
pub const PROTOBUF_WIRE_ID: WireIdentity =
    WireIdentity::new("org.eclipse.uprotocol.wire.protobuf", 0x0002);

/// Payload-family identity for Protocol Buffers application payload bytes.
pub const PROTOBUF_PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("protobuf", 0x0002);

/// Identity reserved for the external first-wave XCDRv2 selected wire.
///
/// The production `XcdrV2Wire` type is intentionally owned by the external
/// `up-wire-xcdrv2-rust` crate. `up-rust` exposes only the shared identity.
pub const XCDR_V2_WIRE_ID: WireIdentity =
    WireIdentity::new("org.eclipse.uprotocol.wire.xcdr-v2", 0x0003);

/// Payload-family identity reserved for external XCDRv2 payload bytes.
pub const XCDR_V2_PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("xcdr-v2", 0x0003);

/// Identity for the first-wave stable-container selected wire.
pub const STABLE_CONTAINER_WIRE_ID: WireIdentity =
    WireIdentity::new("org.eclipse.uprotocol.wire.stable-container", 0x0004);

/// Payload-family identity for stable-container payloads.
pub const STABLE_CONTAINER_PAYLOAD_FAMILY_ID: WireIdentity =
    WireIdentity::new("stable-container", 0x0004);

/// Identity for the first-wave native-prefix metadata layout.
pub const NATIVE_PREFIX_METADATA_LAYOUT_ID: WireIdentity =
    WireIdentity::new("org.eclipse.uprotocol.metadata.native-prefix", 0x0001);

/// Metadata layout identity for the canonical UFrame metadata field block
/// carried behind native-prefix framing.
///
/// Distinct from [`NATIVE_PREFIX_METADATA_LAYOUT_ID`] (the legacy
/// protobuf-`UAttributes` block) so that canonical and legacy metadata are
/// selectable, rejectable profiles: decoding bytes of one profile with the
/// other codec fails as `UWireMetadataError::UnknownMetadataLayoutId`,
/// never as generic malformed metadata.
pub const UFRAME_FIELDS_METADATA_LAYOUT_ID: WireIdentity =
    WireIdentity::new("org.eclipse.uprotocol.metadata.uframe-fields", 0x0002);

/// Stable wire, payload-family, or metadata-layout identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireIdentity {
    literal_id: &'static str,
    compact_id: u16,
}

impl WireIdentity {
    /// Creates an identity from its literal and compact representations.
    #[must_use]
    pub const fn new(literal_id: &'static str, compact_id: u16) -> Self {
        Self {
            literal_id,
            compact_id,
        }
    }

    /// Returns the language-neutral literal identity.
    #[must_use]
    pub fn literal_id(self) -> &'static str {
        self.literal_id
    }

    /// Returns the compact first-wave identity.
    #[must_use]
    pub fn compact_id(self) -> u16 {
        self.compact_id
    }
}

/// Identity reference decoded from native-prefix metadata bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireIdentityRef {
    /// Compact `u16` identity reference.
    Compact(u16),
    /// Literal UTF-8 identity reference.
    Literal(String),
}

impl WireIdentityRef {
    /// Returns whether this reference identifies `expected`.
    #[must_use]
    pub fn matches(&self, expected: WireIdentity) -> bool {
        match self {
            Self::Compact(actual) => *actual == expected.compact_id(),
            Self::Literal(actual) => actual == expected.literal_id(),
        }
    }
}

/// Compatibility decision between decoded metadata and a selected wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireCompatibility {
    /// Decoded metadata is compatible with the selected wire.
    Compatible,
    /// Decoded metadata is incompatible with the selected wire.
    Incompatible,
}

impl WireCompatibility {
    /// Returns true when the compatibility result is compatible.
    #[must_use]
    pub fn is_compatible(self) -> bool {
        matches!(self, Self::Compatible)
    }
}

/// Static selected-wire contract.
pub trait UWire {
    /// Stable identity for the full selected wire.
    const WIRE_ID: WireIdentity;
    /// Payload operation family supported by this selected wire.
    const PAYLOAD_FAMILY_ID: WireIdentity;
    /// Metadata byte layout used by this selected wire.
    const METADATA_LAYOUT_ID: WireIdentity;
    /// Metadata byte-format version supported by this selected wire.
    const FORMAT_VERSION: u16;

    /// Checks selected-wire compatibility before public frame exposure.
    fn wire_compatibility(actual: &WireIdentityRef) -> WireCompatibility {
        if actual.matches(Self::WIRE_ID) {
            WireCompatibility::Compatible
        } else {
            WireCompatibility::Incompatible
        }
    }

    /// Checks payload-family compatibility before public frame exposure.
    fn payload_family_compatibility(actual: &WireIdentityRef) -> WireCompatibility {
        if actual.matches(Self::PAYLOAD_FAMILY_ID) {
            WireCompatibility::Compatible
        } else {
            WireCompatibility::Incompatible
        }
    }

    /// Returns the selected-wire metadata context consumed by metadata codecs.
    #[cfg(feature = "selected-wire-codec-core")]
    fn metadata_context() -> UWireMetadataContext
    where
        Self: Sized,
    {
        UWireMetadataContext::from_wire::<Self>()
    }
}

/// Selected-wire metadata context consumed by metadata codecs.
#[cfg(feature = "selected-wire-codec-core")]
#[derive(Clone, Copy, Debug)]
pub struct UWireMetadataContext {
    /// Stable identity for the full selected wire.
    pub wire_id: WireIdentity,
    /// Payload operation family supported by this selected wire.
    pub payload_family_id: WireIdentity,
    /// Metadata byte layout used by this selected wire.
    pub metadata_layout_id: WireIdentity,
    /// Metadata byte-format version supported by this selected wire.
    pub format_version: u16,
    wire_compatibility: Option<fn(&WireIdentityRef) -> WireCompatibility>,
    payload_family_compatibility: Option<fn(&WireIdentityRef) -> WireCompatibility>,
}

#[cfg(feature = "selected-wire-codec-core")]
impl UWireMetadataContext {
    /// Builds a metadata context for a selected wire marker type.
    #[must_use]
    pub fn from_wire<W>() -> Self
    where
        W: UWire,
    {
        Self {
            wire_id: W::WIRE_ID,
            payload_family_id: W::PAYLOAD_FAMILY_ID,
            metadata_layout_id: W::METADATA_LAYOUT_ID,
            format_version: W::FORMAT_VERSION,
            wire_compatibility: Some(W::wire_compatibility),
            payload_family_compatibility: Some(W::payload_family_compatibility),
        }
    }

    /// Builds a metadata context with exact identity matching.
    #[must_use]
    pub fn new_exact(
        wire_id: WireIdentity,
        payload_family_id: WireIdentity,
        metadata_layout_id: WireIdentity,
        format_version: u16,
    ) -> Self {
        Self {
            wire_id,
            payload_family_id,
            metadata_layout_id,
            format_version,
            wire_compatibility: None,
            payload_family_compatibility: None,
        }
    }

    /// Returns this context with the metadata layout identity replaced.
    ///
    /// Metadata codecs overlay their own profile identity so the layout id on
    /// the wire always names the codec's byte profile, independent of the
    /// legacy per-wire declaration.
    #[must_use]
    pub fn with_metadata_layout(mut self, metadata_layout_id: WireIdentity) -> Self {
        self.metadata_layout_id = metadata_layout_id;
        self
    }

    /// Checks selected-wire compatibility before public frame exposure.
    #[must_use]
    pub fn wire_compatibility(&self, actual: &WireIdentityRef) -> WireCompatibility {
        if let Some(wire_compatibility) = self.wire_compatibility {
            return wire_compatibility(actual);
        }
        if actual.matches(self.wire_id) {
            WireCompatibility::Compatible
        } else {
            WireCompatibility::Incompatible
        }
    }

    /// Checks payload-family compatibility before public frame exposure.
    #[must_use]
    pub fn payload_family_compatibility(&self, actual: &WireIdentityRef) -> WireCompatibility {
        if let Some(payload_family_compatibility) = self.payload_family_compatibility {
            return payload_family_compatibility(actual);
        }
        if actual.matches(self.payload_family_id) {
            WireCompatibility::Compatible
        } else {
            WireCompatibility::Incompatible
        }
    }
}

/// Default first-wave native wire for explicit native payload paths.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UProtocolNativeWire;

impl UWire for UProtocolNativeWire {
    const WIRE_ID: WireIdentity = UPROTOCOL_NATIVE_WIRE_ID;
    const PAYLOAD_FAMILY_ID: WireIdentity = NATIVE_EXPLICIT_PAYLOAD_FAMILY_ID;
    const METADATA_LAYOUT_ID: WireIdentity = NATIVE_PREFIX_METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = FORMAT_VERSION;
}

/// Selected wire for Protocol Buffers application payloads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProtobufWire;

impl UWire for ProtobufWire {
    const WIRE_ID: WireIdentity = PROTOBUF_WIRE_ID;
    const PAYLOAD_FAMILY_ID: WireIdentity = PROTOBUF_PAYLOAD_FAMILY_ID;
    const METADATA_LAYOUT_ID: WireIdentity = NATIVE_PREFIX_METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = FORMAT_VERSION;
}

#[cfg(feature = "protobuf-support")]
impl PayloadFormat for ProtobufWire {
    fn name() -> &'static str {
        ProtobufPayload::name()
    }

    fn encoding() -> PayloadEncoding {
        ProtobufPayload::encoding()
    }
}

#[cfg(feature = "protobuf-support")]
impl<T> EncodePayload<T> for ProtobufWire
where
    T: ProtobufMappable,
{
    fn payload_layout(value: &T) -> Result<PayloadLayout, UWireError> {
        ProtobufPayload::payload_layout(value)
    }

    fn encode_payload(value: &T, dst: &mut [u8]) -> Result<(), UWireError> {
        ProtobufPayload::encode_payload(value, dst)
    }
}

#[cfg(feature = "protobuf-support")]
impl<'a, T> DecodePayload<'a, T> for ProtobufWire
where
    T: ProtobufMappable,
{
    fn decode_payload(src: &'a [u8]) -> Result<T, UWireError> {
        ProtobufPayload::decode_payload(src)
    }
}

#[cfg(feature = "protobuf-support")]
impl<T> ReadDecodePayload<T> for ProtobufWire
where
    T: ProtobufMappable,
{
    fn decode_payload_from_reader<R: Read>(reader: R, payload_len: usize) -> Result<T, UWireError> {
        ProtobufPayload::decode_payload_from_reader(reader, payload_len)
    }
}

/// Selected wire for stable-container payloads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StableContainerWireFormat;

impl UWire for StableContainerWireFormat {
    const WIRE_ID: WireIdentity = STABLE_CONTAINER_WIRE_ID;
    const PAYLOAD_FAMILY_ID: WireIdentity = STABLE_CONTAINER_PAYLOAD_FAMILY_ID;
    const METADATA_LAYOUT_ID: WireIdentity = NATIVE_PREFIX_METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = FORMAT_VERSION;
}

/// Explicit metadata codec used by selected-wire transport adapters.
#[cfg(feature = "selected-wire-codec-core")]
pub trait UWireMetadataCodec {
    /// Metadata byte-profile identity this codec writes and accepts.
    ///
    /// Defaults to the legacy native-prefix protobuf layout for compatibility
    /// with pre-R2 external codecs; profile codecs override it. Codecs MUST
    /// overlay this identity onto the context they encode/decode with so that
    /// cross-profile decode fails as
    /// [`UWireMetadataError::UnknownMetadataLayoutId`].
    const METADATA_LAYOUT_ID: WireIdentity = NATIVE_PREFIX_METADATA_LAYOUT_ID;

    /// Encodes native frame metadata for the selected-wire context.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata is invalid or cannot be serialized.
    fn encode_frame_metadata(
        &self,
        context: UWireMetadataContext,
        metadata: &UFrameMetadata,
    ) -> Result<Vec<u8>, UWireMetadataError>;

    /// Decodes and validates native frame metadata for the selected-wire context.
    ///
    /// # Errors
    ///
    /// Returns an error when bytes are malformed, use an unsupported layout or
    /// version, or declare an incompatible wire or payload family.
    fn decode_frame_metadata(
        &self,
        context: UWireMetadataContext,
        src: &[u8],
    ) -> Result<UFrameMetadata, UWireMetadataError>;
}

/// Metadata codec that is statically compatible with selected wire `W`.
///
/// This marker keeps the ordinary metadata codec contract reusable while
/// allowing adapter constructors to reject invalid wire/codec pairings at
/// compile time.
#[cfg(feature = "selected-wire-codec-core")]
pub trait UWireMetadataCodecFor<W>: UWireMetadataCodec
where
    W: UWire,
{
}

/// Canonical native-prefix metadata codec carrying the clean UFrame metadata
/// field block.
///
/// This codec serializes [`UFrameMetadata`] directly using the canonical
/// field block defined in [`crate::frame::codec`] — no protobuf, no legacy
/// `UAttributes` projection, full fidelity for open payload encodings. The
/// outer native-prefix framing (magic, metadata layout id, format version,
/// wire id, payload family id) is identical to the legacy codec so that
/// transports can carry either block behind the same placement layouts.
#[cfg(feature = "selected-wire-codec-core")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativePrefixFrameMetadataCodec;

#[cfg(feature = "selected-wire-codec-core")]
impl UWireMetadataCodec for NativePrefixFrameMetadataCodec {
    const METADATA_LAYOUT_ID: WireIdentity = UFRAME_FIELDS_METADATA_LAYOUT_ID;

    fn encode_frame_metadata(
        &self,
        context: UWireMetadataContext,
        metadata: &UFrameMetadata,
    ) -> Result<Vec<u8>, UWireMetadataError> {
        let context = context.with_metadata_layout(Self::METADATA_LAYOUT_ID);
        let fields = crate::frame::codec::encode_frame_metadata_fields(metadata)
            .map_err(|error| UWireMetadataError::FrameMetadata(error.to_string()))?;
        let mut out = Vec::with_capacity(
            NATIVE_PREFIX_MAGIC.len()
                + (3 * (1 + std::mem::size_of::<u16>()))
                + (2 * std::mem::size_of::<u16>())
                + std::mem::size_of::<u32>()
                + fields.len(),
        );
        out.extend_from_slice(NATIVE_PREFIX_MAGIC);
        write_identity_ref(&mut out, context.metadata_layout_id);
        write_u16(&mut out, context.format_version);
        write_identity_ref(&mut out, context.wire_id);
        write_identity_ref(&mut out, context.payload_family_id);
        write_u16(&mut out, 0);
        write_len_prefixed_bytes(&mut out, &fields)?;
        Ok(out)
    }

    fn decode_frame_metadata(
        &self,
        context: UWireMetadataContext,
        src: &[u8],
    ) -> Result<UFrameMetadata, UWireMetadataError> {
        let context = context.with_metadata_layout(Self::METADATA_LAYOUT_ID);
        let mut reader = MetadataReader::new(src);
        read_and_check_native_prefix(&mut reader, context)?;
        let fields = reader.read_len_prefixed_bytes()?;
        reader.finish()?;
        crate::frame::codec::decode_frame_metadata_fields(fields)
            .map_err(|error| UWireMetadataError::MalformedMetadata(error.to_string()))
    }
}

#[cfg(feature = "selected-wire-codec-core")]
impl<W> UWireMetadataCodecFor<W> for NativePrefixFrameMetadataCodec where W: UWire {}

/// Wire-level encode helper alias for existing payload codecs.
pub trait UWireEncode<T: ?Sized>: EncodePayload<T> {}

impl<C, T: ?Sized> UWireEncode<T> for C where C: EncodePayload<T> {}

/// Wire-level borrowed decode helper alias for existing payload codecs.
pub trait UWireDecode<'a, T>: DecodePayload<'a, T> {}

impl<'a, C, T> UWireDecode<'a, T> for C where C: DecodePayload<'a, T> {}

/// Wire-level owned decode helper alias for existing payload codecs.
pub trait UWireDecodeOwned<T>: for<'a> DecodePayload<'a, T> {}

impl<C, T> UWireDecodeOwned<T> for C where C: for<'a> DecodePayload<'a, T> {}

/// Wire-level reader decode helper alias for existing payload codecs.
pub trait UWireReadDecode<T>: ReadDecodePayload<T> {}

impl<C, T> UWireReadDecode<T> for C where C: ReadDecodePayload<T> {}

/// Selected-wire payload mapping for typed payloads.
///
/// The wire marker, not the payload type, chooses one payload codec for `T`.
/// The associated codec need not be the wire marker itself. Its implemented
/// capability traits decide which operations are available: encode/decode,
/// initialized TX loan, uninitialized TX loan, and receive-side typed borrow.
/// Implementing this mapping does not grant capabilities the codec does not
/// implement and does not make [`ReadDecodePayload`] a borrowed or in-place
/// decode API.
pub trait UWirePayload<T>: UWire {
    /// Concrete payload codec used by this selected wire for `T`.
    type Codec: PayloadCodec;
}

impl<T> UWirePayload<T> for StableContainerWireFormat
where
    T: StablePayload,
{
    type Codec = StableContainerPayload<T>;
}

#[cfg(feature = "protobuf-support")]
impl<T> UWirePayload<T> for ProtobufWire
where
    T: ProtobufMappable,
{
    type Codec = ProtobufPayload;
}

/// Errors returned by native-prefix selected-wire metadata handling.
#[cfg(feature = "selected-wire-codec-core")]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UWireMetadataError {
    /// The input did not start with the native-prefix metadata magic bytes.
    WrongMagic,
    /// The metadata layout id was not native-prefix.
    UnknownMetadataLayoutId {
        /// The metadata layout id actually present in the input.
        actual: WireIdentityRef,
    },
    /// The metadata layout id that was actually present.
    /// The metadata version is unsupported by the selected wire.
    UnsupportedVersion {
        /// The version this codec supports.
        expected: u16,
        /// The version present in the input.
        actual: u16,
    },
    /// The selected-wire id is incompatible with `W`.
    WrongWireMetadata {
        /// The selected wire's identity.
        expected: WireIdentity,
        /// The wire identity present in the input.
        actual: WireIdentityRef,
    },
    /// The payload-family id is incompatible with `W`.
    PayloadFamilyMismatch {
        /// The selected wire's payload-family identity.
        expected: WireIdentity,
        /// The payload-family identity present in the input.
        actual: WireIdentityRef,
    },
    /// The reserved flags field contained unsupported bits.
    UnsupportedReservedFlags(u16),
    /// The payload encoding block is unsupported or malformed.
    UnsupportedPayloadEncoding(String),
    /// The metadata bytes are malformed.
    MalformedMetadata(String),
    /// Frame metadata validation failed.
    FrameMetadata(String),
    /// Metadata serialization or parsing failed.
    SerializationError(String),
}

#[cfg(feature = "selected-wire-codec-core")]
impl Display for UWireMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongMagic => f.write_str("wrong native-prefix metadata magic"),
            Self::UnknownMetadataLayoutId { actual } => {
                write!(f, "unknown metadata layout id: {actual:?}")
            }
            Self::UnsupportedVersion { expected, actual } => write!(
                f,
                "unsupported metadata version: expected {expected}, got {actual}"
            ),
            Self::WrongWireMetadata { expected, actual } => write!(
                f,
                "wrong selected wire metadata: expected {expected:?}, got {actual:?}"
            ),
            Self::PayloadFamilyMismatch { expected, actual } => write!(
                f,
                "payload family mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::UnsupportedReservedFlags(flags) => {
                write!(f, "unsupported native-prefix reserved flags: {flags:#06x}")
            }
            Self::UnsupportedPayloadEncoding(message) => {
                write!(f, "unsupported payload encoding: {message}")
            }
            Self::MalformedMetadata(message) => write!(f, "malformed wire metadata: {message}"),
            Self::FrameMetadata(message) => write!(f, "invalid frame metadata: {message}"),
            Self::SerializationError(message) => {
                write!(f, "metadata serialization error: {message}")
            }
        }
    }
}

#[cfg(feature = "selected-wire-codec-core")]
impl Error for UWireMetadataError {}

#[cfg(feature = "selected-wire-codec-core")]
impl From<UFrameMetadataError> for UWireMetadataError {
    fn from(value: UFrameMetadataError) -> Self {
        Self::FrameMetadata(value.to_string())
    }
}

#[cfg(feature = "selected-wire-codec-core")]
impl From<UWireMetadataError> for UStatus {
    fn from(value: UWireMetadataError) -> Self {
        UStatus::fail_with_code(UCode::InvalidArgument, value.to_string())
    }
}

/// Reads and validates the shared native-prefix framing: magic, metadata
/// layout id, format version, wire id, payload family id, and reserved flags.
#[cfg(feature = "selected-wire-codec-core")]
fn read_and_check_native_prefix(
    reader: &mut MetadataReader<'_>,
    context: UWireMetadataContext,
) -> Result<(), UWireMetadataError> {
    if reader.take(NATIVE_PREFIX_MAGIC.len())? != NATIVE_PREFIX_MAGIC.as_slice() {
        return Err(UWireMetadataError::WrongMagic);
    }

    let layout_id = reader.read_identity_ref()?;
    if !layout_id.matches(context.metadata_layout_id) {
        return Err(UWireMetadataError::UnknownMetadataLayoutId { actual: layout_id });
    }

    let version = reader.read_u16()?;
    if version != context.format_version {
        return Err(UWireMetadataError::UnsupportedVersion {
            expected: context.format_version,
            actual: version,
        });
    }

    let wire_id = reader.read_identity_ref()?;
    if !context.wire_compatibility(&wire_id).is_compatible() {
        return Err(UWireMetadataError::WrongWireMetadata {
            expected: context.wire_id,
            actual: wire_id,
        });
    }

    let payload_family_id = reader.read_identity_ref()?;
    if !context
        .payload_family_compatibility(&payload_family_id)
        .is_compatible()
    {
        return Err(UWireMetadataError::PayloadFamilyMismatch {
            expected: context.payload_family_id,
            actual: payload_family_id,
        });
    }

    let flags = reader.read_u16()?;
    if flags != 0 {
        return Err(UWireMetadataError::UnsupportedReservedFlags(flags));
    }
    Ok(())
}

#[cfg(feature = "selected-wire-codec-core")]
fn write_identity_ref(out: &mut Vec<u8>, identity: WireIdentity) {
    out.push(ID_REF_COMPACT);
    write_u16(out, identity.compact_id());
}

#[cfg(feature = "selected-wire-codec-core")]
fn write_len_prefixed_bytes(out: &mut Vec<u8>, value: &[u8]) -> Result<(), UWireMetadataError> {
    let len = u32::try_from(value.len()).map_err(|_| {
        UWireMetadataError::MalformedMetadata(format!(
            "length {} exceeds u32 native-prefix limit",
            value.len()
        ))
    })?;
    write_u32(out, len);
    out.extend_from_slice(value);
    Ok(())
}

#[cfg(feature = "selected-wire-codec-core")]
fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(feature = "selected-wire-codec-core")]
fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(feature = "selected-wire-codec-core")]
struct MetadataReader<'a> {
    src: &'a [u8],
    pos: usize,
}

#[cfg(feature = "selected-wire-codec-core")]
impl<'a> MetadataReader<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    fn finish(&self) -> Result<(), UWireMetadataError> {
        if self.pos == self.src.len() {
            Ok(())
        } else {
            Err(UWireMetadataError::MalformedMetadata(format!(
                "{} trailing byte(s)",
                self.src.len() - self.pos
            )))
        }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], UWireMetadataError> {
        let end = self.pos.checked_add(len).ok_or_else(|| {
            UWireMetadataError::MalformedMetadata("metadata length overflow".to_string())
        })?;
        let chunk = self.src.get(self.pos..end).ok_or_else(|| {
            UWireMetadataError::MalformedMetadata(format!(
                "needed {len} byte(s) at offset {}, input has {} byte(s)",
                self.pos,
                self.src.len()
            ))
        })?;
        self.pos = end;
        Ok(chunk)
    }

    fn read_u8(&mut self) -> Result<u8, UWireMetadataError> {
        let bytes = self.take(1)?;
        bytes
            .first()
            .copied()
            .ok_or_else(|| UWireMetadataError::MalformedMetadata("missing u8 field".to_string()))
    }

    fn read_u16(&mut self) -> Result<u16, UWireMetadataError> {
        let bytes = self.take(2)?;
        let array = <[u8; 2]>::try_from(bytes)
            .map_err(|_| UWireMetadataError::MalformedMetadata("invalid u16 field".to_string()))?;
        Ok(u16::from_le_bytes(array))
    }

    fn read_u32(&mut self) -> Result<u32, UWireMetadataError> {
        let bytes = self.take(4)?;
        let array = <[u8; 4]>::try_from(bytes)
            .map_err(|_| UWireMetadataError::MalformedMetadata("invalid u32 field".to_string()))?;
        Ok(u32::from_le_bytes(array))
    }

    fn read_identity_ref(&mut self) -> Result<WireIdentityRef, UWireMetadataError> {
        match self.read_u8()? {
            ID_REF_COMPACT => Ok(WireIdentityRef::Compact(self.read_u16()?)),
            ID_REF_LITERAL => {
                let bytes = self.read_len_prefixed_bytes()?;
                let value = std::str::from_utf8(bytes).map_err(|error| {
                    UWireMetadataError::MalformedMetadata(format!(
                        "identity literal is not UTF-8: {error}"
                    ))
                })?;
                Ok(WireIdentityRef::Literal(value.to_string()))
            }
            tag => Err(UWireMetadataError::MalformedMetadata(format!(
                "unknown identity reference tag {tag}"
            ))),
        }
    }

    fn read_len_prefixed_bytes(&mut self) -> Result<&'a [u8], UWireMetadataError> {
        let len = usize::try_from(self.read_u32()?).map_err(|_| {
            UWireMetadataError::MalformedMetadata("length prefix does not fit usize".to_string())
        })?;
        self.take(len)
    }
}

/// Binds payload types to a wire whose marker is its own codec
/// (`Codec = Self`), removing the most repetitive line of an all-in-one
/// wire implementation.
///
/// ```rust,no_run
/// # #[cfg(feature = "wire-implementer-api")]
/// # fn main() {
/// # use up_rust::{bind_wire_self_codec, DecodePayload, EncodePayload,
/// #     PayloadCodec, PayloadEncoding, PayloadLayout, UWire, UWireError,
/// #     WireIdentity};
/// # struct MyWire;
/// # struct A(u32); struct B(u32);
/// # impl UWire for MyWire {
/// #     const WIRE_ID: WireIdentity = WireIdentity::new("demo.self-codec", 0x8043);
/// #     const PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("demo.self", 0x8043);
/// #     const METADATA_LAYOUT_ID: WireIdentity =
/// #         WireIdentity::new("uprotocol.native-prefix.v1", 0x0001);
/// #     const FORMAT_VERSION: u16 = 1;
/// # }
/// # impl PayloadCodec for MyWire {
/// #     fn codec_name() -> &'static str { "demo-self-codec" }
/// #     fn payload_encoding() -> PayloadEncoding { PayloadEncoding::RAW }
/// # }
/// # impl EncodePayload<A> for MyWire {
/// #     fn payload_layout(_: &A) -> Result<PayloadLayout, UWireError> { PayloadLayout::new(4, 4) }
/// #     fn encode_payload(v: &A, dst: &mut [u8]) -> Result<(), UWireError> {
/// #         dst.copy_from_slice(&v.0.to_le_bytes());
/// #         Ok(())
/// #     }
/// # }
/// # impl<'a> DecodePayload<'a, A> for MyWire {
/// #     fn decode_payload(src: &'a [u8]) -> Result<A, UWireError> {
/// #         Ok(A(u32::from_le_bytes(src.try_into().map_err(|_| UWireError::invalid_payload("demo payload must be 4 bytes"))?)))
/// #     }
/// # }
/// # impl EncodePayload<B> for MyWire {
/// #     fn payload_layout(_: &B) -> Result<PayloadLayout, UWireError> { PayloadLayout::new(4, 4) }
/// #     fn encode_payload(v: &B, dst: &mut [u8]) -> Result<(), UWireError> {
/// #         dst.copy_from_slice(&v.0.to_le_bytes());
/// #         Ok(())
/// #     }
/// # }
/// # impl<'a> DecodePayload<'a, B> for MyWire {
/// #     fn decode_payload(src: &'a [u8]) -> Result<B, UWireError> {
/// #         Ok(B(u32::from_le_bytes(src.try_into().map_err(|_| UWireError::invalid_payload("demo payload must be 4 bytes"))?)))
/// #     }
/// # }
/// // One wire identity, two payload types, marker as codec for both:
/// bind_wire_self_codec!(MyWire: A, B);
/// # }
/// # #[cfg(not(feature = "wire-implementer-api"))]
/// # fn main() {}
/// ```
///
/// Why not a trait default or a blanket impl? `type Codec = Self;` as an
/// associated-type default is unstable Rust, and a blanket
/// `impl<W, T> UWirePayload<T> for W` would forbid, by coherence, exactly the
/// wire that binds a *different* codec for some payload type — the case the
/// three-trait split exists to support. The macro adds the convenience without
/// closing that door.
#[macro_export]
macro_rules! bind_wire_self_codec {
    ($wire:ty : $($payload:ty),+ $(,)?) => {
        $(
            impl $crate::UWirePayload<$payload> for $wire {
                type Codec = $wire;
            }
        )+
    };
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "protobuf-support")]
    use protobuf::well_known_types::wrappers::StringValue;

    use super::*;
    use crate::payload::{
        codec::PayloadCodec,
        loan::{BorrowPayload, LoanPayload},
    };
    use crate::test_support::StableTestBytes as WireStableBytes;

    fn assert_wire_payload<T, W>()
    where
        W: UWirePayload<T>,
        <W as UWirePayload<T>>::Codec: LoanPayload<T> + BorrowPayload<T> + PayloadCodec,
    {
    }

    #[cfg(feature = "zero-copy-transport")]
    fn assert_wire_uninit_payload<T, W>()
    where
        W: UWirePayload<T>,
        <W as UWirePayload<T>>::Codec: crate::payload::loan::LoanUninitPayload<T>,
    {
    }

    #[test]
    fn first_wave_wire_identity_constants_match_register() {
        assert_eq!(
            PROTOBUF_WIRE_ID.literal_id(),
            "org.eclipse.uprotocol.wire.protobuf"
        );
        assert_eq!(PROTOBUF_WIRE_ID.compact_id(), 0x0002);
        assert_eq!(PROTOBUF_PAYLOAD_FAMILY_ID.literal_id(), "protobuf");
        assert_eq!(PROTOBUF_PAYLOAD_FAMILY_ID.compact_id(), 0x0002);
        assert_eq!(
            XCDR_V2_WIRE_ID.literal_id(),
            "org.eclipse.uprotocol.wire.xcdr-v2"
        );
        assert_eq!(XCDR_V2_WIRE_ID.compact_id(), 0x0003);
        assert_eq!(XCDR_V2_PAYLOAD_FAMILY_ID.literal_id(), "xcdr-v2");
        assert_eq!(XCDR_V2_PAYLOAD_FAMILY_ID.compact_id(), 0x0003);
        assert_eq!(
            STABLE_CONTAINER_WIRE_ID.literal_id(),
            "org.eclipse.uprotocol.wire.stable-container"
        );
        assert_eq!(STABLE_CONTAINER_WIRE_ID.compact_id(), 0x0004);
        assert_eq!(
            STABLE_CONTAINER_PAYLOAD_FAMILY_ID.literal_id(),
            "stable-container"
        );
        assert_eq!(STABLE_CONTAINER_PAYLOAD_FAMILY_ID.compact_id(), 0x0004);
    }

    #[cfg(feature = "protobuf-support")]
    #[test]
    fn protobuf_wire_delegates_application_payload_codec() {
        let value = StringValue {
            value: "wire".to_string(),
            special_fields: Default::default(),
        };

        let encoded = ProtobufWire::encode_payload_owned(&value).expect("encode protobuf wire");
        let decoded: StringValue =
            ProtobufWire::decode_payload(&encoded).expect("decode protobuf wire");
        assert_eq!(decoded.value, "wire");
        assert_eq!(
            ProtobufWire::payload_encoding(),
            ProtobufPayload::payload_encoding()
        );
    }

    #[test]
    fn stable_container_wire_maps_type_to_capable_codec() {
        assert_wire_payload::<WireStableBytes, StableContainerWireFormat>();
        #[cfg(feature = "zero-copy-transport")]
        assert_wire_uninit_payload::<WireStableBytes, StableContainerWireFormat>();

        let encoding =
            <StableContainerWireFormat as UWirePayload<WireStableBytes>>::Codec::payload_encoding();
        let expected = StableContainerPayload::<WireStableBytes>::payload_encoding();
        assert_eq!(encoding, expected);
    }
}
