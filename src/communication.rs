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

/*!
Traits representing uProtocol's [Communication Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l2/api.adoc) for publishing and subscribing to topics and for invoking RPC methods. Also contains default implementations of the traits employing uProtocol's [Transport & Session Layer API](crate::UTransport).
*/

use bytes::Bytes;
use thiserror::Error;

use crate::{UCode, UPayloadFormat, UPriority, UStatus, UUID};

mod notification;
#[cfg(any(test, feature = "test-util"))]
pub use notification::MockNotifier;
pub use notification::{NotificationError, Notifier};

#[cfg(feature = "up-l2-notifier")]
mod simple_notifier;
#[cfg(feature = "up-l2-notifier")]
pub use simple_notifier::SimpleNotifier;

mod pubsub;
#[cfg(any(test, feature = "test-util"))]
pub use pubsub::MockSubscriptionChangeHandler;
pub use pubsub::{PubSubError, Publisher, Subscriber, SubscriptionChangeHandler};

#[cfg(feature = "up-l2-publisher")]
mod simple_publisher;
#[cfg(feature = "up-l2-publisher")]
pub use simple_publisher::SimplePublisher;

#[cfg(feature = "up-l2-subscriber")]
mod in_memory_subscriber;
#[cfg(feature = "up-l2-subscriber")]
pub use in_memory_subscriber::InMemorySubscriber;

mod rpc;
#[cfg(any(test, feature = "test-util"))]
pub use rpc::{MockRequestHandler, MockRpcClient, MockRpcServerImpl};
pub use rpc::{RequestHandler, RpcClient, RpcServer, ServiceInvocationError};

#[cfg(feature = "up-l2-rpc-client")]
mod in_memory_rpc_client;
#[cfg(feature = "up-l2-rpc-client")]
pub use in_memory_rpc_client::InMemoryRpcClient;

#[cfg(feature = "up-l2-rpc-server")]
mod in_memory_rpc_server;
#[cfg(feature = "up-l2-rpc-server")]
pub use in_memory_rpc_server::InMemoryRpcServer;

/// Moves all common call options into the given message builder.
///
/// In particular, the following options are moved:
/// * ttl
/// * message ID
/// * priority
#[cfg(any(feature = "up-l2-notifier", feature = "up-l2-publisher"))]
pub(crate) fn apply_common_options(
    call_options: CallOptions,
    message_builder: &mut crate::UMessageBuilder,
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
#[cfg(any(
    feature = "up-l2-notifier",
    feature = "up-l2-publisher",
    feature = "up-l2-rpc-client",
    feature = "up-l2-rpc-server"
))]
pub(crate) fn build_message(
    message_builder: &mut crate::UMessageBuilder,
    payload: Option<UPayload>,
) -> Result<crate::UMessage, crate::UMessageError> {
    if let Some(pl) = payload {
        let format = pl.payload_format();
        message_builder.build_with_payload(pl.payload, format)
    } else {
        message_builder.build()
    }
}

/// The current status of a client's subscription to a topic.
///
/// The status goes through different stages during its lifecycle as defined in
/// [uProtocol Specification, section 3.3.5](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l3/usubscription/v3/README.adoc#usubscription-states).
#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub enum SubscriptionStatus {
    Unsubscribed,
    SubscribePending,
    Subscribed,
    UnsubscribePending,
}

#[cfg(all(feature = "up-core-types", feature = "usubscription"))]
mod core_types_support {
    use super::{SubscriptionStatus, UCode, UStatus};
    use crate::up_core_api::usubscription::subscription_status::State;
    use crate::up_core_api::usubscription::SubscriptionStatus as SubscriptionStatusProto;

    impl TryFrom<&SubscriptionStatusProto> for SubscriptionStatus {
        type Error = UStatus;

        fn try_from(status_proto: &SubscriptionStatusProto) -> Result<Self, Self::Error> {
            let state = status_proto.state.enum_value();
            match state {
                Ok(State::UNSUBSCRIBED) => Ok(SubscriptionStatus::Unsubscribed),
                Ok(State::SUBSCRIBE_PENDING) => Ok(SubscriptionStatus::SubscribePending),
                Ok(State::SUBSCRIBED) => Ok(SubscriptionStatus::Subscribed),
                Ok(State::UNSUBSCRIBE_PENDING) => Ok(SubscriptionStatus::UnsubscribePending),
                Err(v) => Err(UStatus::fail_with_code(
                    UCode::InvalidArgument,
                    format!("unknown subscription status {:?}", v),
                )),
            }
        }
    }
}

