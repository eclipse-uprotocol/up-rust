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
use protobuf::Message;

use crate::uattributes::UAttributesError;
use crate::{
    Data, PublishValidator, RequestValidator, ResponseValidator, UAttributes, UAttributesValidator,
    UMessage, UMessageType, UPayload, UPayloadFormat, UPriority, UUIDBuilder, UUri, UUID,
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
pub struct UMessageBuilder<'a> {
    validator: Box<dyn UAttributesValidator>,
    message_type: UMessageType,
    source: Option<&'a UUri>,
    sink: Option<&'a UUri>,
    priority: UPriority,
    ttl: Option<i32>,
    token: Option<&'a String>,
    permission_level: Option<i32>,
    comm_status: Option<i32>,
    request_id: Option<&'a UUID>,
    payload: Option<Bytes>,
    payload_format: UPayloadFormat,
}

impl<'a> Default for UMessageBuilder<'a> {
    fn default() -> Self {
        UMessageBuilder {
            validator: Box::new(PublishValidator),
            comm_status: None,
            message_type: UMessageType::UMESSAGE_TYPE_UNSPECIFIED,
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

impl<'a> UMessageBuilder<'a> {
    /// Gets a builder for creating a *publish* message.
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let topic = UUri::try_from("my-vehicle/cabin/1/doors.driver_side#status")?;
    /// let message = UMessageBuilder::publish(&topic)
    ///                    .build_with_payload(&uuid_builder, "closed".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_PUBLISH.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(message.attributes.source, Some(topic).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn publish(topic: &'a UUri) -> UMessageBuilder<'a> {
        UMessageBuilder {
            validator: Box::new(PublishValidator),
            message_type: UMessageType::UMESSAGE_TYPE_PUBLISH,
            source: Some(topic),
            ..Default::default()
        }
    }

    /// Gets a builder for creating a *notification* message.
    ///
    /// A notification is used to inform a specific consumer about an event that has occurred.
    ///
    /// # Arguments
    ///
    /// * `destination` - The URI identifying the destination to send the notification to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let destination = UUri::try_from("my-cloud/companion/1/alarm")?;
    /// let message = UMessageBuilder::notification(&destination)
    ///                    .build_with_payload(&uuid_builder, "unexpected movement".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_PUBLISH.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(message.attributes.sink, Some(destination).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn notification(destination: &'a UUri) -> UMessageBuilder<'a> {
        UMessageBuilder {
            validator: Box::new(PublishValidator),
            message_type: UMessageType::UMESSAGE_TYPE_PUBLISH,
            sink: Some(destination),
            ..Default::default()
        }
    }

    /// Gets a builder for creating an RPC *request* message.
    ///
    /// A request message is used to invoke a service's method with some input data, expecting
    /// the service to reply with a response message which is correlated by means of the `request_id`.
    ///
    /// # Arguments
    ///
    /// * `method_to_invoke` - The URI identifying the method to invoke.
    /// * `reply_to_address` - The URI that the sender of the request expects the response message at.
    /// * `request_id` - The identifier that the response needs to contain to be correlated to this request.
    /// * `ttl` - The number of milliseconds after which the request should no longer be processed
    ///           by the target service. The value is capped at [`i32::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let method_to_invoke = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let message = UMessageBuilder::request(&method_to_invoke, &reply_to_address, &request_id, 5000)
    ///                     .build_with_payload(&uuid_builder, "lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_REQUEST.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(message.attributes.source, Some(reply_to_address).into());
    /// assert_eq!(message.attributes.sink, Some(method_to_invoke).into());
    /// assert_eq!(message.attributes.reqid, Some(request_id).into());
    /// assert_eq!(message.attributes.ttl, Some(5000));
    /// # Ok(())
    /// # }
    /// ```
    pub fn request(
        method_to_invoke: &'a UUri,
        reply_to_address: &'a UUri,
        request_id: &'a UUID,
        ttl: u32,
    ) -> UMessageBuilder<'a> {
        UMessageBuilder {
            validator: Box::new(RequestValidator),
            message_type: UMessageType::UMESSAGE_TYPE_REQUEST,
            source: Some(reply_to_address),
            sink: Some(method_to_invoke),
            request_id: Some(request_id),
            ttl: Some(i32::try_from(ttl).unwrap_or(i32::MAX)),
            ..Default::default()
        }
    }

