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

use protobuf::Enum;

use crate::{UAttributes, UMessageType, UPriority, UUri};

use crate::UAttributesError;

/// `UAttributes` is the struct that defines the Payload. It serves as the configuration for various aspects
/// like time to live, priority, security tokens, and more. Each variant of `UAttributes` defines a different
/// type of message payload. The payload could represent a simple published payload with some state change,
/// an RPC request payload, or an RPC response payload.
///
/// `UAttributesValidator` is a trait implemented by all validators for `UAttributes`. It provides functionality
/// to help validate that a given `UAttributes` instance is correctly configured to define the Payload.
pub trait UAttributesValidator: Send {
    /// Checks if a given set of attributes complies with the rules specified for
    /// the type of message they describe.
    ///
    /// # Errors
    ///
    /// Returns an error if the attributes are not consistent with the rules specified for the message type.
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError>;

    /// Verifies that this validator is appropriate for a set of attributes.
    ///
    /// # Errors
    ///
    /// Returns an error if [`UAttributes::type_`] does not match the type returned by [`UAttributesValidator::message_type`].
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let expected_type = self.message_type();
        match attributes.type_.enum_value() {
            Ok(mt) if mt == expected_type => Ok(()),
            Ok(mt) => Err(UAttributesError::validation_error(format!(
                "Wrong Message Type [{}]",
                mt.to_cloudevent_type()
            ))),
            Err(unknown_code) => Err(UAttributesError::validation_error(format!(
                "Unknown Message Type code [{}]",
                unknown_code
            ))),
        }
    }

    /// Verifies that a set of attributes contains a valid message ID.
    ///
    /// # Errors
    ///
    /// Returns an error if [`UAttributes::id`] does not contain a [valid uProtocol UUID](`crate::UUID::is_uprotocol_uuid`).
    fn validate_id(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-id~1]
        if attributes
            .id
            .as_ref()
            .is_some_and(|id| id.is_uprotocol_uuid())
        {
            Ok(())
        } else {
            Err(UAttributesError::validation_error(
                "Attributes must contain valid uProtocol UUID in id property",
            ))
        }
    }

    /// Returns the type of message that this validator can be used with.
    fn message_type(&self) -> UMessageType;

    /// Verifies that a set of attributes contains a valid source URI.
    ///
    /// # Errors
    ///
    /// If the [`UAttributes::source`] property does not contain a valid URI as required by the type of message, an error is returned.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError>;

    /// Verifies that a set of attributes contains a valid sink URI.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError>;
}

/// Verifies that a set of attributes contains a priority that is appropriate for an RPC request message.
///
/// # Errors
///
/// If [`UAttributes::priority`] contains a value that is less [`UPriority::UPRIORITY_CS4`].
pub fn validate_rpc_priority(attributes: &UAttributes) -> Result<(), UAttributesError> {
    attributes
        .priority
        .enum_value()
        .map_err(|unknown_code| {
            UAttributesError::ValidationError(format!(
                "RPC message must have a valid priority [{}]",
                unknown_code
            ))
        })
        .and_then(|prio| {
            if prio.value() < UPriority::UPRIORITY_CS4.value() {
                Err(UAttributesError::ValidationError(
                    "RPC message must have a priority of at least CS4".to_string(),
                ))
            } else {
                Ok(())
            }
        })
}

/// Enum that hold the implementations of uattributesValidator according to type.
pub enum UAttributesValidators {
    Publish,
    Notification,
    Request,
    Response,
}

