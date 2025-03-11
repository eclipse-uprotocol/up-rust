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
use protobuf::{well_known_types::any::Any, Enum, EnumOrUnknown, Message, MessageFull};

use crate::uattributes::NotificationValidator;
use crate::{
    PublishValidator, RequestValidator, ResponseValidator, UAttributes, UAttributesValidator,
    UCode, UMessage, UMessageError, UMessageType, UPayloadFormat, UPriority, UUri, UUID,
};

const PRIORITY_DEFAULT: UPriority = UPriority::UPRIORITY_CS1;

/// A builder for creating [`UMessage`]s.
///
/// Messages are being used by a uEntity to inform other entities about the occurrence of events
/// and/or to invoke service operations provided by other entities.
pub struct UMessageBuilder {
    comm_status: Option<EnumOrUnknown<UCode>>,
    message_id: Option<UUID>,
    message_type: UMessageType,
    payload: Option<Bytes>,
    payload_format: UPayloadFormat,
    permission_level: Option<u32>,
    priority: UPriority,
    request_id: Option<UUID>,
    sink: Option<UUri>,
    source: Option<UUri>,
    token: Option<String>,
    traceparent: Option<String>,
    ttl: Option<u32>,
    validator: Box<dyn UAttributesValidator>,
}

impl Default for UMessageBuilder {
    fn default() -> Self {
        UMessageBuilder {
            comm_status: None,
            message_id: None,
            message_type: UMessageType::UMESSAGE_TYPE_UNSPECIFIED,
            payload: None,
            payload_format: UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED,
            permission_level: None,
            priority: UPriority::UPRIORITY_UNSPECIFIED,
            request_id: None,
            sink: None,
            source: None,
            token: None,
            traceparent: None,
            ttl: None,
            validator: Box::new(PublishValidator),
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
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic.clone())
    ///                    .build_with_payload("closed", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.type_unchecked(), UMessageType::UMESSAGE_TYPE_PUBLISH);
    /// assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_UNSPECIFIED);
    /// assert_eq!(message.source_unchecked(), &topic);
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
    /// let origin = UUri::try_from("//my-vehicle/4210/5/F20B")?;
    /// let destination = UUri::try_from("//my-cloud/CCDD/2/0")?;
    /// let message = UMessageBuilder::notification(origin.clone(), destination.clone())
    ///                    .build_with_payload("unexpected movement", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.type_unchecked(), UMessageType::UMESSAGE_TYPE_NOTIFICATION);
    /// assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_UNSPECIFIED);
    /// assert_eq!(message.source_unchecked(), &origin);
    /// assert_eq!(message.sink_unchecked(), &destination);
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
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let message = UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
    ///                    .build_with_payload("lock", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.type_unchecked(), UMessageType::UMESSAGE_TYPE_REQUEST);
    /// assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS4);
    /// assert_eq!(message.source_unchecked(), &reply_to_address);
    /// assert_eq!(message.sink_unchecked(), &method_to_invoke);
    /// assert_eq!(message.ttl_unchecked(), 5000);
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
    /// assert_eq!(message.type_unchecked(), UMessageType::UMESSAGE_TYPE_RESPONSE);
    /// assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS4);
    /// assert_eq!(message.source_unchecked(), &invoked_method);
    /// assert_eq!(message.sink_unchecked(), &reply_to_address);
    /// assert_eq!(message.request_id_unchecked(), &request_id);
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_message_id = UUID::build();
    /// let request_message = UMessageBuilder::request(method_to_invoke.clone(), reply_to_address.clone(), 5000)
    ///                           .with_message_id(request_message_id.clone()) // normally not needed, used only for asserts below
    ///                           .build_with_payload("lock", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    ///
    /// let response_message = UMessageBuilder::response_for_request(&request_message.attributes)
    ///                           .with_priority(UPriority::UPRIORITY_CS5)
    ///                           .build()?;
    /// assert_eq!(response_message.type_unchecked(), UMessageType::UMESSAGE_TYPE_RESPONSE);
    /// assert_eq!(response_message.priority_unchecked(), UPriority::UPRIORITY_CS5);
    /// assert_eq!(response_message.source_unchecked(), &method_to_invoke);
    /// assert_eq!(response_message.sink_unchecked(), &reply_to_address);
    /// assert_eq!(response_message.request_id_unchecked(), &request_message_id);
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let mut builder = UMessageBuilder::publish(topic);
    /// builder.with_priority(UPriority::UPRIORITY_CS2);
    /// let message_one = builder
    ///                     .with_message_id(UUID::build())
    ///                     .build_with_payload("closed", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// let message_two = builder
    ///                     // use new message ID but retain all other attributes
    ///                     .with_message_id(UUID::build())
    ///                     .build_with_payload("open", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_ne!(message_one.id_unchecked(), message_two.id_unchecked());
    /// assert_eq!(message_one.source_unchecked(), message_two.source_unchecked());
    /// assert_eq!(message_one.priority_unchecked(), UPriority::UPRIORITY_CS2);
    /// assert_eq!(message_two.priority_unchecked(), UPriority::UPRIORITY_CS2);
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
    /// # Panics
    ///
    /// if the builder is used for creating an RPC message but the given priority is less than CS4.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/4210/1/B24D")?;
    /// let message = UMessageBuilder::publish(topic)
    ///                   .with_priority(UPriority::UPRIORITY_CS5)
    ///                   .build_with_payload("closed", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS5);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_priority(&mut self, priority: UPriority) -> &mut UMessageBuilder {
        if self.message_type == UMessageType::UMESSAGE_TYPE_REQUEST
            || self.message_type == UMessageType::UMESSAGE_TYPE_RESPONSE
        {
            assert!(priority.value() >= UPriority::UPRIORITY_CS4.value())
        }
        if priority != PRIORITY_DEFAULT {
            // only set priority explicitly if it differs from the default priority
            self.priority = priority;
        } else {
            // in all other cases set to UNSPECIFIED which will result in the
            // priority not being included in the serialized protobuf
            self.priority = UPriority::UPRIORITY_UNSPECIFIED;
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
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let token = String::from("this-is-my-token");
    /// let message = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000)
    ///                     .with_token(token.clone())
    ///                     .build_with_payload("lock", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.token(), Some(&token));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_token<T: Into<String>>(&mut self, token: T) -> &mut UMessageBuilder {
        assert!(self.message_type == UMessageType::UMESSAGE_TYPE_REQUEST);
        self.token = Some(token.into());
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
    /// let method_to_invoke = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let message = UMessageBuilder::request(method_to_invoke, reply_to_address, 5000)
    ///                     .with_permission_level(12)
    ///                     .build_with_payload("lock", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.permission_level(), Some(12));
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
    /// use up_rust::{UCode, UMessageBuilder, UPriority, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_msg_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_msg_id, invoked_method)
    ///                     .with_comm_status(UCode::OK)
    ///                     .build()?;
    /// assert_eq!(message.commstatus_unchecked(), UCode::OK);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_comm_status(&mut self, comm_status: UCode) -> &mut UMessageBuilder {
        assert!(self.message_type == UMessageType::UMESSAGE_TYPE_RESPONSE);
        self.comm_status = Some(comm_status.into());
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
    /// let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string();
    /// let message = UMessageBuilder::publish(topic.clone())
    ///                    .with_traceparent(&traceparent)
    ///                    .build_with_payload("closed", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.traceparent(), Some(&traceparent));
    /// # Ok(())
    /// # }
    pub fn with_traceparent<T: Into<String>>(&mut self, traceparent: T) -> &mut UMessageBuilder {
        self.traceparent = Some(traceparent.into());
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
    /// ## Not setting `id` explicitly with [`UMessageBuilder::with_message_id()']
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
    /// ## Setting `id` explicitly with [`UMessageBuilder::with_message_id()']
    ///
    /// Note that explicitly using [`UMessageBuilder::with_message_id()'] is not required as shown
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
    /// assert_eq!(message.id_unchecked(), &message_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(&self) -> Result<UMessage, UMessageError> {
        let message_id = self
            .message_id
            .clone()
            .map_or_else(|| Some(UUID::build()), Some);
        let attributes = UAttributes {
            commstatus: self.comm_status,
            id: message_id.into(),
            payload_format: self.payload_format.into(),
            permission_level: self.permission_level,
            priority: self.priority.into(),
            reqid: self.request_id.clone().into(),
            sink: self.sink.clone().into(),
            source: self.source.clone().into(),
            token: self.token.clone(),
            traceparent: self.traceparent.clone(),
            ttl: self.ttl,
            type_: self.message_type.into(),
            ..Default::default()
        };
        self.validator
            .validate(&attributes)
            .map_err(UMessageError::from)
            .map(|_| UMessage {
                attributes: Some(attributes).into(),
                payload: self.payload.to_owned(),
                ..Default::default()
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
    ///                    .build_with_payload("locked", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert!(message.payload.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_payload<T: Into<Bytes>>(
        &mut self,
        payload: T,
        format: UPayloadFormat,
    ) -> Result<UMessage, UMessageError> {
        self.payload = Some(payload.into());
        self.payload_format = format;

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
    /// [`UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF`] set as its payload format.
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
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UStatus, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_id, invoked_method)
    ///                    .with_comm_status(UCode::INVALID_ARGUMENT)
    ///                    .build_with_protobuf_payload(&UStatus::fail("failed to parse request"))?;
    /// assert!(message.payload.is_some());
    /// assert_eq!(message.payload_format_unchecked(), UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF);
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_protobuf_payload<T: Message>(
        &mut self,
        payload: &T,
    ) -> Result<UMessage, UMessageError> {
        payload
            .write_to_bytes()
            .map_err(UMessageError::from)
            .and_then(|serialized_payload| {
                self.build_with_payload(
                    serialized_payload,
                    UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF,
                )
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
    /// [`UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`] set as its payload format.
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
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UStatus, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/4210/5/64AB")?;
    /// let reply_to_address = UUri::try_from("//my-cloud/BA4C/1/0")?;
    /// let request_id = UUID::build();
    /// // a service implementation would normally use
    /// // `UMessageBuilder::response_for_request(&request_message.attributes)` instead
    /// let message = UMessageBuilder::response(reply_to_address, request_id, invoked_method)
    ///                    .with_comm_status(UCode::INVALID_ARGUMENT)
    ///                    .build_with_wrapped_protobuf_payload(&UStatus::fail("failed to parse request"))?;
    /// assert!(message.payload.is_some());
    /// assert_eq!(message.payload_format_unchecked(), UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY);
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_wrapped_protobuf_payload<T: MessageFull>(
        &mut self,
        payload: &T,
    ) -> Result<UMessage, UMessageError> {
        Any::pack(payload)
            .map_err(UMessageError::DataSerializationError)
            .and_then(|any| any.write_to_bytes().map_err(UMessageError::from))
            .and_then(|serialized_payload| {
                self.build_with_payload(
                    serialized_payload,
                    UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::UCode;

    use super::*;

    use test_case::test_case;

    const METHOD_TO_INVOKE: &str = "//my-vehicle/4D123/2/6FA3";
    const REPLY_TO_ADDRESS: &str = "//my-cloud/9CB3/1/0";
    const TOPIC: &str = "//my-vehicle/4210/1/B24D";

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
    #[test_case(None, Some(UCode::NOT_FOUND), None; "with commstatus")]
    #[test_case(None, None, Some(String::from("my-token")); "with token")]
    #[should_panic]
    fn test_publish_message_builder_panics(
        perm_level: Option<u32>,
        comm_status: Option<UCode>,
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
        let request_id = UUID::build();
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

    #[test_case(Some(UCode::NOT_FOUND), None; "for comm status")]
    #[should_panic]
    fn test_request_message_builder_panics(comm_status: Option<UCode>, perm_level: Option<u32>) {
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
            .with_message_id(UUID::build())
            .build_with_payload("locked", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
            .expect("should have been able to create message");
        let message_two = builder
            .with_message_id(UUID::build())
            .build_with_payload("unlocked", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
            .expect("should have been able to create message");
        assert_eq!(message_one.attributes.type_, message_two.attributes.type_);
        assert_ne!(message_one.attributes.id, message_two.attributes.id);
        assert_eq!(message_one.attributes.source, message_two.attributes.source);
        assert_ne!(message_one.payload, message_two.payload);
    }

    #[test]
    fn test_build_retains_all_publish_attributes() {
        let message_id = UUID::build();
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let message = UMessageBuilder::publish(topic.clone())
            .with_message_id(message_id.clone())
            .with_priority(UPriority::UPRIORITY_CS2)
            .with_ttl(5000)
            .build_with_payload("locked", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
            .expect("should have been able to create message");
        assert_eq!(message.id_unchecked(), &message_id);
        assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS2);
        assert_eq!(message.source_unchecked(), &topic);
        assert_eq!(message.ttl_unchecked(), 5000);
        assert_eq!(
            message.type_unchecked(),
            UMessageType::UMESSAGE_TYPE_PUBLISH
        );
    }

    #[test]
    fn test_build_retains_all_request_attributes() {
        let message_id = UUID::build();
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
                .build_with_payload("unlock", UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
                .expect("should have been able to create message");

        assert_eq!(message.id_unchecked(), &message_id);
        assert_eq!(message.attributes.permission_level, Some(5));
        assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS4);
        assert_eq!(message.sink_unchecked(), &method_to_invoke);
        assert_eq!(message.source_unchecked(), &reply_to_address);
        assert_eq!(message.token(), Some(&token));
        assert_eq!(message.ttl_unchecked(), 5000);
        assert_eq!(
            message.type_unchecked(),
            UMessageType::UMESSAGE_TYPE_REQUEST
        );
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
                .with_priority(UPriority::UPRIORITY_CS5)
                .build()
                .expect("should have been able to create message");
        let message = UMessageBuilder::response_for_request(&request_message.attributes)
            .with_message_id(response_message_id.clone())
            .with_comm_status(UCode::DEADLINE_EXCEEDED)
            .with_ttl(4000)
            .build()
            .expect("should have been able to create message");
        assert_eq!(message.id_unchecked(), &response_message_id);
        assert_eq!(message.commstatus_unchecked(), UCode::DEADLINE_EXCEEDED);
        assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS5);
        assert_eq!(message.request_id_unchecked(), &request_message_id);
        assert_eq!(message.sink_unchecked(), &reply_to_address);
        assert_eq!(message.source_unchecked(), &method_to_invoke);
        assert_eq!(message.ttl_unchecked(), 4000);
        assert_eq!(
            message.type_unchecked(),
            UMessageType::UMESSAGE_TYPE_RESPONSE
        );
    }

    #[test]
    fn test_build_retains_all_response_attributes() {
        let message_id = UUID::build();
        let request_id = UUID::build();
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
        .with_comm_status(UCode::DEADLINE_EXCEEDED)
        .with_priority(UPriority::UPRIORITY_CS5)
        .with_ttl(0)
        .build()
        .expect("should have been able to create message");
        assert_eq!(message.id_unchecked(), &message_id);
        assert_eq!(message.commstatus_unchecked(), UCode::DEADLINE_EXCEEDED);
        assert_eq!(message.priority_unchecked(), UPriority::UPRIORITY_CS5);
        assert_eq!(message.request_id_unchecked(), &request_id);
        assert_eq!(message.sink_unchecked(), &reply_to_address);
        assert_eq!(message.source_unchecked(), &method_to_invoke);
        assert_eq!(message.ttl_unchecked(), 0);
        assert_eq!(
            message.type_unchecked(),
            UMessageType::UMESSAGE_TYPE_RESPONSE
        );
    }
}
