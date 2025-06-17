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

mod umessagebuilder;
mod umessagetype;

use bytes::Bytes;
use protobuf::{well_known_types::any::Any, Enum, Message, MessageFull};

pub use umessagebuilder::*;

pub use crate::up_core_api::umessage::UMessage;

use crate::{UAttributesError, UCode, UMessageType, UPayloadFormat, UPriority, UUri, UUID};

#[derive(Debug)]
pub enum UMessageError {
    AttributesValidationError(UAttributesError),
    DataSerializationError(protobuf::Error),
    PayloadError(String),
}

impl std::fmt::Display for UMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AttributesValidationError(e) => f.write_fmt(format_args!(
                "Builder state is not consistent with message type: {}",
                e
            )),
            Self::DataSerializationError(e) => {
                f.write_fmt(format_args!("Failed to serialize payload: {}", e))
            }
            Self::PayloadError(e) => f.write_fmt(format_args!("UMessage payload error: {}", e)),
        }
    }
}

impl std::error::Error for UMessageError {}

impl From<UAttributesError> for UMessageError {
    fn from(value: UAttributesError) -> Self {
        Self::AttributesValidationError(value)
    }
}

impl From<protobuf::Error> for UMessageError {
    fn from(value: protobuf::Error) -> Self {
        Self::DataSerializationError(value)
    }
}

impl From<String> for UMessageError {
    fn from(value: String) -> Self {
        Self::PayloadError(value)
    }
}

impl From<&str> for UMessageError {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl UMessage {
    /// Gets this message's type.
    pub fn type_(&self) -> Option<UMessageType> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.type_.enum_value().ok())
    }

    /// Gets this message's type.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn type_unchecked(&self) -> UMessageType {
        self.type_().expect("message has no type")
    }

    /// Gets this message's identifier.
    pub fn id(&self) -> Option<&UUID> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.id.as_ref())
    }

    /// Gets this message's identifier.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn id_unchecked(&self) -> &UUID {
        self.id().expect("message has no ID")
    }

    /// Gets this message's source address.
    pub fn source(&self) -> Option<&UUri> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.source.as_ref())
    }

    /// Gets this message's source address.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn source_unchecked(&self) -> &UUri {
        self.source().expect("message has no source")
    }

    /// Gets this message's sink address.
    pub fn sink(&self) -> Option<&UUri> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.sink.as_ref())
    }

    /// Gets this message's sink address.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn sink_unchecked(&self) -> &UUri {
        self.sink().expect("message has no sink")
    }

    /// Gets this message's priority.
    pub fn priority(&self) -> Option<UPriority> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.priority.enum_value().ok())
            // [impl->dsn~up-attributes-priority~1]
            .map(|prio| {
                if prio == UPriority::UPRIORITY_UNSPECIFIED {
                    UPriority::UPRIORITY_CS1
                } else {
                    prio
                }
            })
    }

    /// Gets this message's priority.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn priority_unchecked(&self) -> UPriority {
        self.priority().expect("message has no priority")
    }

    /// Gets this message's commstatus.
    pub fn commstatus(&self) -> Option<UCode> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.commstatus)
            .and_then(|v| v.enum_value().ok())
    }

    /// Gets this message's commstatus.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn commstatus_unchecked(&self) -> UCode {
        self.commstatus().expect("message has no commstatus")
    }

    /// Gets this message's time-to-live.
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    pub fn ttl(&self) -> Option<u32> {
        self.attributes.as_ref().and_then(|attribs| attribs.ttl)
    }

    /// Gets this message's time-to-live.
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn ttl_unchecked(&self) -> u32 {
        self.ttl().expect("message has no time-to-live")
    }

    /// Gets this message's permission level.
    pub fn permission_level(&self) -> Option<u32> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.permission_level)
    }

    /// Gets this message's token.
    pub fn token(&self) -> Option<&String> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.token.as_ref())
    }

    /// Gets this message's traceparent.
    pub fn traceparent(&self) -> Option<&String> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.traceparent.as_ref())
    }

    /// Gets this message's request identifier.
    pub fn request_id(&self) -> Option<&UUID> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.reqid.as_ref())
    }

    /// Gets this message's request identifier.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn request_id_unchecked(&self) -> &UUID {
        self.request_id().expect("message has no request ID")
    }

    /// Gets this message's payload format.
    pub fn payload_format(&self) -> Option<UPayloadFormat> {
        self.attributes
            .as_ref()
            .and_then(|attribs| attribs.payload_format.enum_value().ok())
    }

    /// Gets this message's payload format.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    pub fn payload_format_unchecked(&self) -> UPayloadFormat {
        self.payload_format()
            .expect("message has no payload format")
    }

    /// Checks if this is a Publish message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessage, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
    ///   ..Default::default()
    /// };
    /// let msg = UMessage {
    ///   attributes: Some(attribs).into(),
    ///   ..Default::default()
    /// };
    /// assert!(msg.is_publish());
    /// ```
    pub fn is_publish(&self) -> bool {
        self.attributes
            .as_ref()
            .is_some_and(|attribs| attribs.is_publish())
    }

    /// Checks if this is an RPC Request message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessage, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
    ///   ..Default::default()
    /// };
    /// let msg = UMessage {
    ///   attributes: Some(attribs).into(),
    ///   ..Default::default()
    /// };
    /// assert!(msg.is_request());
    /// ```
    pub fn is_request(&self) -> bool {
        self.attributes
            .as_ref()
            .is_some_and(|attribs| attribs.is_request())
    }

    /// Checks if this is an RPC Response message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessage, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_RESPONSE.into(),
    ///   ..Default::default()
    /// };
    /// let msg = UMessage {
    ///   attributes: Some(attribs).into(),
    ///   ..Default::default()
    /// };
    /// assert!(msg.is_response());
    /// ```
    pub fn is_response(&self) -> bool {
        self.attributes
            .as_ref()
            .is_some_and(|attribs| attribs.is_response())
    }

    /// Checks if this is a Notification message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessage, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
    ///   ..Default::default()
    /// };
    /// let msg = UMessage {
    ///   attributes: Some(attribs).into(),
    ///   ..Default::default()
    /// };
    /// assert!(msg.is_notification());
    /// ```
    pub fn is_notification(&self) -> bool {
        self.attributes
            .as_ref()
            .is_some_and(|attribs| attribs.is_notification())
    }

    /// Deserializes this message's protobuf payload into a type.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Errors
    ///
    /// Returns an error if the message payload format is neither [UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF]
    /// nor [UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY] or if the bytes in the
    /// payload cannot be deserialized into the target type.
    pub fn extract_protobuf<T: MessageFull + Default>(&self) -> Result<T, UMessageError> {
        if let Some(payload) = self.payload.as_ref() {
            let payload_format = self.attributes.payload_format.enum_value_or_default();
            deserialize_protobuf_bytes(payload, &payload_format)
        } else {
            Err(UMessageError::PayloadError(
                "Message has no payload".to_string(),
            ))
        }
    }
}

