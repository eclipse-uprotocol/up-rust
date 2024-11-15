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
use protobuf::{well_known_types::any::Any, Message, MessageFull};
use std::{error::Error, fmt::Display};

pub use default_notifier::SimpleNotifier;
#[cfg(feature = "usubscription")]
pub use default_pubsub::{InMemorySubscriber, SimplePublisher};
pub use in_memory_rpc_client::InMemoryRpcClient;
pub use in_memory_rpc_server::InMemoryRpcServer;
#[cfg(any(test, feature = "test-util"))]
pub use notification::MockNotifier;
pub use notification::{NotificationError, Notifier};
#[cfg(any(test, feature = "test-util"))]
pub use pubsub::MockSubscriptionChangeHandler;
#[cfg(feature = "usubscription")]
pub use pubsub::{PubSubError, Publisher, Subscriber};
#[cfg(any(test, feature = "test-util"))]
pub use rpc::{MockRequestHandler, MockRpcClient, MockRpcServerImpl};
pub use rpc::{RequestHandler, RpcClient, RpcServer, ServiceInvocationError};
#[cfg(feature = "usubscription")]
pub use usubscription_client::RpcClientUSubscription;

use crate::{
    umessage::{self, UMessageError},
    UCode, UMessage, UMessageBuilder, UPayloadFormat, UPriority, UStatus, UUID,
};

mod default_notifier;
mod default_pubsub;
mod in_memory_rpc_client;
mod in_memory_rpc_server;
mod notification;
#[cfg(feature = "usubscription")]
mod pubsub;
mod rpc;
#[cfg(feature = "usubscription")]
mod usubscription_client;

/// An error indicating a problem with registering or unregistering a message listener.
#[derive(Clone, Debug)]
pub enum RegistrationError {
    /// Indicates that a listener for a given address already exists.
    AlreadyExists,
    /// Indicates that the maximum number of listeners supported by the Transport Layer implementation
    /// has already been registered.
    MaxListenersExceeded,
    /// Indicates that no listener is registered for given pattern URIs.
    NoSuchListener,
    /// Indicates that the underlying Transport Layer implementation does not support registration and
    /// notification of message handlers.
    PushDeliveryMethodNotSupported,
    /// Indicates that some of the given filters are inappropriate in this context.
    InvalidFilter(String),
    /// Indicates a generic error.
    Unknown(UStatus),
}

impl Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::AlreadyExists => {
                f.write_str("a listener for the given filter criteria already exists")
            }
            RegistrationError::MaxListenersExceeded => {
                f.write_str("maximum number of listeners has been reached")
            }
            RegistrationError::NoSuchListener => {
                f.write_str("no listener registered for given pattern")
            }
            RegistrationError::PushDeliveryMethodNotSupported => f.write_str(
                "the underlying transport implementation does not support the push delivery method",
            ),
            RegistrationError::InvalidFilter(msg) => {
                f.write_fmt(format_args!("invalid filter(s): {}", msg))
            }
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
            Ok(UCode::ALREADY_EXISTS) => RegistrationError::AlreadyExists,
            Ok(UCode::NOT_FOUND) => RegistrationError::NoSuchListener,
            Ok(UCode::RESOURCE_EXHAUSTED) => RegistrationError::MaxListenersExceeded,
            Ok(UCode::UNIMPLEMENTED) => RegistrationError::PushDeliveryMethodNotSupported,
            Ok(UCode::INVALID_ARGUMENT) => RegistrationError::InvalidFilter(value.get_message()),
            _ => RegistrationError::Unknown(value),
        }
    }
}

