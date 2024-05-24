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
pub use umessagebuilder::*;

use crate::{UAttributes, UAttributesError, UPayloadFormat};
use protobuf::{well_known_types::any::Any, Message};

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

/// A container for a message's attributes and payload.
#[derive(Debug, Clone)]
pub struct UMessage {
    attributes: UAttributes,
    payload: Option<Bytes>,
}

impl UMessage {
    pub fn attributes(&self) -> &UAttributes {
        &self.attributes
    }

    pub fn payload(&self) -> Option<&Bytes> {
        self.payload.as_ref()
    }

    /// Extracts the payload-contained protobuf message from a `UMessage`.
    ///
    /// This function is used to extract strongly-typed data from a `UMessage` object,
    /// taking into account `UMessage::UPayloadFormat` (will only succeed if payload format is
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
    pub fn extract_protobuf_payload<T: Message + Default>(&self) -> Result<T, UMessageError> {
        if let Some(payload) = self.payload.as_ref() {
            match self.attributes.payload_format.enum_value_or_default() {
                UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF => {
                    return T::parse_from_bytes(payload.as_ref())
                        .map_err(UMessageError::DataSerializationError);
                }
                UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY => {
                    return Any::parse_from_bytes(payload.as_ref())
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
        } else {
            Err(UMessageError::from("Payload is empty"))
        }
    }
}
