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

use bytes::Bytes;

#[cfg(all(feature = "up-l2-api", feature = "protobuf-support"))]
pub(crate) use protobuf_support::deserialize_protobuf_bytes;
pub use umessagebuilder::*;

use crate::{
    SerializationError, UAttributes, UAttributesError, UCode, UMessageType, UPayloadFormat,
    UPriority, UUri, UUID,
};

mod umessagebuilder;

pub(crate) type Payload = Bytes;

#[derive(Debug)]
pub enum UMessageError {
    AttributesValidationError(UAttributesError),
    DataSerializationError(SerializationError),
    PayloadError(String),
}

impl std::fmt::Display for UMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AttributesValidationError(e) => f.write_fmt(format_args!(
                "Builder state is not consistent with message type: {e}"
            )),
            Self::DataSerializationError(e) => {
                f.write_fmt(format_args!("Failed to serialize payload: {e}"))
            }
            Self::PayloadError(e) => f.write_fmt(format_args!("UMessage payload error: {e}")),
        }
    }
}

impl std::error::Error for UMessageError {}

impl From<UAttributesError> for UMessageError {
    fn from(value: UAttributesError) -> Self {
        Self::AttributesValidationError(value)
    }
}

impl From<SerializationError> for UMessageError {
    fn from(value: SerializationError) -> Self {
        Self::DataSerializationError(value)
    }
}

#[cfg(feature = "protobuf-support")]
impl From<protobuf::Error> for UMessageError {
    fn from(value: protobuf::Error) -> Self {
        Self::DataSerializationError(value.into())
    }
}

impl From<String> for UMessageError {
    fn from(value: String) -> Self {
        Self::PayloadError(value)
    }
}

impl From<&str> for UMessageError {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UMessage {
    attributes: UAttributes,
    payload: Option<Payload>,
}

impl UMessage {
    // This convenience constructor is used internally only, e.g. by the UMessageBuilder for creating
    // the final message after validation.
    // Client code can create UMessages using the UMessageBuilder only.
    pub(crate) fn new(
        attributes: UAttributes,
        payload: Option<Bytes>,
    ) -> Result<Self, UMessageError> {
        Ok(UMessage {
            attributes,
            payload,
        })
    }

    /// Get this message's attributes.
    ///
    /// This function simply delegates to [`UAttributes::type_`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).build()?;
    /// assert_eq!(msg.type_(), UMessageType::Publish);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn attributes(&self) -> &UAttributes {
        &self.attributes
    }

    /// Gets this message's type.
    #[must_use]
    pub fn type_(&self) -> UMessageType {
        self.attributes().type_()
    }

