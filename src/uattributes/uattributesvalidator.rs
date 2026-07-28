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
        if expected_type == attributes.type_() {
            Ok(())
        } else {
            Err(UAttributesError::validation_error(format!(
                "Wrong Message Type [expected {}, got {}]",
                expected_type.to_cloudevent_type(),
                attributes.type_().to_cloudevent_type()
            )))
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
        .priority()
        .ok_or_else(|| {
            UAttributesError::ValidationError("RPC message must have a priority".to_string())
        })
        .and_then(|prio| {
            if prio < UPriority::CS4 {
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
    /// use up_rust::{UAttributesValidators, UMessageBuilder, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).build()?;
    /// let validator = UAttributesValidators::Publish.validator();
    /// assert!(validator.validate(msg.attributes()).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
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
    /// use up_rust::{UAttributesValidators, UMessageBuilder, UMessageType, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).build()?;
    /// let validator = UAttributesValidators::get_validator_for_attributes(msg.attributes());
    /// assert!(validator.validate(msg.attributes()).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_validator_for_attributes(attributes: &UAttributes) -> Box<dyn UAttributesValidator> {
        Self::get_validator(attributes.type_())
    }

    /// Gets a validator that can be used to check attributes of a given type of message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributesValidators, UMessageBuilder, UMessageType, UUri};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let topic = UUri::try_from("//my-vehicle/D45/23/A001")?;
    /// let msg = UMessageBuilder::publish(topic).build()?;
    /// let validator = UAttributesValidators::get_validator(UMessageType::Publish);
    /// assert!(validator.validate(msg.attributes()).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_validator(message_type: UMessageType) -> Box<dyn UAttributesValidator> {
        match message_type {
            UMessageType::Publish => Box::new(PublishValidator),
            UMessageType::Notification => Box::new(NotificationValidator),
            UMessageType::Request => Box::new(RequestValidator),
            UMessageType::Response => Box::new(ResponseValidator),
        }
    }
}

/// Validates attributes describing a Publish message.
pub struct PublishValidator;

impl UAttributesValidator for PublishValidator {
    fn message_type(&self) -> UMessageType {
        UMessageType::Publish
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// publish messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
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
    /// * if the source URI contains any wildcards, or
    /// * if the source URI has a resource ID of 0.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        // [impl->dsn~up-attributes-publish-source~1]
        attributes
            .source()
            .verify_event()
            .map_err(|e| UAttributesError::validation_error(format!("Invalid source URI: {e}")))
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
        UMessageType::Notification
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// notification messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
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
        let source = attributes.source();
        if source.is_rpc_response() {
            Err(UAttributesError::validation_error(
                "Origin must not be an RPC response URI",
            ))
        } else {
            source
                .verify_no_wildcards()
                .map_err(|e| UAttributesError::validation_error(format!("Invalid source URI: {e}")))
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
                    UAttributesError::validation_error(format!("Invalid sink URI: {e}"))
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
        UMessageType::Request
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// RPC request messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`RequestValidator::validate_ttl`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    /// * `validate_rpc_priority`
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_ttl(attributes),
            self.validate_source(attributes),
            self.validate_sink(attributes),
            // [impl->dsn~up-attributes-request-priority~1]
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
        attributes
            .source()
            .verify_rpc_response()
            .map_err(|e| UAttributesError::validation_error(format!("Invalid source URI: {e}")))
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
                .map_err(|e| UAttributesError::validation_error(format!("Invalid sink URI: {e}")))
        } else {
            Err(UAttributesError::validation_error("Attributes for a request message must contain a method-to-invoke in the sink property"))
        }
    }
}

/// Validate `UAttributes` with type `UMessageType::Response`
pub struct ResponseValidator;

impl ResponseValidator {
    /// Verifies that the attributes contain a request ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the attributes do not contain a request ID.
    pub fn validate_reqid(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if attributes.reqid.as_ref().is_none() {
            Err(UAttributesError::validation_error("Request ID is missing"))
        } else {
            Ok(())
        }
    }
}

impl UAttributesValidator for ResponseValidator {
    fn message_type(&self) -> UMessageType {
        UMessageType::Response
    }

    /// Checks if a given set of attributes complies with the rules specified for
    /// RPC response messages.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following checks fail for the given attributes:
    ///
    /// * [`UAttributesValidator::validate_type`]
    /// * [`UAttributesValidator::validate_source`]
    /// * [`UAttributesValidator::validate_sink`]
    /// * [`ResponseValidator::validate_reqid`]
    /// * `validate_rpc_priority`
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_source(attributes),
            self.validate_sink(attributes),
            self.validate_reqid(attributes),
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
        attributes
            .source()
            .verify_rpc_method()
            .map_err(|e| UAttributesError::validation_error(format!("Invalid source URI: {e}")))
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
                .map_err(|e| UAttributesError::validation_error(format!("Invalid sink URI: {e}")))
        } else {
            Err(UAttributesError::validation_error("Missing Sink"))
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{uattributes::TokenString, UCode, UPriority, UUri, UUID};

    #[test_case(UMessageType::Publish, UMessageType::Publish; "succeeds for Publish message")]
    #[test_case(UMessageType::Notification, UMessageType::Notification; "succeeds for Notification message")]
    #[test_case(UMessageType::Request, UMessageType::Request; "succeeds for Request message")]
    #[test_case(UMessageType::Response, UMessageType::Response; "succeeds for Response message")]
    fn test_get_validator_returns_matching_validator(
        message_type: UMessageType,
        expected_validator_type: UMessageType,
    ) {
        let validator: Box<dyn UAttributesValidator> =
            UAttributesValidators::get_validator(message_type);
        assert_eq!(validator.message_type(), expected_validator_type);
    }

    #[test_case(publish_topic(), None, None, true; "succeeds for topic only")]
    // [utest->dsn~up-attributes-publish-sink~1]
    #[test_case(publish_topic(), Some(destination()), None, false; "fails for message containing destination")]
    #[test_case(publish_topic(), None, Some(100), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-publish-source~1]
    #[test_case(method_to_invoke(), None, None, false; "fails for invalid topic")]
    fn test_validate_attributes_for_publish_message(
        source: UUri,
        sink: Option<UUri>,
        ttl: Option<u32>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            commstatus: None,
            id: UUID::build(),
            payload_format: None,
            permission_level: None,
            priority: UPriority::CS1.into(),
            reqid: None,
            sink,
            source,
            token: None,
            traceparent: None,
            ttl,
            type_: UMessageType::Publish,
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
    #[test_case(origin(), None, None, false; "fails for missing destination")]
    #[test_case(origin(), Some(destination()), None, true; "succeeds for both origin and destination")]
    #[test_case(origin(), Some(destination()), Some(100), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-notification-source~1]
    #[test_case(reply_to_address(), Some(destination()), None, false; "fails for invalid origin")]
    // [utest->dsn~up-attributes-notification-sink~1]
    #[test_case(origin(), Some(method_to_invoke()), None, false; "fails for invalid destination")]
    fn test_validate_attributes_for_notification_message(
        source: UUri,
        sink: Option<UUri>,
        ttl: Option<u32>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            commstatus: None,
            id: UUID::build(),
            payload_format: None,
            permission_level: None,
            priority: UPriority::CS1.into(),
            reqid: None,
            sink,
            source,
            token: None,
            traceparent: None,
            ttl,
            type_: UMessageType::Notification,
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

    #[test_case(Some(method_to_invoke()), reply_to_address(), None, Some(2000), Some(UPriority::CS4), None, true; "succeeds for mandatory attributes")]
    #[test_case(Some(method_to_invoke()), reply_to_address(), Some(1), Some(2000), Some(UPriority::CS4), Some("token"), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-request-source~1]
    #[test_case(Some(method_to_invoke()), origin(), None, Some(2000), Some(UPriority::CS4), None, false; "fails for invalid reply-to-address")]
    // [utest->dsn~up-attributes-request-sink~1]
    #[test_case(None, reply_to_address(), None, Some(2000), Some(UPriority::CS4), None, false; "fails for missing method-to-invoke")]
    // [utest->dsn~up-attributes-request-sink~1]
    #[test_case(Some(destination()), reply_to_address(), None, Some(2000), Some(UPriority::CS4), None, false; "fails for invalid method-to-invoke")]
    #[test_case(Some(method_to_invoke()), reply_to_address(), Some(1), Some(2000), None, None, false; "fails for missing priority")]
    #[test_case(Some(method_to_invoke()), reply_to_address(), Some(1), Some(2000), Some(UPriority::CS3), None, false; "fails for invalid priority")]
    // [utest->dsn~up-attributes-request-ttl~1]
    #[test_case(Some(method_to_invoke()), reply_to_address(), Some(1), None, Some(UPriority::CS4), None, false; "fails for missing ttl")]
    // [utest->dsn~up-attributes-request-ttl~1]
    #[test_case(Some(method_to_invoke()), reply_to_address(), Some(1), Some(0), Some(UPriority::CS4), None, false; "fails for ttl = 0")]
    #[test_case(Some(method_to_invoke()), reply_to_address(), Some(1), Some(2000), Some(UPriority::CS4), None, true; "succeeds for valid permission level")]
    #[allow(clippy::too_many_arguments)]
    fn test_validate_attributes_for_rpc_request_message(
        method_to_invoke: Option<UUri>,
        reply_to_address: UUri,
        perm_level: Option<u32>,
        ttl: Option<u32>,
        priority: Option<UPriority>,
        token: Option<&str>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            commstatus: None,
            id: UUID::build(),
            payload_format: None,
            permission_level: perm_level,
            priority,
            reqid: None,
            sink: method_to_invoke,
            source: reply_to_address,
            token: token.map(TokenString::from),
            traceparent: None,
            ttl,
            type_: UMessageType::Request,
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

    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), None, None, Some(UPriority::CS4), true; "succeeds for mandatory attributes")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), Some(UCode::Cancelled), Some(100), Some(UPriority::CS4), true; "succeeds for valid attributes")]
    // [utest->dsn~up-attributes-response-sink~1]
    #[test_case(None, method_to_invoke(), Some(UUID::build()), None, None, Some(UPriority::CS4), false; "fails for missing reply-to-address")]
    // [utest->dsn~up-attributes-response-sink~1]
    #[test_case(Some(origin()), method_to_invoke(), Some(UUID::build()), None, None, Some(UPriority::CS4), false; "fails for invalid reply-to-address")]
    // [utest->dsn~up-attributes-response-source~1]
    #[test_case(Some(reply_to_address()), origin(), Some(UUID::build()), None, None, Some(UPriority::CS4), false; "fails for invalid invoked-method")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), Some(UCode::Cancelled), None, Some(UPriority::CS4), true; "succeeds for valid commstatus")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), None, Some(100), Some(UPriority::CS4), true; "succeeds for ttl > 0)")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), None, Some(0), Some(UPriority::CS4), true; "succeeds for ttl = 0")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), Some(UCode::Cancelled), Some(100), None, false; "fails for missing priority")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), Some(UUID::build()), Some(UCode::Cancelled), Some(100), Some(UPriority::CS3), false; "fails for invalid priority")]
    #[test_case(Some(reply_to_address()), method_to_invoke(), None, None, None, Some(UPriority::CS4), false; "fails for missing request id")]
    #[allow(clippy::too_many_arguments)]
    fn test_validate_attributes_for_rpc_response_message(
        reply_to_address: Option<UUri>,
        invoked_method: UUri,
        reqid: Option<UUID>,
        commstatus: Option<UCode>,
        ttl: Option<u32>,
        priority: Option<UPriority>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            commstatus,
            id: UUID::build(),
            payload_format: None,
            permission_level: None,
            priority,
            reqid,
            sink: reply_to_address,
            source: invoked_method,
            token: None,
            traceparent: None,
            ttl,
            type_: UMessageType::Response,
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
        UUri::try_from_parts("vcu.somevin", 0x0000_5410, 0x01, 0xa010)
            .expect("failed to create publish topic URI")
    }

    fn origin() -> UUri {
        UUri::try_from_parts("vcu.somevin", 0x0000_3c00, 0x02, 0x9a00)
            .expect("failed to create origin URI")
    }

    fn destination() -> UUri {
        UUri::try_from_parts("vcu.somevin", 0x0000_3d07, 0x01, 0x0000)
            .expect("failed to create destination URI")
    }

    fn reply_to_address() -> UUri {
        UUri::try_from_parts("vcu.somevin", 0x0000_010b, 0x01, 0x0000)
            .expect("failed to create reply-to-address URI")
    }

    fn method_to_invoke() -> UUri {
        UUri::try_from_parts("vcu.somevin", 0x0000_03ae, 0x01, 0x00e2)
            .expect("failed to create method-to-invoke URI")
    }
}
