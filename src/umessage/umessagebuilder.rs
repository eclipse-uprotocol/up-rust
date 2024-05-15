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
use protobuf::{Enum, EnumOrUnknown, Message};

use crate::uattributes::{NotificationValidator, UAttributesError};
use crate::{
    PublishValidator, RequestValidator, ResponseValidator, UAttributes, UAttributesValidator,
    UCode, UMessage, UMessageType, UPayload, UPayloadFormat, UPriority, UUIDBuilder, UUri, UUID,
};

#[derive(Debug)]
pub enum UMessageBuilderError {
    DataSerializationError(protobuf::Error),
    AttributesValidationError(UAttributesError),
}

impl std::fmt::Display for UMessageBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataSerializationError(e) => {
                f.write_fmt(format_args!("Failed to serialize payload: {}", e))
            }
            Self::AttributesValidationError(e) => f.write_fmt(format_args!(
                "Builder state is not consistent with message type: {}",
                e
            )),
        }
    }
}

impl std::error::Error for UMessageBuilderError {}

impl From<UAttributesError> for UMessageBuilderError {
    fn from(value: UAttributesError) -> Self {
        Self::AttributesValidationError(value)
    }
}

impl From<protobuf::Error> for UMessageBuilderError {
    fn from(value: protobuf::Error) -> Self {
        Self::DataSerializationError(value)
    }
}

/// A builder for creating [`UMessage`]s.
///
/// Messages are being used by a uEntity to inform other entities about the occurrence of events
/// and/or to invoke service operations provided by other entities.
pub struct UMessageBuilder {
    validator: Box<dyn UAttributesValidator>,
    message_type: UMessageType,
    message_id: Option<UUID>,
    source: Option<UUri>,
    sink: Option<UUri>,
    priority: UPriority,
    ttl: Option<u32>,
    token: Option<String>,
    permission_level: Option<u32>,
    comm_status: Option<EnumOrUnknown<UCode>>,
    request_id: Option<UUID>,
    payload: Option<Bytes>,
    payload_format: UPayloadFormat,
}

impl Default for UMessageBuilder {
    fn default() -> Self {
        UMessageBuilder {
            validator: Box::new(PublishValidator),
            comm_status: None,
            message_type: UMessageType::UMESSAGE_TYPE_UNSPECIFIED,
            message_id: None,
            payload: None,
            payload_format: UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED,
            permission_level: None,
            priority: UPriority::UPRIORITY_CS1,
            request_id: None,
            sink: None,
            source: None,
            token: None,
            ttl: None,
        }
    }
}

impl UMessageBuilder {
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
    /// let topic = UUri::try_from("my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic.clone())
    ///                    .build_with_payload("closed".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_PUBLISH.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(message.attributes.source, Some(topic).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn publish(topic: UUri) -> UMessageBuilder {
        UMessageBuilder {
            validator: Box::new(PublishValidator),
            message_type: UMessageType::UMESSAGE_TYPE_PUBLISH,
            source: Some(topic),
            ..Default::default()
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
    /// let origin = UUri::try_from("my-vehicle/4210/5/F20B")?;
    /// let destination = UUri::try_from("my-cloud/CCDD/2/75FD")?;
    /// let message = UMessageBuilder::notification(origin.clone(), destination.clone())
    ///                    .build_with_payload("unexpected movement".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_NOTIFICATION.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(message.attributes.source, Some(origin).into());
    /// assert_eq!(message.attributes.sink, Some(destination).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn notification(origin: UUri, destination: UUri) -> UMessageBuilder {
        UMessageBuilder {
            validator: Box::new(NotificationValidator),
            message_type: UMessageType::UMESSAGE_TYPE_NOTIFICATION,
            source: Some(origin),
            sink: Some(destination),
            ..Default::default()
        }
    }

