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

//! Golden byte vectors for the UFrame metadata envelope and metadata profiles.
//!
//! These vectors are the cross-language conformance seed: a non-Rust
//! implementation of the canonical UFrame field-block profile (and, where
//! applicable, the legacy protobuf profile) should reproduce these bytes
//! exactly for the documented fixtures, and decode them back to the documented
//! semantics. Fixtures are fully deterministic — message ids are fixed valid
//! v7 UUIDs, never generated.
//!
//! Default mode COMPARES against the checked-in vectors under
//! `tests/golden/wire-metadata/` and fails on any drift (an intentional wire
//! change must regenerate the vectors in the same commit and say so).
//! Set `UP_GOLDEN_UPDATE=1` to regenerate.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use bytes::Bytes;
use up_rust::{
    NativePrefixFrameMetadataCodec, PayloadEncoding, ProtobufMappable, UAttributes, UFrameMetadata,
    UMessageBuilder, UPayloadFormat, UProtocolNativeWire, UUri, UWire, UWireMetadataCodec, UUID,
};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/wire-metadata")
}

fn update_mode() -> bool {
    std::env::var_os("UP_GOLDEN_UPDATE").is_some_and(|v| v == "1")
}

/// Fixed, valid v7/RFC4122 UUIDs (version nibble 0x7 in msb bits 12..=15,
/// variant 0b10 in lsb bits 62..=63). Deterministic by construction.
fn fixed_uuid(seq: u64) -> UUID {
    UUID::from_u64_pair(
        0x0000_018d_0000_7000 | (seq << 16),
        0x8000_0000_0000_0100 | seq,
    )
    .expect("fixed v7 uuid")
}

fn topic() -> UUri {
    UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic")
}

fn method() -> UUri {
    UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x0007).expect("method")
}

fn reply_to() -> UUri {
    UUri::try_from_parts("cloud", 0x0042, 0x01, 0x0000).expect("reply-to")
}

fn project(
    message: &up_rust::UMessage,
    payload_encoding: Option<PayloadEncoding>,
) -> UFrameMetadata {
    up_rust::frame::metadata::try_project_attributes_to_frame_metadata(
        message.attributes(),
        payload_encoding,
    )
    .expect("metadata")
}

/// The deterministic fixture set. Names are stable file names; extend by
/// appending, never by renaming.
fn fixtures() -> Vec<(&'static str, &'static str, UFrameMetadata)> {
    let publish_no_payload = {
        let m = UMessageBuilder::publish(topic())
            .with_message_id(fixed_uuid(1))
            .build()
            .expect("message");
        project(&m, None)
    };
    let publish_raw_payload = {
        let m = UMessageBuilder::publish(topic())
            .with_message_id(fixed_uuid(2))
            .build_with_payload(Bytes::from_static(b"payload"), UPayloadFormat::Raw)
            .expect("message");
        project(&m, Some(PayloadEncoding::RAW))
    };
    let publish_custom_payload = {
        let m = UMessageBuilder::publish(topic())
            .with_message_id(fixed_uuid(3))
            .build()
            .expect("message");
        project(
            &m,
            Some(
                PayloadEncoding::custom("com.example.native", "application/vnd.example.native")
                    .expect("custom encoding"),
            ),
        )
    };
    let notification = {
        let m = UMessageBuilder::notification(topic(), reply_to())
            .with_message_id(fixed_uuid(4))
            .build()
            .expect("message");
        project(&m, None)
    };
    let request = {
        let m = UMessageBuilder::request(method(), reply_to(), 5000)
            .with_message_id(fixed_uuid(5))
            .build()
            .expect("message");
        project(&m, None)
    };
    let response = {
        let m = UMessageBuilder::response(reply_to(), fixed_uuid(5), method())
            .with_message_id(fixed_uuid(6))
            .build()
            .expect("message");
        project(&m, None)
    };
    vec![
        (
            "publish-no-payload",
            "publish; no payload encoding; source only",
            publish_no_payload,
        ),
        (
            "publish-raw-payload",
            "publish; standard RAW payload encoding",
            publish_raw_payload,
        ),
        (
            "publish-custom-payload",
            "publish; custom payload encoding (name + media type)",
            publish_custom_payload,
        ),
        (
            "notification",
            "notification; source + entity sink (resource 0)",
            notification,
        ),
        (
            "request",
            "rpc request; method sink, reply-to source, ttl=5000",
            request,
        ),
        (
            "response",
            "rpc response; reqid = request fixture id",
            response,
        ),
    ]
}