    /// Gets this message's identifier.
    ///
    /// This function simply delegates to [`UAttributes::id`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UUri, UUID};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg_id = UUID::build();
    /// let msg = UMessageBuilder::publish(topic).with_message_id(msg_id.clone()).build()?;
    /// assert_eq!(msg.id(), &msg_id);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn id(&self) -> &UUID {
        self.attributes().id()
    }

    /// Gets this message's source address.
    ///
    /// This function simply delegates to [`UAttributes::source`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic.clone()).build()?;
    /// assert_eq!(msg.source(), &topic);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn source(&self) -> &UUri {
        self.attributes().source()
    }

    /// Gets this message's sink address.
    ///
    /// This function simply delegates to [`UAttributes::sink`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let origin = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let dest = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::notification(origin, dest.clone()).build()?;
    /// assert!(msg.sink().is_some_and(|sink| sink == &dest));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn sink(&self) -> Option<&UUri> {
        self.attributes().sink()
    }

    /// Gets this message's sink address.
    ///
    /// This function simply delegates to [`UAttributes::sink_unchecked`].
    ///
    /// # Panics
    ///
    /// if the property has no value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let origin = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let dest = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::notification(origin, dest.clone()).build()?;
    /// assert_eq!(msg.sink_unchecked(), &dest);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn sink_unchecked(&self) -> &UUri {
        self.attributes().sink_unchecked()
    }

    /// Gets this message's priority.
    ///
    /// This function simply delegates to [`UAttributes::priority`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).with_priority(UPriority::CS3).build()?;
    /// assert!(msg.priority().is_some_and(|prio| prio == UPriority::CS3));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn priority(&self) -> Option<UPriority> {
        self.attributes().priority()
    }

    /// Gets this message's priority.
    ///
    /// This function simply delegates to [`UAttributes::priority_unchecked`].
    ///
    /// # Panics
    ///
    /// if the property has no value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UMessageType, UPriority, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).with_priority(UPriority::CS3).build()?;
    /// assert_eq!(msg.priority_unchecked(), UPriority::CS3);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn priority_unchecked(&self) -> UPriority {
        self.attributes().priority_unchecked()
    }

    /// Gets this message's commstatus.
    ///
    /// This function simply delegates to [`UAttributes::commstatus`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::response(reply_to, UUID::build(), invoked_method)
    ///   .with_comm_status(UCode::Ok)
    ///   .build()?;
    /// assert!(msg.commstatus().is_some_and(|status| status == UCode::Ok));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn commstatus(&self) -> Option<UCode> {
        self.attributes().commstatus()
    }

    /// Gets this message's commstatus.
    ///
    /// This function simply delegates to [`UAttributes::commstatus_unchecked`].
    ///
    /// # Panics
    ///
    /// if the property has no value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UMessageType, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::response(reply_to, UUID::build(), invoked_method)
    ///   .with_comm_status(UCode::Internal)
    ///   .build()?;
    /// assert_eq!(msg.commstatus_unchecked(), UCode::Internal);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn commstatus_unchecked(&self) -> UCode {
        self.attributes().commstatus_unchecked()
    }

    /// Gets this message's time-to-live.
    ///
    /// This function simply delegates to [`UAttributes::ttl`].
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::request(invoked_method, reply_to, 5000)
    ///   .build()?;
    /// assert!(msg.ttl().is_some_and(|ttl| ttl == 5000));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn ttl(&self) -> Option<u32> {
        self.attributes().ttl()
    }

    /// Gets this message's time-to-live.
    ///
    /// This function simply delegates to [`UAttributes::ttl_unchecked`].
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::request(invoked_method, reply_to, 5000)
    ///   .build()?;
    /// assert!(msg.ttl_unchecked() == 5000);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn ttl_unchecked(&self) -> u32 {
        self.attributes().ttl_unchecked()
    }

    /// Gets this message's permission level.
    ///
    /// This function simply delegates to [`UAttributes::permission_level`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::request(invoked_method, reply_to, 5000)
    ///   .with_permission_level(3)
    ///   .build()?;
    /// assert!(msg.permission_level().is_some_and(|pl| pl == 3));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn permission_level(&self) -> Option<u32> {
        self.attributes().permission_level()
    }

    /// Gets this message's token.
    ///
    /// This function simply delegates to [`UAttributes::token`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let token = "my_token";
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::request(invoked_method, reply_to, 5000)
    ///   .with_token(token)
    ///   .build()?;
    /// assert!(msg.token().is_some_and(|t| t == token));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.attributes().token()
    }

    /// Gets this message's traceparent.
    ///
    /// This function simply delegates to [`UAttributes::traceparent`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let traceparent = "my_traceparent";
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::request(invoked_method, reply_to, 5000)
    ///   .with_traceparent(traceparent)
    ///   .build()?;
    /// assert!(msg.traceparent().is_some_and(|tp| tp == traceparent));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn traceparent(&self) -> Option<&str> {
        self.attributes().traceparent()
    }

    /// Gets this message's request identifier.
    ///
    /// This function simply delegates to [`UAttributes::request_id`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let request_id = UUID::build();
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::response(reply_to, request_id.clone(), invoked_method)
    ///   .build()?;
    /// assert!(msg.request_id().is_some_and(|id| id == &request_id));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn request_id(&self) -> Option<&UUID> {
        self.attributes().request_id()
    }

    /// Gets this message's request identifier.
    ///
    /// This function simply delegates to [`UAttributes::request_id_unchecked`].
    ///
    /// # Panics
    ///
    /// if the property has no value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let request_id = UUID::build();
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::response(reply_to, request_id.clone(), invoked_method)
    ///   .build()?;
    /// assert_eq!(msg.request_id_unchecked(), &request_id);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn request_id_unchecked(&self) -> &UUID {
        self.attributes().request_id_unchecked()
    }

    /// Gets this message's payload format.
    ///
    /// This function simply delegates to [`UAttributes::payload_format`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UPayloadFormat, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic)
    ///   .build_with_payload("hello".as_bytes(), UPayloadFormat::Text)?;
    /// assert!(msg.payload_format().is_some_and(|format| format == UPayloadFormat::Text));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn payload_format(&self) -> Option<UPayloadFormat> {
        self.attributes().payload_format()
    }

    /// Gets this message's payload format.
    ///
    /// This function simply delegates to [`UAttributes::payload_format_unchecked`].
    ///
    /// # Panics
    ///
    /// if the property has no value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UPayloadFormat, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic)
    ///   .build_with_payload("hello".as_bytes(), UPayloadFormat::Text)?;
    /// assert_eq!(msg.payload_format_unchecked(), UPayloadFormat::Text);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn payload_format_unchecked(&self) -> UPayloadFormat {
        self.attributes().payload_format_unchecked()
    }

    #[must_use]
    pub fn payload(&self) -> Option<Bytes> {
        self.payload.clone()
    }

    /// Checks if this is a Publish message.
    ///
    /// This function simply delegates to [`UAttributes::is_publish`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).build()?;
    /// assert!(msg.is_publish());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_publish(&self) -> bool {
        self.attributes().is_publish()
    }

    /// Checks if this is an RPC Request message.
    ///
    /// This function simply delegates to [`UAttributes::is_request`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::request(invoked_method, reply_to, 5000)
    ///   .build()?;
    /// assert!(msg.is_request());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_request(&self) -> bool {
        self.attributes().is_request()
    }

    /// Checks if this is an RPC Response message.
    ///
    /// This function simply delegates to [`UAttributes::is_response`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UMessageBuilder, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let invoked_method = UUri::try_from("//my-vehicle/D45/2/101")?;
    /// let reply_to = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::response(reply_to, UUID::build(), invoked_method)
    ///   .build()?;
    /// assert!(msg.is_response());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_response(&self) -> bool {
        self.attributes().is_response()
    }

    /// Checks if this is a Notification message.
    ///
    /// This function simply delegates to [`UAttributes::is_notification`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let origin = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let dest = UUri::try_from("//other-vehicle/D10/3/0")?;
    /// let msg = UMessageBuilder::notification(origin, dest).build()?;
    /// assert!(msg.is_notification());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_notification(&self) -> bool {
        self.attributes().is_notification()
    }

    /// Checks if this message should be considered expired.
    ///
    /// This function simply delegates to [`UAttributes::check_expired`].
    pub fn check_expired(&self) -> Result<(), UAttributesError> {
        self.attributes().check_expired()
    }

    /// Checks if this message should be considered expired for a given reference time.
    ///
    /// This function simply delegates to [`UAttributes::check_expired_for_reference`].
    ///
    /// # Arguments
    ///
    /// * `reference_time` - The reference time as milliseconds since UNIX epoch. The check will
    ///   be performed in relation to this point in time.
    pub fn check_expired_for_reference(
        &self,
        reference_time: u128,
    ) -> Result<(), UAttributesError> {
        self.attributes()
            .check_expired_for_reference(reference_time)
    }
}