impl UAttributesValidators {
    /// Gets the validator corresponding to this enum value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UAttributesValidators, UMessageBuilder, UMessageType, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let attributes = UAttributes {
    ///    type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
    ///    id: Some(UUID::build()).into(),
    ///    source: Some(topic).into(),
    ///    ..Default::default()
    /// };
    /// let validator = UAttributesValidators::Publish.validator();
    /// assert!(validator.validate(&attributes).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn validator(&self) -> Box<dyn UAttributesValidator> {
        match self {
            UAttributesValidators::Publish => Box::new(PublishValidator),
            UAttributesValidators::Notification => Box::new(NotificationValidator),
            UAttributesValidators::Request => Box::new(RequestValidator),
            UAttributesValidators::Response => Box::new(ResponseValidator),
        }
    }

    /// Gets a validator that can be used to check a given set of attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UAttributesValidators, UMessageBuilder, UMessageType, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let attributes = UAttributes {
    ///    type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
    ///    id: Some(UUID::build()).into(),
    ///    source: Some(topic).into(),
    ///    ..Default::default()
    /// };
    /// let validator = UAttributesValidators::get_validator_for_attributes(&attributes);
    /// assert!(validator.validate(&attributes).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_validator_for_attributes(attributes: &UAttributes) -> Box<dyn UAttributesValidator> {
        Self::get_validator(attributes.type_.enum_value_or_default())
    }

    /// Gets a validator that can be used to check attributes of a given type of message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UAttributesValidators, UMessageBuilder, UMessageType, UUID, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let attributes = UAttributes {
    ///    type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
    ///    id: Some(UUID::build()).into(),
    ///    source: Some(topic).into(),
    ///    ..Default::default()
    /// };
    /// let validator = UAttributesValidators::get_validator(UMessageType::UMESSAGE_TYPE_PUBLISH);
    /// assert!(validator.validate(&attributes).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_validator(message_type: UMessageType) -> Box<dyn UAttributesValidator> {
        match message_type {
            UMessageType::UMESSAGE_TYPE_REQUEST => Box::new(RequestValidator),
            UMessageType::UMESSAGE_TYPE_RESPONSE => Box::new(ResponseValidator),
            UMessageType::UMESSAGE_TYPE_NOTIFICATION => Box::new(NotificationValidator),
            _ => Box::new(PublishValidator),
        }
    }
}

/// Validates attributes describing a Publish message.
pub struct PublishValidator;

impl UAttributesValidator for PublishValidator {
    fn message_type(&self) -> UMessageType {
        UMessageType::UMESSAGE_TYPE_PUBLISH
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// publish messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_id`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_id(attributes),
            self.validate_source(attributes),
            self.validate_sink(attributes),
        ]
        .into_iter()
        .filter_map(Result::err)
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ");

        if error_message.is_empty() {
            Ok(())
        } else {
            Err(UAttributesError::validation_error(error_message))
        }
    }

    /// Verifies that attributes for a publish message contain a valid source URI.
    ///
    /// # Errors
    ///
    /// Returns an error
    ///
    /// * if the attributes do not contain a source URI, or
    /// * if the source URI contains any wildcards, or
    /// * if the source URI has a resource ID of 0.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-publish-source~1]
        if let Some(source) = attributes.source.as_ref() {
            source.verify_event().map_err(|e| {
                UAttributesError::validation_error(format!("Invalid source URI: {}", e))
            })
        } else {
            Err(UAttributesError::validation_error(
                "Attributes for a publish message must contain a source URI",
            ))
        }
    }

    /// Verifies that attributes for a publish message do not contain a sink URI.
    ///
    /// # Errors
    ///
    /// If the [`UAttributes::sink`] property contains any URI, an error is returned.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-publish-sink~1]
        if attributes.sink.as_ref().is_some() {
            Err(UAttributesError::validation_error(
                "Attributes for a publish message must not contain a sink URI",
            ))
        } else {
            Ok(())
        }
    }
}

/// Validates attributes describing a Notification message.
pub struct NotificationValidator;

impl UAttributesValidator for NotificationValidator {
    fn message_type(&self) -> UMessageType {
        UMessageType::UMESSAGE_TYPE_NOTIFICATION
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// notification messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_id`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_id(attributes),
            self.validate_source(attributes),
            self.validate_sink(attributes),
        ]
        .into_iter()
        .filter_map(Result::err)
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ");

