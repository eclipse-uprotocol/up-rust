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

use crate::uattributes::NotificationValidator;
#[cfg(feature = "protobuf-support")]
use crate::ProtobufMappable;
use crate::{
    PublishValidator, RequestValidator, ResponseValidator, UAttributes, UAttributesValidator,
    UCode, UMessage, UMessageError, UMessageType, UPayloadFormat, UPriority, UUri, UUID,
};

mod sealed {
    pub trait Sealed {}
}

/// Represents the type of message being built by a [UMessageBuilder].
/// This is used to limit the available builder functions and to ensure that the
/// builder is used in a consistent way.
/// For example, only a builder for a request message can set the `permission_level` and `token` attributes
/// and only a builder for a response message can set the `comm_status` and `request_id` attributes.
/// This is achieved by having different builder state types for each message type and by using the
/// [BuilderState::merge_into_attributes] function to add the state-specific attributes to the
/// final message attributes when building the message.
///
/// _Note_: Only this crate can provide implementations of [Self] because of the dependency
/// on a sealed trait. This is relevant because otherwise, external crates could add custom
/// constructor functions to [UMessageBuilder] that return malicious implementations of [Self]
/// which might produce invalid [UMessage] instances by means of the [Self::merge_into_attributes]
/// function.
pub trait BuilderState: sealed::Sealed {
    fn merge_into_attributes(&self, _attributes: &mut UAttributes) {}
}

pub struct InitialBuilderState;
impl sealed::Sealed for InitialBuilderState {}
impl BuilderState for InitialBuilderState {}

pub struct PublishBuilderState;
impl sealed::Sealed for PublishBuilderState {}
impl BuilderState for PublishBuilderState {}

pub struct NotificationBuilderState;
impl sealed::Sealed for NotificationBuilderState {}
impl BuilderState for NotificationBuilderState {}

pub struct RequestBuilderState {
    permission_level: Option<u32>,
    token: Option<String>,
}
impl sealed::Sealed for RequestBuilderState {}
impl BuilderState for RequestBuilderState {
    fn merge_into_attributes(&self, attributes: &mut UAttributes) {
        attributes.permission_level = self.permission_level;
        attributes.token = self.token.clone();
    }
}

pub struct ResponseBuilderState {
    comm_status: Option<UCode>,
    request_id: UUID,
}
impl sealed::Sealed for ResponseBuilderState {}
impl BuilderState for ResponseBuilderState {
    fn merge_into_attributes(&self, attributes: &mut UAttributes) {
        attributes.commstatus = self.comm_status;
        attributes.reqid = Some(self.request_id.clone());
    }
}

struct CommonAttributes {
    message_type: UMessageType,
    source: UUri,
    message_id: Option<UUID>,
    sink: Option<UUri>,
    ttl: Option<u32>,
    priority: Option<UPriority>,
    traceparent: Option<String>,
    payload_format: Option<UPayloadFormat>,
    payload: Option<Bytes>,
    validator: Box<dyn UAttributesValidator>,
}

/// A builder for creating [`UMessage`]s.
///
/// Messages are used by uEntities to inform other entities about the occurrence of events
/// and/or to invoke service operations provided by other entities.
///
/// For each type of message there is a dedicated builder which ensures that only attributes
/// relevant to that message type are set.
pub struct UMessageBuilder<S: BuilderState> {
    common: CommonAttributes,
    extra: S,
}

