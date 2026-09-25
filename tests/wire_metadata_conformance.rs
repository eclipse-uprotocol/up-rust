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

//! Native-prefix wire metadata conformance suite over the projection methods.

use bytes::Bytes;
use pretty_assertions::assert_eq;
use test_case::test_case;
use up_rust::{
    NativePrefixFrameMetadataCodec, PayloadEncoding, UFrameMetadata, UMessageBuilder,
    UProtocolNativeWire, UUri, UWire, UWireMetadataCodec, UWireMetadataError, WireIdentity,
};

const LAYOUT_COMPACT_LO: usize = 5;
const LAYOUT_COMPACT_HI: usize = 6;
const VERSION_LO: usize = 7;
const SELECTED_WIRE_COMPACT_LO: usize = 10;
const SELECTED_WIRE_COMPACT_HI: usize = 11;
const PAYLOAD_FAMILY_COMPACT_LO: usize = 13;
const PAYLOAD_FAMILY_COMPACT_HI: usize = 14;
const FLAGS_LO: usize = 15;

struct WrongWire;

impl UWire for WrongWire {
    const WIRE_ID: WireIdentity = WireIdentity::new("org.eclipse.uprotocol.wire.test", 0x7001);
    const PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("native-explicit", 0x0001);
    const METADATA_LAYOUT_ID: WireIdentity =
        WireIdentity::new("org.eclipse.uprotocol.metadata.native-prefix", 0x0001);
    const FORMAT_VERSION: u16 = 1;
}

struct WrongPayloadFamily;

impl UWire for WrongPayloadFamily {
    const WIRE_ID: WireIdentity = WireIdentity::new("org.eclipse.uprotocol.wire.native", 0x0001);
    const PAYLOAD_FAMILY_ID: WireIdentity =
        WireIdentity::new("org.eclipse.uprotocol.payload.test", 0x7002);
    const METADATA_LAYOUT_ID: WireIdentity =
        WireIdentity::new("org.eclipse.uprotocol.metadata.native-prefix", 0x0001);
    const FORMAT_VERSION: u16 = 1;
}

fn topic() -> UUri {
    UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic")
}

fn metadata_without_payload() -> UFrameMetadata {
    let message = UMessageBuilder::publish(topic()).build().expect("message");
    message.to_frame_metadata_unencoded().expect("metadata")
}

fn metadata_with_standard_payload() -> UFrameMetadata {
    let message = UMessageBuilder::publish(topic())
        .build_with_payload(Bytes::from_static(b"payload"), PayloadEncoding::RAW)
        .expect("message");
    message
        .to_frame_metadata(PayloadEncoding::RAW)
        .expect("metadata")
}

fn metadata_with_private_use_payload() -> UFrameMetadata {
    let message = UMessageBuilder::publish(topic()).build().expect("message");
    let encoding = PayloadEncoding::from_id(0xFABC).expect("private-use encoding");
    message
        .attributes()
        .to_frame_metadata(encoding)
        .expect("metadata")
}

fn encode<W>(metadata: &UFrameMetadata) -> Vec<u8>
where
    W: UWire,
{
    NativePrefixFrameMetadataCodec
        .encode_frame_metadata(W::metadata_context(), metadata)
        .expect("encode")
}

fn decode<W>(encoded: &[u8]) -> Result<UFrameMetadata, UWireMetadataError>
where
    W: UWire,
{
    NativePrefixFrameMetadataCodec.decode_frame_metadata(W::metadata_context(), encoded)
}

fn encoded_without_payload() -> Vec<u8> {
    encode::<UProtocolNativeWire>(&metadata_without_payload())
}

fn set_byte(bytes: &mut [u8], offset: usize, value: u8) {
    *bytes.get_mut(offset).expect("offset in test vector") = value;
}

#[test]
fn identity_constants_match_first_wave_register() {
    assert_eq!(
        UProtocolNativeWire::WIRE_ID.literal_id(),
        "org.eclipse.uprotocol.wire.native"
    );
    assert_eq!(UProtocolNativeWire::WIRE_ID.compact_id(), 0x0001);
    assert_eq!(
        UProtocolNativeWire::PAYLOAD_FAMILY_ID.literal_id(),
        "native-explicit"
    );
    assert_eq!(UProtocolNativeWire::PAYLOAD_FAMILY_ID.compact_id(), 0x0001);
    assert_eq!(
        UProtocolNativeWire::METADATA_LAYOUT_ID.literal_id(),
        "org.eclipse.uprotocol.metadata.native-prefix"
    );
    assert_eq!(UProtocolNativeWire::METADATA_LAYOUT_ID.compact_id(), 0x0001);
    assert_eq!(UProtocolNativeWire::FORMAT_VERSION, 1);
}