/// Deserializes a protobuf message from a byte array.
///
/// # Type Parameters
///
/// * `T`: The target type of the data to be unpacked.
///
/// # Arguments
///
/// * `payload` - The payload data.
/// * `payload_format` - The format/encoding of the data. Must be one of
///    - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF`
///    - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`
///
/// # Errors
///
/// Returns an error if the payload format is unsupported or if the data can not be deserialized
/// into the target type based on the given format.
pub(crate) fn deserialize_protobuf_bytes<T: MessageFull + Default>(
    payload: &Bytes,
    payload_format: &UPayloadFormat,
) -> Result<T, UMessageError> {
    match payload_format {
        UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF => {
            T::parse_from_tokio_bytes(payload).map_err(UMessageError::DataSerializationError)
        }
        UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY => {
            Any::parse_from_tokio_bytes(payload)
                .map_err(UMessageError::DataSerializationError)
                .and_then(|any| match any.unpack() {
                    Ok(Some(v)) => Ok(v),
                    Ok(None) => Err(UMessageError::PayloadError(
                        "cannot deserialize payload, message type mismatch".to_string(),
                    )),
                    Err(e) => Err(UMessageError::DataSerializationError(e)),
                })
        }
        _ => {
            let detail_msg = payload_format.to_media_type().map_or_else(
                || format!("Unknown payload format: {}", payload_format.value()),
                |mt| format!("Invalid/unsupported payload format: {mt}"),
            );
            Err(UMessageError::from(detail_msg))
        }
    }
}

