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

mod uattributesvalidator;
mod umessagetype;
mod upayloadformat;
mod upriority;

use thiserror::Error;

pub use uattributesvalidator::*;
pub use umessagetype::UMessageType;
pub use upayloadformat::*;
pub use upriority::*;

use crate::uuid::UuidConversionError;
use crate::{UCode, UUri, UUriError, UUID};

pub(crate) const UPRIORITY_DEFAULT: UPriority = UPriority::CS1;
pub(crate) type TokenString = String;
pub(crate) type TraceparentString = String;

/// Errors produced when building or validating [`UAttributes`].
#[derive(Debug, Error)]
pub enum UAttributesError {
    /// The attributes violate a validation rule for their message type.
    #[error("Validation failure: {0}")]
    ValidationError(String),
    /// The message's time-to-live has elapsed.
    #[error("Message expired")]
    ExpiredError,
    /// The attributes could not be parsed from their serialized form.
    #[error("Parsing error: {0}")]
    ParsingError(String),
}

impl UAttributesError {
    /// Creates a validation error with the given message.
    pub fn validation_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ValidationError(message.into())
    }

    /// Creates a parsing error with the given message.
    pub fn parsing_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ParsingError(message.into())
    }
}

impl From<UUriError> for UAttributesError {
    fn from(value: UUriError) -> Self {
        UAttributesError::parsing_error(value.to_string())
    }
}

impl From<UuidConversionError> for UAttributesError {
    fn from(value: UuidConversionError) -> Self {
        UAttributesError::parsing_error(value.to_string())
    }
}

impl From<UPayloadError> for UAttributesError {
    fn from(value: UPayloadError) -> Self {
        UAttributesError::parsing_error(value.to_string())
    }
}

/// The attributes of a uProtocol message.
///
/// These attributes provide metadata about the message, such as its type, identifier, source and destination addresses, priority, time-to-live, etc. They are used by the uProtocol library to manage the message and its
/// lifecycle, and can also be accessed by applications to make informed decisions about how to handle the message.
///
/// Attributes are immutable and are set when the message is created using [crate::UMessageBuilder]. They cannot be
/// changed afterwards.
///
/// The attributes of a message can be accessed via the [`crate::UMessage::attributes`] function. For convenience,
/// [`crate::UMessage`] also has functions for directly accessing the attribute values, mimicking the functions
/// provided by this struct.
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UAttributes {
    pub(crate) type_: UMessageType,
    pub(crate) id: UUID,
    pub(crate) source: UUri,
    pub(crate) sink: Option<UUri>,
    pub(crate) priority: Option<UPriority>,
    pub(crate) commstatus: Option<UCode>,
    pub(crate) ttl: Option<u32>,
    pub(crate) permission_level: Option<u32>,
    pub(crate) token: Option<TokenString>,
    pub(crate) traceparent: Option<TraceparentString>,
    pub(crate) reqid: Option<UUID>,
    pub(crate) payload_format: Option<UPayloadFormat>,
}

impl UAttributes {
    /// Gets the type of message these are the attributes of.
    #[must_use]
    pub fn type_(&self) -> UMessageType {
        self.type_
    }

    /// Gets the identifier of the message these attributes belong to.
    #[must_use]
    pub fn id(&self) -> &UUID {
        &self.id
    }

    /// Gets the source address of the message these attributes belong to.
    #[must_use]
    pub fn source(&self) -> &UUri {
        &self.source
    }

    /// Gets the sink address of the message these attributes belong to.
    #[must_use]
    pub fn sink(&self) -> Option<&UUri> {
        self.sink.as_ref()
    }

