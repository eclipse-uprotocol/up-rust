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
use protobuf::{well_known_types::any::Any, Message};

pub use umessagebuilder::*;
pub use umessagetype::*;

use crate::up_core_api::uattributes::UAttributes as UAttributesProto;
use crate::up_core_api::umessage::UMessage as UMessageProto;
use crate::{
    ProtobufMappable, SerializationError, UAttributes, UAttributesError, UCode, UPayloadFormat,
    UPriority, UUri, UUID,
};

pub(crate) type PayloadVec = Vec<u8>;

#[derive(Debug)]
pub enum UMessageError {
    AttributesValidationError(UAttributesError),
    DataSerializationError(SerializationError),
    PayloadError(String),
}

impl std::fmt::Display for UMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AttributesValidationError(e) => f.write_fmt(format_args!(
                "Builder state is not consistent with message type: {e}"
            )),
            Self::DataSerializationError(e) => {
                f.write_fmt(format_args!("Failed to serialize payload: {e}"))
            }
            Self::PayloadError(e) => f.write_fmt(format_args!("UMessage payload error: {e}")),
        }
    }
}

impl std::error::Error for UMessageError {}

impl From<UAttributesError> for UMessageError {
    fn from(value: UAttributesError) -> Self {
        Self::AttributesValidationError(value)
    }
}

impl From<SerializationError> for UMessageError {
    fn from(value: SerializationError) -> Self {
        Self::DataSerializationError(value)
    }
}

impl From<protobuf::Error> for UMessageError {
    fn from(value: protobuf::Error) -> Self {
        Self::DataSerializationError(value.into())
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

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UMessage {
    attributes: UAttributes,
    payload: Option<PayloadVec>,
}

impl UMessage {
    pub(crate) fn new(
        attributes: UAttributes,
        payload: Option<Bytes>,
    ) -> Result<Self, UMessageError> {
        Ok(UMessage {
            attributes,
            payload: payload.map(|p| p.to_vec()),
        })
    }

    /// Get this message's attributes.
    #[must_use]
    pub fn attributes(&self) -> &UAttributes {
        &self.attributes
    }

    /// Gets this message's type.
    #[must_use]
    pub fn type_(&self) -> UMessageType {
        self.attributes().type_()
    }

    /// Gets this message's identifier.
    #[must_use]
    pub fn id(&self) -> &UUID {
        self.attributes().id()
    }

    /// Gets this message's source address.
    #[must_use]
    pub fn source(&self) -> &UUri {
        self.attributes().source()
    }

    /// Gets this message's sink address.
    #[must_use]
    pub fn sink(&self) -> Option<&UUri> {
        self.attributes().sink()
    }

    /// Gets this message's sink address.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn sink_unchecked(&self) -> &UUri {
        self.attributes().sink_unchecked()
    }

    /// Gets this message's priority.
    #[must_use]
    pub fn priority(&self) -> Option<UPriority> {
        self.attributes().priority()
    }

    /// Gets this message's priority.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn priority_unchecked(&self) -> UPriority {
        self.attributes().priority_unchecked()
    }

    /// Gets this message's commstatus.
    #[must_use]
    pub fn commstatus(&self) -> Option<UCode> {
        self.attributes().commstatus()
    }