fn classic_attribute_fixtures() -> Vec<(&'static str, &'static str, UAttributes)> {
    let legacy_raw = UMessageBuilder::publish(topic())
        .with_message_id(fixed_uuid(7))
        .build_with_payload(Bytes::from_static(b"payload"), UPayloadFormat::Raw)
        .expect("message")
        .attributes()
        .clone();
    let open_xcdr2 = UMessageBuilder::publish(topic())
        .with_message_id(fixed_uuid(8))
        .build_with_payload_encoding(
            Bytes::from_static(b"cdr-bytes"),
            PayloadEncoding::custom("up.xcdr-v2", "application/vnd.uprotocol.xcdr-v2")
                .expect("encoding"),
        )
        .expect("message")
        .attributes()
        .clone();

    vec![
        (
            "classic-publish-raw-payload",
            "classic UAttributes; legacy RAW payload_format",
            legacy_raw,
        ),
        (
            "classic-publish-open-xcdr2-payload",
            "classic UAttributes; open XCDRv2 payload encoding fields",
            open_xcdr2,
        ),
    ]
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        write!(s, "{b:02x}").expect("hex write");
    }
    s.push('\n');
    s
}

fn check_or_update(name: &str, profile: &str, bytes: &[u8], manifest: &mut String) {
    let dir = golden_dir();
    let path = dir.join(format!("{name}.{profile}.hex"));
    let encoded = hex(bytes);
    writeln!(
        manifest,
        "  {{\"fixture\": \"{name}\", \"profile\": \"{profile}\", \"file\": \"{name}.{profile}.hex\", \"len\": {}}},",
        bytes.len()
    )
    .expect("manifest write");
    if update_mode() {
        fs::create_dir_all(&dir).expect("golden dir");
        fs::write(&path, &encoded).expect("write golden");
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden vector {}; run with UP_GOLDEN_UPDATE=1 to generate",
            path.display()
        )
    });
    assert_eq!(
        expected, encoded,
        "golden vector drift for {name} ({profile}); if the wire change is \
         intentional, regenerate with UP_GOLDEN_UPDATE=1 and commit the new \
         vectors together with the change"
    );
}

#[test]
fn canonical_field_block_golden_vectors() {
    let mut manifest = String::from("[\n");
    for (name, _desc, metadata) in fixtures() {
        let bytes = NativePrefixFrameMetadataCodec
            .encode_frame_metadata(UProtocolNativeWire::metadata_context(), &metadata)
            .expect("canonical encode");
        // Every golden vector must decode back to the exact fixture semantics.
        let decoded = NativePrefixFrameMetadataCodec
            .decode_frame_metadata(UProtocolNativeWire::metadata_context(), &bytes)
            .expect("canonical decode");
        assert_eq!(decoded, metadata, "canonical round-trip for {name}");
        check_or_update(name, "uframe-fields", &bytes, &mut manifest);
    }
    finish_manifest("uframe-fields", manifest);
}

#[test]
fn classic_uattributes_golden_vectors() {
    let mut manifest = String::from("[\n");
    for (name, _desc, attributes) in classic_attribute_fixtures() {
        let bytes = attributes
            .write_to_protobuf_bytes()
            .expect("classic encode");
        let decoded = UAttributes::parse_from_protobuf_bytes(&bytes).expect("classic decode");
        assert_eq!(decoded, attributes, "classic round-trip for {name}");
        check_or_update(name, "uattributes-classic", &bytes, &mut manifest);
    }
    finish_manifest("uattributes-classic", manifest);
}

fn finish_manifest(profile: &str, mut manifest: String) {
    // Trim the trailing comma for valid JSON and close the array.
    if manifest.ends_with(",\n") {
        manifest.truncate(manifest.len() - 2);
        manifest.push('\n');
    }
    manifest.push_str("]\n");
    let path = golden_dir().join(format!("manifest.{profile}.json"));
    if update_mode() {
        fs::create_dir_all(golden_dir()).expect("golden dir");
        fs::write(&path, &manifest).expect("write manifest");
    } else if let Ok(existing) = fs::read_to_string(&path) {
        assert_eq!(existing, manifest, "golden manifest drift for {profile}");
    }
}
