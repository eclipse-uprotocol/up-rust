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
use protobuf::{well_known_types::any::Any, Message, MessageFull};

pub use umessagebuilder::*;

pub use crate::up_core_api::umessage::UMessage;

use crate::{UAttributesError, UPayloadFormat};

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

impl From<&str> for UMessageError {
    fn from(value: &str) -> Self {
        Self::PayloadError(value.into())
    }
}

impl UMessage {
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
            .map_or(false, |attribs| attribs.is_publish())
    }

    /// Checks if this is an RPC Request message.
    pub fn is_request(&self) -> bool {
        self.attributes.get_or_default().is_request()
    }

    /// Checks if this is an RPC Response message.
    pub fn is_response(&self) -> bool {
        self.attributes.get_or_default().is_response()
    }

    /// Checks if this is a Notification message.
    pub fn is_notification(&self) -> bool {
        self.attributes.get_or_default().is_notification()
    }

    /// If `UMessage` payload is available, deserialize it as a protobuf `Message`.
    ///
    /// This function is used to extract strongly-typed data from a `UMessage` object,
    /// taking into account the payload format (will only succeed if payload format is
    /// `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF` or `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`)
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Returns
    ///
    /// * `Ok(T)`: The deserialized protobuf message contained in the payload.
    ///
    /// # Errors
    ///
    /// * Err(`UMessageError`) if the unpacking process fails, for example if the payload could
    /// not be deserialized into the target type `T`.
    pub fn extract_protobuf<T: MessageFull + Default>(&self) -> Result<T, UMessageError> {
        if let Some(payload) = &self.payload {
            let payload_format = self.attributes.payload_format.enum_value_or_default();
            deserialize_protobuf_bytes(payload, &payload_format)
        } else {
            Err(UMessageError::PayloadError(
                "No embedded payload".to_string(),
            ))
        }
    }
}

/// Deserializes a protobuf message from a byte array.
///
/// Will only succeed if payload format is one of
/// - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF`
/// - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`
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
        _ => Err(UMessageError::from(
            "Unknown/invalid/unsupported payload format",
        )),
    }
}

#[cfg(test)]
mod test {
    use protobuf::well_known_types::{any::Any, wrappers::StringValue};

    use crate::UStatus;

    use super::*;

    #[test]
    fn test_deserialize_protobuf_bytes_succeeds() {
        let mut data = StringValue::new();
        data.value = "hello world".to_string();
        let any = Any::pack(&data).unwrap();
        let buf: Bytes = any.write_to_bytes().unwrap().into();
        let result = deserialize_protobuf_bytes::<StringValue>(
            &buf,
            &UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY,
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
}
