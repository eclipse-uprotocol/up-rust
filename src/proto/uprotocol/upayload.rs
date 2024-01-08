/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
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

use prost::Message;
use prost_types::Any;

use crate::{
    cloudevent::serializer::SerializationError,
    uprotocol::{Data, UPayload, UPayloadFormat},
};

impl TryFrom<Any> for UPayload {
    type Error = SerializationError;
    fn try_from(value: Any) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Any> for UPayload {
    type Error = SerializationError;

    fn try_from(value: &Any) -> Result<Self, Self::Error> {
        let vec = value.encode_to_vec();
        i32::try_from(vec.len())
            .map(|len| UPayload {
                data: Some(Data::Value(vec)),
                length: Some(len),
                ..Default::default()
            })
            .map_err(|_e| SerializationError::new("Any object does not fit into UPayload"))
    }
}

impl TryFrom<UPayload> for Any {
    type Error = SerializationError;

    fn try_from(value: UPayload) -> Result<Self, Self::Error> {
        match value.format() {
            UPayloadFormat::UpayloadFormatProtobuf | UPayloadFormat::UpayloadFormatUnspecified => {
                if let Some(bytes) = data_to_slice(&value) {
                    if !bytes.is_empty() {
                        return Any::decode(bytes).map_err(|e| {
                            SerializationError::new(format!("UPayload does not contain Any: {}", e))
                        });
                    }
                }
                Err(SerializationError::new(
                    "UPayload does not contain any data",
                ))
            }
            _ => Err(SerializationError::new("UPayload has incompatible format")),
        }
    }
}

fn data_to_slice(payload: &UPayload) -> Option<&[u8]> {
    if let Some(data) = &payload.data {
        match data {
            Data::Reference(bytes) => {
                if let Some(length) = payload.length {
                    return Some(unsafe { read_memory(*bytes, length) });
                }
            }
            Data::Value(bytes) => {
                return Some(bytes.as_slice());
            }
        }
    }
    None
}

// Please no one use this...
unsafe fn read_memory(_address: u64, _length: i32) -> &'static [u8] {
    // Convert the raw address to a pointer
    // let ptr = address as *const u8;
    // Create a slice from the pointer and the length
    // slice::from_raw_parts(ptr, length as usize)

    todo!("This is not implemented yet")
}

#[cfg(test)]
mod tests {
    use crate::uprotocol::UPayloadFormat;
    use prost_types::{Any, Timestamp};
    use test_case::test_case;

    use super::*;

    #[test_case(0, true; "unspecified succeeds")]
    #[test_case(1, true; "protobuf succeeds")]
    #[test_case(2, false; "json fails")]
    #[test_case(3, false; "SOME/IP fails")]
    #[test_case(4, false; "SOME/IP TLV fails")]
    #[test_case(5, false; "raw fails")]
    #[test_case(6, false; "text fails")]
    fn test_into_any_with_payload_format(format: i32, should_succeed: bool) {
        let timestamp = Timestamp::default();
        let data = Any::from_msg(&timestamp).unwrap().encode_to_vec();
        let payload = UPayload {
            format,
            data: Some(Data::Value(data)),
            length: None,
        };

        let any = Any::try_from(payload);
        assert_eq!(any.is_ok(), should_succeed);
        if should_succeed {
            assert_eq!(any.unwrap().to_msg::<Timestamp>().unwrap(), timestamp);
        }
    }

    #[test]
    fn test_into_any_fails_for_empty_data() {
        let payload = UPayload {
            format: UPayloadFormat::UpayloadFormatProtobuf as i32,
            data: Some(Data::Value(vec![])),
            length: None,
        };

        let any = Any::try_from(payload);
        assert!(any.is_err());
    }

    #[test]
    fn test_from_any() {
        let timestamp = Timestamp::default();
        let any = Any::from_msg(&timestamp).unwrap();

        let payload = UPayload::try_from(&any).unwrap();
        assert_eq!(
            payload.format,
            UPayloadFormat::UpayloadFormatUnspecified as i32
        );
        assert_eq!(payload.data.unwrap(), Data::Value(any.encode_to_vec()));
    }
}