    /// Gets a builder for creating an RPC *response* message.
    ///
    /// A response message is used to send the outcome of processing a request message
    /// to the original sender of the request message.
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
    /// let uuid_builder = UUIDBuilder::new();
    /// let invoked_method = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let message = UMessageBuilder::response(&reply_to_address, &request_id, &invoked_method).build(&uuid_builder)?;
    /// assert_eq!(message.attributes.type_, UMessageType::UMESSAGE_TYPE_RESPONSE.into());
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(message.attributes.source, Some(invoked_method).into());
    /// assert_eq!(message.attributes.sink, Some(reply_to_address).into());
    /// assert_eq!(message.attributes.reqid, Some(request_id).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn response(
        reply_to_address: &'a UUri,
        request_id: &'a UUID,
        invoked_method: &'a UUri,
    ) -> UMessageBuilder<'a> {
        UMessageBuilder {
            validator: Box::new(ResponseValidator),
            message_type: UMessageType::UMESSAGE_TYPE_RESPONSE,
            source: Some(invoked_method),
            sink: Some(reply_to_address),
            request_id: Some(request_id),
            ..Default::default()
        }
    }

    /// Gets a builder for creating an RPC *response* message in reply to a *request*.
    ///
    /// A response message is used to send the outcome of processing a request message
    /// to the original sender of the request message.
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
    /// let uuid_builder = UUIDBuilder::new();
    /// let method_to_invoke = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let request_message = UMessageBuilder::request(&method_to_invoke, &reply_to_address, &request_id, 5000)
    ///                     .build_with_payload(&uuid_builder, "lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    ///
    /// let response_message = UMessageBuilder::response_for_request(&request_message.attributes)
    ///                           .build(&uuid_builder)?;
    /// assert_eq!(response_message.attributes.type_, UMessageType::UMESSAGE_TYPE_RESPONSE.into());
    /// assert_eq!(response_message.attributes.priority, UPriority::UPRIORITY_CS1.into());
    /// assert_eq!(response_message.attributes.source, Some(method_to_invoke).into());
    /// assert_eq!(response_message.attributes.sink, Some(reply_to_address).into());
    /// assert_eq!(response_message.attributes.reqid, Some(request_id).into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn response_for_request(request_attributes: &'a UAttributes) -> UMessageBuilder<'a> {
        UMessageBuilder {
            validator: Box::new(ResponseValidator),
            message_type: UMessageType::UMESSAGE_TYPE_RESPONSE,
            source: request_attributes.sink.as_ref(),
            sink: request_attributes.source.as_ref(),
            request_id: request_attributes.reqid.as_ref(),
            ..Default::default()
        }
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let topic = UUri::try_from("my-vehicle/cabin/1/doors.driver_side#status")?;
    /// let message = UMessageBuilder::publish(&topic)
    ///                 .with_priority(UPriority::UPRIORITY_CS5)
    ///                 .build_with_payload(&uuid_builder, "closed".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS5.into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_priority(&'a mut self, priority: UPriority) -> &'a mut UMessageBuilder {
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
    /// let uuid_builder = UUIDBuilder::new();
    /// let invoked_method = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let message = UMessageBuilder::response(&reply_to_address, &request_id, &invoked_method)
    ///                     .with_ttl(2000)
    ///                     .build(&uuid_builder)?;
    /// assert_eq!(message.attributes.ttl, Some(2000));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_ttl(&'a mut self, ttl: u32) -> &'a mut UMessageBuilder {
        self.ttl = Some(i32::try_from(ttl).unwrap_or(i32::MAX));
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
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let method_to_invoke = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let token = String::from("this-is-my-token");
    /// let message = UMessageBuilder::request(&method_to_invoke, &reply_to_address, &uuid_builder.build(), 5000)
    ///                     .with_token(&token)
    ///                     .build_with_payload(&uuid_builder, "lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.token, Some(token));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_token(&'a mut self, token: &'a String) -> &'a mut UMessageBuilder {
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
    /// if the given level is 0 or greater than [`i32::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let method_to_invoke = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let message = UMessageBuilder::request(&method_to_invoke, &reply_to_address, &uuid_builder.build(), 5000)
    ///                     .with_permission_level(12)
    ///                     .build_with_payload(&uuid_builder, "lock".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert_eq!(message.attributes.permission_level, Some(12));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_permission_level(&'a mut self, level: u32) -> &'a mut UMessageBuilder {
        assert!(level > 0, "permission level must be a positive integer");
        self.permission_level =
            Some(i32::try_from(level).expect("permission level must not exceed i32::MAX"));
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
    /// # Examples
    ///
    /// ```rust
    /// use protobuf::Enum;
    /// use up_rust::{UCode, UMessageBuilder, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let invoked_method = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let status = UCode::OK.value();
    /// let message = UMessageBuilder::response(&reply_to_address, &request_id, &invoked_method)
    ///                     .with_comm_status(status)
    ///                     .build(&uuid_builder)?;
    /// assert_eq!(message.attributes.commstatus, Some(status));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_comm_status(&'a mut self, comm_status: i32) -> &'a mut UMessageBuilder {
        self.comm_status = Some(comm_status);
        self
    }

    /// Creates the message based on the builder's state.
    ///
    /// # Arguments
    ///
    /// * `uuid_builder` - A (shared) builder for creating the message ID.
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// # let lsb = uuid_builder.build().lsb;
    /// let invoked_method = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let message = UMessageBuilder::response(&reply_to_address, &request_id, &invoked_method).build(&uuid_builder)?;
    /// assert!(message.attributes.id.is_some());
    /// # assert_eq!(message.attributes.id.clone().unwrap().lsb, lsb);
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(&self, uuid_builder: &UUIDBuilder) -> Result<UMessage, UMessageBuilderError> {
        let attributes = UAttributes {
            id: Some(uuid_builder.build()).into(),
            type_: self.message_type.into(),
            source: self.source.cloned().into(),
            sink: self.sink.cloned().into(),
            priority: self.priority.into(),
            ttl: self.ttl,
            token: self.token.cloned(),
            permission_level: self.permission_level,
            commstatus: self.comm_status,
            reqid: self.request_id.cloned().into(),
            ..Default::default()
        };
        self.validator
            .validate(&attributes)
            .map_err(UMessageBuilderError::from)
            .map(|_| {
                let payload = self
                    .payload
                    .as_ref()
                    .map(|bytes| Some(Data::Value(bytes.to_vec())))
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
    /// * `uuid_builder` - A (shared) builder for creating the message ID.
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
    /// use up_rust::{UMessageBuilder, UMessageType, UPayloadFormat, UPriority, UUIDBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let uuid_builder = UUIDBuilder::new();
    /// let topic = UUri::try_from("my-vehicle/cabin/1/doors.driver_side#status")?;
    /// let message = UMessageBuilder::publish(&topic)
    ///                 .build_with_payload(&uuid_builder, "locked".into(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)?;
    /// assert!(message.payload.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_payload(
        &'a mut self,
        uuid_builder: &UUIDBuilder,
        payload: Bytes,
        format: UPayloadFormat,
    ) -> Result<UMessage, UMessageBuilderError> {
        self.payload = Some(payload);
        self.payload_format = format;
        self.build(uuid_builder)
    }

    /// Creates the message based on the builder's state and some payload.
    ///
    /// # Arguments
    ///
    /// * `uuid_builder` - A (shared) builder for creating the message ID.
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
    /// let uuid_builder = UUIDBuilder::new();
    /// let invoked_method = UUri::try_from("my-vehicle/cabin/1/rpc.doors")?;
    /// let reply_to_address = UUri::try_from("my-cloud/dashboard/1/rpc.response")?;
    /// let request_id = uuid_builder.build();
    /// let message = UMessageBuilder::response(&reply_to_address, &request_id, &invoked_method)
    ///                 .with_comm_status(UCode::INVALID_ARGUMENT.value())
    ///                 .build_with_protobuf_payload(&uuid_builder, &UStatus::fail("failed to parse request"))?;
    /// assert!(message.payload.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_protobuf_payload<T: Message>(
        &'a mut self,
        uuid_builder: &UUIDBuilder,
        payload: &T,
    ) -> Result<UMessage, UMessageBuilderError> {
        payload
            .write_to_bytes()
            .map_err(UMessageBuilderError::from)
            .and_then(|serialized_payload| {
                self.build_with_payload(
                    uuid_builder,
                    serialized_payload.into(),
                    UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    const METHOD_TO_INVOKE: &str = "my-vehicle/cabin/1/rpc.doors";
    const REPLY_TO_ADDRESS: &str = "my-cloud/dashboard/1/rpc.response";
    const TOPIC: &str = "my-vehicle/cabin/1/doors.driver_side#status";

    #[test_case(0; "for level 0")]
    #[test_case(i32::MAX as u32 + 1; "for non i32 value")]
    #[should_panic]
    fn test_with_permission_level_panics(level: u32) {
        let uuid_builder = UUIDBuilder::new();
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let _ = UMessageBuilder::publish(&topic)
            .with_permission_level(level)
            .build_with_payload(
                &uuid_builder,
                "locked".into(),
                UPayloadFormat::UPAYLOAD_FORMAT_TEXT,
            );
    }

    #[test]
    fn test_with_ttl_caps_value() {
        let uuid_builder = UUIDBuilder::new();
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let message = UMessageBuilder::publish(&topic)
            .with_ttl(i32::MAX as u32 + 10)
            .build_with_payload(
                &uuid_builder,
                "locked".into(),
                UPayloadFormat::UPAYLOAD_FORMAT_TEXT,
            )
            .expect("should have been able to create message");
        assert_eq!(message.attributes.ttl, Some(i32::MAX));
    }

    #[test]
    fn test_build_retains_all_publish_attributes() {
        let uuid_builder = UUIDBuilder::new();
        let topic = UUri::try_from(TOPIC).expect("should have been able to create UUri");
        let message = UMessageBuilder::publish(&topic)
            .with_priority(UPriority::UPRIORITY_CS2)
            .with_ttl(5000)
            .build_with_payload(
                &uuid_builder,
                "locked".into(),
                UPayloadFormat::UPAYLOAD_FORMAT_TEXT,
            )
            .expect("should have been able to create message");
        assert!(message.attributes.id.is_some());
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
        let uuid_builder = UUIDBuilder::new();
        let request_id = uuid_builder.build();
        let token = String::from("token");
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let message =
            UMessageBuilder::request(&method_to_invoke, &reply_to_address, &request_id, 5000)
                .with_permission_level(5)
                .with_priority(UPriority::UPRIORITY_CS4)
                .with_token(&token)
                .build_with_payload(
                    &uuid_builder,
                    "unlock".into(),
                    UPayloadFormat::UPAYLOAD_FORMAT_TEXT,
                )
                .expect("should have been able to create message");

        assert!(message.attributes.id.is_some());
        assert_eq!(message.attributes.permission_level, Some(5));
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS4.into());
        assert_eq!(message.attributes.reqid, Some(request_id).into());
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
        let uuid_builder = UUIDBuilder::new();
        let request_id = uuid_builder.build();
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let request_message =
            UMessageBuilder::request(&method_to_invoke, &reply_to_address, &request_id, 5000)
                .build(&uuid_builder)
                .expect("should have been able to create message");
        let message = UMessageBuilder::response_for_request(&request_message.attributes)
            .with_comm_status(0)
            .with_priority(UPriority::UPRIORITY_CS4)
            .with_ttl(0)
            .build(&uuid_builder)
            .expect("should have been able to create message");
        assert!(message.attributes.id.is_some());
        assert_eq!(message.attributes.commstatus, Some(0));
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS4.into());
        assert_eq!(message.attributes.reqid, Some(request_id).into());
        assert_eq!(message.attributes.sink, Some(reply_to_address).into());
        assert_eq!(message.attributes.source, Some(method_to_invoke).into());
        assert_eq!(message.attributes.ttl, Some(0));
        assert_eq!(
            message.attributes.type_,
            UMessageType::UMESSAGE_TYPE_RESPONSE.into()
        );
    }

    #[test]
    fn test_build_retains_all_response_attributes() {
        let uuid_builder = UUIDBuilder::new();
        let request_id = uuid_builder.build();
        let method_to_invoke = UUri::try_from(METHOD_TO_INVOKE)
            .expect("should have been able to create destination UUri");
        let reply_to_address = UUri::try_from(REPLY_TO_ADDRESS)
            .expect("should have been able to create reply-to UUri");
        let message = UMessageBuilder::response(&reply_to_address, &request_id, &method_to_invoke)
            .with_comm_status(0)
            .with_priority(UPriority::UPRIORITY_CS4)
            .with_ttl(0)
            .build(&uuid_builder)
            .expect("should have been able to create message");
        assert!(message.attributes.id.is_some());
        assert_eq!(message.attributes.commstatus, Some(0));
        assert_eq!(message.attributes.priority, UPriority::UPRIORITY_CS4.into());
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
