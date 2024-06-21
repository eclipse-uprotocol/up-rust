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

use bytes::{Buf, Bytes};
use protobuf::{well_known_types::any::Any, Message};

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
    pub fn extract_protobuf<T: Message + Default>(&self) -> Result<T, UMessageError> {
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

// Deserialize a proto-generated `Message`-type from payload `Bytes`, according to `UPayloadFormat`
// Will only succeed if payload format is one of
// - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF`
// - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`
pub(crate) fn deserialize_protobuf_bytes<T: Message + Default>(
    payload: &Bytes,
    payload_format: &UPayloadFormat,
) -> Result<T, UMessageError> {
    match payload_format {
        UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF => {
            return T::parse_from_bytes(payload.chunk())
                .map_err(UMessageError::DataSerializationError);
        }
        UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY => {
            return Any::parse_from_bytes(payload.chunk())
                .map_err(UMessageError::DataSerializationError)
                .and_then(|any| {
                    T::parse_from_bytes(any.value.as_slice())
                        .map_err(UMessageError::DataSerializationError)
                });
        }
        _ => Err(UMessageError::from(
            "Unknown/invalid/unsupported payload format",
        )),
    }
}