/// An error indicating a problem with registering or unregistering a message listener.
#[derive(Clone, Debug, Error)]
pub enum RegistrationError {
    /// Indicates that a listener for a given address already exists.
    #[error("a listener for the given filter criteria already exists")]
    AlreadyExists,
    /// Indicates that the maximum number of listeners supported by the Transport Layer implementation
    /// has already been registered.
    #[error("maximum number of listeners has been reached")]
    MaxListenersExceeded,
    /// Indicates that no listener is registered for given pattern URIs.
    #[error("no listener registered for given pattern")]
    NoSuchListener,
    /// Indicates that the underlying Transport Layer implementation does not support registration and
    /// notification of message handlers.
    #[error("the underlying transport implementation does not support the push delivery method")]
    PushDeliveryMethodNotSupported,
    /// Indicates that some of the given filters are inappropriate in this context.
    #[error("invalid filter(s): {0}")]
    InvalidFilter(String),
    /// Indicates a generic error.
    #[error("error un-/registering listener: {0}")]
    Unknown(Box<UStatus>),
}

impl From<UStatus> for RegistrationError {
    fn from(value: UStatus) -> Self {
        match value.get_code() {
            UCode::AlreadyExists => RegistrationError::AlreadyExists,
            UCode::NotFound => RegistrationError::NoSuchListener,
            UCode::ResourceExhausted => RegistrationError::MaxListenersExceeded,
            UCode::Unimplemented => RegistrationError::PushDeliveryMethodNotSupported,
            UCode::InvalidArgument => {
                RegistrationError::InvalidFilter(value.get_message().unwrap_or("N/A").to_string())
            }
            UCode::Ok
            | UCode::Cancelled
            | UCode::Unknown
            | UCode::DeadlineExceeded
            | UCode::PermissionDenied
            | UCode::Unauthenticated
            | UCode::FailedPrecondition
            | UCode::Aborted
            | UCode::OutOfRange
            | UCode::Internal
            | UCode::Unavailable
            | UCode::DataLoss => RegistrationError::Unknown(Box::from(value)),
        }
    }
}

/// General options that clients might want to specify when sending a uProtocol message.
#[must_use = "CallOptions should be used when sending messages to specify message parameters"]
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
    /// let uuid = UUID::build();
    /// let token = String::from("token");
    /// let options = CallOptions::for_rpc_request(15_000, Some(uuid.clone()), Some(token.clone()), Some(UPriority::CS6));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(&uuid));
    /// assert_eq!(options.token(), Some(&token));
    /// assert_eq!(options.priority(), Some(UPriority::CS6));
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
    /// let uuid = UUID::build();
    /// let options = CallOptions::for_notification(Some(15_000), Some(uuid.clone()), Some(UPriority::CS2));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(&uuid));
    /// assert_eq!(options.priority(), Some(UPriority::CS2));
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
    /// let uuid = UUID::build();
    /// let options = CallOptions::for_publish(Some(15_000), Some(uuid.clone()), Some(UPriority::CS2));
    /// assert_eq!(options.ttl(), 15_000);
    /// assert_eq!(options.message_id(), Some(&uuid));
    /// assert_eq!(options.priority(), Some(UPriority::CS2));
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
    pub fn message_id(&self) -> Option<&UUID> {
        self.message_id.as_ref()
    }

    /// Gets the token to use for authenticating to infrastructure and service endpoints.
    pub fn token(&self) -> Option<&String> {
        self.token.as_ref()
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
    /// let payload = UPayload::new(data, UPayloadFormat::Raw);
    /// assert_eq!(payload.payload_format(), UPayloadFormat::Raw);
    /// assert_eq!(payload.payload().len(), 3);
    /// ```
    pub fn new<T: Into<Bytes>>(payload: T, payload_format: UPayloadFormat) -> Self {
        UPayload {
            payload_format,
            payload: payload.into(),
        }
    }

    /// Creates a new UPayload from an object that can be mapped to/from a protobuf.
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
    ///     pl.payload_format() == UPayloadFormat::ProtobufWrappedInAny
    ///         && pl.payload().len() > 0));
    /// ```
    #[cfg(feature = "protobuf-support")]
    pub fn try_from_protobuf<T>(obj: T) -> Result<Self, crate::UMessageError>
    where
        T: crate::ProtobufMappable,
    {
        obj.write_to_packed_protobuf_bytes()
            .map(|buf| UPayload::new(buf, UPayloadFormat::ProtobufWrappedInAny))
            .map_err(crate::UMessageError::from)
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
    #[must_use]
    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// Extracts the object that is contained in this message's payload.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Returns
    ///
    /// * The deserialized object contained in the payload.
    ///
    /// # Errors
    ///
    /// * Returns an error if the unpacking process fails, for example if the payload format
    ///   is neither [`UPayloadFormat::Protobuf`] nor [`UPayloadFormat::ProtobufWrappedInAny`],
    ///   or if the payload could not be deserialized into the target type `T`.
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
    #[cfg(feature = "protobuf-support")]
    pub fn extract_protobuf<T>(&self) -> Result<T, crate::UMessageError>
    where
        T: crate::ProtobufMappable + Default,
    {
        crate::umessage::deserialize_protobuf_bytes(&self.payload, &self.payload_format)
    }
}
