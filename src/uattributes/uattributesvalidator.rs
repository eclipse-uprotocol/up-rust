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

use std::time::SystemTime;

use protobuf::Enum;

use crate::{UAttributes, UCode, UMessageType, UriValidator, UUID};

use crate::UAttributesError;

/// `UAttributes` is the struct that defines the Payload. It serves as the configuration for various aspects
/// like time to live, priority, security tokens, and more. Each variant of `UAttributes` defines a different
/// type of message payload. The payload could represent a simple published payload with some state change,
/// an RPC request payload, or an RPC response payload.
///
/// `UAttributesValidator` is a trait implemented by all validators for `UAttributes`. It provides functionality
/// to help validate that a given `UAttributes` instance is correctly configured to define the Payload.
pub trait UAttributesValidator {
    /// Takes a `UAttributes` object and runs validations.
    ///
    /// # Arguments
    /// * `attributes` - The `UAttributes` to validate.
    ///
    /// # Returns
    /// Returns a `UStatus` that indicates success or failure. If failed, it includes a message containing
    /// all validation errors for invalid configurations.
    /// # Errors
    ///
    /// Returns a `UAttributesError` when one or more validations fail. The error will contain a concatenated message of all the validation errors separated by a semicolon (`;`). Each part of the message corresponds to a failure from one of the specific validation functions called within `validate`. These may include errors from:
    ///
    /// - `validate_type` if the message type in `UAttributes` fails validation.
    /// - `validate_ttl` if the time-to-live value is invalid.
    /// - `validate_source_and_sink` if any of source or sink URI does not pass validation.
    /// - `validate_commstatus` if the communication status is invalid.
    /// - `validate_permission_level` if the permission level is below the required threshold.
    /// - `validate_reqid` if the request ID is not a valid UUID.
    ///
    /// If all validations pass, the function returns `Ok(())`, indicating no errors were found.
    fn validate(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_ttl(attributes),
            self.validate_source_and_sink(attributes),
            self.validate_commstatus(attributes),
            self.validate_permission_level(attributes),
            self.validate_reqid(attributes),
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

    /// Return the name of the specific Validator implementation.
    fn type_name(&self) -> &'static str;

    /// Indicates whether the payload with these [`UAttributes`] has expired.
    ///
    /// # Parameters
    ///
    /// * `attributes`: Reference to a [`UAttributes`] struct containing the time-to-live value.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    ///
    /// # Errors
    ///
    /// Returns a `UAttributesError` in the following cases:
    ///
    /// - "Payload is expired": If the `ttl` (time-to-live) is present, valid, and greater than 0, but the payload has expired. This is determined by comparing the current time duration since the UNIX epoch against the timestamp extracted from the UUID and the `ttl` value.
    ///
    /// - "Invalid duration": If there is an error in converting the current time duration to a valid `u64` format, indicating an issue with the duration calculation.
    ///
    /// - "Invalid TTL": If the `ttl` value is present but cannot be converted to a valid `u64`, suggesting an invalid `ttl` value.
    ///
    /// - System error message: If there is an error in calculating the current time duration since the UNIX epoch, possibly due to a system time error.
    ///
    /// The function returns `Ok(())` (indicating no error) in cases where `ttl` is not present, is less than or equal to 0, or if no UUID is present, or if the UUID does not contain a valid time component.
    fn is_expired(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        let ttl = match attributes.ttl {
            Some(t) if t > 0 => t,
            Some(_) => return Ok(()),
            None => 0,
        };

        if let Some(time) = attributes.id.as_ref().and_then(UUID::get_time) {
            let delta = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(duration) => {
                    if let Ok(duration) = u64::try_from(duration.as_millis()) {
                        duration - time
                    } else {
                        return Err(UAttributesError::validation_error("Invalid duration"));
                    }
                }
                Err(e) => return Err(UAttributesError::validation_error(e.to_string())),
            };

            if ttl <= 0 {
                return Ok(());
            }

            if let Ok(ttl) = u64::try_from(ttl) {
                if delta >= ttl {
                    return Err(UAttributesError::validation_error("Payload is expired"));
                }
            } else {
                return Err(UAttributesError::validation_error("Invalid TTL"));
            }
        }
        Ok(())
    }

