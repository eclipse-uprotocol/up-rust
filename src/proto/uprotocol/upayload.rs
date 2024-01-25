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

use mediatype::{names::*, MediaType, Name};
use protobuf::{well_known_types::any::Any, Message};

pub use crate::types::serializationerror::SerializationError;

pub use crate::types::serializationerror::SerializationError;
use crate::uprotocol::{Data, UPayload, UPayloadFormat};

const SUBTYPE_PROTOBUF: Name = Name::new_unchecked("protobuf");
const SUBTYPE_PROTOBUF_WRAPPED: Name = Name::new_unchecked("x-protobuf");
const SUBTYPE_SOMEIP: Name = Name::new_unchecked("x-someip");
const SUBTYPE_SOMEIP_TLV: Name = Name::new_unchecked("x-someip_tlv");

const MEDIA_TYPE_APPLICATION_JSON: MediaType = MediaType::new(APPLICATION, JSON);
const MEDIA_TYPE_APPLICATION_OCTET_STREAM: MediaType = MediaType::new(APPLICATION, OCTET_STREAM);
const MEDIA_TYPE_APPLICATION_PROTOBUF: MediaType = MediaType::new(APPLICATION, SUBTYPE_PROTOBUF);
const MEDIA_TYPE_APPLICATION_PROTOBUF_WRAPPED: MediaType =
    MediaType::new(APPLICATION, SUBTYPE_PROTOBUF_WRAPPED);
const MEDIA_TYPE_APPLICATION_SOMEIP: MediaType = MediaType::new(APPLICATION, SUBTYPE_SOMEIP);
const MEDIA_TYPE_APPLICATION_SOMEIPTLV: MediaType = MediaType::new(APPLICATION, SUBTYPE_SOMEIP_TLV);
const MEDIA_TYPE_TEXT_PLAIN: MediaType = MediaType::new(TEXT, PLAIN);

impl UPayloadFormat {
    /// Gets the payload format that corresponds to a given MIME type.
    ///
    /// # Returns
    ///
    /// The corresponding payload format or None if the MIME type is unsupported.
    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        if let Ok(mime) = MediaType::parse(mime_type) {
            if mime.ty == APPLICATION {
                if mime.subty == JSON {
                    return Some(UPayloadFormat::UpayloadFormatJson);
                }
                if mime.subty == OCTET_STREAM {
                    return Some(UPayloadFormat::UpayloadFormatRaw);
                }
                if mime.subty == SUBTYPE_PROTOBUF {
                    return Some(UPayloadFormat::UpayloadFormatProtobuf);
                }
                if mime.subty == SUBTYPE_PROTOBUF_WRAPPED {
                    return Some(UPayloadFormat::UpayloadFormatProtobufWrappedInAny);
                }
                if mime.subty == SUBTYPE_SOMEIP {
                    return Some(UPayloadFormat::UpayloadFormatSomeip);
                }
                if mime.subty == SUBTYPE_SOMEIP_TLV {
                    return Some(UPayloadFormat::UpayloadFormatSomeipTlv);
                }
            }
            if mime.ty == TEXT && mime.subty == PLAIN {
                return Some(UPayloadFormat::UpayloadFormatText);
            }
        }
        None
    }

    /// Gets the MIME type corresponding to this payload format.
    ///
    /// # Returns
    ///
    /// The corresponding MIME type or None if the payload format is [`UPayloadFormat::UpayloadFormatUnspecified`].
    pub fn to_mime_type(&self) -> Option<String> {
        match self {
            UPayloadFormat::UpayloadFormatJson => Some(MEDIA_TYPE_APPLICATION_JSON.to_string()),
            UPayloadFormat::UpayloadFormatProtobuf => {
                Some(MEDIA_TYPE_APPLICATION_PROTOBUF.to_string())
            }
            UPayloadFormat::UpayloadFormatProtobufWrappedInAny => {
                Some(MEDIA_TYPE_APPLICATION_PROTOBUF_WRAPPED.to_string())
            }
            UPayloadFormat::UpayloadFormatRaw => {
                Some(MEDIA_TYPE_APPLICATION_OCTET_STREAM.to_string())
            }
            UPayloadFormat::UpayloadFormatSomeip => Some(MEDIA_TYPE_APPLICATION_SOMEIP.to_string()),
            UPayloadFormat::UpayloadFormatSomeipTlv => {
                Some(MEDIA_TYPE_APPLICATION_SOMEIPTLV.to_string())
            }
            UPayloadFormat::UpayloadFormatText => Some(MEDIA_TYPE_TEXT_PLAIN.to_string()),
            _ => None,
        }
    }
}

