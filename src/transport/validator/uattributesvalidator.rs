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

use crate::types::ValidationResult;
use crate::uprotocol::{UAttributes, UCode, UMessageType, Uuid};
use crate::uri::validator::UriValidator;
use crate::uuid::builder::UuidUtils;

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
    fn validate(&self, attributes: &UAttributes) -> ValidationResult {
        let error_messages: Vec<String> = vec![
            self.validate_type(attributes),
            self.validate_ttl(attributes),
            self.validate_sink(attributes),
            self.validate_commstatus(attributes),
            self.validate_permission_level(attributes),
            self.validate_reqid(attributes),
        ]
        .into_iter()
        .filter(|status| status.is_failure())
        .map(|status| status.get_message())
        .collect();

        let error_message = error_messages.join(", ");
        if error_message.is_empty() {
            ValidationResult::Success
        } else {
            ValidationResult::Failure(error_message)
        }
    }

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
    fn is_expired(&self, attributes: &UAttributes) -> ValidationResult {
        let ttl = match attributes.ttl {
            Some(t) if t > 0 => t,
            Some(_) => return ValidationResult::Success,
            None => 0,
        };

        if let Some(uuid) = &attributes.id {
            if let Some(time) = UuidUtils::get_time(uuid) {
                let delta = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
                    - time;

                if ttl <= 0 {
                    return ValidationResult::Success;
                }

                if delta >= ttl as u64 {
                    return ValidationResult::Failure("Payload is expired".to_string());
                }
            }
        }
        ValidationResult::Success
    }

    /// Validate the time to live configuration. If the UAttributes does not contain a time to live
    /// then the UStatus is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message priority to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_ttl(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(ttl) = attributes.ttl {
            if ttl < 1 {
                return ValidationResult::Failure(format!("Invalid TTL [{}]", ttl));
            }
        }
        ValidationResult::Success
    }

    /// Validates the sink URI for the default case. If the UAttributes does not contain a sink
    /// then the ValidationResult is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_sink(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(sink) = &attributes.sink {
            return UriValidator::validate(sink);
        }
        ValidationResult::Success
    }

    /// Validates the permission level for the default case. If the UAttributes does not contain
    /// a permission level then the ValidationResult is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the permission level to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_permission_level(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(plevel) = &attributes.permission_level {
            if *plevel < 1 {
                return ValidationResult::Failure("Invalid Permission Level".to_string());
            }
        }
        ValidationResult::Success
    }

    /// Validates the communication status (`commStatus`) for the default case. If the UAttributes
    /// does not contain a request id then the UStatus is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the communication status to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_commstatus(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(cs) = attributes.commstatus {
            if UCode::try_from(cs).is_err() {
                return ValidationResult::Failure("Invalid Communication Status Code".into());
            }
        }
        ValidationResult::Success
    }

    /// Validates the `correlationId` for the default case. If the UAttributes does not contain
    /// a request id then the UStatus is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `Attributes` object containing the request id to validate.
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` that is success or failed with a failure message.
    fn validate_reqid(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(reqid) = &attributes.reqid {
            if !UuidUtils::is_uuid(reqid) {
                return ValidationResult::Failure("Invalid UUID".into());
            }
        }
        ValidationResult::Success
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
    fn validate_type(&self, attributes: &UAttributes) -> ValidationResult;
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
                _ => {}
            }
        }
        Box::new(PublishValidator)
    }
}

/// Validate UAttributes with type UMessageType::Publish
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
    fn validate_type(&self, attributes: &UAttributes) -> ValidationResult {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypePublish => return ValidationResult::Success,
                _ => {
                    return ValidationResult::Failure(format!(
                        "Wrong Attribute Type [{}]",
                        mt.as_str_name()
                    ));
                }
            }
        }
        ValidationResult::Failure(format!("Unknown Attribute Type [{}]", attributes.r#type))
    }
}

/// Validate UAttributes with type UMessageType::Request
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
    fn validate_type(&self, attributes: &UAttributes) -> ValidationResult {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypeRequest => return ValidationResult::Success,
                _ => {
                    return ValidationResult::Failure(format!(
                        "Wrong Attribute Type [{}]",
                        mt.as_str_name()
                    ));
                }
            }
        }
        ValidationResult::Failure(format!("Unknown Attribute Type [{}]", attributes.r#type))
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
    fn validate_sink(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(sink) = &attributes.sink {
            UriValidator::validate_rpc_response(sink)
        } else {
            ValidationResult::Failure("Missing Sink".to_string())
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
    fn validate_ttl(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(ttl) = attributes.ttl {
            if ttl > 0 {
                ValidationResult::Success
            } else {
                ValidationResult::Failure(format!("Invalid TTL [{}]", ttl))
            }
        } else {
            ValidationResult::Failure("Missing TTL".to_string())
        }
    }
}

/// Validate UAttributes with type UMessageType::Response
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
    fn validate_type(&self, attributes: &UAttributes) -> ValidationResult {
        if let Ok(mt) = UMessageType::try_from(attributes.r#type) {
            match mt {
                UMessageType::UmessageTypeResponse => return ValidationResult::Success,
                _ => {
                    return ValidationResult::Failure(format!(
                        "Wrong Attribute Type [{}]",
                        mt.as_str_name()
                    ));
                }
            }
        }
        ValidationResult::Failure(format!("Unknown Attribute Type {}", attributes.r#type))
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
    fn validate_sink(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(sink) = &attributes.sink {
            UriValidator::validate_rpc_method(sink)
        } else {
            ValidationResult::Failure("Missing Sink".to_string())
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
    fn validate_reqid(&self, attributes: &UAttributes) -> ValidationResult {
        if let Some(reqid) = &attributes.reqid {
            if *reqid == Uuid::default() {
                return ValidationResult::Failure("Missing correlation Id".to_string());
            }
            if UuidUtils::is_uuid(reqid) {
                return ValidationResult::Success;
            }
        }
        ValidationResult::Failure("Missing correlation Id".to_string())
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
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
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
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
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
        assert!(status.is_failure());
        assert_eq!(
            status.get_message(),
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
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid TTL [0]");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_invalid_sink() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_sink(UUri::default())
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Uri is empty");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_invalid_permission_level() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_permission_level(0)
            .build();

        // might not work - need to set an invalid (negative) plevel manually...

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid Permission Level");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_communication_status() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_commstatus(-42)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid Communication Status Code");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_request_id() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_reqid(Uuid::from(uuid::Uuid::new_v4()))
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid UUID");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload() {
        let attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000).build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
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
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
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
        assert!(status.is_failure());
        assert_eq!(
            status.get_message(),
            "Wrong Attribute Type [UMESSAGE_TYPE_RESPONSE]"
        );
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_ttl() {
        let attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 0).build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid TTL [0]");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_sink() {
        let attributes =
            UAttributesBuilder::request(UPriority::UpriorityCs4, UUri::default(), 1000).build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Uri is empty");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_permission_level() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_permission_level(0)
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid Permission Level");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_communication_status() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_commstatus(-42)
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid Communication Status Code");
    }

    #[test]
    fn test_validate_attributes_for_rpc_request_message_payload_invalid_request_id() {
        let attributes = UAttributesBuilder::request(UPriority::UpriorityCs4, build_sink(), 1000)
            .with_reqid(Uuid::from(uuid::Uuid::new_v4()))
            .build();

        let validator = Validators::Request.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid UUID");
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
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
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
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_invalid_type() {
        let attributes =
            UAttributesBuilder::notification(UPriority::UpriorityCs4, build_sink()).build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert!(status
            .get_message()
            .contains("Wrong Attribute Type [UMESSAGE_TYPE_PUBLISH]"));
        assert!(status.get_message().contains("Missing correlation Id"));
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
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid TTL [0]");
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
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid Permission Level");
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
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Invalid Communication Status Code");
    }

    #[test]
    fn test_validate_attributes_for_rpc_response_message_payload_missing_request_id() {
        let attributes_builder =
            UAttributesBuilder::response(UPriority::UpriorityCs4, build_sink(), Uuid::default());
        let attributes = attributes_builder.build();

        let validator = Validators::Response.validator();
        let status = validator.validate(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Missing correlation Id");
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
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Missing correlation Id");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_not_expired() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0).build();

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_not_expired_with_ttl_zero() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(0)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_not_expired_with_ttl() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(10000)
            .build();

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_success());
        assert_eq!(status.get_message(), "");
    }

    #[test]
    fn test_validate_attributes_for_publish_message_payload_expired() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_ttl(1)
            .build();

        std::thread::sleep(std::time::Duration::from_millis(800));

        let validator = Validators::Publish.validator();
        let status = validator.is_expired(&attributes);
        assert!(status.is_failure());
        assert_eq!(status.get_message(), "Payload is expired");
    }

    #[test]
    fn test_validating_request_containing_token() {
        let attributes = UAttributesBuilder::publish(UPriority::UpriorityCs0)
            .with_token("None")
            .build();

        let validator = Validators::get_validator(&attributes);
        assert_eq!("UAttributesValidator.Publish", validator.type_name());
        let status = validator.validate(&attributes);
        assert!(status.is_success());
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
