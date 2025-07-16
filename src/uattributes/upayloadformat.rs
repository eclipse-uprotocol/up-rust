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

use crate::up_core_api::uattributes::UPayloadFormat;
use mediatype::MediaType;
use protobuf::EnumFull;

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
            Self::SerializationError(e) => f.write_fmt(format_args!("Serialization error: {e}")),
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

                    crate::up_core_api::uoptions::exts::mime_type
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
        crate::up_core_api::uoptions::exts::mime_type.get(desc_proto.options.get_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

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
}