        if error_message.is_empty() {
            Ok(())
        } else {
            Err(UAttributesError::validation_error(error_message))
        }
    }

    /// Verifies that attributes for a notification message contain a source URI.
    ///
    /// # Errors
    ///
    /// Returns an error
    ///
    /// * if the attributes do not contain a source URI, or
    /// * if the source URI is an RPC response URI, or
    /// * if the source URI contains any wildcards.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-notification-source~1]
        if let Some(source) = attributes.source.as_ref() {
            if source.is_rpc_response() {
                Err(UAttributesError::validation_error(
                    "Origin must not be an RPC response URI",
                ))
            } else {
                source.verify_no_wildcards().map_err(|e| {
                    UAttributesError::validation_error(format!("Invalid source URI: {}", e))
                })
            }
        } else {
            Err(UAttributesError::validation_error(
                "Attributes must contain a source URI",
            ))
        }
    }

    /// Verifies that attributes for a notification message contain a sink URI.
    ///
    /// # Errors
    ///
    /// Returns an error
    ///
    /// * if the attributes do not contain a sink URI, or
    /// * if the sink URI's resource ID is != 0, or
    /// * if the sink URI contains any wildcards.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-notification-sink~1]
        if let Some(sink) = attributes.sink.as_ref() {
            if !sink.is_notification_destination() {
                Err(UAttributesError::validation_error(
                    "Destination's resource ID must be 0",
                ))
            } else {
                sink.verify_no_wildcards().map_err(|e| {
                    UAttributesError::validation_error(format!("Invalid sink URI: {}", e))
                })
            }
        } else {
            Err(UAttributesError::validation_error(
                "Attributes for a notification message must contain a sink URI",
            ))
        }
    }
}

/// Validate `UAttributes` with type `UMessageType::Request`
pub struct RequestValidator;

impl RequestValidator {
    /// Verifies that a set of attributes representing an RPC request contain a valid time-to-live.
    ///
    /// # Errors
    ///
    /// Returns an error if [`UAttributes::ttl`] (time-to-live) is empty or contains a value less than 1.
    pub fn validate_ttl(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-request-ttl~1]
        match attributes.ttl {
            Some(ttl) if ttl > 0 => Ok(()),
            Some(invalid_ttl) => Err(UAttributesError::validation_error(format!(
                "RPC request message's TTL must be a positive integer [{invalid_ttl}]"
            ))),
            None => Err(UAttributesError::validation_error(
                "RPC request message must contain a TTL",
            )),
        }
    }
}

impl UAttributesValidator for RequestValidator {
    fn message_type(&self) -> UMessageType {
        UMessageType::UMESSAGE_TYPE_REQUEST
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// RPC request messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_id`]
    /// * [`RequestValidator::validate_ttl`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    /// * `validate_rpc_priority`
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_id(attributes),
            self.validate_ttl(attributes),
            self.validate_source(attributes),
            self.validate_sink(attributes),
            validate_rpc_priority(attributes),
        ]
        .into_iter()
        .filter_map(Result::err)
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ");

        if error_message.is_empty() {
            Ok(())
        } else {
            Err(UAttributesError::validation_error(error_message))
        }
    }

    /// Verifies that attributes for a message representing an RPC request contain a reply-to-address.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`UAttributes::source`] property does not contain a valid reply-to-address according to
    /// [`UUri::verify_rpc_response`].
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-request-source~1]
        if let Some(source) = attributes.source.as_ref() {
            UUri::verify_rpc_response(source).map_err(|e| {
                UAttributesError::validation_error(format!("Invalid source URI: {}", e))
            })
        } else {
            Err(UAttributesError::validation_error("Attributes for a request message must contain a reply-to address in the source property"))
        }
    }

    /// Verifies that attributes for a message representing an RPC request indicate the method to invoke.
    ///
    /// # Errors
    ///
    /// Returns an erro if the [`UAttributes::sink`] property does not contain a URI representing a method according to
    /// [`UUri::verify_rpc_method`].
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-request-sink~1]
        if let Some(sink) = attributes.sink.as_ref() {
            UUri::verify_rpc_method(sink)
                .map_err(|e| UAttributesError::validation_error(format!("Invalid sink URI: {}", e)))
        } else {
            Err(UAttributesError::validation_error("Attributes for a request message must contain a method-to-invoke in the sink property"))
        }
    }
}

/// Validate `UAttributes` with type `UMessageType::Response`
pub struct ResponseValidator;

