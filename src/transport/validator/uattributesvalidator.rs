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

use crate::transport::validator::ValidationError;
use crate::uprotocol::{UAttributes, UCode, UMessageType, Uuid};
use crate::uri::validator::UriValidator;

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
    /// Returns a `ValidationError` when one or more validations fail. The error will contain a concatenated message of all the validation errors separated by a semicolon (`;`). Each part of the message corresponds to a failure from one of the specific validation functions called within `validate`. These may include errors from:
    ///
    /// - `validate_type` if the message type in `UAttributes` fails validation.
    /// - `validate_ttl` if the time-to-live value is invalid.
    /// - `validate_sink` if the sink URI does not pass validation.
    /// - `validate_commstatus` if the communication status is invalid.
    /// - `validate_permission_level` if the permission level is below the required threshold.
    /// - `validate_reqid` if the request ID is not a valid UUID.
    ///
    /// If all validations pass, the function returns `Ok(())`, indicating no errors were found.
    fn validate(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        let error_message = vec![
            self.validate_type(attributes),
            self.validate_ttl(attributes),
            self.validate_sink(attributes),
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
            Err(ValidationError::new(error_message))
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
    /// Returns a `ValidationError` in the following cases:
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
    fn is_expired(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        let ttl = match attributes.ttl {
            Some(t) if t > 0 => t,
            Some(_) => return Ok(()),
            None => 0,
        };

        if let Some(time) = attributes.id.as_ref().and_then(Uuid::get_time) {
            let delta = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(duration) => {
                    if let Ok(duration) = u64::try_from(duration.as_millis()) {
                        duration - time
                    } else {
                        return Err(ValidationError::new("Invalid duration"));
                    }
                }
                Err(e) => return Err(ValidationError::new(e.to_string())),
            };

            if ttl <= 0 {
                return Ok(());
            }

            if let Ok(ttl) = u64::try_from(ttl) {
                if delta >= ttl {
                    return Err(ValidationError::new("Payload is expired"));
                }
            } else {
                return Err(ValidationError::new("Invalid TTL"));
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
    /// Returns a `ValidationError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `ttl` (time to live) and it is less than 1.
    ///   The error message will specify the invalid TTL value, indicating that it does not meet the minimum threshold considered valid.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `ttl`, considering it a valid case.
    fn validate_ttl(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(ttl) = attributes.ttl {
            if ttl < 1 {
                return Err(ValidationError::new(format!("Invalid TTL [{ttl}]")));
            }
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
    /// Returns a `ValidationError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `sink` and the sink URI does not pass the validation criteria set by `UriValidator`.
    ///   The error will provide details about why the sink URI is considered invalid, such as format issues or other validation failures.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `sink`, considering it a valid case.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(sink) = &attributes.sink {
            return UriValidator::validate(sink);
        }
        Ok(())
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
    /// Returns a `ValidationError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `permission_level` and it is less than 1.
    ///   The error will indicate that the permission level is invalid, suggesting that it does not meet the minimum required level.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `permission_level`, considering it a valid case.
    fn validate_permission_level(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(plevel) = &attributes.permission_level {
            if *plevel < 1 {
                return Err(ValidationError::new("Invalid Permission Level"));
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
    /// Returns a `ValidationError` in the following case:
    ///
    /// - If the `UAttributes` object contains a `reqid` (request ID) and it is not a valid UUID.
    ///   The error will indicate that the request ID format is invalid, helping to identify issues with the input `UAttributes`.
    ///
    /// The function does not return an error if the `UAttributes` object does not contain a `reqid`, considering it a valid case.
    fn validate_commstatus(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(cs) = attributes.commstatus {
            if UCode::try_from(cs).is_err() {
                return Err(ValidationError::new(format!(
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
    /// This function returns a `ValidationError` in the following scenario:
    ///
    /// - If `UAttributes` contains a request ID (`reqid`), but it is not a valid UUID.
    ///   The `ValidationError` will contain a message such as "Invalid UUID" to indicate the nature of the validation failure.
    ///
    /// The function considers the absence of a request ID in `UAttributes` as a valid case and does not return an error in such a scenario.
    fn validate_reqid(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(reqid) = &attributes.reqid {
            if !reqid.is_uprotocol_uuid() {
                return Err(ValidationError::new("Invalid UUID"));
            }
        }
        Ok(())
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
    /// This function returns a `ValidationError` in cases where:
    ///
    /// - The `MessageType` in `UAttributes` does not conform to the expected format or value.
    /// - The `UAttributes` object contains inconsistent or invalid data that fails the validation criteria.
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), ValidationError>;
}

/// Enum that hold the implementations of uattributesValidator according to type.
pub enum Validators {
    Publish,
    Request,
    Response,
}

impl Validators {
    pub fn validator(&self) -> Box<dyn UAttributesValidator> {
        match self {
            Validators::Publish => Box::new(PublishValidator),
            Validators::Request => Box::new(RequestValidator),
            Validators::Response => Box::new(ResponseValidator),
        }
    }

    pub fn get_validator(attributes: &UAttributes) -> Box<dyn UAttributesValidator> {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypePublish => return Box::new(PublishValidator),
                UMessageType::UmessageTypeRequest => return Box::new(RequestValidator),
                UMessageType::UmessageTypeResponse => return Box::new(ResponseValidator),
                UMessageType::UmessageTypeUnspecified => {}
            }
        }
        Box::new(PublishValidator)
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
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypePublish => return Ok(()),
                _ => {
                    return Err(ValidationError::new(format!(
                        "Wrong Attribute Type [{}]",
                        mt.as_str_name()
                    )));
                }
            }
        }
        Err(ValidationError::new(format!(
            "Unknown Attribute Type [{}]",
            attributes.r#type
        )))
    }
}

/// Validate `UAttributes` with type `UMessageType::Request`
pub struct RequestValidator;

impl UAttributesValidator for RequestValidator {
    fn type_name(&self) -> &'static str {
        "UAttributesValidator.Request"
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
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypeRequest => return Ok(()),
                _ => {
                    return Err(ValidationError::new(format!(
                        "Wrong Attribute Type [{}]",
                        mt.as_str_name()
                    )));
                }
            }
        }
        Err(ValidationError::new(format!(
            "Unknown Attribute Type [{}]",
            attributes.r#type
        )))
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
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(sink) = &attributes.sink {
            UriValidator::validate_rpc_method(sink)
        } else {
            Err(ValidationError::new("Missing Sink"))
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
    fn validate_ttl(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(ttl) = attributes.ttl {
            if ttl > 0 {
                Ok(())
            } else {
                Err(ValidationError::new(format!("Invalid TTL [{ttl}]")))
            }
        } else {
            Err(ValidationError::new("Missing TTL"))
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
    fn validate_type(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypeResponse => return Ok(()),
                _ => {
                    return Err(ValidationError::new(format!(
                        "Wrong Attribute Type [{}]",
                        mt.as_str_name()
                    )));
                }
            }
        }
        Err(ValidationError::new(format!(
            "Unknown Attribute Type [{}]",
            attributes.r#type
        )))
    }

    /// Validates that attributes for a message meant for an RPC response has a destination sink.
    /// In the case of an RPC response, the sink is required.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_sink(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(sink) = &attributes.sink {
            UriValidator::validate_rpc_response(sink)
        } else {
            Err(ValidationError::new("Missing Sink"))
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
    fn validate_reqid(&self, attributes: &UAttributes) -> Result<(), ValidationError> {
        if let Some(reqid) = &attributes.reqid {
            if *reqid == Uuid::default() {
                return Err(ValidationError::new("Missing correlation Id"));
            }
            if reqid.is_uprotocol_uuid() {
                return Ok(());
            }
        }
        Err(ValidationError::new("Missing correlation Id"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::builder::UAttributesBuilder;
    use crate::uprotocol::{Remote, UAuthority, UEntity, UPriority, UUri, Uuid};
    use crate::uri::builder::resourcebuilder::UResourceBuilder;
    use crate::uuid::builder::UUIDv8Builder;

    #[test]
    fn test_fetching_validator_for_valid_types() {
        // Test for PUBLISH type
        let publish_attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0).build();
        let publish_validator: Box<dyn UAttributesValidator> =
            Validators::get_validator(&publish_attributes);
        assert_eq!(
            publish_validator.type_name(),
            "UAttributesValidator.Publish"
        );

        // Test for REQUEST type
        let request_attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000).build();
        let request_validator = Validators::get_validator(&request_attributes);
        assert_eq!(
            request_validator.type_name(),
            "UAttributesValidator.Request"
        );

        // Test for RESPONSE type
        let response_attributes =
            UAttributesBuilder::response(UPriority::UpriorityCs4, UUri::default(), Uuid::default())
                .build();
        let response_validator = Validators::get_validator(&response_attributes);
        assert_eq!(
            response_validator.type_name(),
            "UAttributesValidator.Response"
        );
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0).build();
        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_all_values() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(1000)
            .with_sink(build_sink())
            .with_permission_level(2)
            .with_commstatus(3)
            .with_reqid(UUIDv8Builder::new().build())
            .build();
        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_invalid_type() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs0,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Wrong Attribute Type [UMESSAGE_TYPE_RESPONSE]"
        );
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_invalid_ttl() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(0)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid TTL [0]");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_invalid_sink() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_sink(UUri::default())
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Uri is empty");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_invalid_permission_level() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_permission_level(0)
            .build();

        // might not work - need to set an invalid (negative) plevel manually...

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid Permission Level");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_communication_status() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_commstatus(-42)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid Communication Status Code [-42]"
        );
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_request_id() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_reqid(Uuid::from(uuid::Uuid::new_v4()))
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid UUID");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload() {
        let attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000).build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_all_values() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_permission_level(2)
            .with_commstatus(3)
            .with_reqid(UUIDv8Builder::new().build())
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_type() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .with_ttl(1000)
        .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Wrong Attribute Type [UMESSAGE_TYPE_RESPONSE]"
        );
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_ttl() {
        let attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 0).build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid TTL [0]");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_sink() {
        let attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, UUri::default(), 1000).build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Uri is empty");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_permission_level() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_permission_level(0)
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid Permission Level");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_communication_status() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_commstatus(-42)
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid Communication Status Code [-42]"
        );
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_request_id() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_reqid(Uuid::from(uuid::Uuid::new_v4()))
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid UUID");
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_all_values() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .with_permission_level(2)
        .with_commstatus(3)
        .build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_invalid_type() {
        let attributes =
            UAttributesBuilder::notification(UPriority::UpriorityCs4, build_sink()).build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Wrong Attribute Type [UMESSAGE_TYPE_PUBLISH]"));
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Missing correlation Id"));
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_invalid_ttl() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .with_ttl(0)
        .build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid TTL [0]");
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_invalid_permission_level() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .with_permission_level(0)
        .build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid Permission Level");
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_invalid_communication_status() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .with_commstatus(-42)
        .build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid Communication Status Code [-42]"
        );
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_missing_request_id() {
        let attributes_builder =
            UAttributesBuilder::response(UPriority::UpriorityCs4, build_sink(), Uuid::default());
        let attributes = attributes_builder.build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Missing correlation Id");
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_invalid_request_id() {
        let attributes = UAttributesBuilder::response(
            UPriority::UpriorityCs4,
            build_sink(),
            UUIDv8Builder::new().build(),
        )
        .with_reqid(Uuid::from(uuid::Uuid::new_v4()))
        .build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Missing correlation Id");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_not_expired() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0).build();

        let validator = Validators::Publish.validator();
        let status: Result<(), ValidationError> = validator.is_expired(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_not_expired_with_ttl_zero() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(0)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_not_expired_with_ttl() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(10000)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_expired() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(1)
            .build();

        std::thread::sleep(std::time::Duration::from_millis(800));

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Payload is expired");
    }

    #[test]
    fn test_validating_request_containing_token() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_token("None")
            .build();

        let validator = Validators::get_validator(&attributes);
        assert_eq!("UAttributesValidator.Publish", validator.type_name());
        let status = validator.validate(&attributes);
        assert!(status.is_ok());
    }

    fn build_sink() -> UUri {
        UUri {
            authority: Some(UAuthority {
                remote: Some(Remote::Name(
                    "vcu.someVin.veh.uprotocol.corp.com".to_string(),
                )),
            }),
            entity: Some(UEntity {
                name: "petapp.uprotocol.corp.com".to_string(),
                version_major: Some(1),
                ..Default::default()
            }),
            resource: Some(UResourceBuilder::for_rpc_response()),
        }
    }
}