    /// Gets the sink address of the message these attributes belong to.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn sink_unchecked(&self) -> &UUri {
        self.sink().expect("message has no sink")
    }

    /// Gets the priority of the message these attributes belong to.
    #[must_use]
    pub fn priority(&self) -> Option<UPriority> {
        self.priority
    }

    /// Gets the priority of the message these attributes belong to.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn priority_unchecked(&self) -> UPriority {
        self.priority().expect("message has no priority")
    }

    /// Gets the commstatus of the message these attributes belong to.
    #[must_use]
    pub fn commstatus(&self) -> Option<UCode> {
        self.commstatus
    }

    /// Gets the commstatus of the message these attributes belong to.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn commstatus_unchecked(&self) -> UCode {
        self.commstatus().expect("message has no commstatus")
    }

    /// Gets the time-to-live of the message these attributes belong to.
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    #[must_use]
    pub fn ttl(&self) -> Option<u32> {
        self.ttl
    }

    /// Gets the time-to-live of the message these attributes belong to.
    ///
    /// # Returns
    ///
    /// the time-to-live in milliseconds.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn ttl_unchecked(&self) -> u32 {
        self.ttl().expect("message has no time-to-live")
    }

    /// Gets the permission level of the message these attributes belong to.
    #[must_use]
    pub fn permission_level(&self) -> Option<u32> {
        self.permission_level
    }

    /// Gets the token of the message these attributes belong to.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Gets the traceparent of the message these attributes belong to.
    #[must_use]
    pub fn traceparent(&self) -> Option<&str> {
        self.traceparent.as_deref()
    }

    /// Gets the request identifier of the message these attributes belong to.
    #[must_use]
    pub fn request_id(&self) -> Option<&UUID> {
        self.reqid.as_ref()
    }

    /// Gets the request identifier of the message these attributes belong to.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn request_id_unchecked(&self) -> &UUID {
        self.request_id().expect("message has no request ID")
    }

    /// Gets the payload format of the message these attributes belong to.
    #[must_use]
    pub fn payload_format(&self) -> Option<UPayloadFormat> {
        self.payload_format
    }

    /// Gets the payload format of the message these attributes belong to.
    ///
    /// # Panics
    ///
    /// if the property has no value.
    #[must_use]
    pub fn payload_format_unchecked(&self) -> UPayloadFormat {
        self.payload_format()
            .expect("message has no payload format")
    }

    /// Checks if a given priority class is the default priority class.
    ///
    /// Messages that do not have a priority class set explicity, are assigned to
    /// the default priority class.
    pub(crate) fn is_default_priority(prio: UPriority) -> bool {
        prio == UPRIORITY_DEFAULT
    }

    /// Checks if these are the attributes for a Publish message.
    #[must_use]
    pub fn is_publish(&self) -> bool {
        self.type_ == UMessageType::Publish
    }

    /// Checks if these are the attributes for an RPC Request message.
    #[must_use]
    pub fn is_request(&self) -> bool {
        self.type_ == UMessageType::Request
    }

    /// Checks if these are the attributes for an RPC Response message.
    #[must_use]
    pub fn is_response(&self) -> bool {
        self.type_ == UMessageType::Response
    }

    /// Checks if these are the attributes for a Notification message.
    #[must_use]
    pub fn is_notification(&self) -> bool {
        self.type_ == UMessageType::Notification
    }

    /// Checks if the message that is described by these attributes should be considered expired.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::ttl`] (time-to-live) contains a value greater than 0, but
    /// * the current system time cannot be determined, or
    /// * the message has expired according to the timestamp extracted from [`Self::id`] and the time-to-live value.
    ///
    /// # Example
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use up_rust::{UMessageBuilder, UUri};
    /// let publish_msg = UMessageBuilder::publish(UUri::try_from("//my-vehicle/D45/1/A001")?)
    ///     .build()?;
    /// assert!(publish_msg.attributes().check_expired().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_expired(&self) -> Result<(), UAttributesError> {
        if let Some(ttl) = self.ttl() {
            self.id().check_expired(ttl)
        } else {
            Ok(())
        }
    }

    /// Checks if the message that is described by these attributes should be considered expired.
    ///
    /// # Arguments
    ///
    /// * `reference_time` - The reference time as milliseconds since UNIX epoch. The check will
    ///   be performed in relation to this point in time.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::ttl`] (time-to-live) contains a value greater than 0, but
    /// the message has expired according to the timestamp extracted from [`Self::id`], the
    /// time-to-live value and the provided reference time.
    ///
    /// # Example
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::time::SystemTime;
    /// use up_rust::{UMessageBuilder, UUri};
    ///
    /// let now = SystemTime::now()
    ///     .duration_since(SystemTime::UNIX_EPOCH)?
    ///     .as_millis();
    ///
    /// let reference_time = now + 300_000; // 5 minutes from now
    /// let publish_msg = UMessageBuilder::publish(UUri::try_from("//my-vehicle/D45/1/A001")?)
    ///     .with_ttl(5_000) // Set a TTL of 5 seconds
    ///     .build()?;
    /// assert!(publish_msg.attributes().check_expired_for_reference(reference_time).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_expired_for_reference(
        &self,
        reference_time: u128,
    ) -> Result<(), UAttributesError> {
        if let Some(ttl) = self.ttl() {
            self.id().check_expired_for_reference(ttl, reference_time)
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "up-core-api")]
mod core_types_support {
    use protobuf::{well_known_types::any::Any, Message};