    /// Gets this message's commstatus.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn commstatus_unchecked(&self) -> UCode {
        self.attributes().commstatus_unchecked()
    }

    /// Gets this message's time-to-live.
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    #[must_use]
    pub fn ttl(&self) -> Option<u32> {
        self.attributes().ttl()
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
    #[must_use]
    pub fn ttl_unchecked(&self) -> u32 {
        self.attributes().ttl_unchecked()
    }

    /// Gets this message's permission level.
    #[must_use]
    pub fn permission_level(&self) -> Option<u32> {
        self.attributes().permission_level()
    }

    /// Gets this message's token.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.attributes().token()
    }

    /// Gets this message's traceparent.
    #[must_use]
    pub fn traceparent(&self) -> Option<&str> {
        self.attributes().traceparent()
    }

    /// Gets this message's request identifier.
    #[must_use]
    pub fn request_id(&self) -> Option<&UUID> {
        self.attributes().request_id()
    }

    /// Gets this message's request identifier.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn request_id_unchecked(&self) -> &UUID {
        self.attributes().request_id_unchecked()
    }

    /// Gets this message's payload format.
    #[must_use]
    pub fn payload_format(&self) -> Option<UPayloadFormat> {
        self.attributes().payload_format()
    }

    /// Gets this message's payload format.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn payload_format_unchecked(&self) -> UPayloadFormat {
        self.attributes().payload_format_unchecked()
    }

    #[must_use]
    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    /// Checks if this is a Publish message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// let msg = UMessageBuilder::publish(
    ///   UUri::try_from_parts("origin", 0xabcd, 0x01, 0x9000).unwrap(),
    /// ).build().unwrap();
    /// assert!(msg.is_publish());
    /// ```
    #[must_use]
    pub fn is_publish(&self) -> bool {
        self.attributes().is_publish()
    }

    /// Checks if this is an RPC Request message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUID, UUri};
    ///
    /// let msg = UMessageBuilder::request(
    ///   UUri::try_from_parts("server", 0x1234, 0x01, 0x1000).unwrap(),
    ///   UUri::try_from_parts("client", 0xabcd, 0x01, 0x0000).unwrap(),
    ///   5000,
    /// ).build().unwrap();
    /// assert!(msg.is_request());
    /// ```
    #[must_use]
    pub fn is_request(&self) -> bool {
        self.attributes().is_request()
    }

    /// Checks if this is an RPC Response message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUID, UUri};
    ///
    /// let msg = UMessageBuilder::response(
    ///   UUri::try_from_parts("client", 0xabcd, 0x01, 0x0000).unwrap(),
    ///   UUID::build(),
    ///   UUri::try_from_parts("server", 0x1234, 0x01, 0x1000).unwrap(),
    /// ).build().unwrap();
    /// assert!(msg.is_response());
    /// ```
    #[must_use]
    pub fn is_response(&self) -> bool {
        self.attributes().is_response()
    }

    /// Checks if this is a Notification message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// let msg = UMessageBuilder::notification(
    ///   UUri::try_from_parts("origin", 0xabcd, 0x01, 0x9000).unwrap(),
    ///   UUri::try_from_parts("dest", 0x1234, 0x01, 0x0000).unwrap(),
    /// ).build().unwrap();
    /// assert!(msg.is_notification());
    /// ```
    #[must_use]
    pub fn is_notification(&self) -> bool {
        self.attributes().is_notification()
    }

    /// Deserializes this message's protobuf payload into a type.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Errors
    ///
    /// Returns an error if the message payload format is neither [UPayloadFormat::Protobuf] nor
    /// [UPayloadFormat::ProtobufWrappedInAny] or if the bytes in the
    /// payload cannot be deserialized into the target type.
    pub fn extract_protobuf<T: ProtobufMappable>(&self) -> Result<T, UMessageError> {
        if let Some(payload) = self.payload.as_ref() {
            let payload_format = self.payload_format().unwrap_or(UPayloadFormat::Unspecified);
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
pub(crate) fn deserialize_protobuf_bytes<T: ProtobufMappable>(
    payload: &[u8],
    payload_format: &UPayloadFormat,
) -> Result<T, UMessageError> {
    match payload_format {
        UPayloadFormat::Protobuf => {
            T::parse_from_protobuf_bytes(payload).map_err(UMessageError::DataSerializationError)
        }
        UPayloadFormat::ProtobufWrappedInAny => T::parse_from_packed_protobuf_bytes(payload)
            .map_err(UMessageError::DataSerializationError),
        UPayloadFormat::Unspecified
        | UPayloadFormat::Json
        | UPayloadFormat::Raw
        | UPayloadFormat::Shm
        | UPayloadFormat::Someip
        | UPayloadFormat::SomeipTlv
        | UPayloadFormat::Text => {
            let detail_msg = payload_format.to_media_type().map_or_else(
                || format!("Unknown payload format: {}", *payload_format as i32),
                |mt| format!("Invalid/unsupported payload format: {mt}"),
            );
            Err(UMessageError::from(detail_msg))
        }
    }
}

impl From<&UMessage> for UMessageProto {
    fn from(value: &UMessage) -> Self {
        let attributes = UAttributesProto::from(&value.attributes);
        UMessageProto {
            attributes: Some(attributes).into(),
            payload: value
                .payload
                .as_ref()
                .map(|p| Bytes::copy_from_slice(p.as_slice())),
            ..Default::default()
        }
    }
}

impl TryFrom<&UMessageProto> for UMessage {
    type Error = UMessageError;
    fn try_from(value: &UMessageProto) -> Result<Self, Self::Error> {
        let attributes = value.attributes.as_ref().map_or_else(
            || {
                Err(UAttributesError::validation_error(
                    "UMessageProto missing attributes",
                ))
            },
            UAttributes::try_from,
        )?;
        UMessage::new(attributes, value.payload.clone())
    }
}

impl ProtobufMappable for UMessage {
    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
        let proto = UMessageProto::parse_from_bytes(proto)?;
        UMessage::try_from(&proto).map_err(|e| SerializationError(e.to_string()))
    }

    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError(err.to_string()))
            .and_then(|any| match any.unpack::<UMessageProto>() {
                Ok(Some(umessage_proto)) => UMessage::try_from(&umessage_proto)
                    .map_err(|e| crate::SerializationError(e.to_string())),
                Ok(None) => Err(crate::SerializationError(
                    "Protobuf Any does not contain UMessage".to_string(),
                )),
                Err(e) => Err(crate::SerializationError(format!(
                    "Protobuf Any unpack error: {e}"
                ))),
            })
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        Ok(UMessageProto::from(self).write_to_bytes()?)
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        Any::pack(&UMessageProto::from(self))
            .map_err(|e| crate::SerializationError(format!("Failed to pack UMessage: {e}")))
            .and_then(|any| any.write_to_protobuf_bytes())
    }
}