    /// Validate the time to live configuration. If the `UAttributes` does not contain a time to live
    /// then the `UStatus` is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message priority to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    /// # Errors
    ///
    /// Returns a `UAttributesError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `ttl` (time to live) and it is less than 1.
    ///   The error message will specify the invalid TTL value, indicating that it does not meet the minimum threshold considered valid.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `ttl`, considering it a valid case.
    fn validate_ttl(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(ttl) = attributes.ttl {
            if ttl < 0 {
                return Err(UAttributesError::validation_error(format!(
                    "Invalid TTL [{ttl}]"
                )));
            }
        }
        Ok(())
    }

    /// Validates the source URI for the default case. If the `UAttributes` does not contain a source
    /// then the `ValidationResult` is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.    
    /// # Errors
    ///
    /// Returns a `UAttributesError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `source` and the source URI does not pass the validation criteria set by `UriValidator`.
    ///   The error will provide details about why the source URI is considered invalid, such as format issues or other validation failures.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `source`, considering it a valid case.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(source) = attributes.source.as_ref() {
            return UriValidator::validate(source)
                .map_err(|e| UAttributesError::validation_error(e.to_string()));
        }
        Ok(())
    }

    /// Validates the sink URI for the default case. If the `UAttributes` does not contain a sink
    /// then the `ValidationResult` is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.    
    /// # Errors
    ///
    /// Returns a `UAttributesError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `sink` and the sink URI does not pass the validation criteria set by `UriValidator`.
    ///   The error will provide details about why the sink URI is considered invalid, such as format issues or other validation failures.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `sink`, considering it a valid case.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(sink) = attributes.sink.as_ref() {
            return UriValidator::validate(sink)
                .map_err(|e| UAttributesError::validation_error(e.to_string()));
        }
        Ok(())
    }

    fn validate_source_and_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        self.validate_source(attributes)
            .and_then(|_| self.validate_sink(attributes))
    }

    /// Validates the permission level for the default case. If the `UAttributes` does not contain
    /// a permission level then the `ValidationResult` is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the permission level to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    /// # Errors
    ///
    /// Returns a `UAttributesError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `permission_level` and it is less than 1.
    ///   The error will indicate that the permission level is invalid, suggesting that it does not meet the minimum required level.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `permission_level`, considering it a valid case.
    fn validate_permission_level(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(plevel) = attributes.permission_level {
            if plevel < 1 {
                return Err(UAttributesError::validation_error(
                    "Invalid Permission Level",
                ));
            }
        }
        Ok(())
    }

    /// Validates the communication status (`commStatus`) for the default case. If the `UAttributes`
    /// does not contain a request id then the `UStatus` is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the communication status to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    ///
    /// # Errors
    ///
    /// Returns a `UAttributesError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `reqid` (request ID) and it is not a valid UUID.
    ///   The error will indicate that the request ID format is invalid, helping to identify issues with the input `UAttributes`.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `reqid`, considering it a valid case.
    fn validate_commstatus(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(cs) = attributes.commstatus {
            if UCode::from_i32(cs).is_none() {
                return Err(UAttributesError::validation_error(format!(
                    "Invalid Communication Status Code [{cs}]"
                )));
            }
        }
        Ok(())
    }

    /// Validates the `correlationId` for the default case. If the `UAttributes` does not contain
    /// a request id then the `UStatus` is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `Attributes` object containing the request id to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    /// # Errors
    ///
    /// This function returns a `UAttributesError` in the following scenario:
    ///
    /// - If `UAttributes` contains a request ID (`reqid`), but it is not a valid UUID.
    ///   The `UAttributesError` will contain a message such as "Invalid UUID" to indicate the nature of the validation failure.
    /// - If the attributes do not contain a request ID but the message type requires one as indicated by [`UAttributesValidator::requires_request_id`].
    /// The function considers the absence of a request ID in `UAttributes` as a valid case and does not return an error in such a scenario.
    fn validate_reqid(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(reqid) = attributes.reqid.as_ref() {
            if !reqid.is_uprotocol_uuid() {
                return Err(UAttributesError::validation_error("Invalid UUID"));
            }
        } else if self.requires_request_id() {
            return Err(UAttributesError::validation_error("Missing Request ID"));
        }
        Ok(())
    }

    /// Checks if the type of message covered by this validator requires a *request ID* to be set.
    ///
    /// # Returns
    ///
    /// `true` if the message requires a request ID. This default implementation returns `false`.
    fn requires_request_id(&self) -> bool {
        false
    }

    /// Validates the `MessageType` of `UAttributes`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    ///
    /// # Errors
    ///
    /// This function returns a `UAttributesError` in cases where:
    ///
    /// - The `MessageType` in `UAttributes` does not conform to the expected format or value.
    /// - The `UAttributes` object contains inconsistent or invalid data that fails the validation criteria.
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), UAttributesError>;
}