#[test]
fn no_payload_metadata_round_trips() {
    let metadata = metadata_without_payload();

    let encoded = encode::<UProtocolNativeWire>(&metadata);
    let decoded = decode::<UProtocolNativeWire>(&encoded).expect("decode");

    assert_eq!(decoded, metadata);
    assert!(decoded.payload_encoding().is_none());
}

#[test]
fn standard_payload_metadata_round_trips() {
    let metadata = metadata_with_standard_payload();

    let encoded = encode::<UProtocolNativeWire>(&metadata);
    let decoded = decode::<UProtocolNativeWire>(&encoded).expect("decode");

    assert_eq!(decoded, metadata);
    assert_eq!(decoded.payload_encoding(), Some(&PayloadEncoding::RAW));
}

#[test]
fn private_use_payload_encoding_is_preserved() {
    let metadata = metadata_with_private_use_payload();

    let encoded = encode::<UProtocolNativeWire>(&metadata);
    let decoded = decode::<UProtocolNativeWire>(&encoded).expect("decode");

    assert_eq!(decoded, metadata);
    assert_eq!(
        decoded.payload_encoding(),
        Some(&PayloadEncoding::from_id(0xFABC).expect("private-use encoding"))
    );
}

#[test]
fn wrong_magic_is_rejected() {
    let mut encoded = encoded_without_payload();
    set_byte(&mut encoded, 0, b'X');

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert_eq!(error, UWireMetadataError::WrongMagic);
}

#[test]
fn unknown_metadata_layout_is_rejected() {
    let mut encoded = encoded_without_payload();
    set_byte(&mut encoded, LAYOUT_COMPACT_LO, 0xFF);
    set_byte(&mut encoded, LAYOUT_COMPACT_HI, 0xFF);

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert!(matches!(
        error,
        UWireMetadataError::UnknownMetadataLayoutId { .. }
    ));
}

#[test]
fn unsupported_version_is_rejected() {
    let mut encoded = encoded_without_payload();
    set_byte(&mut encoded, VERSION_LO, 0x02);

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert!(matches!(
        error,
        UWireMetadataError::UnsupportedVersion { actual: 2, .. }
    ));
}

#[test]
fn wrong_selected_wire_id_is_rejected() {
    let encoded = encoded_without_payload();

    let error = decode::<WrongWire>(&encoded).unwrap_err();

    assert!(matches!(
        error,
        UWireMetadataError::WrongWireMetadata { .. }
    ));
}

#[test]
fn unknown_selected_wire_id_is_rejected() {
    let mut encoded = encoded_without_payload();
    set_byte(&mut encoded, SELECTED_WIRE_COMPACT_LO, 0xFF);
    set_byte(&mut encoded, SELECTED_WIRE_COMPACT_HI, 0xFF);

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert!(matches!(
        error,
        UWireMetadataError::WrongWireMetadata { .. }
    ));
}

#[test]
fn payload_family_mismatch_is_rejected() {
    let encoded = encoded_without_payload();

    let error = decode::<WrongPayloadFamily>(&encoded).unwrap_err();

    assert!(matches!(
        error,
        UWireMetadataError::PayloadFamilyMismatch { .. }
    ));
}

#[test]
fn unknown_payload_family_id_is_rejected() {
    let mut encoded = encoded_without_payload();
    set_byte(&mut encoded, PAYLOAD_FAMILY_COMPACT_LO, 0xFF);
    set_byte(&mut encoded, PAYLOAD_FAMILY_COMPACT_HI, 0xFF);

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert!(matches!(
        error,
        UWireMetadataError::PayloadFamilyMismatch { .. }
    ));
}

#[test]
fn reserved_flags_are_rejected() {
    let mut encoded = encoded_without_payload();
    set_byte(&mut encoded, FLAGS_LO, 0x01);

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert_eq!(error, UWireMetadataError::UnsupportedReservedFlags(1));
}

#[test]
fn malformed_length_is_rejected() {
    let mut encoded = encoded_without_payload();
    encoded.truncate(encoded.len().saturating_sub(2));

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert!(matches!(error, UWireMetadataError::MalformedMetadata(_)));
}