#[cfg(test)]
mod test {
    use std::io;

    use protobuf::well_known_types::{any::Any, duration::Duration, wrappers::StringValue};
    use test_case::test_case;

    use crate::{UAttributes, UStatus};

    use super::*;

    #[test]
    fn test_deserialize_protobuf_bytes_succeeds() {
        let mut data = StringValue::new();
        data.value = "hello world".to_string();
        let any = Any::pack(&data.clone()).unwrap();
        let buf: Bytes = any.write_to_bytes().unwrap().into();

        let result = deserialize_protobuf_bytes::<StringValue>(
            &buf,
            &UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY,
        );
        assert!(result.is_ok_and(|v| v.value == *"hello world"));

        let result = deserialize_protobuf_bytes::<StringValue>(
            &data.write_to_bytes().unwrap().into(),
            &UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF,
        );
        assert!(result.is_ok_and(|v| v.value == *"hello world"));
    }

    #[test]
    fn test_deserialize_protobuf_bytes_fails_for_payload_type_mismatch() {
        let mut data = StringValue::new();
        data.value = "hello world".to_string();
        let any = Any::pack(&data).unwrap();
        let buf: Bytes = any.write_to_bytes().unwrap().into();
        let result = deserialize_protobuf_bytes::<UStatus>(
            &buf,
            &UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY,
        );
        assert!(result.is_err_and(|e| matches!(e, UMessageError::PayloadError(_))));
    }

    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_JSON; "JSON format")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_RAW; "RAW format")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_SHM; "SHM format")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_SOMEIP; "SOMEIP format")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_SOMEIP_TLV; "SOMEIP TLV format")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_TEXT; "TEXT format")]
    #[test_case(UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED; "UNSPECIFIED format")]
    fn test_deserialize_protobuf_bytes_fails_for_(format: UPayloadFormat) {
        let result = deserialize_protobuf_bytes::<UStatus>(&"hello".into(), &format);
        assert!(result.is_err_and(|e| matches!(e, UMessageError::PayloadError(_))));
    }

    #[test]
    fn test_deserialize_protobuf_bytes_fails_for_invalid_encoding() {
        let any = Any {
            type_url: "type.googleapis.com/google.protobuf.Duration".to_string(),
            value: vec![0x0A],
            ..Default::default()
        };
        let buf = any.write_to_bytes().unwrap();
        let result = deserialize_protobuf_bytes::<Duration>(
            &buf.into(),
            &UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY,
        );
        assert!(result.is_err_and(|e| matches!(e, UMessageError::DataSerializationError(_))))
    }

    #[test]
    fn extract_payload_succeeds() {
        let payload = StringValue {
            value: "hello".to_string(),
            ..Default::default()
        };
        let buf = Any::pack(&payload)
            .and_then(|a| a.write_to_bytes())
            .unwrap();
        let msg = UMessage {
            attributes: Some(UAttributes {
                payload_format: UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY.into(),
                ..Default::default()
            })
            .into(),
            payload: Some(buf.into()),
            ..Default::default()
        };
        assert!(msg
            .extract_protobuf::<StringValue>()
            .is_ok_and(|v| v.value == *"hello"));
    }

    #[test]
    fn extract_payload_fails_for_no_payload() {
        let msg = UMessage {
            attributes: Some(UAttributes {
                payload_format: UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY.into(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        assert!(msg
            .extract_protobuf::<StringValue>()
            .is_err_and(|e| matches!(e, UMessageError::PayloadError(_))));
    }

    #[test]
    fn test_from_attributes_error() {
        let attributes_error = UAttributesError::validation_error("failed to validate");
        let message_error = UMessageError::from(attributes_error);
        assert!(matches!(
            message_error,
            UMessageError::AttributesValidationError(UAttributesError::ValidationError(_))
        ));
    }

    #[test]
    fn test_from_protobuf_error() {
        let protobuf_error = protobuf::Error::from(io::Error::last_os_error());
        let message_error = UMessageError::from(protobuf_error);
        assert!(matches!(
            message_error,
            UMessageError::DataSerializationError(_)
        ));
    }

    #[test]
    fn test_from_error_msg() {
        let message_error = UMessageError::from("an error occurred");
        assert!(matches!(message_error, UMessageError::PayloadError(_)));
    }
}
