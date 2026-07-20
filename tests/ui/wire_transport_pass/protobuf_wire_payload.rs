/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::io::Cursor;

use protobuf::well_known_types::wrappers::StringValue;
use up_rust::{
    DecodePayload, EncodePayload, PayloadCodec, ProtobufWire, ReadDecodePayload, UWire,
    UWireReadDecode,
};

fn assert_wire<W: UWire>() {}

fn assert_read_decode<T, W>()
where
    W: UWireReadDecode<T>,
{
}

fn main() {
    assert_wire::<ProtobufWire>();
    assert_read_decode::<StringValue, ProtobufWire>();

    let value = StringValue {
        value: "protobuf-wire".to_string(),
        special_fields: Default::default(),
    };
    let encoded = ProtobufWire::encode_payload_owned(&value).expect("encode protobuf wire");
    let decoded: StringValue = ProtobufWire::decode_payload(&encoded).expect("decode protobuf wire");
    let streamed: StringValue =
        ProtobufWire::decode_payload_from_reader(Cursor::new(&encoded), encoded.len())
            .expect("stream decode protobuf wire");

    assert_eq!(decoded.value, "protobuf-wire");
    assert_eq!(streamed.value, "protobuf-wire");
    assert_eq!(ProtobufWire::codec_name(), "protobuf");
}