impl ResponseValidator {
    /// Verifies that the attributes contain a valid request ID.
    ///
    /// # Errors
    ///
    /// Returns an error if [`UAttributes::reqid`] is empty or contains a value which is not
    /// a [valid uProtocol UUID](`crate::UUID::is_uprotocol_uuid`).
    pub fn validate_reqid(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if !attributes
            .reqid
            .as_ref()
            .is_some_and(|id| id.is_uprotocol_uuid())
        {
            Err(UAttributesError::validation_error(
                "Request ID is not a valid uProtocol UUID",
            ))
        } else {
            Ok(())
        }
    }

    /// Verifies that a set of attributes contains a valid communication status.
    ///
    /// # Errors
    ///
    /// Returns an error if [`UAttributes::commstatus`] does not contain a value that is a `UCode`.
    pub fn validate_commstatus(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(status) = attributes.commstatus {
            match status.enum_value() {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(UAttributesError::validation_error(format!(
                        "Invalid Communication Status code: {e}"
                    )));
                }
            }
        }
        Ok(())
    }
}

impl UAttributesValidator for ResponseValidator {
    fn message_type(&self) -> UMessageType {
        UMessageType::UMESSAGE_TYPE_RESPONSE
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// RPC response messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_id`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    /// * [`ResponseValidator::validate_reqid`]
    /// * [`ResponseValidator::validate_commstatus`]
    /// * `validate_rpc_priority`
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_id(attributes),
            self.validate_source(attributes),
            self.validate_sink(attributes),
            self.validate_reqid(attributes),
            self.validate_commstatus(attributes),
            validate_rpc_priority(attributes),
        ]
        .into_iter()
        .filter_map(Result::err)
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ");

        if error_message.is_empty() {
            Ok(())
        } else {
            Err(UAttributesError::validation_error(error_message))
        }
    }

    /// Verifies that attributes for a message representing an RPC response indicate the method that has
    /// been invoked.
    ///  
    /// # Errors
    ///
    /// Returns an error if the [`UAttributes::source`] property does not contain a URI representing a method according to
    /// [`UUri::verify_rpc_method`].
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-response-source~1]
        if let Some(source) = attributes.source.as_ref() {
            UUri::verify_rpc_method(source).map_err(|e| {
                UAttributesError::validation_error(format!("Invalid source URI: {}", e))
            })
        } else {
            Err(UAttributesError::validation_error("Missing Source"))
        }
    }

    /// Verifies that attributes for a message representing an RPC response contain a valid
    /// reply-to-address.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`UAttributes::sink`] property does not contain a valid reply-to-address according to
    /// [`UUri::verify_rpc_response`].
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-response-sink~1]
        if let Some(sink) = &attributes.sink.as_ref() {
            UUri::verify_rpc_response(sink)
                .map_err(|e| UAttributesError::validation_error(format!("Invalid sink URI: {}", e)))
        } else {
            Err(UAttributesError::validation_error("Missing Sink"))
        }
    }
}

#[cfg(test)]
mod tests {
    use protobuf::EnumOrUnknown;
    use test_case::test_case;

    use super::*;
    use crate::{UCode, UPriority, UUri, UUID};

    #[test]
    fn test_validate_type_fails_for_unknown_type_code() {
        let attributes = UAttributes {
            type_: EnumOrUnknown::from_i32(20),
            ..Default::default()
        };
        assert!(UAttributesValidators::Publish
            .validator()
            .validate_type(&attributes)
            .is_err());
        assert!(UAttributesValidators::Notification
            .validator()
            .validate_type(&attributes)
            .is_err());
        assert!(UAttributesValidators::Request
            .validator()
            .validate_type(&attributes)
            .is_err());
        assert!(UAttributesValidators::Response
            .validator()
            .validate_type(&attributes)
            .is_err());
    }

