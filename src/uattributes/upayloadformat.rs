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
use protobuf::EnumFull;

use crate::up_core_api::uattributes::UPayloadFormat as UPayloadFormatProto;

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
    Unspecified = UPayloadFormatProto::UPAYLOAD_FORMAT_UNSPECIFIED as isize,
    Json = UPayloadFormatProto::UPAYLOAD_FORMAT_JSON as isize,
    Protobuf = UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF as isize,
    ProtobufWrappedInAny = UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY as isize,
    Raw = UPayloadFormatProto::UPAYLOAD_FORMAT_RAW as isize,
    Shm = UPayloadFormatProto::UPAYLOAD_FORMAT_SHM as isize,
    Someip = UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP as isize,
    SomeipTlv = UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP_TLV as isize,
    Text = UPayloadFormatProto::UPAYLOAD_FORMAT_TEXT as isize,
}

impl UPayloadFormat {
    /// Returns the integer value of the payload format as defined in the protobuf enum.
    #[must_use]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    #[must_use]
    pub fn from_i32(value: i32) -> Option<UPayloadFormat> {
        match value {
            0 => Some(UPayloadFormat::Unspecified),
            1 => Some(UPayloadFormat::ProtobufWrappedInAny),
            2 => Some(UPayloadFormat::Protobuf),
            3 => Some(UPayloadFormat::Json),
            4 => Some(UPayloadFormat::Someip),
            5 => Some(UPayloadFormat::SomeipTlv),
            6 => Some(UPayloadFormat::Raw),
            7 => Some(UPayloadFormat::Text),
            8 => Some(UPayloadFormat::Shm),
            _ => None,
        }
    }
}

impl TryFrom<UPayloadFormatProto> for UPayloadFormat {
    type Error = UPayloadError;

    fn try_from(value: UPayloadFormatProto) -> Result<Self, Self::Error> {
        match value {
            UPayloadFormatProto::UPAYLOAD_FORMAT_UNSPECIFIED => Ok(UPayloadFormat::Unspecified),
            UPayloadFormatProto::UPAYLOAD_FORMAT_JSON => Ok(UPayloadFormat::Json),
            UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF => Ok(UPayloadFormat::Protobuf),
            UPayloadFormatProto::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY => {
                Ok(UPayloadFormat::ProtobufWrappedInAny)
            }
            UPayloadFormatProto::UPAYLOAD_FORMAT_RAW => Ok(UPayloadFormat::Raw),
            UPayloadFormatProto::UPAYLOAD_FORMAT_SHM => Ok(UPayloadFormat::Shm),
            UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP => Ok(UPayloadFormat::Someip),
            UPayloadFormatProto::UPAYLOAD_FORMAT_SOMEIP_TLV => Ok(UPayloadFormat::SomeipTlv),
            UPayloadFormatProto::UPAYLOAD_FORMAT_TEXT => Ok(UPayloadFormat::Text),
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
            let descriptor = UPayloadFormatProto::enum_descriptor();
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
                                    return desc.cast::<UPayloadFormatProto>().and_then(
                                        |format_proto| UPayloadFormat::try_from(format_proto).ok(),
                                    );
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
        let desc = UPayloadFormatProto::from(&self).descriptor();
        let desc_proto = desc.proto();
        crate::up_core_api::uoptions::exts::mime_type.get(desc_proto.options.get_or_default())
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
