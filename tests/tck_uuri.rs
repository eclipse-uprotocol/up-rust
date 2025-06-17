/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
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

use std::str::FromStr;

use cucumber::{given, then, when};
use protobuf::Message;
use up_rust::UUri;

mod common;

const FEATURES_GLOB_PATTERN: &str = "up-spec/basics/uuri_*.feature";

fn value_as_u8(value: String) -> u8 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u8::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

fn value_as_u16(value: String) -> u16 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u16::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

fn value_as_u32(value: String) -> u32 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u32::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

fn trimhex(s: &str) -> &str {
    s.strip_prefix("0x")
        .unwrap_or(s.strip_prefix("0X").unwrap_or(s))
}

#[derive(cucumber::World, Default, Debug)]
struct UUriWorld {
    uuri: UUri,
    uri: String,
    protobuf: Vec<u8>,
    error: Option<Box<dyn std::error::Error>>,
}

#[given(expr = "a URI string {word}")]
async fn with_uri_string(w: &mut UUriWorld, uri_string: String) {
    w.uri = uri_string;
}

#[given(expr = "a UUri having authority {string}")]
async fn with_authority(w: &mut UUriWorld, authority_name: String) {
    w.uuri.authority_name = authority_name;
}

#[given(expr = "having entity identifier {word}")]
async fn with_entity_id(w: &mut UUriWorld, entity_id: String) {
    w.uuri.ue_id = value_as_u32(entity_id);
}

#[given(expr = "having major version {word}")]
async fn with_major_version(w: &mut UUriWorld, major_version: String) {
    w.uuri.ue_version_major = value_as_u32(major_version);
}

#[given(expr = "having resource identifier {word}")]
async fn with_resource_id(w: &mut UUriWorld, resource_id: String) {
    w.uuri.resource_id = value_as_u32(resource_id);
}

#[when(expr = "serializing the UUri to a URI")]
async fn serialize_to_uri(w: &mut UUriWorld) {
    w.uri = w.uuri.to_uri(true);
}

#[when(expr = "serializing the UUri to its protobuf wire format")]
async fn serialize_to_protobuf(w: &mut UUriWorld) {
    w.protobuf = w
        .uuri
        .write_to_bytes()
        .expect("failed to serialize UUri to protobuf");
}

#[when(expr = "deserializing the URI to a UUri")]
async fn deserialize_from_uri(w: &mut UUriWorld) {
    match UUri::from_str(w.uri.as_str()) {
        Ok(uuri) => {
            w.uuri = uuri;
        }
        Err(e) => {
            w.error = Some(Box::from(e));
        }
    }
}

#[then(expr = "the resulting URI string is {word}")]
async fn assert_uri_string(w: &mut UUriWorld, expected_uri: String) {
    assert_eq!(w.uri, expected_uri);
}

#[then(expr = "the UUri has authority {string}")]
async fn assert_authority(w: &mut UUriWorld, value: String) {
    assert_eq!(w.uuri.authority_name(), value);
}

#[then(expr = "has entity identifier {word}")]
async fn assert_entity_id(w: &mut UUriWorld, entity_id: String) {
    assert_eq!(w.uuri.ue_id, value_as_u32(entity_id));
}

#[then(expr = "has major version {word}")]
async fn assert_major_version(w: &mut UUriWorld, major_version: String) {
    assert_eq!(w.uuri.uentity_major_version(), value_as_u8(major_version));
}

#[then(expr = "has resource identifier {word}")]
async fn assert_resource_id(w: &mut UUriWorld, resource_id: String) {
    assert_eq!(w.uuri.resource_id(), value_as_u16(resource_id));
}

#[then(expr = "the attempt fails")]
async fn assert_failure(w: &mut UUriWorld) {
    assert!(w.error.is_some());
}

#[then(expr = "the original UUri can be recreated from the protobuf wire format")]
async fn assert_original_uuri_can_be_recreated_from_protobuf(w: &mut UUriWorld) {
    assert!(UUri::parse_from_bytes(&w.protobuf).is_ok_and(|uuri| w.uuri.eq(&uuri)));
}

#[then(expr = "the same UUri can be deserialized from {word}")]
async fn assert_uuri_can_be_deserialized_from_bytes(w: &mut UUriWorld, hex_string: String) {
    let buf = hex::decode(trimhex(&hex_string)).expect("not a valid hex string");
    assert!(UUri::parse_from_bytes(buf.as_slice()).is_ok_and(|uuri| w.uuri.eq(&uuri)));
}

#[then(expr = "the original UUri can be recreated from the URI string")]
async fn assert_original_uuri_can_be_recreated_from_uri_string(w: &mut UUriWorld) {
    assert!(w.uri.parse::<UUri>().is_ok_and(|uuri| w.uuri.eq(&uuri)));
}

#[then(expr = "the UUri matches pattern {word}")]
async fn assert_uuri_matches_pattern(w: &mut UUriWorld, pattern: String) {
    assert!(pattern.parse::<UUri>().is_ok_and(|p| p.matches(&w.uuri)));
}

#[then(expr = "the UUri does not match pattern {word}")]
async fn assert_uuri_does_not_match_pattern(w: &mut UUriWorld, pattern: String) {
    assert!(pattern.parse::<UUri>().is_ok_and(|p| !p.matches(&w.uuri)));
}

// [utest->req~uri-data-model-proto~1]
// [utest->dsn~uri-pattern-matching~2]
// [utest->req~uri-serialization~1]
// [utest->dsn~uri-scheme~1]
// [utest->dsn~uri-host-only~2]
// [utest->dsn~uri-authority-mapping~1]
// [utest->dsn~uri-path-mapping~1]
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    common::run::<UUriWorld>(FEATURES_GLOB_PATTERN, "uuri").await
}