    #[test_case(UMessageType::UMESSAGE_TYPE_UNSPECIFIED, UMessageType::UMESSAGE_TYPE_PUBLISH; "succeeds for Unspecified message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, UMessageType::UMESSAGE_TYPE_PUBLISH; "succeeds for Publish message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_NOTIFICATION, UMessageType::UMESSAGE_TYPE_NOTIFICATION; "succeeds for Notification message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, UMessageType::UMESSAGE_TYPE_REQUEST; "succeeds for Request message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, UMessageType::UMESSAGE_TYPE_RESPONSE; "succeeds for Response message")]
    fn test_get_validator_returns_matching_validator(
        message_type: UMessageType,
        expected_validator_type: UMessageType,
    ) {
        let validator: Box<dyn UAttributesValidator> =
            UAttributesValidators::get_validator(message_type);
        assert_eq!(validator.message_type(), expected_validator_type);
    }

    #[test_case(Some(UUID::build()), Some(publish_topic()), None, None, true; "succeeds for topic only")]
    // [utest->dsn~up-attributes-publish-sink~1]
    #[test_case(Some(UUID::build()), Some(publish_topic()), Some(destination()), None, false; "fails for message containing destination")]
    #[test_case(Some(UUID::build()), Some(publish_topic()), None, Some(100), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-publish-source~1]
    #[test_case(Some(UUID::build()), None, None, None, false; "fails for missing topic")]
    // [utest->dsn~up-attributes-publish-source~1]
    #[test_case(Some(UUID::build()), Some(UUri { resource_id: 0x54, ..Default::default()}), None, None, false; "fails for invalid topic")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(None, Some(publish_topic()), None, None, false; "fails for missing message ID")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        Some(publish_topic()),
        None,
        None,
        false;
        "fails for invalid message id")]
    fn test_validate_attributes_for_publish_message(
        id: Option<UUID>,
        source: Option<UUri>,
        sink: Option<UUri>,
        ttl: Option<u32>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
            id: id.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            source: source.into(),
            sink: sink.into(),
            ttl,
            ..Default::default()
        };
        let validator = UAttributesValidators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok() == expected_result);
        if status.is_ok() {
            assert!(UAttributesValidators::Notification
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Request
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Response
                .validator()
                .validate(&attributes)
                .is_err());
        }
    }

    // [utest->dsn~up-attributes-notification-sink~1]
    #[test_case(Some(UUID::build()), Some(origin()), None, None, false; "fails for missing destination")]
    #[test_case(Some(UUID::build()), Some(origin()), Some(destination()), None, true; "succeeds for both origin and destination")]
    #[test_case(Some(UUID::build()), Some(origin()), Some(destination()), Some(100), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-notification-source~1]
    #[test_case(Some(UUID::build()), None, Some(destination()), None, false; "fails for missing origin")]
    // [utest->dsn~up-attributes-notification-source~1]
    #[test_case(Some(UUID::build()), Some(UUri::default()), Some(destination()), None, false; "fails for invalid origin")]
    // [utest->dsn~up-attributes-notification-sink~1]
    #[test_case(Some(UUID::build()), Some(origin()), Some(UUri { ue_id: 0xabcd, ue_version_major: 0x01, resource_id: 0x0011, ..Default::default() }), None, false; "fails for invalid destination")]
    #[test_case(Some(UUID::build()), None, None, None, false; "fails for neither origin nor destination")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(None, Some(origin()), Some(destination()), None, false; "fails for missing message ID")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        Some(origin()),
        Some(destination()),
        None,
        false;
        "fails for invalid message id")]
    fn test_validate_attributes_for_notification_message(
        id: Option<UUID>,
        source: Option<UUri>,
        sink: Option<UUri>,
        ttl: Option<u32>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            id: id.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            source: source.into(),
            sink: sink.into(),
            ttl,
            ..Default::default()
        };
        let validator = UAttributesValidators::Notification.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok() == expected_result);
        if status.is_ok() {
            assert!(UAttributesValidators::Publish
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Request
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Response
                .validator()
                .validate(&attributes)
                .is_err());
        }
    }

    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), None, Some(2000), Some(UPriority::UPRIORITY_CS4), None, true; "succeeds for mandatory attributes")]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), Some(1), Some(2000), Some(UPriority::UPRIORITY_CS4), Some(String::from("token")), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(None, Some(method_to_invoke()), Some(reply_to_address()), Some(1), Some(2000), Some(UPriority::UPRIORITY_CS4), Some(String::from("token")), false; "fails for missing message ID")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        Some(method_to_invoke()),
        Some(reply_to_address()),
        None,
        Some(2000),
        Some(UPriority::UPRIORITY_CS4),
        None,
        false;
        "fails for invalid message id")]
    // [utest->dsn~up-attributes-request-source~1]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), None, None, Some(2000), Some(UPriority::UPRIORITY_CS4), None, false; "fails for missing reply-to-address")]
    // [utest->dsn~up-attributes-request-source~1]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(UUri { resource_id: 0x0001, ..Default::default()}), None, Some(2000), Some(UPriority::UPRIORITY_CS4), None, false; "fails for invalid reply-to-address")]
    // [utest->dsn~up-attributes-request-sink~1]
    #[test_case(Some(UUID::build()), None, Some(reply_to_address()), None, Some(2000), Some(UPriority::UPRIORITY_CS4), None, false; "fails for missing method-to-invoke")]
    // [utest->dsn~up-attributes-request-sink~1]
    #[test_case(Some(UUID::build()), Some(UUri::default()), Some(reply_to_address()), None, Some(2000), Some(UPriority::UPRIORITY_CS4), None, false; "fails for invalid method-to-invoke")]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), Some(1), Some(2000), None, None, false; "fails for missing priority")]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), Some(1), Some(2000), Some(UPriority::UPRIORITY_CS3), None, false; "fails for invalid priority")]
    // [utest->dsn~up-attributes-request-ttl~1]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), None, None, Some(UPriority::UPRIORITY_CS4), None, false; "fails for missing ttl")]
    // [utest->dsn~up-attributes-request-ttl~1]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), None, Some(0), Some(UPriority::UPRIORITY_CS4), None, false; "fails for ttl = 0")]
    #[test_case(Some(UUID::build()), Some(method_to_invoke()), Some(reply_to_address()), Some(1), Some(2000), Some(UPriority::UPRIORITY_CS4), None, true; "succeeds for valid permission level")]
    #[allow(clippy::too_many_arguments)]
    fn test_validate_attributes_for_rpc_request_message(
        id: Option<UUID>,
        method_to_invoke: Option<UUri>,
        reply_to_address: Option<UUri>,
        perm_level: Option<u32>,
        ttl: Option<u32>,
        priority: Option<UPriority>,
        token: Option<String>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
            id: id.into(),
            priority: priority.unwrap_or(UPriority::UPRIORITY_UNSPECIFIED).into(),
            source: reply_to_address.into(),
            sink: method_to_invoke.into(),
            permission_level: perm_level,
            ttl,
            token,
            ..Default::default()
        };
        let status = UAttributesValidators::Request
            .validator()
            .validate(&attributes);
        assert!(status.is_ok() == expected_result);
        if status.is_ok() {
            assert!(UAttributesValidators::Publish
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Notification
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Response
                .validator()
                .validate(&attributes)
                .is_err());
        }
    }

    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), None, None, Some(UPriority::UPRIORITY_CS4), true; "succeeds for mandatory attributes")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), Some(EnumOrUnknown::from(UCode::CANCELLED)), Some(100), Some(UPriority::UPRIORITY_CS4), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(None, Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), Some(EnumOrUnknown::from(UCode::CANCELLED)), Some(100), Some(UPriority::UPRIORITY_CS4), false; "fails for missing message ID")]
    // [utest->dsn~up-attributes-id~1]
    #[test_case(
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        Some(reply_to_address()),
        Some(method_to_invoke()),
        Some(UUID::build()),
        None,
        None,
        Some(UPriority::UPRIORITY_CS4),
        false;
        "fails for invalid message id")]
    // [utest->dsn~up-attributes-response-sink~1]
    #[test_case(Some(UUID::build()), None, Some(method_to_invoke()), Some(UUID::build()), None, None, Some(UPriority::UPRIORITY_CS4), false; "fails for missing reply-to-address")]
    // [utest->dsn~up-attributes-response-sink~1]
    #[test_case(Some(UUID::build()), Some(UUri { resource_id: 0x0001, ..Default::default()}), Some(method_to_invoke()), Some(UUID::build()), None, None, Some(UPriority::UPRIORITY_CS4), false; "fails for invalid reply-to-address")]
    // [utest->dsn~up-attributes-response-source~1]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), None, Some(UUID::build()), None, None, Some(UPriority::UPRIORITY_CS4), false; "fails for missing invoked-method")]
    // [utest->dsn~up-attributes-response-source~1]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(UUri::default()), Some(UUID::build()), None, None, Some(UPriority::UPRIORITY_CS4), false; "fails for invalid invoked-method")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), Some(EnumOrUnknown::from(UCode::CANCELLED)), None, Some(UPriority::UPRIORITY_CS4), true; "succeeds for valid commstatus")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), Some(EnumOrUnknown::from_i32(-42)), None, Some(UPriority::UPRIORITY_CS4), false; "fails for invalid commstatus")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), None, Some(100), Some(UPriority::UPRIORITY_CS4), true; "succeeds for ttl > 0)")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), None, Some(0), Some(UPriority::UPRIORITY_CS4), true; "succeeds for ttl = 0")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), Some(EnumOrUnknown::from(UCode::CANCELLED)), Some(100), None, false; "fails for missing priority")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), Some(UUID::build()), Some(EnumOrUnknown::from(UCode::CANCELLED)), Some(100), Some(UPriority::UPRIORITY_CS3), false; "fails for invalid priority")]
    #[test_case(Some(UUID::build()), Some(reply_to_address()), Some(method_to_invoke()), None, None, None, Some(UPriority::UPRIORITY_CS4), false; "fails for missing request id")]
    #[test_case(
        Some(UUID::build()),
        Some(reply_to_address()),
        Some(method_to_invoke()),
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        None,
        None,
        Some(UPriority::UPRIORITY_CS4),
        false;
        "fails for invalid request id")]
    #[allow(clippy::too_many_arguments)]
    fn test_validate_attributes_for_rpc_response_message(
        id: Option<UUID>,
        reply_to_address: Option<UUri>,
        invoked_method: Option<UUri>,
        reqid: Option<UUID>,
        commstatus: Option<EnumOrUnknown<UCode>>,
        ttl: Option<u32>,
        priority: Option<UPriority>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_RESPONSE.into(),
            id: id.into(),
            priority: priority.unwrap_or(UPriority::UPRIORITY_UNSPECIFIED).into(),
            reqid: reqid.into(),
            source: invoked_method.into(),
            sink: reply_to_address.into(),
            commstatus,
            ttl,
            ..Default::default()
        };
        let status = UAttributesValidators::Response
            .validator()
            .validate(&attributes);
        assert!(status.is_ok() == expected_result);
        if status.is_ok() {
            assert!(UAttributesValidators::Publish
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Notification
                .validator()
                .validate(&attributes)
                .is_err());
            assert!(UAttributesValidators::Request
                .validator()
                .validate(&attributes)
                .is_err());
        }
    }

    fn publish_topic() -> UUri {
        UUri {
            authority_name: String::from("vcu.someVin"),
            ue_id: 0x0000_5410,
            ue_version_major: 0x01,
            resource_id: 0xa010,
            ..Default::default()
        }
    }

    fn origin() -> UUri {
        UUri {
            authority_name: String::from("vcu.someVin"),
            ue_id: 0x0000_3c00,
            ue_version_major: 0x02,
            resource_id: 0x9a00,
            ..Default::default()
        }
    }

    fn destination() -> UUri {
        UUri {
            authority_name: String::from("vcu.someVin"),
            ue_id: 0x0000_3d07,
            ue_version_major: 0x01,
            resource_id: 0x0000,
            ..Default::default()
        }
    }

    fn reply_to_address() -> UUri {
        UUri {
            authority_name: String::from("vcu.someVin"),
            ue_id: 0x0000_010b,
            ue_version_major: 0x01,
            resource_id: 0x0000,
            ..Default::default()
        }
    }

    fn method_to_invoke() -> UUri {
        UUri {
            authority_name: String::from("vcu.someVin"),
            ue_id: 0x0000_03ae,
            ue_version_major: 0x01,
            resource_id: 0x00e2,
            ..Default::default()
        }
    }
}