#[cfg(test)]
mod test {
    use std::io;

    use protobuf::well_known_types::{any::Any, duration::Duration, wrappers::StringValue};
    use test_case::test_case;

    use crate::{UStatus, UUri};

    use super::*;

    #[test]
    fn test_deserialize_protobuf_bytes_succeeds() {
        let mut data = StringValue::new();
        data.value = "hello world".to_string();

        let result = deserialize_protobuf_bytes::<StringValue>(
            &data
                .write_to_bytes()
                .expect("Failed to write protobuf bytes"),
            &UPayloadFormat::Protobuf,
        );
        assert!(result.is_ok_and(|v| v.value == *"hello world"));

        let any = Any::pack(&data).expect("Failed to pack Any");
        let buf: Bytes = any
            .write_to_bytes()
            .expect("Failed to write protobuf bytes")
            .into();

        let result =
            deserialize_protobuf_bytes::<StringValue>(&buf, &UPayloadFormat::ProtobufWrappedInAny);
        assert!(result.is_ok_and(|v| v.value == *"hello world"));
    }

    #[test]
    fn test_deserialize_protobuf_bytes_fails_for_payload_type_mismatch() {
        let mut data = StringValue::new();
        data.value = "hello world".to_string();
        let any = Any::pack(&data).unwrap();
        let buf: Bytes = any.write_to_bytes().unwrap().into();
        let result =
            deserialize_protobuf_bytes::<UStatus>(&buf, &UPayloadFormat::ProtobufWrappedInAny);
        assert!(result.is_err_and(|e| matches!(e, UMessageError::DataSerializationError(_))));
    }

    #[test_case(UPayloadFormat::Json; "JSON format")]
    #[test_case(UPayloadFormat::Raw; "RAW format")]
    #[test_case(UPayloadFormat::Shm; "SHM format")]
    #[test_case(UPayloadFormat::Someip; "SOMEIP format")]
    #[test_case(UPayloadFormat::SomeipTlv; "SOMEIP TLV format")]
    #[test_case(UPayloadFormat::Text; "TEXT format")]
    #[test_case(UPayloadFormat::Unspecified; "UNSPECIFIED format")]
    fn test_deserialize_protobuf_bytes_fails_for_(format: UPayloadFormat) {
        let result = deserialize_protobuf_bytes::<UStatus>("hello".as_bytes(), &format);
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
            buf.as_slice(),
            &UPayloadFormat::ProtobufWrappedInAny,
        );
        assert!(result.is_err_and(|e| matches!(e, UMessageError::DataSerializationError(_))))
    }

    #[test]
    fn extract_payload_succeeds() {
        let payload = StringValue {
            value: "hello".to_string(),
            ..Default::default()
        };
        let topic =
            UUri::try_from_parts("local", 0xabcd, 0x01, 0x9000).expect("failed to create topic");
        let msg = UMessageBuilder::publish(topic)
            .build_with_protobuf_payload(&payload)
            .expect("failed to create message");
        assert!(msg
            .extract_protobuf::<StringValue>()
            .is_ok_and(|v| v.value == *"hello"));
    }

    #[test]
    fn extract_payload_fails_for_no_payload() {
        let topic =
            UUri::try_from_parts("local", 0xabcd, 0x01, 0x9000).expect("failed to create topic");
        let msg = UMessageBuilder::publish(topic)
            .build()
            .expect("failed to create message");
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
