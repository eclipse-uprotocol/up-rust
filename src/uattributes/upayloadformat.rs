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

use crate::SerializationError;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum UPayloadFormat {
    Unspecified = 0,
    ProtobufWrappedInAny = 1,
    Protobuf = 2,
    Json = 3,
    Someip = 4,
    SomeipTlv = 5,
    Raw = 6,
    Text = 7,
    Shm = 8,
}

impl UPayloadFormat {
    /// Returns the integer value of the payload format as defined in the protobuf enum.
    #[must_use]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    pub fn try_from_i32(value: i32) -> Result<UPayloadFormat, SerializationError> {
        match value {
            x if x == UPayloadFormat::Unspecified as i32 => Ok(UPayloadFormat::Unspecified),
            x if x == UPayloadFormat::ProtobufWrappedInAny as i32 => {
                Ok(UPayloadFormat::ProtobufWrappedInAny)
            }
            x if x == UPayloadFormat::Protobuf as i32 => Ok(UPayloadFormat::Protobuf),
            x if x == UPayloadFormat::Json as i32 => Ok(UPayloadFormat::Json),
            x if x == UPayloadFormat::Someip as i32 => Ok(UPayloadFormat::Someip),
            x if x == UPayloadFormat::SomeipTlv as i32 => Ok(UPayloadFormat::SomeipTlv),
            x if x == UPayloadFormat::Raw as i32 => Ok(UPayloadFormat::Raw),
            x if x == UPayloadFormat::Text as i32 => Ok(UPayloadFormat::Text),
            x if x == UPayloadFormat::Shm as i32 => Ok(UPayloadFormat::Shm),
            _ => Err(SerializationError::new(format!(
                "unknown payload format code {value}"
            ))),
        }
    }
}

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
    /// assert!(parse_attempt.is_ok_and(|f| f == UPayloadFormat::Json));
    ///
    /// let parse_attempt = UPayloadFormat::from_media_type("application/unsupported");
    /// assert!(parse_attempt.is_err());
    /// ```
    pub fn from_media_type(media_type_string: &str) -> Result<Self, UPayloadError> {
        if let Ok(media_type) = MediaType::parse(media_type_string) {
            match (media_type.ty.as_str(), media_type.subty.as_str()) {
                ("application", "json") => return Ok(UPayloadFormat::Json),
                ("application", "protobuf") => return Ok(UPayloadFormat::Protobuf),
                ("application", "x-protobuf") => return Ok(UPayloadFormat::ProtobufWrappedInAny),
                ("application", "octet-stream") => return Ok(UPayloadFormat::Raw),
                ("application", "x-someip") => return Ok(UPayloadFormat::Someip),
                ("application", "x-someip_tlv") => return Ok(UPayloadFormat::SomeipTlv),
                ("text", "plain") => return Ok(UPayloadFormat::Text),
                ("application", "x-shm") => return Ok(UPayloadFormat::Shm),
                _ => return Err(UPayloadError::mediatype_error()),
            }
        }
        Err(UPayloadError::mediatype_error())
    }

    /// Gets the media type corresponding to this payload format.
    ///
    /// # Returns
    ///
    /// None if the payload format is [`UPayloadFormat::Unspecified`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UPayloadFormat;
    ///
    /// assert_eq!(UPayloadFormat::Json.to_media_type().unwrap(), "application/json");
    /// assert!(UPayloadFormat::Unspecified.to_media_type().is_none());
    /// ```
    #[must_use]
    pub fn to_media_type(self) -> Option<String> {
        match self {
            UPayloadFormat::Unspecified => None,
            UPayloadFormat::ProtobufWrappedInAny => Some("application/x-protobuf".to_string()),
            UPayloadFormat::Protobuf => Some("application/protobuf".to_string()),
            UPayloadFormat::Json => Some("application/json".to_string()),
            UPayloadFormat::Someip => Some("application/x-someip".to_string()),
            UPayloadFormat::SomeipTlv => Some("application/x-someip_tlv".to_string()),
            UPayloadFormat::Raw => Some("application/octet-stream".to_string()),
            UPayloadFormat::Text => Some("text/plain".to_string()),
            UPayloadFormat::Shm => Some("application/x-shm".to_string()),
        }
    }
}

#[cfg(feature = "up-core-types")]
mod core_types_support {
    use super::*;
    use crate::up_core_api::uattributes::UPayloadFormat as UPayloadFormatProto;