    /// Gets a builder for creating RPC *request* messages.
    ///
    /// A request message is used to invoke a service's method with some input data, expecting
    /// the service to reply with a response message which is correlated by means of the `request_id`.
    ///
    /// The builder will be initialized with [`UPriority::UPRIORITY_CS4`].
    ///
    /// # Arguments
    ///
    /// * `method_to_invoke` - The URI identifying the method to invoke.
    /// * `reply_to_address` - The URI that the sender of the request expects the response message at.
    /// * `ttl` - The number of milliseconds after which the request should no longer be processed
    ///           by the target service. The value is capped at [`i32::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let message = UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
    ///                    .build_with_payload("lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_REQUEST.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS4.into());
    /// assert_eq!(message.attributes.source, Some(reply_to_address).into());
    /// assert_eq!(message.attributes.sink, Some(method_to_invoke).into());
    /// assert_eq!(message.attributes.ttl, Some(5000));
    /// # Ok(())
    /// # }
    /// ```
    pub fn request(method_to_invoke: UUri, reply_to_address: UUri, ttl: u32) -> UMessageBuilder {
        UMessageBuilder {
            validator: Box::new(RequestValidator),
            message_type: UMessageType::UMESSAGE_TYPE_REQUEST,
            source: Some(reply_to_address),
            sink: Some(method_to_invoke),
            ttl: Some(ttl),
            priority: UPriority::UPRIORITY_CS4,
            ..Default::default()
        }
    }