/// General options that clients might want to specify when sending a uProtocol message.
#[derive(Clone, Debug, PartialEq)]
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
    /// let options = CallOptions::for_rpc_request(15_000, Some(uuid.clone()), Some("token".to_string()), Some(UPriority::UPRIORITY_CS6));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(uuid));
    /// assert_eq!(options.token(), Some("token".to_string()));
    /// assert_eq!(options.priority(), Some(UPriority::UPRIORITY_CS6));
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

    /// Creates new call options for a Notification message.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The message's time-to-live in milliseconds.
    /// * `message_id` - The identifier to use for the message or `None` to use a generated identifier.
    /// * `priority` - The message's priority or `None` to use the default priority for Notifications.
    ///
    /// # Returns
    ///
    /// Options suitable for sending a Notification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UPriority, UUID, communication::CallOptions};
    ///
    /// let uuid = UUID::new();
    /// let options = CallOptions::for_notification(Some(15_000), Some(uuid.clone()), Some(UPriority::UPRIORITY_CS2));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(uuid));
    /// assert_eq!(options.priority(), Some(UPriority::UPRIORITY_CS2));
    /// ```
    pub fn for_notification(
        ttl: Option<u32>,
        message_id: Option<UUID>,
        priority: Option<UPriority>,
    ) -> Self {
        CallOptions {
            ttl: ttl.unwrap_or(0),
            message_id,
            token: None,
            priority,
        }
    }

    /// Creates new call options for a Publish message.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The message's time-to-live in milliseconds or `None` if the message should not expire at all.
    /// * `message_id` - The identifier to use for the message or `None` to use a generated identifier.
    /// * `priority` - The message's priority or `None` to use the default priority for Publish messages.
    ///
    /// # Returns
    ///
    /// Options suitable for sending a Publish message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UPriority, UUID, communication::CallOptions};
    ///
    /// let uuid = UUID::new();
    /// let options = CallOptions::for_publish(Some(15_000), Some(uuid.clone()), Some(UPriority::UPRIORITY_CS2));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(uuid));
    /// assert_eq!(options.priority(), Some(UPriority::UPRIORITY_CS2));
    /// ```
    pub fn for_publish(
        ttl: Option<u32>,
        message_id: Option<UUID>,
        priority: Option<UPriority>,
    ) -> Self {
        CallOptions {
            ttl: ttl.unwrap_or(0),
            message_id,
            token: None,
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
#[derive(Clone, Debug, PartialEq)]
pub struct UPayload {
    payload_format: UPayloadFormat,
    payload: Bytes,
}

impl UPayload {
    /// Creates a new payload for some data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UPayloadFormat;
    /// use up_rust::communication::UPayload;
    ///
    /// let data: Vec<u8> = vec![0x00_u8, 0x01_u8, 0x02_u8];
    /// let payload = UPayload::new(data, UPayloadFormat::UPAYLOAD_FORMAT_RAW);
    /// assert_eq!(payload.payload_format(), UPayloadFormat::UPAYLOAD_FORMAT_RAW);
    /// assert_eq!(payload.payload().len(), 3);
    /// ```
    pub fn new<T: Into<Bytes>>(payload: T, payload_format: UPayloadFormat) -> Self {
        UPayload {
            payload_format,
            payload: payload.into(),
        }
    }

    /// Creates a new UPayload from a protobuf message.
    ///
    /// The resulting payload will have `UPayloadType::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`.
    ///
    /// # Errors
    ///
    /// Returns an error if the given message cannot be serialized to bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{communication::UPayload, UPayloadFormat};
    /// use protobuf::{well_known_types::wrappers::StringValue};
    ///
    /// let mut data = StringValue::new();
    /// data.value = "hello world".to_string();
    /// assert!(UPayload::try_from_protobuf(data).is_ok_and(|pl|
    ///     pl.payload_format() == UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY
    ///         && pl.payload().len() > 0));
    /// ```
    pub fn try_from_protobuf<M>(message: M) -> Result<Self, UMessageError>
    where
        M: MessageFull,
    {
        Any::pack(&message)
            .and_then(|any| any.write_to_bytes())
            .map(|buf| UPayload::new(buf, UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY))
            .map_err(UMessageError::DataSerializationError)
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
    ///   not be deserialized into the target type `T`.
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{communication::UPayload, UPayloadFormat};
    /// use protobuf::{well_known_types::wrappers::StringValue};
    ///
    /// let mut data = StringValue::new();
    /// data.value = "hello world".to_string();
    /// let payload = UPayload::try_from_protobuf(data).expect("should be able to create UPayload from StringValue");
    ///
    /// let string_value: StringValue = payload.extract_protobuf().expect("should be able to extract StringValue from UPayload");
    /// assert_eq!(string_value.value, *"hello world");
    /// ```
    pub fn extract_protobuf<T: MessageFull + Default>(&self) -> Result<T, UMessageError> {
        umessage::deserialize_protobuf_bytes(&self.payload, &self.payload_format)
    }
}

/// Moves all common call options into the given message builder.
///
/// In particular, the following options are moved:
/// * ttl
/// * message ID
/// * priority
pub(crate) fn apply_common_options(
    call_options: CallOptions,
    message_builder: &mut UMessageBuilder,
) {
    message_builder.with_ttl(call_options.ttl);
    if let Some(v) = call_options.message_id {
        message_builder.with_message_id(v);
    }
    if let Some(v) = call_options.priority {
        message_builder.with_priority(v);
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