impl TryFrom<Any> for UPayload {
    type Error = SerializationError;
    fn try_from(value: Any) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Any> for UPayload {
    type Error = SerializationError;

    fn try_from(value: &Any) -> Result<Self, Self::Error> {
        let buf = value
            .write_to_bytes()
            .map_err(|_e| SerializationError::new("Failed to serialize Any value"))?;
        i32::try_from(buf.len())
            .map(|len| UPayload {
                data: Some(Data::Value(buf)),
                length: Some(len),
                ..Default::default()
            })
            .map_err(|_e| SerializationError::new("Any object does not fit into UPayload"))
    }
}

impl TryFrom<UPayload> for Any {
    type Error = SerializationError;

    fn try_from(value: UPayload) -> Result<Self, Self::Error> {
        match value.format.enum_value_or_default() {
            UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY
            | UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED => {
                if let Some(bytes) = data_to_slice(&value) {
                    if !bytes.is_empty() {
                        return Any::parse_from_bytes(bytes).map_err(|e| {
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
    use crate::uprotocol::upayload::UPayloadFormat;
    use protobuf::well_known_types::any::Any;
    use protobuf::well_known_types::timestamp::Timestamp;
    use protobuf::EnumOrUnknown;
    use test_case::test_case;

    use super::*;

    #[test_case("application/json", Some(UPayloadFormat::UpayloadFormatJson))]
    #[test_case(
        "application/json; charset=utf-8",
        Some(UPayloadFormat::UpayloadFormatJson)
    )]
    #[test_case("application/protobuf", Some(UPayloadFormat::UpayloadFormatProtobuf))]
    #[test_case(
        "application/x-protobuf",
        Some(UPayloadFormat::UpayloadFormatProtobufWrappedInAny)
    )]
    #[test_case("application/octet-stream", Some(UPayloadFormat::UpayloadFormatRaw))]
    #[test_case("application/x-someip", Some(UPayloadFormat::UpayloadFormatSomeip))]
    #[test_case(
        "application/x-someip_tlv",
        Some(UPayloadFormat::UpayloadFormatSomeipTlv)
    )]
    #[test_case("text/plain", Some(UPayloadFormat::UpayloadFormatText))]
    #[test_case("application/unsupported; foo=bar", None)]
    fn test_from_mime_time(media_type: &str, expected_format: Option<UPayloadFormat>) {
        assert_eq!(UPayloadFormat::from_mime_type(media_type), expected_format);
    }

    #[test_case(0, true; "unspecified succeeds")]
    #[test_case(1, true; "wrapped protobuf succeeds")]
    #[test_case(2, false; "protobuf fails")]
    #[test_case(3, false; "json fails")]
    #[test_case(4, false; "SOME/IP fails")]
    #[test_case(5, false; "SOME/IP TLV fails")]
    #[test_case(6, false; "raw fails")]
    #[test_case(7, false; "text fails")]
    fn test_into_any_with_payload_format(format: i32, should_succeed: bool) {
        let timestamp = Timestamp::default();
        let data = Any::pack(&timestamp).unwrap().write_to_bytes().unwrap();
        let payload = UPayload {
            format: EnumOrUnknown::from_i32(format),
            data: Some(Data::Value(data)),
            length: None,
            ..Default::default()
        };

        let any = Any::try_from(payload);
        assert_eq!(any.is_ok(), should_succeed);
        if should_succeed {
            assert_eq!(
                any.unwrap().unpack::<Timestamp>().unwrap().unwrap(),
                timestamp
            );
        }
    }

    #[test]
    fn test_into_any_fails_for_empty_data() {
        let payload = UPayload {
            format: UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF.into(),
            data: Some(Data::Value(vec![])),
            length: None,
            ..Default::default()
        };

        let any = Any::try_from(payload);
        assert!(any.is_err());
    }

    #[test]
    fn test_from_any() {
        let timestamp = Timestamp::default();
        let any = Any::pack(&timestamp).unwrap();

        let payload = UPayload::try_from(&any).unwrap();
        assert_eq!(
            payload.format,
            UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED.into()
        );
        assert_eq!(
            payload.data.unwrap(),
            Data::Value(any.write_to_bytes().unwrap())
        );
    }
}