impl UMessageBuilder<InitialBuilderState> {
    /// Gets a builder for creating *publish* messages.
    ///
    /// A publish message is used to notify all interested consumers of an event that has occurred.
    /// Consumers usually indicate their interest by *subscribing* to a particular topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to publish the message to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic.clone())
    ///                    .build_with_payload("closed", UPayloadFormat::Text)?;
    /// assert_eq!(message.type_(), UMessageType::Publish);
    /// assert!(message.priority().is_none());
    /// assert_eq!(message.source(), &topic);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn publish(topic: UUri) -> UMessageBuilder<PublishBuilderState> {
        let common = CommonAttributes {
            // [impl->dsn~up-attributes-publish-type~1]
            message_type: UMessageType::Publish,
            source: topic,
            message_id: None,
            sink: None,
            ttl: None,
            priority: None,
            traceparent: None,
            payload_format: None,
            payload: None,
            validator: Box::new(PublishValidator),
        };
        UMessageBuilder {
            common,
            extra: PublishBuilderState,
        }
    }

    /// Gets a builder for creating *notification* messages.
    ///
    /// A notification is used to inform a specific consumer about an event that has occurred.
    ///
    /// # Arguments
    ///
    /// * `origin` - The component that the notification originates from.
    /// * `destination` - The URI identifying the destination to send the notification to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let origin = UUri::try_from("//my-vehicle/4210/5/F20B")?;
    /// let destination = UUri::try_from("//my-cloud/CCDD/2/0")?;
    /// let message = UMessageBuilder::notification(origin.clone(), destination.clone())
    ///                    .build_with_payload("unexpected movement", UPayloadFormat::Text)?;
    /// assert_eq!(message.type_(), UMessageType::Notification);
    /// assert!(message.priority().is_none());
    /// assert_eq!(message.source(), &origin);
    /// assert_eq!(message.sink_unchecked(), &destination);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn notification(
        origin: UUri,
        destination: UUri,
    ) -> UMessageBuilder<NotificationBuilderState> {
        let common = CommonAttributes {
            // [impl->dsn~up-attributes-notification-type~1]
            message_type: UMessageType::Notification,
            source: origin,
            message_id: None,
            sink: Some(destination),
            ttl: None,
            priority: None,
            traceparent: None,
            payload_format: None,
            payload: None,
            validator: Box::new(NotificationValidator),
        };
        UMessageBuilder {
            common,
            extra: NotificationBuilderState,
        }
    }

    /// Gets a builder for creating RPC *request* messages.
    ///
    /// A request message is used to invoke a service's method with some input data, expecting
    /// the service to reply with a response message which is correlated by means of the `request_id`.
    ///
    /// The builder will be initialized with [`UPriority::CS4`].
    ///
    /// # Arguments
    ///
    /// * `method_to_invoke` - The URI identifying the method to invoke.
    /// * `reply_to_address` - The URI that the sender of the request expects the response message at.
    /// * `ttl` - The number of milliseconds after which the request should no longer be processed
    ///   by the target service. The value is capped at [`i32::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let message = UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
    ///                    .build_with_payload("lock", UPayloadFormat::Text)?;
    /// assert_eq!(message.type_(), UMessageType::Request);
    /// assert_eq!(message.priority_unchecked(), UPriority::CS4);
    /// assert_eq!(message.source(), &reply_to_address);
    /// assert_eq!(message.sink_unchecked(), &method_to_invoke);
    /// assert_eq!(message.ttl_unchecked(), 5000);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn request(
        method_to_invoke: UUri,
        reply_to_address: UUri,
        ttl: u32,
    ) -> UMessageBuilder<RequestBuilderState> {
        let common = CommonAttributes {
            // [impl->dsn~up-attributes-request-type~1]
            message_type: UMessageType::Request,
            source: reply_to_address,
            message_id: None,
            sink: Some(method_to_invoke),
            ttl: Some(ttl),
            priority: Some(UPriority::CS4),
            traceparent: None,
            payload_format: None,
            payload: None,
            validator: Box::new(RequestValidator),
        };
        UMessageBuilder {
            common,
            extra: RequestBuilderState {
                permission_level: None,
                token: None,
            },
        }
    }

    /// Gets a builder for creating RPC *response* messages.
    ///
    /// A response message is used to send the outcome of processing a request message
    /// to the original sender of the request message.
    ///
    /// The builder will be initialized with [`UPriority::CS4`].
    ///
    /// # Arguments
    ///
    /// * `reply_to_address` - The URI that the sender of the request expects to receive the response message at.
    /// * `request_id` - The identifier of the request that this is the response to.
    /// * `invoked_method` - The URI identifying the method that has been invoked and which the created message is
    ///   the outcome of.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address.clone(), request_id.clone(), invoked_method.clone())
    ///                    .build()?;
    /// assert_eq!(message.type_(), UMessageType::Response);
    /// assert_eq!(message.priority_unchecked(), UPriority::CS4);
    /// assert_eq!(message.source(), &invoked_method);
    /// assert_eq!(message.sink_unchecked(), &reply_to_address);
    /// assert_eq!(message.request_id_unchecked(), &request_id);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn response(
        reply_to_address: UUri,
        request_id: UUID,
        invoked_method: UUri,
    ) -> UMessageBuilder<ResponseBuilderState> {
        let common = CommonAttributes {
            // [impl->dsn~up-attributes-response-type~1]
            message_type: UMessageType::Response,
            source: invoked_method,
            message_id: None,
            sink: Some(reply_to_address),
            ttl: None,
            priority: Some(UPriority::CS4),
            traceparent: None,
            payload_format: None,
            payload: None,
            validator: Box::new(ResponseValidator),
        };
        UMessageBuilder {
            common,
            extra: ResponseBuilderState {
                comm_status: None,
                request_id,
            },
        }
    }

    /// Gets a builder for creating an RPC *response* message in reply to a *request*.
    ///
    /// A response message is used to send the outcome of processing a request message
    /// to the original sender of the request message.
    ///
    /// The builder will be initialized with values from the given request attributes.
    ///
    /// # Arguments
    ///
    /// * `request_attributes` - The attributes from the request message. The response message
    ///   builder will be initialized with the corresponding attribute values.
    ///
    /// # Panics
    ///
    /// if the given attributes do not represent a request message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_message_id = UUID::build();
    /// let request_message = UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
    ///                           .with_message_id(request_message_id.clone()) // normally not needed, used only for asserts below
    ///                           .build_with_payload("lock", UPayloadFormat::Text)?;
    ///
    /// let response_message = UMessageBuilder::response_for_request(request_message.attributes())
    ///                           .with_priority(UPriority::CS5)
    ///                           .build()?;
    /// assert_eq!(response_message.type_(), UMessageType::Response);
    /// assert_eq!(response_message.priority_unchecked(), UPriority::CS5);
    /// assert_eq!(response_message.source(), &method_to_invoke);
    /// assert_eq!(response_message.sink_unchecked(), &reply_to_address);
    /// assert_eq!(response_message.request_id_unchecked(), &request_message_id);
    /// assert_eq!(response_message.ttl_unchecked(), 5000);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn response_for_request(
        request_attributes: &UAttributes,
    ) -> UMessageBuilder<ResponseBuilderState> {
        assert!(
            request_attributes.is_request(),
            "Given attributes do not represent a request message"
        );
        let common = CommonAttributes {
            // [impl->dsn~up-attributes-response-type~1]
            message_type: UMessageType::Response,
            source: request_attributes
                .sink()
                .expect("Request attributes must contain a sink")
                .to_owned(),
            message_id: None,
            sink: Some(request_attributes.source().to_owned()),
            ttl: request_attributes.ttl(),
            priority: request_attributes.priority(),
            traceparent: None,
            payload_format: None,
            payload: None,
            validator: Box::new(ResponseValidator),
        };
        UMessageBuilder {
            common,
            extra: ResponseBuilderState {
                comm_status: None,
                request_id: request_attributes.id().to_owned(),
            },
        }
    }
}