    use super::*;
    use crate::up_core_api::uattributes::{
        UAttributes as UAttributesProto, UMessageType as UMessageTypeProto,
        UPayloadFormat as UPayloadFormatProto, UPriority as UPriorityProto,
    };
    use crate::up_core_api::{
        ucode::UCode as UCodeProto, uri::UUri as UUriProto, uuid::UUID as UUIDProto,
    };
    use crate::ProtobufMappable;

    impl From<&UAttributes> for UAttributesProto {
        fn from(attribs: &UAttributes) -> Self {
            UAttributesProto {
                id: Some(UUIDProto::from(&attribs.id)).into(),
                type_: UMessageTypeProto::from(&attribs.type_).into(),
                source: Some(UUriProto::from(&attribs.source)).into(),
                sink: attribs.sink.as_ref().map(UUriProto::from).into(),
                priority: attribs
                    .priority
                    .map_or(UPriorityProto::UPRIORITY_UNSPECIFIED, |p| {
                        UPriorityProto::from(&p)
                    })
                    .into(),
                ttl: attribs.ttl,
                permission_level: attribs.permission_level,
                reqid: attribs.reqid.as_ref().map(UUIDProto::from).into(),
                commstatus: attribs.commstatus.map(|cs| UCodeProto::from(cs).into()),
                token: attribs.token().map(|t| t.to_string()),
                traceparent: attribs.traceparent().map(|t| t.to_string()),
                payload_format: attribs
                    .payload_format
                    .map_or(UPayloadFormatProto::UPAYLOAD_FORMAT_UNSPECIFIED, |pf| {
                        UPayloadFormatProto::from(&pf)
                    })
                    .into(),
                ..Default::default()
            }
        }
    }

    impl TryFrom<&UAttributesProto> for UAttributes {
        type Error = UAttributesError;

        fn try_from(attribs_proto: &UAttributesProto) -> Result<Self, Self::Error> {
            Ok(UAttributes {
                commstatus: match attribs_proto.commstatus {
                    None => None,
                    Some(cs) => cs
                        .enum_value()
                        .map_err(|e| {
                            UAttributesError::parsing_error(format!(
                                "unknown commstatus value: {e}"
                            ))
                        })
                        .map(UCode::from)
                        .map(Some)?,
                },
                id: UUID::try_from(
                    attribs_proto
                        .id
                        .as_ref()
                        .ok_or_else(|| UAttributesError::parsing_error("missing message ID"))?,
                )?,
                type_: attribs_proto
                    .type_
                    .enum_value()
                    .map_err(|e| {
                        UAttributesError::parsing_error(format!("unknown message type value: {e}"))
                    })
                    .and_then(UMessageType::try_from)?,
                source: UUri::try_from(
                    attribs_proto
                        .source
                        .as_ref()
                        .ok_or_else(|| UAttributesError::parsing_error("missing source URI"))?,
                )?,
                sink: match attribs_proto.sink.as_ref() {
                    Some(s) => Some(UUri::try_from(s)?),
                    None => None,
                },
                priority: match attribs_proto.priority.enum_value() {
                    Ok(UPriorityProto::UPRIORITY_UNSPECIFIED) => None,
                    Ok(p) => Some(UPriority::try_from(p)?),
                    Err(e) => {
                        return Err(UAttributesError::parsing_error(format!(
                            "unknown priority value: {e}"
                        )))
                    }
                },
                ttl: attribs_proto.ttl,
                permission_level: attribs_proto.permission_level,
                token: attribs_proto.token.as_ref().map(|t| t.to_owned()),
                traceparent: attribs_proto.traceparent.as_ref().map(|t| t.to_owned()),
                reqid: match attribs_proto.reqid.as_ref() {
                    Some(r) => Some(UUID::try_from(r)?),
                    None => None,
                },
                payload_format: match attribs_proto.payload_format.enum_value() {
                    Ok(pf) => Some(UPayloadFormat::from(pf)),
                    Err(e) => {
                        return Err(UAttributesError::parsing_error(format!(
                            "unknown payload format value: {e}"
                        )))
                    }
                },
            })
        }
    }

