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

use common::CustomTckOpts;
use cucumber::{cli, given, then, when, writer, World};
use protobuf::Message;
use up_rust::UUID;

mod common;

const FEATURES_PATH: &str = "features/uuid";

fn value_as_u64(value: String) -> u64 {
    if value.starts_with("0x") || value.starts_with("0X") {
        u64::from_str_radix(trimhex(&value), 16).expect("not a hex number")
    } else {
        value.parse().expect("not an integer number")
    }
}

fn trimhex(s: &str) -> &str {
    s.strip_prefix("0x")
        .unwrap_or(s.strip_prefix("0X").unwrap_or(s))
}

#[derive(cucumber::World, Default, Debug)]
struct UUIDWorld {
    uuid: UUID,
    hyphenated_string: String,
    protobuf: Vec<u8>,
    error: Option<Box<dyn std::error::Error>>,
}

#[given(expr = "a UUID string representation {word}")]
async fn with_hyphenated_string(w: &mut UUIDWorld, hyphenated_string: String) {
    w.hyphenated_string = hyphenated_string;
}

#[given(expr = "a UUID having MSB {word} and LSB {word}")]
async fn with_msb_lsb(w: &mut UUIDWorld, msb_hex_string: String, lsb_hex_string: String) {
    w.uuid.msb = value_as_u64(msb_hex_string);
    w.uuid.lsb = value_as_u64(lsb_hex_string);
}

#[when(expr = "serializing the UUID to a hyphenated string")]
async fn serialize_to_hyphenated_string(w: &mut UUIDWorld) {
    w.hyphenated_string = w.uuid.to_hyphenated_string();
}

#[when(expr = "serializing the UUID to its protobuf wire format")]
async fn serialize_to_protobuf(w: &mut UUIDWorld) {
    w.protobuf = w
        .uuid
        .write_to_bytes()
        .expect("failed to serialize UUID to protobuf");
}

#[when(expr = "deserializing the hyphenated string to a UUID")]
async fn deserialize_from_hyphenated_string(w: &mut UUIDWorld) {
    match UUID::from_str(&w.hyphenated_string) {
        Ok(uuid) => {
            w.uuid = uuid;
        }
        Err(e) => {
            w.error = Some(Box::from(e));
        }
    }
}

#[then(expr = "the attempt fails")]
async fn assert_failure(w: &mut UUIDWorld) {
    assert!(w.error.is_some());
}

#[then(expr = "the resulting hyphenated string is {word}")]
async fn assert_hyphenated_string(w: &mut UUIDWorld, expected_string: String) {
    assert_eq!(w.hyphenated_string, expected_string);
}

#[then(expr = "the original UUID can be recreated from the hyphenated string")]
async fn assert_original_uuid_can_be_recreated_from_hyphenated_string(w: &mut UUIDWorld) {
    assert!(w
        .hyphenated_string
        .parse::<UUID>()
        .is_ok_and(|uuid| w.uuid.eq(&uuid)));
}

#[then(expr = "the original UUID can be recreated from the protobuf wire format")]
async fn assert_original_uuid_can_be_recreated_from_protobuf(w: &mut UUIDWorld) {
    assert!(UUID::parse_from_bytes(&w.protobuf).is_ok_and(|uuid| w.uuid.eq(&uuid)));
}

#[then(expr = "the same UUID can be deserialized from {word}")]
async fn assert_deserialize_uuid_from_protobuf(w: &mut UUIDWorld, hex_string: String) {
    let buf = hex::decode(trimhex(&hex_string)).expect("not a valid hex string");
    assert!(UUID::parse_from_bytes(buf.as_slice()).is_ok_and(|uuid| w.uuid.eq(&uuid)));
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let opts = cli::Opts::<_, _, _, CustomTckOpts>::parsed();
    if let Some(file) = opts.custom.get_junit_out_file("uuid") {
        UUIDWorld::cucumber()
            .with_cli(opts)
            .with_writer(cucumber::writer::JUnit::new(file, 0))
            .run(FEATURES_PATH)
            .await;
    } else {
        UUIDWorld::cucumber()
            .with_cli(opts)
            .with_writer(writer::Basic::stdout())
            .run(FEATURES_PATH)
            .await;
    }
    Ok(())
}