#[cfg(feature = "protobuf-support")]
mod protobuf_support {
    use super::*;
    use crate::ProtobufMappable;

    impl UMessage {
        /// Deserializes this message's protobuf payload into a type.
        ///
        /// # Type Parameters
        ///
        /// * `T`: The target type of the data to be unpacked.
        ///
        /// # Errors
        ///
        /// Returns an error if the message payload format is neither [UPayloadFormat::Protobuf] nor
        /// [UPayloadFormat::ProtobufWrappedInAny] or if the bytes in the
        /// payload cannot be deserialized into the target type.
        #[cfg(feature = "protobuf-support")]
        pub fn extract_protobuf<T: ProtobufMappable>(&self) -> Result<T, UMessageError> {
            if let Some(payload) = self.payload.as_ref() {
                let payload_format = self.payload_format().unwrap_or(UPayloadFormat::Unspecified);
                deserialize_protobuf_bytes(payload, &payload_format)
            } else {
                Err(UMessageError::PayloadError(
                    "Message has no payload".to_string(),
                ))
            }
        }
    }

    /// Deserializes a protobuf message from a byte array.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload data.
    /// * `payload_format` - The format/encoding of the data. Must be one of
    ///    - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF`
    ///    - `UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`
    ///
    /// # Errors
    ///
    /// Returns an error if the payload format is unsupported or if the data can not be deserialized
    /// into the target type based on the given format.
    #[cfg(feature = "protobuf-support")]
    pub(crate) fn deserialize_protobuf_bytes<T: ProtobufMappable>(
        payload: &[u8],
        payload_format: &UPayloadFormat,
    ) -> Result<T, UMessageError> {
        match payload_format {
            UPayloadFormat::Protobuf => {
                T::parse_from_protobuf_bytes(payload).map_err(UMessageError::DataSerializationError)
            }
            UPayloadFormat::ProtobufWrappedInAny => T::parse_from_packed_protobuf_bytes(payload)
                .map_err(UMessageError::DataSerializationError),
            UPayloadFormat::Unspecified
            | UPayloadFormat::Json
            | UPayloadFormat::Raw
            | UPayloadFormat::Shm
            | UPayloadFormat::Someip
            | UPayloadFormat::SomeipTlv
            | UPayloadFormat::Text => {
                let detail_msg = payload_format.to_media_type().map_or_else(
                    || format!("Unknown payload format: {}", *payload_format as i32),
                    |mt| format!("Invalid/unsupported payload format: {mt}"),
                );
                Err(UMessageError::from(detail_msg))
            }
        }
    }