    impl ProtobufMappable for UAttributes {
        fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
            let uattributes_proto = UAttributesProto::parse_from_bytes(proto)?;
            UAttributes::try_from(&uattributes_proto).map_err(|e| {
                crate::SerializationError::new(format!("UAttributes conversion error: {e}"))
            })
        }
        fn parse_from_packed_protobuf_bytes(
            proto: &[u8],
        ) -> Result<Self, crate::SerializationError> {
            Any::parse_from_bytes(proto)
                .map_err(|err| crate::SerializationError::new(err.to_string()))
                .and_then(|any| match any.unpack::<UAttributesProto>() {
                    Ok(Some(uattributes_proto)) => UAttributes::try_from(&uattributes_proto)
                        .map_err(|e| crate::SerializationError::new(e.to_string())),
                    Ok(None) => Err(crate::SerializationError::new(
                        "Protobuf Any does not contain UAttributes".to_string(),
                    )),
                    Err(e) => Err(crate::SerializationError::new(format!(
                        "Protobuf Any unpack error: {e}"
                    ))),
                })
        }
        fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
            Ok(UAttributesProto::from(self).write_to_bytes()?)
        }
        fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
            Any::pack(&UAttributesProto::from(self))
                .map_err(|e| {
                    crate::SerializationError::new(format!("Failed to pack UAttributes: {e}"))
                })
                .and_then(|any| any.write_to_protobuf_bytes())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use protobuf::EnumOrUnknown;

        #[test_case::test_case(|attribs| attribs => matches Ok(()); "succeeds for valid values")]
        #[test_case::test_case(|mut attribs| { attribs.type_ = EnumOrUnknown::from_i32(-1); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid message type")]
        #[test_case::test_case(|mut attribs| { attribs.id = None.into(); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for missing message id")]
        #[test_case::test_case(|mut attribs| { attribs.id.as_mut().unwrap().msb = 0x0000000000018000_u64; attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid message id")]
        #[test_case::test_case(|mut attribs| { attribs.source = None.into(); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for missing source")]
        #[test_case::test_case(|mut attribs| { attribs.source.as_mut().unwrap().authority_name = "INVALID".to_string(); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid source")]
        #[test_case::test_case(|mut attribs| { attribs.sink.as_mut().unwrap().authority_name = "INVALID".to_string(); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid sink")]
        #[test_case::test_case(|mut attribs| { attribs.priority = EnumOrUnknown::from_i32(-1); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid priority")]
        #[test_case::test_case(|mut attribs| { attribs.commstatus = Some(EnumOrUnknown::from_i32(-1)); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid commstatus")]
        #[test_case::test_case(|mut attribs| { attribs.reqid.as_mut().unwrap().msb = 0x0000000000018000_u64; attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid request id")]
        #[test_case::test_case(|mut attribs| { attribs.payload_format = EnumOrUnknown::from_i32(-1); attribs } => matches Err(UAttributesError::ParsingError(_)); "fails for invalid payload format")]
        fn test_try_from_attributes<F: FnOnce(UAttributesProto) -> UAttributesProto>(
            mutator: F,
        ) -> Result<(), UAttributesError> {
            let valid_attribs_proto = UAttributesProto {
                type_: EnumOrUnknown::from_i32(3), // corresponds to Response
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
                sink: Some(crate::up_core_api::uri::UUri {
                    ue_id: 0x1000,
                    ue_version_major: 0x01,
                    resource_id: 0x0000,
                    ..Default::default()
                })
                .into(),
                priority: EnumOrUnknown::from_i32(4), // CS4
                commstatus: Some(EnumOrUnknown::from_i32(5)), // NOT_FOUND
                ttl: None,
                permission_level: None,
                token: None,
                traceparent: None,
                reqid: Some(crate::up_core_api::uuid::UUID {
                    msb: 0x0000000000017000_u64,
                    lsb: 0x8010101010101a1b_u64,
                    ..Default::default()
                })
                .into(),
                payload_format: EnumOrUnknown::from_i32(3), // JSON
                ..Default::default()
            };
            let attribs_proto = mutator(valid_attribs_proto);
            UAttributes::try_from(&attribs_proto).map(|_| ())
        }
    }
}