#[test_case(0 => matches Ok(0); "explicit zero is preserved")]
#[test_case(8 => matches Ok(8); "unassigned is preserved")]
#[test_case(0xEFFF => matches Ok(0xEFFF); "reserved is preserved")]
#[test_case(0xFFFF => matches Ok(0xFFFF); "private is preserved")]
#[test_case(65536 => matches Err(UWireMetadataError::MalformedMetadata(_)); "above sixteen bits")]
#[test_case(u32::MAX => matches Err(UWireMetadataError::MalformedMetadata(_)); "maximum storage integer")]
fn identifier_range_is_checked_without_losing_presence(id: u32) -> Result<u32, UWireMetadataError> {
    let metadata = metadata_with_standard_payload();
    let mut encoded = encode::<UProtocolNativeWire>(&metadata);
    let last = encoded
        .len()
        .checked_sub(4)
        .expect("standard payload bytes");
    encoded
        .get_mut(last..)
        .expect("encoding slot")
        .copy_from_slice(&id.to_le_bytes());
    let decoded = decode::<UProtocolNativeWire>(&encoded)?;
    Ok(decoded
        .payload_encoding()
        .expect("encoding presence retained")
        .id())
}

#[test]
fn trailing_bytes_are_rejected() {
    let mut encoded = encoded_without_payload();
    encoded.push(0x00);

    let error = decode::<UProtocolNativeWire>(&encoded).unwrap_err();

    assert!(matches!(error, UWireMetadataError::MalformedMetadata(_)));
}

#[test]
fn metadata_size_budget_cases_are_recordable() {
    let no_payload = encode::<UProtocolNativeWire>(&metadata_without_payload());
    let standard = encode::<UProtocolNativeWire>(&metadata_with_standard_payload());
    let private_use = encode::<UProtocolNativeWire>(&metadata_with_private_use_payload());
    let full_attributes = standard.clone();

    assert!(!no_payload.is_empty());
    assert!(standard.len() >= no_payload.len());
    assert!(private_use.len() > no_payload.len());
    assert_eq!(full_attributes.len(), standard.len());
}

#[test]
fn canonical_profile_identity_is_distinct_and_round_trips() {
    use up_rust::UFRAME_FIELDS_METADATA_LAYOUT_ID;
    assert_ne!(
        UFRAME_FIELDS_METADATA_LAYOUT_ID,
        up_rust::NATIVE_PREFIX_METADATA_LAYOUT_ID
    );
    let metadata = metadata_with_private_use_payload();
    let encoded = NativePrefixFrameMetadataCodec
        .encode_frame_metadata(UProtocolNativeWire::metadata_context(), &metadata)
        .expect("canonical encode");
    // The on-wire layout compact id names the canonical profile.
    let compact_lo = *encoded.get(LAYOUT_COMPACT_LO).expect("compact id low byte");
    let compact_hi = *encoded
        .get(LAYOUT_COMPACT_HI)
        .expect("compact id high byte");
    let compact = u16::from_le_bytes([compact_lo, compact_hi]);
    assert_eq!(compact, UFRAME_FIELDS_METADATA_LAYOUT_ID.compact_id());
    let decoded = NativePrefixFrameMetadataCodec
        .decode_frame_metadata(UProtocolNativeWire::metadata_context(), &encoded)
        .expect("canonical decode");
    assert_eq!(decoded, metadata);
}

#[test]
fn private_use_encoding_rides_message_surface_losslessly() {
    let encoding = PayloadEncoding::from_id(0xFC0D).expect("private-use encoding");
    let message = UMessageBuilder::publish(topic())
        .build_with_payload(Bytes::from_static(b"cdr-bytes"), encoding)
        .expect("message");
    let attributes = message.attributes();

    assert_eq!(attributes.payload_encoding(), Some(encoding));

    use up_rust::ProtobufMappable as _;
    let bytes = attributes.write_to_protobuf_bytes().expect("serialize");
    let restored = up_rust::UAttributes::parse_from_protobuf_bytes(&bytes).expect("deserialize");
    assert_eq!(restored.payload_encoding(), attributes.payload_encoding());

    let metadata = message.to_frame_metadata(encoding).expect("projection");
    assert_eq!(metadata.payload_encoding(), Some(&encoding));
    let back = metadata.try_project_to_attributes().expect("projection");
    assert_eq!(back.payload_encoding(), attributes.payload_encoding());
}

#[test]
fn registered_encodings_survive_classic_projection() {
    let metadata = metadata_with_standard_payload();
    let attributes = metadata.try_project_to_attributes().expect("projection");

    assert_eq!(attributes.payload_encoding(), Some(PayloadEncoding::RAW));
}