    #[cfg(test)]
    mod protobuf_support_test {
        use super::*;
        use crate::UUri;

        use protobuf::well_known_types::{
            any::Any,
            duration::Duration,
            wrappers::{DoubleValue, StringValue},
        };
        use protobuf::Message;
        use test_case::test_case;

        #[test]
        fn test_from_protobuf_error() {
            let protobuf_error = protobuf::Error::from(std::io::Error::last_os_error());
            let message_error = UMessageError::from(protobuf_error);
            assert!(matches!(
                message_error,
                UMessageError::DataSerializationError(_)
            ));
        }

        #[test]
        fn test_deserialize_protobuf_bytes_succeeds() {
            let mut data = StringValue::new();
            data.value = "hello world".to_string();

            let result = deserialize_protobuf_bytes::<StringValue>(
                &data
                    .write_to_bytes()
                    .expect("Failed to write protobuf bytes"),
                &UPayloadFormat::Protobuf,
            );
            assert!(result.is_ok_and(|v| v.value == *"hello world"));

            let any = Any::pack(&data).expect("Failed to pack Any");
            let buf: Bytes = any
                .write_to_bytes()
                .expect("Failed to write protobuf bytes")
                .into();

            let result = deserialize_protobuf_bytes::<StringValue>(
                &buf,
                &UPayloadFormat::ProtobufWrappedInAny,
            );
            assert!(result.is_ok_and(|v| v.value == *"hello world"));
        }

        #[test]
        fn test_deserialize_protobuf_bytes_fails_for_payload_type_mismatch() {
            let mut data = StringValue::new();
            data.value = "hello world".to_string();
            let any = Any::pack(&data).unwrap();
            let buf: Bytes = any.write_to_bytes().unwrap().into();
            let result = deserialize_protobuf_bytes::<DoubleValue>(
                &buf,
                &UPayloadFormat::ProtobufWrappedInAny,
            );
            assert!(result.is_err_and(|e| matches!(e, UMessageError::DataSerializationError(_))));
        }

        #[test_case(UPayloadFormat::Json; "JSON format")]
        #[test_case(UPayloadFormat::Raw; "RAW format")]
        #[test_case(UPayloadFormat::Shm; "SHM format")]
        #[test_case(UPayloadFormat::Someip; "SOMEIP format")]
        #[test_case(UPayloadFormat::SomeipTlv; "SOMEIP TLV format")]
        #[test_case(UPayloadFormat::Text; "TEXT format")]
        #[test_case(UPayloadFormat::Unspecified; "UNSPECIFIED format")]
        fn test_deserialize_protobuf_bytes_fails_for_(format: UPayloadFormat) {
            let result = deserialize_protobuf_bytes::<StringValue>("hello".as_bytes(), &format);
            assert!(result.is_err_and(|e| matches!(e, UMessageError::PayloadError(_))));
        }