/// Enum that hold the implementations of uattributesValidator according to type.
pub enum UAttributesValidators {
    Publish,
    Request,
    Response,
}

impl UAttributesValidators {
    pub fn validator(&self) -> Box<dyn UAttributesValidator> {
        match self {
            UAttributesValidators::Publish => Box::new(PublishValidator),
            UAttributesValidators::Request => Box::new(RequestValidator),
            UAttributesValidators::Response => Box::new(ResponseValidator),
        }
    }

    pub fn get_validator_for_attributes(attributes: &UAttributes) -> Box<dyn UAttributesValidator> {
        Self::get_validator(attributes.type_.enum_value_or_default())
    }

    pub fn get_validator(message_type: UMessageType) -> Box<dyn UAttributesValidator> {
        match message_type {
            UMessageType::UMESSAGE_TYPE_REQUEST => Box::new(RequestValidator),
            UMessageType::UMESSAGE_TYPE_RESPONSE => Box::new(ResponseValidator),
            _ => Box::new(PublishValidator),
        }
    }
}

/// Validate `UAttributes` with type `UMessageType::Publish`
pub struct PublishValidator;

impl UAttributesValidator for PublishValidator {
    fn type_name(&self) -> &'static str {
        "UAttributesValidator.Publish"
    }

    /// Validates that attributes for a message meant to publish state changes has the correct type.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        match attributes.type_.enum_value() {
            Err(unknown_code) => Err(UAttributesError::validation_error(format!(
                "Unknown Message Type [{}]",
                unknown_code
            ))),
            Ok(mt) => match mt {
                UMessageType::UMESSAGE_TYPE_PUBLISH => Ok(()),
                _ => Err(UAttributesError::validation_error(format!(
                    "Wrong Message Type [{}]",
                    mt.to_type_string()
                ))),
            },
        }
    }

    fn validate_source_and_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if (attributes.source.is_none() && attributes.sink.is_none())
            || (attributes.source.is_some() && attributes.sink.is_some())
        {
            return Err(UAttributesError::validation_error(
                "Message of type PUBLISH must contain either a source or sink",
            ));
        }
        self.validate_source(attributes)
            .and_then(|_| self.validate_sink(attributes))
    }
}

/// Validate `UAttributes` with type `UMessageType::Request`
pub struct RequestValidator;

impl UAttributesValidator for RequestValidator {
    fn type_name(&self) -> &'static str {
        "UAttributesValidator.Request"
    }

    fn requires_request_id(&self) -> bool {
        true
    }

    /// Validates that attributes for a message meant for an RPC request has the correct type.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        match attributes.type_.enum_value() {
            Err(unknown_code) => Err(UAttributesError::validation_error(format!(
                "Unknown Attribute Type [{}]",
                unknown_code
            ))),
            Ok(mt) => match mt {
                UMessageType::UMESSAGE_TYPE_REQUEST => Ok(()),
                _ => Err(UAttributesError::validation_error(format!(
                    "Wrong Attribute Type [{}]",
                    mt.to_type_string()
                ))),
            },
        }
    }

    /// Validates that attributes for a message meant for an RPC request contain a reply-to-address.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(source) = attributes.source.as_ref() {
            UriValidator::validate_rpc_response(source)
                .map_err(|e| UAttributesError::validation_error(e.to_string()))
        } else {
            Err(UAttributesError::validation_error("Missing Source"))
        }
    }

    /// Validates that attributes for a message meant for an RPC request has a destination sink.
    /// In the case of an RPC request, the sink is required.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(sink) = attributes.sink.as_ref() {
            UriValidator::validate_rpc_method(sink)
                .map_err(|e| UAttributesError::validation_error(e.to_string()))
        } else {
            Err(UAttributesError::validation_error("Missing Sink"))
        }
    }

    /// Validate the time to live configuration. In the case of an RPC request,
    /// the time to live is required.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the ttl to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_ttl(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(ttl) = attributes.ttl {
            if ttl > 0 {
                Ok(())
            } else {
                Err(UAttributesError::validation_error(format!(
                    "Invalid TTL [{ttl}]"
                )))
            }
        } else {
            Err(UAttributesError::validation_error("Missing TTL"))
        }
    }
}

