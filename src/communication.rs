/********************************************************************************
 * Copyright (c) 2024 Contributors to the Eclipse Foundation
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

use bytes::Bytes;
pub use in_memory_rpc_client::InMemoryRpcClient;
pub use notification::{NotificationError, NotificationListener, Notifier};
use protobuf::Message;
pub use pubsub::{PubSubError, Publisher, Subscriber};
pub use rpc::{RpcClient, RpcServer, ServiceInvocationError};
use std::{error::Error, fmt::Display};

use crate::{
    umessage::{self, UMessageError},
    UCode, UMessage, UMessageBuilder, UPayloadFormat, UPriority, UStatus, UUID,
};

mod in_memory_rpc_client;
mod notification;
mod pubsub;
mod rpc;

/// An error indicating a problem with registering or unregistering a message listener.
#[derive(Debug)]
pub enum RegistrationError {
    /// Indicates that the maximum number of listeners supported by the Transport Layer implementation
    /// has already been registered.
    MaxListenersExceeded,
    /// Indicates that no listener is registered for given pattern URIs.
    NoSuchListener,
    /// Indicates that the underlying Transport Layer implementation does not support registration and
    /// notification of message handlers.
    PushDeliveryMethodNotSupported,
    /// Indicates a generic error.
    Unknown(UStatus),
}

impl Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::MaxListenersExceeded => {
                f.write_str("maximum number of listeners has been reached")
            }
            RegistrationError::NoSuchListener => {
                f.write_str("no listener registered for given pattern")
            }
            RegistrationError::PushDeliveryMethodNotSupported => f.write_str(
                "the underlying transport implementation does not support the push delivery method",
            ),
            RegistrationError::Unknown(status) => f.write_fmt(format_args!(
                "error un-/registering listener: {}",
                status.get_message()
            )),
        }
    }
}

impl Error for RegistrationError {}

impl From<UStatus> for RegistrationError {
    fn from(value: UStatus) -> Self {
        match value.code.enum_value() {
            Ok(UCode::NOT_FOUND) => RegistrationError::NoSuchListener,
            Ok(UCode::RESOURCE_EXHAUSTED) => RegistrationError::MaxListenersExceeded,
            Ok(UCode::UNIMPLEMENTED) => RegistrationError::PushDeliveryMethodNotSupported,
            _ => RegistrationError::Unknown(value),
        }
    }
}

/// General options that clients might want to specify when sending a uProtocol message.
#[derive(Debug)]
pub struct CallOptions {
    ttl: u32,
    message_id: Option<UUID>,
    token: Option<String>,
    priority: Option<UPriority>,
}

impl CallOptions {
    /// Creates new call options for an RPC Request.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The message's time-to-live in milliseconds.
    /// * `message_id` - The identifier to use for the message or `None` to use a generated identifier.
    /// * `token` - The token to use for authenticating to infrastructure and service endpoints.
    /// * `priority` - The message's priority or `None` to use the default priority for RPC Requests.
    ///
    /// # Returns
    ///
    /// Options suitable for invoking an RPC method.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UPriority, UUID, communication::CallOptions};
    ///
    /// let uuid = UUID::new();
    /// let options = CallOptions::for_rpc_request(15_000, Some(uuid.clone()), Some("token".to_string()), Some(UPriority::UPRIORITY_CS2));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(uuid));
    /// assert_eq!(options.token(), Some("token".to_string()));
    /// assert_eq!(options.priority(), Some(UPriority::UPRIORITY_CS2));
    /// ```
    pub fn for_rpc_request(
        ttl: u32,
        message_id: Option<UUID>,
        token: Option<String>,
        priority: Option<UPriority>,
    ) -> Self {
        CallOptions {
            ttl,
            message_id,
            token,
            priority,
        }
    }

    /// Gets the message's time-to-live in milliseconds.
    pub fn ttl(&self) -> u32 {
        self.ttl
    }

    /// Gets the identifier to use for the message.
    pub fn message_id(&self) -> Option<UUID> {
        self.message_id.clone()
    }

    /// Gets the token to use for authenticating to infrastructure and service endpoints.
    pub fn token(&self) -> Option<String> {
        self.token.clone()
    }

    /// Gets the message's priority.
    pub fn priority(&self) -> Option<UPriority> {
        self.priority
    }
}

/// A wrapper around (raw) message payload data and the corresponding payload format.
#[derive(Clone)]
pub struct UPayload {
    payload_format: UPayloadFormat,
    payload: Bytes,
}

impl UPayload {
    pub fn new(payload: Bytes, payload_format: UPayloadFormat) -> Self {
        UPayload {
            payload_format,
            payload,
        }
    }

    /// Creates a new UPayload from a protobuf message.
    ///
    /// The resulting payload will have `UPayloadType::UPAYLOAD_FORMAT_PROTOBUF`.
    ///
    /// # Errors
    ///
    /// Returns an error if the given message cannot be serialized to bytes.
    pub fn try_from_protobuf<M>(message: M) -> Result<Self, UMessageError>
    where
        M: Message,
    {
        match message.write_to_bytes() {
            Ok(bytes) => Ok(UPayload::new(
                bytes.into(),
                UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF,
            )),
            Err(e) => Err(UMessageError::DataSerializationError(e)),
        }
    }

    /// Gets the payload format.
    ///
    /// # Returns
    ///
    /// payload value of `UPayload`.
    pub fn payload_format(&self) -> UPayloadFormat {
        self.payload_format
    }

    /// Gets the payload data.
    ///
    /// Note that this consumes the payload.
    pub fn payload(self) -> Bytes {
        self.payload
    }

    /// Extracts the protobuf `Message` contained in payload.
    ///
    /// This function is used to extract strongly-typed data from a `UPayload` object,
    /// taking into account the payload format (will only succeed if payload format is
    /// `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF` or `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`)
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Returns
    ///
    /// * `Ok(T)`: The deserialized protobuf `Message` contained in the payload.
    ///
    /// # Errors
    ///
    /// * Err(`UMessageError`) if the unpacking process fails, for example if the payload could
    /// not be deserialized into the target type `T`.
    pub fn extract_protobuf<T: Message + Default>(&self) -> Result<T, UMessageError> {
        umessage::deserialize_protobuf_bytes(&self.payload, &self.payload_format)
    }
}

/// Creates a message with given payload from a builder.
pub(crate) fn build_message(
    message_builder: &mut UMessageBuilder,
    payload: Option<UPayload>,
) -> Result<UMessage, UMessageError> {
    if let Some(pl) = payload {
        let format = pl.payload_format();
        message_builder.build_with_payload(pl.payload, format)
    } else {
        message_builder.build()
    }
}
