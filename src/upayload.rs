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

use mediatype::MediaType;
use protobuf::{well_known_types::any::Any, EnumFull, Message};

pub use crate::up_core_api::upayload::{UPayload, UPayloadFormat};

#[derive(Debug)]
pub enum UPayloadError {
    SerializationError(String),
    MediatypeProblem,
}

impl UPayloadError {
    pub fn serialization_error<T>(message: T) -> UPayloadError
    where
        T: Into<String>,
    {
        Self::SerializationError(message.into())
    }

    pub fn mediatype_error() -> UPayloadError {
        Self::MediatypeProblem
    }
}

impl std::fmt::Display for UPayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(e) => f.write_fmt(format_args!("Serialization error: {}", e)),
            Self::MediatypeProblem => {
                f.write_fmt(format_args!("Mediatype problem unsupported or malformed"))
            }
        }
    }
}

impl std::error::Error for UPayloadError {}

impl UPayloadFormat {
    /// Gets the payload format that corresponds to a given media type.
    ///
    /// # Errors
    ///
    /// Returns an error if the given string is not a valid media type string or is unsupported by uProtocol.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UPayloadFormat;
    ///
    /// let parse_attempt = UPayloadFormat::from_media_type("application/json; charset=utf-8");
    /// assert!(parse_attempt.is_ok());
    /// assert_eq!(parse_attempt.unwrap(), UPayloadFormat::UPAYLOAD_FORMAT_JSON);
    ///
    /// let parse_attempt = UPayloadFormat::from_media_type("application/unsupported");
    /// assert!(parse_attempt.is_err());
    /// ```
    pub fn from_media_type(media_type_string: &str) -> Result<Self, UPayloadError> {
        if let Ok(media_type) = MediaType::parse(media_type_string) {
            let descriptor = UPayloadFormat::enum_descriptor();
            return descriptor
                .values()
                .find_map(|desc| {
                    let proto_desc = desc.proto();

                    crate::up_core_api::uprotocol_options::exts::mime_type
                        .get(proto_desc.options.get_or_default())
                        .and_then(|mime_type_option_value| {
                            if let Ok(enum_mime_type) = MediaType::parse(&mime_type_option_value) {
                                if enum_mime_type.ty == media_type.ty
                                    && enum_mime_type.subty == media_type.subty
                                {
                                    return desc.cast::<Self>();
                                }
                            }
                            None
                        })
                })
                .ok_or(UPayloadError::mediatype_error());
        }
        Err(UPayloadError::mediatype_error())
    }

    /// Gets the media type corresponding to this payload format.
    ///
    /// # Returns
    ///
    /// None if the payload format is [`UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UPayloadFormat;
    ///
    /// assert_eq!(UPayloadFormat::UPAYLOAD_FORMAT_JSON.to_media_type().unwrap(), "application/json");
    /// assert!(UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED.to_media_type().is_none());
    /// ```
    pub fn to_media_type(self) -> Option<String> {
        let desc = self.descriptor();
        let desc_proto = desc.proto();
        crate::up_core_api::uprotocol_options::exts::mime_type
            .get(desc_proto.options.get_or_default())
    }
}

impl TryFrom<Any> for UPayload {
    type Error = UPayloadError;
    fn try_from(value: Any) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Any> for UPayload {
    type Error = UPayloadError;

    fn try_from(value: &Any) -> Result<Self, Self::Error> {
        value
            .write_to_bytes()
            .map_err(|_e| UPayloadError::serialization_error("Failed to serialize Any value"))
            .map(|data| UPayload {
                data,
                ..Default::default()
            })
    }
}

impl TryFrom<UPayload> for Any {
    type Error = UPayloadError;

    fn try_from(value: UPayload) -> Result<Self, Self::Error> {
        match value.format.enum_value_or_default() {
            UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY
            | UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED => {
                Any::parse_from_bytes(value.data.as_slice()).map_err(|e| {
                    UPayloadError::serialization_error(format!(
                        "UPayload does not contain Any: {}",
                        e
                    ))
                })
            }
            _ => Err(UPayloadError::serialization_error(
                "UPayload has incompatible format",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    use protobuf::well_known_types::{any::Any, timestamp::Timestamp};
    use protobuf::EnumOrUnknown;

    #[test_case("application/json", Ok(UPayloadFormat::UPAYLOAD_FORMAT_JSON); "map from JSON")]
    #[test_case(
        "application/json; charset=utf-8",
        Ok(UPayloadFormat::UPAYLOAD_FORMAT_JSON);
        "map from JSON with parameter"
    )]
    #[test_case("application/protobuf", Ok(UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF); "map from PROTOBUF")]
    #[test_case(
        "application/x-protobuf",
        Ok(UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY); "map from PROTOBUF_WRAPPED"
    )]
    #[test_case("application/octet-stream", Ok(UPayloadFormat::UPAYLOAD_FORMAT_RAW); "map from RAW")]
    #[test_case("application/x-someip", Ok(UPayloadFormat::UPAYLOAD_FORMAT_SOMEIP); "map from SOMEIP")]
    #[test_case(
        "application/x-someip_tlv",
        Ok(UPayloadFormat::UPAYLOAD_FORMAT_SOMEIP_TLV); "map from SOMEIP_TLV"
    )]
    #[test_case("text/plain", Ok(UPayloadFormat::UPAYLOAD_FORMAT_TEXT); "map from TEXT")]
    #[test_case("application/unsupported; foo=bar", Err(UPayloadError::mediatype_error()); "fail for unsupported media type")]
    fn test_from_media_type(
        media_type: &str,
        expected_format: Result<UPayloadFormat, UPayloadError>,
    ) {
        let parsing_result = UPayloadFormat::from_media_type(media_type);
        assert!(parsing_result.is_ok() == expected_format.is_ok());
        if let Ok(format) = expected_format {
            assert_eq!(format, parsing_result.unwrap());
        }
    }

    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_JSON, Some("application/json".to_string()); "map JSON format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF, Some("application/protobuf".to_string()); "map PROTOBUF format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY, Some("application/x-protobuf".to_string()); "map PROTOBUF_WRAPPED format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_RAW, Some("application/octet-stream".to_string()); "map RAW format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_SOMEIP, Some("application/x-someip".to_string()); "map SOMEIP format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_SOMEIP_TLV, Some("application/x-someip_tlv".to_string()); "map SOMEIP_TLV format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_TEXT, Some("text/plain".to_string()); "map TEXT format to media type")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED, None; "map UNSPECIFIED format to None")]
    fn test_to_media_type(format: UPayloadFormat, expected_media_type: Option<String>) {
        assert_eq!(format.to_media_type(), expected_media_type);
    }

    #[test_case(0, true; "UNSPECIFIED succeeds")]
    #[test_case(1, true; "PROTOBUF_WRAPPED succeeds")]
    #[test_case(2, false; "PROTOBUF fails")]
    #[test_case(3, false; "JSON fails")]
    #[test_case(4, false; "SOMEIP fails")]
    #[test_case(5, false; "SOMEIP_TLV fails")]
    #[test_case(6, false; "RAW fails")]
    #[test_case(7, false; "TEXT fails")]
    fn test_into_any_with_payload_format(format: i32, should_succeed: bool) {
        let timestamp = Timestamp::default();
        let data = Any::pack(&timestamp).unwrap().write_to_bytes().unwrap();
        let payload = UPayload {
            format: EnumOrUnknown::from_i32(format),
            data,
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
            data: vec![],
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
        assert_eq!(payload.data, any.write_to_bytes().unwrap());
    }
}