impl<S: BuilderState> UMessageBuilder<S> {
    /// Sets the message's identifier.
    ///
    /// Every message must have an identifier. However, if this function is not used, an
    /// identifier will be generated for the message when one of the `build` functions is called.
    ///
    /// This method is mainly useful for implementing tests that include assertions on the message ID.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The identifier to use.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let mut builder = UMessageBuilder::publish(topic);
    /// let message_one = builder
    ///                     .with_message_id(UUID::build())
    ///                     .build_with_payload("closed", UPayloadFormat::Text)?;
    /// let message_two = builder
    ///                     // use new message ID but retain all other attributes
    ///                     .with_message_id(UUID::build())
    ///                     .build_with_payload("open", UPayloadFormat::Text)?;
    /// assert_ne!(message_one.id(), message_two.id());
    /// assert_eq!(message_one.source(), message_two.source());
    /// assert_eq!(message_one.payload_format_unchecked(), message_two.payload_format_unchecked());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_message_id(&mut self, message_id: UUID) -> &mut Self {
        self.common.message_id = Some(message_id);
        self
    }

    /// Sets the message's priority.
    ///
    /// If not set explicitly, the default priority as defined in the
    /// [uProtocol specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/upriority.adoc)
    /// is used.
    ///
    /// # Arguments
    ///
    /// * `priority` - The priority to be used for sending the message.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic)
    ///                   .with_priority(UPriority::CS5)
    ///                   .build_with_payload("closed", UPayloadFormat::Text)?;
    /// assert_eq!(message.priority_unchecked(), UPriority::CS5);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_priority(&mut self, priority: UPriority) -> &mut Self {
        if UAttributes::is_default_priority(priority) {
            // no need to explicitly set default priority, as the absence of a
            // priority attribute on the message will be interpreted as the default priority
            self.common.priority = None;
        } else {
            self.common.priority = Some(priority);
        }
        self
    }

    /// Sets the message's time-to-live.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The time-to-live in milliseconds. The value is capped at [`i32::MAX`].
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_msg_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_msg_id, invoked_method)
    ///                     .with_ttl(2000)
    ///                     .build()?;
    /// assert_eq!(message.ttl_unchecked(), 2000);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_ttl(&mut self, ttl: u32) -> &mut Self {
        self.common.ttl = Some(ttl);
        self
    }

    /// Sets the identifier of the W3C Trace Context to convey in the message.
    ///
    /// # Arguments
    ///
    /// * `traceparent` - The identifier.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    /// let message = UMessageBuilder::publish(topic.clone())
    ///                    .with_traceparent(traceparent)
    ///                    .build_with_payload("closed", UPayloadFormat::Text)?;
    /// assert_eq!(message.traceparent(), Some(traceparent));
    /// # Ok(())
    /// # }
    pub fn with_traceparent<T: Into<String>>(&mut self, traceparent: T) -> &mut Self {
        // [impl->dsn~up-attributes-traceparent~1]
        self.common.traceparent = Some(traceparent.into());
        self
    }

    /// Creates the message based on the builder's state.
    ///
    /// # Returns
    ///
    /// A message ready to be sent using [`crate::UTransport::send`].
    ///
    /// # Errors
    ///
    /// If the properties set on the builder do not represent a consistent set of [`UAttributes`],
    /// a [`UMessageError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ## Not setting `id` explicitly with [`UMessageBuilder::with_message_id`]
    ///
    /// The recommended way to use the `UMessageBuilder`.
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UAttributesValidators, UMessageBuilder, UMessageError, UMessageType, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let result = UMessageBuilder::response(reply_to_address, UUID::build(), invoked_method)
    ///                     .build();
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Setting `id` explicitly with [`UMessageBuilder::with_message_id`]
    ///
    /// Note that explicitly using [`UMessageBuilder::with_message_id`] is not required as shown
    /// above.
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let message_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, UUID::build(), invoked_method)
    ///                     .with_message_id(message_id.clone())
    ///                     .build()?;
    /// assert_eq!(message.id(), &message_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(&self) -> Result<UMessage, UMessageError> {
        // [impl->dsn~up-attributes-id~1]
        let message_id = self
            .common
            .message_id
            .as_ref()
            .map_or_else(UUID::build, |id| id.clone());
        let mut attributes = UAttributes {
            type_: self.common.message_type,
            id: message_id,
            source: self.common.source.to_owned(),
            sink: self.common.sink.to_owned(),
            ttl: self.common.ttl,
            priority: self.common.priority,
            traceparent: self.common.traceparent.to_owned(),
            payload_format: self.common.payload_format,
            token: None, // token is only relevant for request messages and is set via the RequestBuilderState
            permission_level: None, // permission_level is only relevant for request messages and is set via the RequestBuilderState
            commstatus: None, // commstatus is only relevant for response messages and is set via the ResponseBuilderState
            reqid: None, // reqid is only relevant for response messages and is set via the ResponseBuilderState
        };
        // add state-specific attributes
        self.extra.merge_into_attributes(&mut attributes);
        // make sure that we have created a valid set of attributes before creating the final message
        self.common
            .validator
            .validate(&attributes)
            .map_err(UMessageError::from)
            .and_then(|_| UMessage::new(attributes, self.common.payload.clone()))
    }

    /// Creates the message based on the builder's state and some payload.
    ///
    /// # Arguments
    ///
    /// * `payload` - The data to set as payload.
    /// * `format` - The payload format.
    ///
    /// # Returns
    ///
    /// A message ready to be sent using [`crate::UTransport::send`].
    ///
    /// # Errors
    ///
    /// If the properties set on the builder do not represent a consistent set of [`UAttributes`],
    /// a [`UMessageError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic)
    ///                    .build_with_payload("locked", UPayloadFormat::Text)?;
    /// assert!(message.payload().is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_payload<T: Into<Bytes>>(
        &mut self,
        payload: T,
        format: UPayloadFormat,
    ) -> Result<UMessage, UMessageError> {
        self.common.payload = Some(payload.into());
        // [impl->dsn~up-attributes-payload-format~1]
        self.common.payload_format = Some(format);

        self.build()
    }

    /// Creates the message based on the builder's state and some payload.
    ///
    /// # Arguments
    ///
    /// * `payload` - The data to set as payload.
    ///
    /// # Returns
    ///
    /// A message ready to be sent using [`crate::UTransport::send`]. The message will have
    /// [`UPayloadFormat::Protobuf`] set as its payload format.
    ///
    /// # Errors
    ///
    /// If the given payload cannot be serialized into a protobuf byte array, a [`UMessageError::DataSerializationError`] is returned.
    /// If the properties set on the builder do not represent a consistent set of [`UAttributes`],
    /// a [`UMessageError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protobuf::well_known_types::wrappers::StringValue;
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UStatus, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let payload = StringValue::new();
    /// let request_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_id, invoked_method)
    ///                    .with_comm_status(UCode::InvalidArgument)
    ///                    .build_with_protobuf_payload(&payload)?;
    /// assert!(message.payload().is_some());
    /// assert_eq!(message.payload_format_unchecked(), UPayloadFormat::Protobuf);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "protobuf-support")]
    pub fn build_with_protobuf_payload<T: ProtobufMappable>(
        &mut self,
        payload: &T,
    ) -> Result<UMessage, UMessageError> {
        payload
            .write_to_protobuf_bytes()
            .map_err(UMessageError::from)
            .and_then(|serialized_payload| {
                self.build_with_payload(serialized_payload, UPayloadFormat::Protobuf)
            })
    }

    /// Creates the message based on the builder's state and some payload.
    ///
    /// # Arguments
    ///
    /// * `payload` - The data to set as payload.
    ///
    /// # Returns
    ///
    /// A message ready to be sent using [`crate::UTransport::send`]. The message will have
    /// [`UPayloadFormat::ProtobufWrappedInAny`] set as its payload format.
    ///
    /// # Errors
    ///
    /// If the given payload cannot be serialized into a protobuf byte array, a [`UMessageError::DataSerializationError`] is returned.
    /// If the properties set on the builder do not represent a consistent set of [`UAttributes`],
    /// a [`UMessageError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protobuf::well_known_types::wrappers::StringValue;
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UStatus, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let payload = StringValue::new();
    /// let request_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_id, invoked_method)
    ///                    .with_comm_status(UCode::InvalidArgument)
    ///                    .build_with_wrapped_protobuf_payload(&payload)?;
    /// assert!(message.payload().is_some());
    /// assert_eq!(message.payload_format_unchecked(), UPayloadFormat::ProtobufWrappedInAny);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "protobuf-support")]
    pub fn build_with_wrapped_protobuf_payload<T: ProtobufMappable>(
        &mut self,
        payload: &T,
    ) -> Result<UMessage, UMessageError> {
        payload
            .write_to_packed_protobuf_bytes()
            .map_err(UMessageError::from)
            .and_then(|serialized_payload| {
                self.build_with_payload(serialized_payload, UPayloadFormat::ProtobufWrappedInAny)
            })
    }
}