        #[test]
        fn test_deserialize_protobuf_bytes_fails_for_invalid_encoding() {
            // GIVEN a protobuf Any message with an embedded Duration message, but with invalid encoding
            // (i.e. the value field does not contain a valid protobuf encoding of a Duration message)
            let any = Any {
                type_url: "type.googleapis.com/google.protobuf.Duration".to_string(),
                value: vec![0x0A],
                ..Default::default()
            };
            let buf = any.write_to_bytes().unwrap();

            // WHEN deserializing the bytes into a Duration message using the ProtobufWrappedInAny format
            let result = deserialize_protobuf_bytes::<Duration>(
                buf.as_slice(),
                &UPayloadFormat::ProtobufWrappedInAny,
            );
            // THEN the deserialization fails with a DataSerializationError
            assert!(result.is_err_and(|e| matches!(e, UMessageError::DataSerializationError(_))))
        }

        #[test]
        fn extract_payload_succeeds() {
            let payload = StringValue {
                value: "hello".to_string(),
                ..Default::default()
            };
            let topic = UUri::try_from_parts("local", 0xabcd, 0x01, 0x9000)
                .expect("failed to create topic");
            let msg = UMessageBuilder::publish(topic)
                .build_with_protobuf_payload(&payload)
                .expect("failed to create message");
            assert!(msg
                .extract_protobuf::<StringValue>()
                .is_ok_and(|v| v.value == *"hello"));
        }

        #[test]
        fn extract_payload_fails_for_no_payload() {
            let topic = UUri::try_from_parts("local", 0xabcd, 0x01, 0x9000)
                .expect("failed to create topic");
            let msg = UMessageBuilder::publish(topic)
                .build()
                .expect("failed to create message");
            assert!(msg
                .extract_protobuf::<StringValue>()
                .is_err_and(|e| matches!(e, UMessageError::PayloadError(_))));
        }
    }
}

#[cfg(feature = "up-core-types")]
mod core_types_support {
    use protobuf::{well_known_types::any::Any, Message};

    use super::*;
    use crate::up_core_api::uattributes::UAttributes as UAttributesProto;
    use crate::up_core_api::umessage::UMessage as UMessageProto;
    use crate::ProtobufMappable;

    impl From<&UMessage> for UMessageProto {
        fn from(value: &UMessage) -> Self {
            let attributes = UAttributesProto::from(&value.attributes);
            UMessageProto {
                attributes: Some(attributes).into(),
                payload: value.payload.clone(),
                ..Default::default()
            }
        }
    }

    impl TryFrom<&UMessageProto> for UMessage {
        type Error = UMessageError;
        fn try_from(value: &UMessageProto) -> Result<Self, Self::Error> {
            let mut attributes = value.attributes.as_ref().map_or_else(
                || {
                    Err(UAttributesError::validation_error(
                        "UMessageProto has no attributes",
                    ))
                },
                UAttributes::try_from,
            )?;
            // The uattributes.proto file in up-spec v1.6.0.alpha.7 does not declare the payload_format field
            // as optional. Consequently, a UMessage protobuf that does not have a payload still ALWAYS has
            // the UNSPECIFIED payload_format when being deserialized.
            // When mapping such a message to the (internal) UMessage struct, we therefore need to also consider
            // whether the message actually has payload or not, in order to set the payload_format field to the
            // proper value, i.e. Some(Unspecified), if the message has payload, or None otherwise.
            // This is relevant, because the presence of a payload_format value in the UMessage struct is used to
            // determine whether the message has payload or not, e.g. when serializing the message back to
            // protobuf or when extracting the payload as a protobuf message.
            //
            // This should no longer be necessary once the payload_format field in uattributes.proto is declared as
            // optional in the UP specification and the protobuf definitions are updated accordingly.
            //
            if value.payload.is_none() {
                attributes.payload_format = None;
            }
            UMessage::new(attributes, value.payload.clone())
        }
    }

