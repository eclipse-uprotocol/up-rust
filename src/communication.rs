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
pub use notification::{NotificationError, NotificationListener, Notifier};
use protobuf::Message;
pub use pubsub::{PubSubError, Publisher, Subscriber};
pub use rpc::{RpcClient, RpcServer, ServiceInvocationError};
use std::{error::Error, fmt::Display};

use crate::{
    umessage::{self, UMessageError},
    UPayloadFormat, UPriority, UUID,
};

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
        }
    }
}

impl Error for RegistrationError {}

const DEFAULT_TTL: u32 = 10_000; // 10 seconds

/// General options that clients might want to specify when sending a uProtocol message.
#[derive(Debug)]
pub struct CallOptions {
    ttl: u32,
    message_id: Option<UUID>,
    token: Option<String>,
    priority: Option<UPriority>,
    traceparent: Option<String>,
}

impl Default for CallOptions {
    /// Creates empty options with a TTL of 10s.
    fn default() -> Self {
        CallOptions {
            ttl: DEFAULT_TTL,
            message_id: None,
            token: None,
            priority: None,
            traceparent: None,
        }
    }
}

impl CallOptions {
    /// Creates new call options.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The message's time-to-live in milliseconds.
    /// * `message_id` - The identifier to use for the message or `None` to use a generated identifier.
    /// * `token` - The token to use for authenticating to infrastructure and service endpoints.
    /// * `priority` - The message's priority or `None` to use the default priority.
    ///
    /// # Returns
    ///
    /// `CallOption` with specified ttl value, token and priority parameters.
    pub fn new(
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
            traceparent: None,
        }
    }

    /// Sets the message's time-to-live.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The time-to-live in milliseconds.
    ///
    /// # Returns
    ///
    /// `CallOption` with specified ttl value.
    pub fn with_ttl(&mut self, ttl: u32) -> &mut Self {
        self.ttl = ttl;
        self
    }

    /// Gets the message's time-to-live in milliseconds.
    pub fn ttl(&self) -> u32 {
        self.ttl
    }

    /// Sets the identifier to use for the message.
    ///
    /// # Returns
    ///
    /// `CallOption` with specified reqid value.
    pub fn with_message_id(&mut self, message_id: UUID) -> &mut Self {
        self.message_id = Some(message_id);
        self
    }

    /// Gets the identifier to use for the message.
    pub fn message_id(&self) -> Option<UUID> {
        self.message_id.clone()
    }

    /// Sets the token to use for authenticating to infrastructure and service endpoints.
    ///
    /// # Returns
    ///
    /// `CallOption` with specified token value.
    pub fn with_token<T: Into<String>>(&mut self, token: T) -> &mut Self {
        self.token = Some(token.into());
        self
    }

    /// Gets the token to use for authenticating to infrastructure and service endpoints.
    pub fn token(&self) -> Option<String> {
        self.token.clone()
    }

    /// Sets the message's priority.
    ///
    /// If not set explicitly, the default priority for RPC calls will be used.
    ///
    /// # Returns
    ///
    /// `CallOption` with specified priority value.
    pub fn with_priority(&mut self, priority: UPriority) -> &mut Self {
        self.priority = Some(priority);
        self
    }

    /// Gets the message's priority.
    pub fn priority(&self) -> Option<UPriority> {
        self.priority
    }

    /// Sets the W3C Trace Context that the message is part of.
    ///
    /// # Arguments
    ///
    /// * `traceparent` - The [traceparent](https://w3c.github.io/trace-context/#traceparent-header) value identifying the trace context.
    ///
    /// # Returns
    ///
    /// `CallOption` with specified priority value.
    pub fn with_traceparent<T: Into<String>>(&mut self, traceparent: T) -> &mut Self {
        self.traceparent = Some(traceparent.into());
        self
    }

    /// Gets the [traceparent](https://w3c.github.io/trace-context/#traceparent-header) value identifying the trace context that
    /// the message is part of.
    pub fn traceparent(&self) -> Option<String> {
        self.traceparent.clone()
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