    /// Gets a builder for creating RPC *response* messages.
    ///
    /// A response message is used to send the outcome of processing a request message
    /// to the original sender of the request message.
    ///
    /// The builder will be initialized with [`UPriority::UPRIORITY_CS4`].
    ///
    /// # Arguments
    ///
    /// * `reply_to_address` - The URI that the sender of the request expects to receive the response message at.
    /// * `request_id` - The identifier of the request that this is the response to.
    /// * `invoked_method` - The URI identifying the method that has been invoked and which the created message is
    ///                      the outcome of.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let request_id = UUIDBuilder::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address.clone(), request_id.clone(), invoked_method.clone())
    ///                    .build()?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_RESPONSE.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS4.into());
    /// assert_eq!(message.attributes.source, Some(invoked_method).into());
    /// assert_eq!(message.attributes.sink, Some(reply_to_address).into());
    /// assert_eq!(message.attributes.reqid, Some(request_id).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn response(
        reply_to_address: UUri,
        request_id: UUID,
        invoked_method: UUri,
    ) -> UMessageBuilder {
        UMessageBuilder {
            validator: Box::new(ResponseValidator),
            message_type: UMessageType::UMESSAGE_TYPE_RESPONSE,
            source: Some(invoked_method),
            sink: Some(reply_to_address),
            request_id: Some(request_id),
            priority: UPriority::UPRIORITY_CS4,
            ..Default::default()
        }
    }

    /// Gets a builder for creating RPC *response* messages in reply to a *request*.
    ///
    /// A response message is used to send the outcome of processing a request message
    /// to the original sender of the request message.
    ///
    /// The builder will be initialized with values from the given request attributes.
    ///
    /// # Arguments
    ///
    /// * `request_attributes` - The attributes from the request message. The response message builder will be initialized
    ///                          with the corresponding attribute values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let request_message_id = UUIDBuilder::build();
    /// let request_message = UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
    ///                           .with_message_id(request_message_id.clone()) // normally not needed, used only for asserts below
    ///                           .build_with_payload("lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    ///
    /// let response_message = UMessageBuilder::response_for_request(&request_message.attributes)
    ///                           .with_priority(UPriority::UPRIORITY_CS5)
    ///                           .build()?;
    /// assert_eq!(response_message.attributes.type_, UMessageType::UMESSAGE_TYPE_RESPONSE.into());
    /// assert_eq!(response_message.attributes.priority, UPriority::UPRIORITY_CS5.into());
    /// assert_eq!(response_message.attributes.source, Some(method_to_invoke).into());
    /// assert_eq!(response_message.attributes.sink, Some(reply_to_address).into());
    /// assert_eq!(response_message.attributes.reqid, Some(request_message_id).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn response_for_request(request_attributes: &UAttributes) -> UMessageBuilder {
        UMessageBuilder {
            validator: Box::new(ResponseValidator),
            message_type: UMessageType::UMESSAGE_TYPE_RESPONSE,
            source: request_attributes.sink.as_ref().cloned(),
            sink: request_attributes.source.as_ref().cloned(),
            request_id: request_attributes.id.as_ref().cloned(),
            priority: request_attributes
                .priority
                .enum_value_or(UPriority::UPRIORITY_CS4),
            ..Default::default()
        }
    }

    /// Sets the message's identifier.
    ///
    /// Every message must have an identifier. If this function is not used, an identifier will be
    /// generated and set on the message when one of the `build` functions is called on the
    /// `UMessageBuilder`.
    ///
    /// It's more typical to _not_ use this function, but could have edge case uses.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The identifier to use.
    ///
    /// # Returns
    ///
    /// The builder.
    ///
    /// # Panics
    ///
    /// Panics if the given UUID is not a [valid uProtocol UUID](`UUID::is_uprotocol_uuid`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("my-vehicle/4210/1/B24D")?;
    /// let mut builder = UMessageBuilder::publish(topic);
    /// builder.with_priority(UPriority::UPRIORITY_CS2);
    /// let message_one = builder
    ///                     .with_message_id(UUIDBuilder::build())
    ///                     .build_with_payload("closed".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// let message_two = builder
    ///                     // use new message ID but retain all other attributes
    ///                     .with_message_id(UUIDBuilder::build())
    ///                     .build_with_payload("open".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_ne!(message_one.attributes.id, message_two.attributes.id);
    /// assert_eq!(message_one.attributes.source, message_two.attributes.source);
    /// assert_eq!(message_one.attributes.priority, UPriority::UPRIORITY_CS2.into());
    /// assert_eq!(message_two.attributes.priority, UPriority::UPRIORITY_CS2.into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_message_id(&mut self, message_id: UUID) -> &mut UMessageBuilder {
        assert!(
            message_id.is_uprotocol_uuid(),
            "Message ID must be a valid uProtocol UUID"
        );
        self.message_id = Some(message_id);
        self
    }

    /// Sets the message's priority.
    ///
    /// If not set explicitly, the default priority as defined in the
    /// [uProtocol specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/qos.adoc)
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
    /// let topic = UUri::try_from("my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic)
    ///                   .with_priority(UPriority::UPRIORITY_CS5)
    ///                   .build_with_payload("closed".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS5.into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_priority(&mut self, priority: UPriority) -> &mut UMessageBuilder {
        if self.message_type == UMessageType::UMESSAGE_TYPE_REQUEST
            || self.message_type == UMessageType::UMESSAGE_TYPE_RESPONSE
        {
            assert!(priority.value() >= UPriority::UPRIORITY_CS4.value())
        }
        self.priority = priority;
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let request_msg_id = UUIDBuilder::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_msg_id, invoked_method)
    ///                     .with_ttl(2000)
    ///                     .build()?;
    /// assert_eq!(message.attributes.ttl, Some(2000));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_ttl(&mut self, ttl: u32) -> &mut UMessageBuilder {
        self.ttl = Some(ttl);
        self
    }

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
    /// # Panics
    ///
    /// * if the message is not an RPC request message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let token = String::from("this-is-my-token");
    /// let message = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000)
    ///                     .with_token(token.clone())
    ///                     .build_with_payload("lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.token, Some(token));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_token(&mut self, token: String) -> &mut UMessageBuilder {
        assert!(self.message_type == UMessageType::UMESSAGE_TYPE_REQUEST);
        self.token = Some(token);
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
    /// # Panics
    ///
    /// * if the given level is greater than [`i32::MAX`]
    /// * if the message is not an RPC request message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let message = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000)
    ///                     .with_permission_level(12)
    ///                     .build_with_payload("lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.permission_level, Some(12));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_permission_level(&mut self, level: u32) -> &mut UMessageBuilder {
        assert!(self.message_type == UMessageType::UMESSAGE_TYPE_REQUEST);
        self.permission_level = Some(level);
        self
    }

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
    /// # Panics
    ///
    /// * if the message is not an RPC response message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protobuf::{Enum, EnumOrUnknown};
    /// use up_rust::{UCode, UMessageBuilder, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let status = UCode::OK.value();
    /// let request_msg_id = UUIDBuilder::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_msg_id, invoked_method)
    ///                     .with_comm_status(status)
    ///                     .build()?;
    /// assert_eq!(message.attributes.commstatus, Some(EnumOrUnknown::from_i32(status)));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_comm_status(&mut self, comm_status: i32) -> &mut UMessageBuilder {
        assert!(self.message_type == UMessageType::UMESSAGE_TYPE_RESPONSE);
        self.comm_status = Some(EnumOrUnknown::from_i32(comm_status));
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
    /// a [`UMessageBuilderError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ## Not setting `id` explicitly with [`UMessageBuilder::with_message_id()']
    ///
    /// The recommended way to use the `UMessageBuilder`.
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UAttributesValidators, UMessageBuilder, UMessageBuilderError, UMessageType, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let result = UMessageBuilder::response(reply_to_address, UUIDBuilder::build(), invoked_method)
    ///                     .build();
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Setting `id` explicitly with [`UMessageBuilder::with_message_id()']
    ///
    /// Note that explicitly using [`UMessageBuilder::with_message_id()'] is not required as shown
    /// above.
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let lsb = UUIDBuilder::build().lsb;
    /// let invoked_method = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let message_id = UUIDBuilder::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, UUIDBuilder::build(), invoked_method)
    ///                     .with_message_id(message_id.clone())
    ///                     .build()?;
    /// assert_eq!(message.attributes.id, Some(message_id).into());
    /// # assert_eq!(message.attributes.id.clone().unwrap().lsb, lsb);
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(&self) -> Result<UMessage, UMessageBuilderError> {
        let message_id = self
            .message_id
            .clone()
            .map_or_else(|| Some(UUIDBuilder::build()), Some);
        let attributes = UAttributes {
            id: message_id.into(),
            type_: self.message_type.into(),
            source: self.source.clone().into(),
            sink: self.sink.clone().into(),
            priority: self.priority.into(),
            ttl: self.ttl,
            token: self.token.clone(),
            permission_level: self.permission_level,
            commstatus: self.comm_status,
            reqid: self.request_id.clone().into(),
            ..Default::default()
        };
        self.validator
            .validate(&attributes)
            .map_err(UMessageBuilderError::from)
            .map(|_| {
                let payload = self
                    .payload
                    .as_ref()
                    .map(|bytes| bytes.to_vec())
                    .map(|data| UPayload {
                        format: self.payload_format.into(),
                        data,
                        ..Default::default()
                    });
                UMessage {
                    attributes: Some(attributes).into(),
                    payload: payload.into(),
                    ..Default::default()
                }
            })
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
    /// a [`UMessageBuilderError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic)
    ///                    .build_with_payload("locked".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert!(message.payload.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_payload(
        &mut self,
        payload: Bytes,
        format: UPayloadFormat,
    ) -> Result<UMessage, UMessageBuilderError> {
        self.payload = Some(payload);
        self.payload_format = format;
        self.build()
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
    /// If the given payload cannot be serialized into a byte array, a [`UMessageBuilderError::DataSerializationError`] is returned.
    /// If the properties set on the builder do not represent a consistent set of [`UAttributes`],
    /// a [`UMessageBuilderError::AttributesValidationError`] is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protobuf::{Enum, Message};
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UPriority, UStatus, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("my-cloud/BA4C/1/0")?;
    /// let request_id = UUIDBuilder::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_id, invoked_method)
    ///                    .with_comm_status(UCode::INVALID_ARGUMENT.value())
    ///                    .build_with_protobuf_payload(&UStatus::fail("failed to parse request"))?;
    /// assert!(message.payload.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_protobuf_payload<T: Message>(
        &mut self,
        payload: &T,
    ) -> Result<UMessage, UMessageBuilderError> {
        payload
            .write_to_bytes()
            .map_err(UMessageBuilderError::from)
            .and_then(|serialized_payload| {
                self.build_with_payload(
                    serialized_payload.into(),
                    UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::{UCode, UUIDBuilder};

    use super::*;

    use test_case::test_case;

    const METHOD_TO_INVOKE: &str = "my-vehicle/4D123/2/6FA3";
    const REPLY_TO_ADDRESS: &str = "my-cloud/9CB3/1/0";
    const TOPIC: &str = "my-vehicle/4210/1/B24D";

    #[test]
    #[should_panic]
    fn test_with_message_id_panics_for_invalid_uuid() {
        let invalid_message_id = UUID {
            msb: 0x00000000000000ab_u64,
            lsb: 0x0000000000018000_u64,
            ..Default::default()
        };
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        UMessageBuilder::publish(topic).with_message_id(invalid_message_id);
    }

    #[test_case(Some(5), None, None; "with permission level")]
    #[test_case(None, Some(5), None; "with commstatus")]
    #[test_case(None, None, Some(String::from("my-token")); "with token")]
    #[should_panic]
    fn test_publish_message_builder_panics(
        perm_level: Option<u32>,
        comm_status: Option<i32>,
        token: Option<String>,
    ) {
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let mut builder = UMessageBuilder::publish(topic);
        if let Some(level) = perm_level {
            builder.with_permission_level(level);
        } else if let Some(status_code) = comm_status {
            builder.with_comm_status(status_code);
        } else if let Some(t) = token {
            builder.with_token(t);
        }
    }

    #[test_case(Some(5), None; "with permission level")]
    #[test_case(None, Some(String::from("my-token")); "with token")]
    #[should_panic]
    fn test_response_message_builder_panics(perm_level: Option<u32>, token: Option<String>) {
        let request_id = UUIDBuilder::build();
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let mut builder = UMessageBuilder::response(reply_to_address, request_id, method_to_invoke);

        if let Some(level) = perm_level {
            builder.with_permission_level(level);
        } else if let Some(t) = token {
            builder.with_token(t);
        }
    }

    #[test_case(Some(5), None; "for comm status")]
    #[should_panic]
    fn test_request_message_builder_panics(comm_status: Option<i32>, perm_level: Option<u32>) {
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let mut builder = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000);

        if let Some(status) = comm_status {
            builder.with_comm_status(status);
        } else if let Some(level) = perm_level {
            builder.with_permission_level(level);
        }
    }

    #[test]
    fn test_build_supports_repeated_invocation() {
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let mut builder = UMessageBuilder::publish(topic);
        let message_one = builder
            .with_message_id(UUIDBuilder::build())
            .build_with_payload("locked".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
            .expect("should have been able to create message");
        let message_two = builder
            .with_message_id(UUIDBuilder::build())
            .build_with_payload("unlocked".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
            .expect("should have been able to create message");
        assert_eq!(message_one.attributes.type_, message_two.attributes.type_);
        assert_ne!(message_one.attributes.id, message_two.attributes.id);
        assert_eq!(message_one.attributes.source, message_two.attributes.source);
        assert_ne!(message_one.payload, message_two.payload);
    }

    #[test]
    fn test_build_retains_all_publish_attributes() {
        let message_id = UUIDBuilder::build();
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let message = UMessageBuilder::publish(topic.clone())
            .with_message_id(message_id.clone())
            .with_priority(UPriority::UPRIORITY_CS2)
            .with_ttl(5000)
            .build_with_payload("locked".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
            .expect("should have been able to create message");
        assert_eq!(message.attributes.id, Some(message_id).into());
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS2.into());
        assert_eq!(message.attributes.source, Some(topic).into());
        assert_eq!(message.attributes.ttl, Some(5000));
        assert_eq!(
            message.attributes.type_,
            UMessageType::UMESSAGE_TYPE_PUBLISH.into()
        );
    }

    #[test]
    fn test_build_retains_all_request_attributes() {
        let message_id = UUIDBuilder::build();
        let token = String::from("token");
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let message =
            UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
                .with_message_id(message_id.clone())
                .with_permission_level(5)
                .with_priority(UPriority::UPRIORITY_CS4)
                .with_token(token.clone())
                .build_with_payload("unlock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
                .expect("should have been able to create message");

        assert_eq!(message.attributes.id, Some(message_id).into());
        assert_eq!(message.attributes.permission_level, Some(5));
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS4.into());
        assert_eq!(message.attributes.sink, Some(method_to_invoke).into());
        assert_eq!(message.attributes.source, Some(reply_to_address).into());
        assert_eq!(message.attributes.token, Some(token));
        assert_eq!(message.attributes.ttl, Some(5000));
        assert_eq!(
            message.attributes.type_,
            UMessageType::UMESSAGE_TYPE_REQUEST.into()
        );
    }

    #[test]
    fn test_builder_copies_request_attributes() {
        let request_message_id = UUIDBuilder::build();
        let response_message_id = UUIDBuilder::build();
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let request_message =
            UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
                .with_message_id(request_message_id.clone())
                .with_priority(UPriority::UPRIORITY_CS5)
                .build()
                .expect("should have been able to create message");
        let message = UMessageBuilder::response_for_request(&request_message.attributes)
            .with_message_id(response_message_id.clone())
            .with_comm_status(UCode::DEADLINE_EXCEEDED.value())
            .with_ttl(4000)
            .build()
            .expect("should have been able to create message");
        assert_eq!(message.attributes.id, Some(response_message_id).into());
        assert_eq!(
            message.attributes.commstatus,
            Some(EnumOrUnknown::from(UCode::DEADLINE_EXCEEDED))
        );
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS5.into());
        assert_eq!(message.attributes.reqid, Some(request_message_id).into());
        assert_eq!(message.attributes.sink, Some(reply_to_address).into());
        assert_eq!(message.attributes.source, Some(method_to_invoke).into());
        assert_eq!(message.attributes.ttl, Some(4000));
        assert_eq!(
            message.attributes.type_,
            UMessageType::UMESSAGE_TYPE_RESPONSE.into()
        );
    }

    #[test]
    fn test_build_retains_all_response_attributes() {
        let message_id = UUIDBuilder::build();
        let request_id = UUIDBuilder::build();
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
        .with_comm_status(UCode::DEADLINE_EXCEEDED.value())
        .with_priority(UPriority::UPRIORITY_CS5)
        .with_ttl(0)
        .build()
        .expect("should have been able to create message");
        assert_eq!(message.attributes.id, Some(message_id).into());
        assert_eq!(
            message.attributes.commstatus,
            Some(EnumOrUnknown::from(UCode::DEADLINE_EXCEEDED))
        );
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS5.into());
        assert_eq!(message.attributes.reqid, Some(request_id).into());
        assert_eq!(message.attributes.sink, Some(reply_to_address).into());
        assert_eq!(message.attributes.source, Some(method_to_invoke).into());
        assert_eq!(message.attributes.ttl, Some(0));
        assert_eq!(
            message.attributes.type_,
            UMessageType::UMESSAGE_TYPE_RESPONSE.into()
        );
    }
}