    impl ProtobufMappable for UMessage {
        fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
            let proto = UMessageProto::parse_from_bytes(proto)?;
            UMessage::try_from(&proto).map_err(|e| SerializationError::new(e.to_string()))
        }

        fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
            Any::parse_from_bytes(proto)
                .map_err(|err| crate::SerializationError::new(err.to_string()))
                .and_then(|any| match any.unpack::<UMessageProto>() {
                    Ok(Some(umessage_proto)) => UMessage::try_from(&umessage_proto)
                        .map_err(|e| crate::SerializationError::new(e.to_string())),
                    Ok(None) => Err(crate::SerializationError::new(
                        "Protobuf Any does not contain UMessage".to_string(),
                    )),
                    Err(e) => Err(crate::SerializationError::new(format!(
                        "Protobuf Any unpack error: {e}"
                    ))),
                })
        }

        fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, SerializationError> {
            Ok(UMessageProto::from(self).write_to_bytes()?)
        }

        fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, SerializationError> {
            Any::pack(&UMessageProto::from(self))
                .map_err(|e| {
                    crate::SerializationError::new(format!("Failed to pack UMessage: {e}"))
                })
                .and_then(|any| any.write_to_protobuf_bytes())
        }
    }

    #[cfg(test)]
    mod test {
        use protobuf::{Enum, EnumOrUnknown};

        use super::*;
        use crate::up_core_api::uattributes::UAttributes as UAttributesProto;
        use crate::up_core_api::umessage::UMessage as UMessageProto;

        #[test]
        fn test_try_from_umessage_proto_fails_for_missing_attributes() {
            let proto = UMessageProto {
                attributes: None.into(),
                ..Default::default()
            };
            let result = UMessage::try_from(&proto);
            assert!(result.is_err_and(|e| matches!(e, UMessageError::AttributesValidationError(_))));
        }

        #[test_case::test_case(None => None; "for no payload")]
        #[test_case::test_case(Some("payload") => Some(UPayloadFormat::Unspecified); "for some payload")]
        fn test_try_from_handles_payload_format(payload: Option<&str>) -> Option<UPayloadFormat> {
            let valid_attribs_proto = UAttributesProto {
                type_: EnumOrUnknown::from_i32(
                    crate::up_core_api::uattributes::UMessageType::UMESSAGE_TYPE_PUBLISH.value(),
                ),
                id: Some(crate::up_core_api::uuid::UUID {
                    msb: 0x0000000000017000_u64,
                    lsb: 0x8010101010101a1a_u64,
                    ..Default::default()
                })
                .into(),
                source: Some(crate::up_core_api::uri::UUri {
                    authority_name: "source".to_string(),
                    ue_id: 0x0001,
                    ue_version_major: 0x01,
                    resource_id: 0x0001,
                    ..Default::default()
                })
                .into(),
                priority: EnumOrUnknown::from_i32(
                    crate::up_core_api::uattributes::UPriority::UPRIORITY_UNSPECIFIED.value(),
                ),
                payload_format: EnumOrUnknown::from_i32(
                    crate::up_core_api::uattributes::UPayloadFormat::UPAYLOAD_FORMAT_UNSPECIFIED
                        .value(),
                ),
                ..Default::default()
            };
            // GIVEN a UMessageProto with valid attributes (including the UNSPECIFIED payload format) but no payload
            let proto = UMessageProto {
                attributes: Some(valid_attribs_proto).into(),
                payload: payload.map(|p| p.as_bytes().to_vec().into()),
                ..Default::default()
            };
            // WHEN converting the UMessageProto to a UMessage
            UMessage::try_from(&proto)
                .expect("failed to convert UMessageProto to UMessage")
                .payload_format()
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_from_attributes_error() {
        let attributes_error = UAttributesError::validation_error("failed to validate");
        let message_error = UMessageError::from(attributes_error);
        assert!(matches!(
            message_error,
            UMessageError::AttributesValidationError(UAttributesError::ValidationError(_))
        ));
    }

    #[test]
    fn test_from_error_msg() {
        let message_error = UMessageError::from("an error occurred");
        assert!(matches!(message_error, UMessageError::PayloadError(_)));
    }
}