    impl From<UPayloadFormatProto> for UPayloadFormat {
        fn from(value: UPayloadFormatProto) -> Self {
            match value {
                UPayloadFormatProto::UPAYLOAD_FORMAT_UNSPECIFIED => UPayloadFormat::Unspecified,
                UPayloadFormatProto::UPAYLOAD_FORMAT_JSON => UPayloadFormat::Json,
                UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF => UPayloadFormat::Protobuf,
                UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY => {
                    UPayloadFormat::ProtobufWrappedInAny
                }
                UPayloadFormatProto::UPAYLOAD_FORMAT_RAW => UPayloadFormat::Raw,
                UPayloadFormatProto::UPAYLOAD_FORMAT_SHM => UPayloadFormat::Shm,
                UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP => UPayloadFormat::Someip,
                UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP_TLV => UPayloadFormat::SomeipTlv,
                UPayloadFormatProto::UPAYLOAD_FORMAT_TEXT => UPayloadFormat::Text,
            }
        }
    }

    impl From<&UPayloadFormat> for UPayloadFormatProto {
        fn from(value: &UPayloadFormat) -> Self {
            match value {
                UPayloadFormat::Unspecified => UPayloadFormatProto::UPAYLOAD_FORMAT_UNSPECIFIED,
                UPayloadFormat::Json => UPayloadFormatProto::UPAYLOAD_FORMAT_JSON,
                UPayloadFormat::Protobuf => UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF,
                UPayloadFormat::ProtobufWrappedInAny => {
                    UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY
                }
                UPayloadFormat::Raw => UPayloadFormatProto::UPAYLOAD_FORMAT_RAW,
                UPayloadFormat::Shm => UPayloadFormatProto::UPAYLOAD_FORMAT_SHM,
                UPayloadFormat::Someip => UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP,
                UPayloadFormat::SomeipTlv => UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP_TLV,
                UPayloadFormat::Text => UPayloadFormatProto::UPAYLOAD_FORMAT_TEXT,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    #[test_case("application/json", Ok(UPayloadFormat::Json); "map from JSON")]
    #[test_case(
        "application/json; charset=utf-8",
        Ok(UPayloadFormat::Json);
        "map from JSON with parameter"
    )]
    #[test_case("application/protobuf", Ok(UPayloadFormat::Protobuf); "map from PROTOBUF")]
    #[test_case(
        "application/x-protobuf",
        Ok(UPayloadFormat::ProtobufWrappedInAny); "map from PROTOBUF_WRAPPED"
    )]
    #[test_case("application/octet-stream", Ok(UPayloadFormat::Raw); "map from RAW")]
    #[test_case("application/x-someip", Ok(UPayloadFormat::Someip); "map from SOMEIP")]
    #[test_case(
        "application/x-someip_tlv",
        Ok(UPayloadFormat::SomeipTlv); "map from SOMEIP_TLV"
    )]
    #[test_case("text/plain", Ok(UPayloadFormat::Text); "map from TEXT")]
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

    #[test_case(UPayloadFormat::Json, Some("application/json".to_string()); "map JSON format to media type")]
    #[test_case(UPayloadFormat::Protobuf, Some("application/protobuf".to_string()); "map PROTOBUF format to media type")]
    #[test_case(UPayloadFormat::ProtobufWrappedInAny, Some("application/x-protobuf".to_string()); "map PROTOBUF_WRAPPED format to media type")]
    #[test_case(UPayloadFormat::Raw, Some("application/octet-stream".to_string()); "map RAW format to media type")]
    #[test_case(UPayloadFormat::Shm, Some("application/x-shm".to_string()); "map SHM format to media type")]
    #[test_case(UPayloadFormat::Someip, Some("application/x-someip".to_string()); "map SOMEIP format to media type")]
    #[test_case(UPayloadFormat::SomeipTlv, Some("application/x-someip_tlv".to_string()); "map SOMEIP_TLV format to media type")]
    #[test_case(UPayloadFormat::Text, Some("text/plain".to_string()); "map TEXT format to media type")]
    #[test_case(UPayloadFormat::Unspecified, None; "map UNSPECIFIED format to None")]
    fn test_to_media_type(format: UPayloadFormat, expected_media_type: Option<String>) {
        assert_eq!(format.to_media_type(), expected_media_type);
    }
}