impl UMessageBuilder<RequestBuilderState> {
    /// Sets the message's authorization token used for TAP.
    ///
    /// # Arguments
    ///
    /// * `token` - The token.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let token = "this-is-my-token";
    /// let message = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000)
    ///                     .with_token(token)
    ///                     .build_with_payload("lock", UPayloadFormat::Text)?;
    /// assert_eq!(message.token(), Some(token));
    /// # Ok(())
    /// # }
    /// ```
    // [impl->dsn~up-attributes-request-token~1]
    pub fn with_token<T: Into<String>>(&mut self, token: T) -> &mut Self {
        self.extra.token = Some(token.into());
        self
    }

    /// Sets the message's permission level.
    ///
    /// # Arguments
    ///
    /// * `level` - The level.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let message = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000)
    ///                     .with_permission_level(12)
    ///                     .build_with_payload("lock", UPayloadFormat::Text)?;
    /// assert_eq!(message.permission_level(), Some(12));
    /// # Ok(())
    /// # }
    /// ```
    // [impl->dsn~up-attributes-permission-level~1]
    pub fn with_permission_level(&mut self, level: u32) -> &mut Self {
        self.extra.permission_level = Some(level);
        self
    }
}

impl UMessageBuilder<ResponseBuilderState> {
    /// Sets the message's communication status.
    ///
    /// # Arguments
    ///
    /// * `comm_status` - The status.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_msg_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_msg_id, invoked_method)
    ///                     .with_comm_status(UCode::Ok)
    ///                     .build()?;
    /// assert_eq!(message.commstatus_unchecked(), UCode::Ok);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_comm_status(&mut self, comm_status: UCode) -> &mut Self {
        self.extra.comm_status = Some(comm_status);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "up-core-types")]
    use crate::ProtobufMappable;

    use test_case::test_case;

    const METHOD_TO_INVOKE: &str = "//my-vehicle/4D123/2/6FA3";
    const REPLY_TO_ADDRESS: &str = "//my-cloud/9CB3/1/0";
    const TOPIC: &str = "//my-vehicle/4210/1/B24D";

    #[test_case(UPriority::CS0; "with priority CS0")]
    #[test_case(UPriority::CS1; "with priority CS1")]
    #[test_case(UPriority::CS2; "with priority CS2")]
    #[test_case(UPriority::CS3; "with priority CS3")]
    // [utest->dsn~up-attributes-request-priority~1]
    fn test_rpc_message_builders_fail(priority: UPriority) {
        let request_id = UUID::build();
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        assert!(
            UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
                .with_priority(priority)
                .build()
                .is_err()
        );
        assert!(
            UMessageBuilder::response(reply_to_address, request_id, method_to_invoke)
                .with_priority(priority)
                .build()
                .is_err()
        );
    }

    #[test]
    fn test_build_supports_repeated_invocation() {
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let mut builder = UMessageBuilder::publish(topic);
        let message_one = builder
            .with_message_id(UUID::build())
            .build_with_payload("locked", UPayloadFormat::Text)
            .expect("should have been able to create message");
        let message_two = builder
            .with_message_id(UUID::build())
            .build_with_payload("unlocked", UPayloadFormat::Text)
            .expect("should have been able to create message");
        assert_eq!(message_one.attributes.type_, message_two.attributes.type_);
        assert_ne!(message_one.attributes.id, message_two.attributes.id);
        assert_eq!(message_one.attributes.source, message_two.attributes.source);
        assert_ne!(message_one.payload, message_two.payload);
    }

    #[test]
    // [utest->req~uattributes-data-model-impl~1]
    // [utest->req~umessage-data-model-impl~1]
    fn test_build_retains_all_publish_attributes() {
        let message_id = UUID::build();
        let traceparent = "traceparent";
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let message = UMessageBuilder::publish(topic.clone())
            .with_message_id(message_id.clone())
            .with_ttl(5000)
            .with_traceparent(traceparent)
            .build_with_payload("locked", UPayloadFormat::Text)
            .expect("should have been able to create message");

        // [utest->dsn~up-attributes-id~1]
        assert_eq!(message.id(), &message_id);
        assert!(message.priority().is_none());
        assert_eq!(message.source(), &topic);
        // [utest->dsn~up-attributes-publish-sink~1]
        assert!(message.sink().is_none());
        assert_eq!(message.ttl_unchecked(), 5000);
        // [utest->dsn~up-attributes-traceparent~1]
        assert_eq!(message.traceparent(), Some(traceparent));
        // [utest->dsn~up-attributes-publish-type~1]
        assert_eq!(message.type_(), UMessageType::Publish);
        // [utest->dsn~up-attributes-payload-format~1]
        assert_eq!(message.payload_format_unchecked(), UPayloadFormat::Text);

        assert!(message.commstatus().is_none());
        assert!(message.permission_level().is_none());
        assert!(message.token().is_none());

        #[cfg(feature = "up-core-types")]
        {
            // [utest->req~uattributes-data-model-proto~1]
            // [utest->req~umessage-data-model-proto~1]
            let proto = message
                .write_to_protobuf_bytes()
                .expect("failed to serialize to protobuf");
            let deserialized_message = UMessage::parse_from_protobuf_bytes(proto.as_slice())
                .expect("failed to deserialize protobuf");
            assert_eq!(message, deserialized_message);
        }
    }

    #[test]
    // [utest->req~uattributes-data-model-impl~1]
    // [utest->req~umessage-data-model-impl~1]
    fn test_build_retains_all_notification_attributes() {
        let message_id = UUID::build();
        let traceparent = "traceparent";
        let origin = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let destination =
            UUri::try_from(REPLY_TO_ADDRESS).expect("should have been able to create UUri");
        let message = UMessageBuilder::notification(origin.clone(), destination.clone())
            .with_message_id(message_id.clone())
            .with_priority(UPriority::CS2)
            .with_ttl(5000)
            .with_traceparent(traceparent)
            .build_with_payload("locked", UPayloadFormat::Text)
            .expect("should have been able to create message");

        // [utest->dsn~up-attributes-id~1]
        assert_eq!(message.id(), &message_id);
        assert_eq!(message.priority_unchecked(), UPriority::CS2);
        assert_eq!(message.source(), &origin);
        assert_eq!(message.sink_unchecked(), &destination);
        assert_eq!(message.ttl_unchecked(), 5000);
        // [utest->dsn~up-attributes-traceparent~1]
        assert_eq!(message.traceparent(), Some(traceparent));
        // [utest->dsn~up-attributes-notification-type~1]
        assert!(message.is_notification());
        // [utest->dsn~up-attributes-payload-format~1]
        assert_eq!(message.payload_format_unchecked(), UPayloadFormat::Text);

        assert!(message.commstatus().is_none());
        assert!(message.permission_level().is_none());
        assert!(message.token().is_none());

        #[cfg(feature = "up-core-types")]
        {
            // [utest->req~uattributes-data-model-proto~1]
            // [utest->req~umessage-data-model-proto~1]
            let proto = message
                .write_to_protobuf_bytes()
                .expect("failed to serialize to protobuf");
            let deserialized_message = UMessage::parse_from_protobuf_bytes(proto.as_slice())
                .expect("failed to deserialize protobuf");
            assert_eq!(message, deserialized_message);
        }
    }

    #[test]
    // [utest->req~uattributes-data-model-impl~1]
    // [utest->req~umessage-data-model-impl~1]
    fn test_build_retains_all_request_attributes() {
        let message_id = UUID::build();
        let token = "token";
        let traceparent = "traceparent";
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let message =
            UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
                .with_message_id(message_id.clone())
                .with_permission_level(5)
                .with_priority(UPriority::CS4)
                .with_token(token)
                .with_traceparent(traceparent)
                .build_with_payload("unlock", UPayloadFormat::Text)
                .expect("should have been able to create message");

        // [utest->dsn~up-attributes-id~1]
        assert_eq!(message.id(), &message_id);
        // [utest->dsn~up-attributes-permission-level~1]
        assert_eq!(message.permission_level(), Some(5));
        assert_eq!(message.priority_unchecked(), UPriority::CS4);
        assert_eq!(message.sink_unchecked(), &method_to_invoke);
        assert_eq!(message.source(), &reply_to_address);
        // [utest->dsn~up-attributes-request-token~1]
        assert_eq!(message.token(), Some(token));
        assert_eq!(message.ttl_unchecked(), 5000);
        // [utest->dsn~up-attributes-traceparent~1]
        assert_eq!(message.traceparent(), Some(traceparent));
        // [utest->dsn~up-attributes-request-type~1]
        assert!(message.is_request());
        // [utest->dsn~up-attributes-payload-format~1]
        assert_eq!(message.payload_format_unchecked(), UPayloadFormat::Text);

        assert!(message.commstatus().is_none());
        assert!(message.request_id().is_none());

        // [utest->req~uattributes-data-model-proto~1]
        // [utest->req~umessage-data-model-proto~1]
        #[cfg(feature = "up-core-types")]
        {
            let proto = message
                .write_to_protobuf_bytes()
                .expect("failed to serialize to protobuf");
            let deserialized_message = UMessage::parse_from_protobuf_bytes(proto.as_slice())
                .expect("failed to deserialize protobuf");
            assert_eq!(message, deserialized_message);
        }
    }

    #[test]
    #[should_panic]
    fn test_response_for_request_panics_for_non_request_attributes() {
        let message_id = UUID::build();
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let request_message = UMessageBuilder::publish(topic.clone())
            .with_message_id(message_id.clone())
            .build()
            .expect("should have been able to create message");

        let _ = UMessageBuilder::response_for_request(&request_message.attributes).build();
    }

    #[test]
    fn test_builder_copies_request_attributes() {
        let request_message_id = UUID::build();
        let response_message_id = UUID::build();
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let request_message =
            UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
                .with_message_id(request_message_id.clone())
                .with_priority(UPriority::CS5)
                .build()
                .expect("should have been able to create message");
        let message = UMessageBuilder::response_for_request(&request_message.attributes)
            .with_message_id(response_message_id.clone())
            .with_comm_status(UCode::DeadlineExceeded)
            .build()
            .expect("should have been able to create message");
        // [utest->dsn~up-attributes-id~1]
        assert_eq!(message.id(), &response_message_id);
        assert_eq!(message.commstatus_unchecked(), UCode::DeadlineExceeded);
        assert_eq!(
            message.priority_unchecked(),
            request_message.priority_unchecked()
        );
        assert_eq!(message.request_id_unchecked(), &request_message_id);
        assert_eq!(message.sink_unchecked(), &reply_to_address);
        assert_eq!(message.source(), &method_to_invoke);
        assert_eq!(message.ttl_unchecked(), 5000);
        // [utest->dsn~up-attributes-response-type~1]
        assert!(message.is_response());
        assert!(message.payload_format().is_none());
        assert!(message.payload.is_none());

        assert!(message.permission_level().is_none());
        assert!(message.token().is_none());
    }

    #[test]
    // [utest->req~uattributes-data-model-impl~1]
    // [utest->req~umessage-data-model-impl~1]
    fn test_build_retains_all_response_attributes() {
        let message_id = UUID::build();
        let request_id = UUID::build();
        let traceparent = "traceparent";
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let message = UMessageBuilder::response(
            reply_to_address.clone(),
            request_id.clone(),
            method_to_invoke.clone(),
        )
        .with_message_id(message_id.clone())
        .with_comm_status(UCode::DeadlineExceeded)
        .with_priority(UPriority::CS5)
        .with_ttl(4000)
        .with_traceparent(traceparent)
        .build()
        .expect("should have been able to create message");

        // [utest->dsn~up-attributes-id~1]
        assert_eq!(message.id(), &message_id);
        assert_eq!(message.commstatus_unchecked(), UCode::DeadlineExceeded);
        assert_eq!(message.priority_unchecked(), UPriority::CS5);
        assert_eq!(message.request_id_unchecked(), &request_id);
        assert_eq!(message.sink_unchecked(), &reply_to_address);
        assert_eq!(message.source(), &method_to_invoke);
        assert_eq!(message.ttl_unchecked(), 4000);
        // [utest->dsn~up-attributes-traceparent~1]
        assert_eq!(message.traceparent(), Some(traceparent));
        // [utest->dsn~up-attributes-response-type~1]
        assert!(message.is_response());
        assert!(message.payload_format().is_none());
        assert!(message.payload.is_none());

        assert!(message.permission_level().is_none());
        assert!(message.token().is_none());

        // [utest->req~uattributes-data-model-proto~1]
        // [utest->req~umessage-data-model-proto~1]
        #[cfg(feature = "up-core-types")]
        {
            let proto = message
                .write_to_protobuf_bytes()
                .expect("failed to serialize to protobuf");
            let deserialized_message = UMessage::parse_from_protobuf_bytes(proto.as_slice())
                .expect("failed to deserialize protobuf");
            assert_eq!(message, deserialized_message);
        }
    }
}