/// Validate `UAttributes` with type `UMessageType::Response`
pub struct ResponseValidator;

impl UAttributesValidator for ResponseValidator {
    fn type_name(&self) -> &'static str {
        "UAttributesValidator.Response"
    }

    /// Validates that attributes for a message meant for an RPC response has the correct type.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        match attributes.type_.enum_value() {
            Err(unknown_code) => Err(UAttributesError::validation_error(format!(
                "Unknown Attribute Type [{}]",
                unknown_code
            ))),
            Ok(mt) => match mt {
                UMessageType::UMESSAGE_TYPE_RESPONSE => Ok(()),
                _ => Err(UAttributesError::validation_error(format!(
                    "Wrong Attribute Type [{}]",
                    mt.to_type_string()
                ))),
            },
        }
    }

    /// Validates that attributes for a message meant for an RPC response contain the invoked method.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_source(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(source) = attributes.source.as_ref() {
            UriValidator::validate_rpc_method(source)
                .map_err(|e| UAttributesError::validation_error(e.to_string()))
        } else {
            Err(UAttributesError::validation_error("Missing Source"))
        }
    }

    /// Validates that attributes for a message meant for an RPC response contain a reply-to-address.
    /// In the case of an RPC response, the sink is required.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(sink) = &attributes.sink.as_ref() {
            UriValidator::validate_rpc_response(sink)
                .map_err(|e| UAttributesError::validation_error(e.to_string()))
        } else {
            Err(UAttributesError::validation_error("Missing Sink"))
        }
    }

    /// Validate the correlationId. In the case of an RPC response, the correlation id is required.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the request id to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_reqid(&self, attributes: &UAttributes) -> Result<(), UAttributesError> {
        if let Some(reqid) = &attributes.reqid.as_ref() {
            if reqid.is_uprotocol_uuid() {
                return Ok(());
            }
        }
        Err(UAttributesError::validation_error("Missing correlation ID"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    use crate::{
        UAuthority, UEntity, UPriority, UResource, UResourceBuilder, UUIDBuilder, UUri, UUID,
    };

    #[test_case(UMessageType::UMESSAGE_TYPE_UNSPECIFIED, "UAttributesValidator.Publish"; "succeeds for Unspecified message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, "UAttributesValidator.Publish"; "succeeds for Publish message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, "UAttributesValidator.Request"; "succeeds for Request message")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, "UAttributesValidator.Response"; "succeeds for Response message")]
    fn test_get_validator_returns_matching_validator(
        message_type: UMessageType,
        expected_validator_type: &str,
    ) {
        let validator: Box<dyn UAttributesValidator> =
            UAttributesValidators::get_validator(message_type);
        assert_eq!(validator.type_name(), expected_validator_type);
    }

    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, None, None, false; "for Publish message without ID nor TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, None, Some(0), false; "for Publish message without ID with TTL 0")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, None, Some(500), false; "for Publish message without ID with TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, Some(create_id_for_timestamp(1000)), None, false; "for Publish message with ID without TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, Some(create_id_for_timestamp(1000)), Some(0), false; "for Publish message with ID and TTL 0")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, Some(create_id_for_timestamp(1000)), Some(500), true; "for Publish message with ID and expired TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, Some(create_id_for_timestamp(1000)), Some(2000), false; "for Publish message with ID and non-expired TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, None, None, false; "for Request message without ID nor TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, None, Some(0), false; "for Request message without ID with TTL 0")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, None, Some(500), false; "for Request message without ID with TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, Some(create_id_for_timestamp(1000)), None, false; "for Request message with ID without TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, Some(create_id_for_timestamp(1000)), Some(0), false; "for Request message with ID and TTL 0")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, Some(create_id_for_timestamp(1000)), Some(500), true; "for Request message with ID and expired TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, Some(create_id_for_timestamp(1000)), Some(2000), false; "for Request message with ID and non-expired TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, None, None, false; "for Response message without ID nor TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, None, Some(0), false; "for Response message without ID with TTL 0")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, None, Some(500), false; "for Response message without ID with TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, Some(create_id_for_timestamp(1000)), None, false; "for Response message with ID without TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, Some(create_id_for_timestamp(1000)), Some(0), false; "for Response message with ID and TTL 0")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, Some(create_id_for_timestamp(1000)), Some(500), true; "for Response message with ID and expired TTL")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, Some(create_id_for_timestamp(1000)), Some(2000), false; "for Response message with ID and non-expired TTL")]
    fn test_is_expired(
        message_type: UMessageType,
        id: Option<UUID>,
        ttl: Option<i32>,
        should_be_expired: bool,
    ) {
        let attributes = UAttributes {
            type_: message_type.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            id: id.into(),
            ttl,
            ..Default::default()
        };
        let validator = UAttributesValidators::get_validator(message_type);
        assert!(validator.is_expired(&attributes).is_err() == should_be_expired);
    }

    #[test_case(Some(publish_topic()), None, None, None, None, None, true; "succeeds for topic only")]
    #[test_case(None, Some(destination()), None, None, None, None, true; "succeeds for destination only")]
    #[test_case(Some(publish_topic()), Some(destination()), None, None, None, None, false; "fails for both topic and destination")]
    #[test_case(None, None, None, None, None, None, false; "fails for neither topic nor destination")]
    #[test_case(Some(publish_topic()), None, Some(UUIDBuilder::new().build()), Some(1), Some(1), Some(100), true; "succeeds for valid attributes")]
    #[test_case(Some(publish_topic()), None, None, None, None, Some(-1), false; "fails for ttl < 0")]
    #[test_case(Some(publish_topic()), None, None, Some(-42), None, None, false; "fails for invalid commstatus")]
    #[test_case(Some(publish_topic()), None, None, None, Some(0), None, false; "fails for permission level < 1")]
    #[test_case(None, Some(UUri::default()), None, None, None, None, false; "fails for invalid destination")]
    #[test_case(
        None,
        None,
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        None,
        None,
        None,
        false;
        "fails for invalid request id")]
    fn test_validate_attributes_for_publish_message(
        source: Option<UUri>,
        sink: Option<UUri>,
        reqid: Option<UUID>,
        commstatus: Option<i32>,
        perm_level: Option<i32>,
        ttl: Option<i32>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            reqid: reqid.into(),
            source: source.into(),
            sink: sink.into(),
            commstatus,
            permission_level: perm_level,
            ttl,
            ..Default::default()
        };
        let validator = UAttributesValidators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok() == expected_result);
        if status.is_ok() {
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

    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, None, Some(2000), None, true; "succeeds for mandatory attributes")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), Some(1), Some(1), Some(2000), Some(String::from("token")), true; "succeeds for valid attributes")]
    #[test_case(Some(method_to_invoke()), None, Some(UUIDBuilder::new().build()), None, None, Some(100), None, false; "fails for missing reply-to-address")]
    #[test_case(Some(method_to_invoke()), Some(UUri::default()), Some(UUIDBuilder::new().build()), None, None, Some(2000), None, false; "fails for invalid reply-to-address")]
    #[test_case(None, Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, None, Some(100), None, false; "fails for missing method-to-invoke")]
    #[test_case(Some(UUri::default()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, None, Some(2000), None, false; "fails for invalid method-to-invoke")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, None, None, None, false; "fails for missing ttl")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, None, Some(0), None, false; "fails for ttl < 1")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), Some(-42), None, Some(2000), None, false; "fails for invalid commstatus")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, Some(1), Some(2000), None, true; "succeeds for valid permission level")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), Some(UUIDBuilder::new().build()), None, Some(0), Some(2000), None, false; "fails for invalid permission level")]
    #[test_case(Some(method_to_invoke()), Some(reply_to_address()), None, None, None, Some(2000), None, false; "fails for missing request ID")]
    #[test_case(
        Some(method_to_invoke()),
        Some(reply_to_address()),
        Some(UUID {
            // invalid UUID version (not 0b1000 but 0b1010)
            msb: 0x000000000001C000u64,
            lsb: 0x8000000000000000u64,
            ..Default::default()
        }),
        None,
        Some(1),
        Some(2000),
        None,
        false;
        "fails for invalid request id")]
    #[allow(clippy::too_many_arguments)]
    fn test_validate_attributes_for_rpc_request_message(
        method_to_invoke: Option<UUri>,
        reply_to_address: Option<UUri>,
        reqid: Option<UUID>,
        commstatus: Option<i32>,
        perm_level: Option<i32>,
        ttl: Option<i32>,
        token: Option<String>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            source: reply_to_address.into(),
            sink: method_to_invoke.into(),
            reqid: reqid.into(),
            commstatus,
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
            assert!(UAttributesValidators::Response
                .validator()
                .validate(&attributes)
                .is_err());
        }
    }

    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, None, None, true; "succeeds for mandatory attributes")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), Some(1), Some(1), Some(100), true; "succeeds for valid attributes")]
    #[test_case(None, Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, None, None, false; "fails for missing reply-to-address")]
    #[test_case(Some(UUri::default()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, None, None, false; "fails for invalid reply-to-address")]
    #[test_case(Some(reply_to_address()), None, Some(UUIDBuilder::new().build()), None, None, None, false; "fails for missing invoked-method")]
    #[test_case(Some(reply_to_address()), Some(UUri::default()), Some(UUIDBuilder::new().build()), None, None, None, false; "fails for invalid invoked-method")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), Some(1), None, None, true; "succeeds for valid commstatus")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), Some(-42), None, None, false; "fails for invalid commstatus")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, Some(1), None, true; "succeeds for valid permission level")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, Some(0), None, false; "fails for invalid permission level")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, None, Some(100), true; "succeeds for ttl > 0)")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, None, Some(0), true; "succeeds for ttl = 0")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), Some(UUIDBuilder::new().build()), None, None, Some(-1), false; "fails for ttl < 0")]
    #[test_case(Some(reply_to_address()), Some(method_to_invoke()), None, None, None, None, false; "fails for missing request id")]
    #[test_case(
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
        None,
        false;
        "fails for invalid request id")]

    fn test_validate_attributes_for_rpc_response_message(
        reply_to_address: Option<UUri>,
        invoked_method: Option<UUri>,
        reqid: Option<UUID>,
        commstatus: Option<i32>,
        perm_level: Option<i32>,
        ttl: Option<i32>,
        expected_result: bool,
    ) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_RESPONSE.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            reqid: reqid.into(),
            source: invoked_method.into(),
            sink: reply_to_address.into(),
            commstatus,
            permission_level: perm_level,
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
            assert!(UAttributesValidators::Request
                .validator()
                .validate(&attributes)
                .is_err());
        }
    }

    fn create_id_for_timestamp(created_millis_ago: u64) -> UUID {
        let now = SystemTime::now();
        let since_epoch_start = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let now_millis_since_epoch_start = u64::try_from(since_epoch_start.as_millis()).unwrap();

        let creation_time = now_millis_since_epoch_start - created_millis_ago;
        UUIDBuilder::new().build_with_instant(creation_time)
    }

    fn publish_topic() -> UUri {
        UUri {
            authority: Some(UAuthority {
                name: Some(String::from("vcu.someVin")),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                name: "cabin".to_string(),
                version_major: Some(1),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                name: "door".to_string(),
                instance: Some("driver_seat".to_string()),
                message: Some("status".to_string()),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        }
    }

    fn destination() -> UUri {
        UUri {
            authority: Some(UAuthority {
                name: Some(String::from("vcu.someVin")),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                name: "dashboard".to_string(),
                version_major: Some(1),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                name: "tire_pressure".to_string(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        }
    }

    fn reply_to_address() -> UUri {
        UUri {
            authority: Some(UAuthority {
                name: Some(String::from("vcu.someVin")),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                name: "consumer".to_string(),
                version_major: Some(1),
                ..Default::default()
            })
            .into(),
            resource: Some(UResourceBuilder::for_rpc_response()).into(),
            ..Default::default()
        }
    }

    fn method_to_invoke() -> UUri {
        UUri {
            authority: Some(UAuthority {
                name: Some(String::from("vcu.someVin")),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                name: "provider".to_string(),
                version_major: Some(1),
                ..Default::default()
            })
            .into(),
            resource: Some(UResourceBuilder::for_rpc_request(
                Some("echo".to_string()),
                None,
            ))
            .into(),
            ..Default::default()
        }
    }
}
