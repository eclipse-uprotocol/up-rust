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

use crate::transport::datamodel::uattributes::UAttributes;
use crate::transport::datamodel::umessagetype::UMessageType;
use crate::transport::datamodel::ustatus::{UCode, UStatus};
use crate::uri::validator::urivalidator::UriValidator;
use crate::uuid::uuidutils::UuidUtils;

use std::time::SystemTime;

/// `UAttributes` is the struct that defines the Payload. It serves as the configuration for various aspects
/// like time to live, priority, security tokens, and more. Each variant of `UAttributes` defines a different
/// type of message payload. The payload could represent a simple published payload with some state change,
/// an RPC request payload, or an RPC response payload.
///
/// `UAttributesValidator` is a trait implemented by all validators for `UAttributes`. It provides functionality
/// to help validate that a given `UAttributes` instance is correctly configured to define the Payload.
pub trait UAttributesValidator {
    fn validate(&self, attributes: &UAttributes) -> UStatus {
        let error_messages: Vec<String> = vec![
            self.validate_type(attributes),
            self.validate_id(attributes),
            self.validate_sink(attributes),
            self.validate_commstatus(attributes),
            self.validate_ttl(attributes),
            self.validate_reqid(attributes),
        ]
        .into_iter()
        .filter(|status| status.is_failed())
        .map(|status| status.message().clone())
        .collect();

        let error_message = error_messages.join(" ");
        if error_message.is_empty() {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason(&error_message, UCode::InvalidArgument)
        }
    }

    fn type_name(&self) -> &'static str;

    /// Validates the `MessageType` of `UAttributes`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the `MessageType` is valid or not.
    fn validate_type(&self, attributes: &UAttributes) -> UStatus;

    /// Indicates whether the payload with these [`UAttributes`] has expired.
    ///
    /// # Parameters
    /// - `attributes`: Reference to a [`UAttributes`] struct containing the time-to-live value.
    ///
    /// # Returns
    /// - Returns a [`UStatus`] that is `Success` if not expired, or `Failure` with a validation message or expiration code.
    fn is_expired(&self, attributes: &UAttributes) -> UStatus {
        match attributes.ttl {
            None => UStatus::ok_with_id("Not Expired"),
            Some(0) => UStatus::ok_with_id("Not Expired"),
            Some(ttl) => {
                // Assuming `UuidUtils::get_time` returns a Result
                if let Ok(time) = UuidUtils::get_time(&attributes.id) {
                    let delta = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64
                        - time;

                    if delta >= ttl as u64 {
                        return UStatus::fail_with_msg_and_reason(
                            "Payload is expired",
                            UCode::DeadlineExceeded,
                        );
                    }
                }
                UStatus::ok_with_id("Not Expired")
            }
        }
    }

    /// Validates the UUID identifier within `UAttributes`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the id to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the UUID is valid or not.
    fn validate_id(&self, attributes: &UAttributes) -> UStatus {
        if UuidUtils::is_valid_uuid(&attributes.id) {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason("Invalid UUID", UCode::InvalidArgument)
        }
    }

    /// Validates the `correlationId` for the default case. If the UAttributes does not contain a request id then the UStatus is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `Attributes` object containing the request id to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the `correlationId` is valid or not.
    fn validate_reqid(&self, attributes: &UAttributes) -> UStatus {
        if let Some(reqid) = attributes.reqid {
            if !UuidUtils::is_valid_uuid(&reqid) {
                return UStatus::fail_with_msg_and_reason(
                    "Invalid correlation UUID",
                    UCode::InvalidArgument,
                );
            }
        }
        UStatus::ok()
    }

    /// Validate the time to live configuration. If the UAttributes does not contain a time to live then the UStatus is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message priority to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` that is success or failed with a failure message.
    fn validate_ttl(&self, attributes: &UAttributes) -> UStatus {
        if let Some(ttl) = attributes.ttl {
            if ttl < 1 {
                return UStatus::fail_with_msg_and_reason("Invalid TTL", UCode::InvalidArgument);
            }
        }
        UStatus::ok()
    }

    /// Validates the sink URI for the default case.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the URI is valid or not.
    fn validate_sink(&self, attributes: &UAttributes) -> UStatus {
        if let Some(sink) = &attributes.sink {
            return UriValidator::validate(sink);
        }
        UStatus::ok()
    }

    /// Validates the communication status (`commStatus`) for the default case. If the UAttributes does not contain a request id then the UStatus is ok.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the communication status to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the communication status (`commStatus`) is valid or not.
    fn validate_commstatus(&self, attributes: &UAttributes) -> UStatus {
        if let Some(cs) = attributes.commstatus {
            if UCode::from(cs) == UCode::Unspecified {
                return UStatus::fail_with_msg_and_reason(
                    "Invalid Communication Status Code",
                    UCode::InvalidArgument,
                );
            }
        }
        UStatus::ok()
    }

    // Java SDK validation not applicable to Rust implementation:
    // - validate_priority checks if prio is present - this is a given in the Rust implementation
    // - validate plevel oks if ttl present and if yes if >= 0 - this is implicit in the Rust implementation
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
        match attributes.message_type {
            UMessageType::Response => Box::new(ResponseValidator),
            UMessageType::Request => Box::new(RequestValidator),
            _ => Box::new(PublishValidator),
        }
    }
}

/// Validate UAttributes with type UMessageType::Publish
pub struct PublishValidator;

impl UAttributesValidator for PublishValidator {
    fn type_name(&self) -> &'static str {
        "PublishValidator"
    }

    /// Validates that attributes for a message meant to publish state changes has the correct type.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the `MessageType` is valid or not.
    fn validate_type(&self, attributes: &UAttributes) -> UStatus {
        if attributes.message_type == UMessageType::Publish {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason("Wrong Attribute Type", UCode::InvalidArgument)
        }
    }
}

/// Validate UAttributes with type UMessageType::Request
pub struct RequestValidator;

impl UAttributesValidator for RequestValidator {
    fn type_name(&self) -> &'static str {
        "RequestValidator"
    }

    /// Validates the `UAttributes` instance for the type `UMessageType::Request`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the `MessageType` is valid or not.
    fn validate_type(&self, attributes: &UAttributes) -> UStatus {
        if attributes.message_type == UMessageType::Request {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason("Wrong Attribute Type", UCode::InvalidArgument)
        }
    }

    /// Validates the sink URI for the type `UMessageType::Request`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the URI is valid or not.
    fn validate_sink(&self, attributes: &UAttributes) -> UStatus {
        if let Some(sink) = &attributes.sink {
            UriValidator::validate(sink)
        } else {
            UStatus::fail_with_msg_and_reason("Missing Sink", UCode::InvalidArgument)
        }
    }

    /// Validates the Time-to-Live (TTL) attribute for the type `UMessageType::Request`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the ttl to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the TTL is valid or not.
    fn validate_ttl(&self, attributes: &UAttributes) -> UStatus {
        if let Some(ttl) = attributes.ttl {
            if ttl > 0 {
                UStatus::ok()
            } else {
                UStatus::fail_with_msg_and_reason("Invalid TTL", UCode::InvalidArgument)
            }
        } else {
            UStatus::fail_with_msg_and_reason("Missing TTL", UCode::InvalidArgument)
        }
    }
}

/// Validate UAttributes with type UMessageType::Response
pub struct ResponseValidator;

impl UAttributesValidator for ResponseValidator {
    fn type_name(&self) -> &'static str {
        "ResponseValidator"
    }

    /// Validates the `UAttributes` instance for the type `UMessageType::Response`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the message type to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the `MessageType` is valid or not.
    fn validate_type(&self, attributes: &UAttributes) -> UStatus {
        if attributes.message_type == UMessageType::Response {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason("Wrong Attribute Type", UCode::InvalidArgument)
        }
    }

    /// Validates the sink URI for the type `UMessageType::Response`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the sink to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the URI is valid or not.
    fn validate_sink(&self, attributes: &UAttributes) -> UStatus {
        if let Some(sink) = &attributes.sink {
            UriValidator::validate(sink)
        } else {
            UStatus::fail_with_msg_and_reason("Missing Sink", UCode::InvalidArgument)
        }
    }

    /// Validates the `correlationId` for the type `UMessageType::Response`.
    ///
    /// # Arguments
    ///
    /// * `attributes` - `UAttributes` object containing the request id to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` indicating whether the `correlationId` is valid or not.
    fn validate_reqid(&self, attributes: &UAttributes) -> UStatus {
        if let Some(reqid) = attributes.reqid {
            if UuidUtils::is_valid_uuid(&reqid) {
                UStatus::ok()
            } else {
                UStatus::fail_with_msg_and_reason(
                    "Invalid correlation UUID",
                    UCode::InvalidArgument,
                )
            }
        } else {
            UStatus::fail_with_msg_and_reason("Missing correlation UUID", UCode::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::datamodel::uattributes::UAttributesBuilder;
    use crate::transport::datamodel::upriority::UPriority;
    use crate::transport::datamodel::userializationhint::USerializationHint;
    use crate::uri::serializer::longuriserializer::LongUriSerializer;
    use crate::uri::serializer::uriserializer::UriSerializer;
    use crate::uuid::uuidbuilder::{UUIDFactory, UUIDv8Factory};

    use uuid::Uuid;

    #[test]
    fn test_fetching_validator_for_valid_types() {
        // Test for PUBLISH type
        let publish_attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .build()
        .unwrap();
        let publish_validator: Box<dyn UAttributesValidator> =
            Validators::get_validator(&publish_attributes);
        assert_eq!(publish_validator.type_name(), "PublishValidator");

        // Test for REQUEST type
        let request_attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Request,
            UPriority::Low,
        )
        .build()
        .unwrap();
        let request_validator = Validators::get_validator(&request_attributes);
        assert_eq!(request_validator.type_name(), "RequestValidator");

        // Test for RESPONSE type
        let response_attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Response,
            UPriority::Low,
        )
        .build()
        .unwrap();
        let response_validator = Validators::get_validator(&response_attributes);
        assert_eq!(response_validator.type_name(), "ResponseValidator");
    }

    // We forego this test, because in Rust we enforce that a UAttributesBuilder has a message_type,
    // and with that this test can not fail.
    // #[test]
    // fn test_validator_invalid_types() {}

    #[test]
    fn test_validating_valid_publish_messagetypes() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);

        let status = validator.validate(&attributes);
        assert!(status.is_success());
        assert_eq!(status.message(), "ok");
    }

    // Can't pass invalid Priority value in Rust
    // #[test]
    // fn test_validating_invalid_priority_attribute() {}

    // No priority validator in Rust, as there's no way for Prio to be missing or invalid
    // #[test]
    // fn test_validating_valid_priority_attribute() {}

    #[test]
    fn test_validating_publish_invalid_ttl_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        // changed this to 0 as the only possible invalid value in Rust implementation
        .with_ttl(0)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);

        let status = validator.validate_ttl(&attributes);
        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Invalid TTL");
    }

    #[test]
    fn test_validating_valid_ttl_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_ttl(100)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);

        let status = validator.validate_ttl(&attributes);
        assert!(status.is_success());
    }

    #[test]
    fn test_validating_invalid_sink_attribute() {
        let uri = LongUriSerializer::deserialize("//".to_string());

        // Build the attributes
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_sink(uri)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);

        // Validate the sink attribute
        let status = validator.validate_sink(&attributes);
        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Uri is empty.");
    }

    #[test]
    fn test_validating_valid_sink_attribute() {
        let uri = LongUriSerializer::deserialize("/haartley/1".to_string());

        // Build the attributes
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_sink(uri)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);

        // Validate the sink attribute
        let status = validator.validate_sink(&attributes);
        assert!(status.is_success());
    }

    #[test]
    fn test_validating_invalid_req_id_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_reqid(Uuid::new_v4())
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate_reqid(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Invalid correlation UUID");
    }

    #[test]
    fn test_validating_valid_req_id_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_reqid(UUIDv8Factory::new().create())
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate_reqid(&attributes);

        assert!(status.is_success());
    }

    // Can't have invalid permission level values in Rust implementation
    // #[test]
    // fn test_validating_invalid_permission_level_attribute() {}

    #[test]
    fn test_validating_valid_permission_level_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_plevel(0)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate(&attributes);

        assert!(status.is_success());
    }

    #[test]
    fn test_validating_invalid_commstatus_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_commstatus(100)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate_commstatus(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Invalid Communication Status Code");
    }

    #[test]
    fn test_validating_valid_commstatus_attribute() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_commstatus(UCode::Aborted as i32)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate_commstatus(&attributes);

        assert!(status.is_success());
    }

    #[test]
    fn test_validating_request_message_types() {
        let sink = LongUriSerializer::deserialize("/hartley/1/rpc.response".to_string());
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Request,
            UPriority::Low,
        )
        .with_sink(sink)
        .with_ttl(100)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate(&attributes);

        assert!(status.is_success());
        assert_eq!(status.message(), "ok");
    }

    #[test]
    fn test_validating_request_validator_with_wrong_messagetype() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::Request.validator();
        let status = validator.validate(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert!(status.message().contains("Wrong Attribute Type"));
        assert!(status.message().contains("Missing Sink"));
        assert!(status.message().contains("Missing TTL"));
    }

    #[test]
    fn test_validating_request_validator_with_wrong_bad_ttl() {
        let sink = LongUriSerializer::deserialize("/hartley/1/rpc.response".to_string());
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Request,
            UPriority::NetworkControl,
        )
        .with_sink(sink)
        .with_ttl(0)
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::Request.validator();
        let status = validator.validate(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Invalid TTL");
    }

    #[test]
    fn test_validating_response_validator_with_wrong_bad_ttl() {
        let sink = LongUriSerializer::deserialize("/hartley/1/rpc.response".to_string());
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Response,
            UPriority::NetworkControl,
        )
        .with_sink(sink)
        .with_ttl(0)
        .with_reqid(UUIDv8Factory::new().create())
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::Response.validator();
        let status = validator.validate(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Invalid TTL");
    }

    #[test]
    fn test_validating_response_validator_with_bad_reqid() {
        let sink = LongUriSerializer::deserialize("/hartley/1/rpc.response".to_string());
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Response,
            UPriority::NetworkControl,
        )
        .with_sink(sink)
        .with_ttl(100)
        .with_reqid(Uuid::new_v4())
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::Response.validator();
        let status = validator.validate(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Invalid correlation UUID");
    }

    #[test]
    fn test_validating_publish_validator_with_wrong_messagetype() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Request,
            UPriority::Low,
        )
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::Publish.validator();
        let status = validator.validate(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert_eq!(status.message(), "Wrong Attribute Type");
    }

    #[test]
    fn test_validating_response_validator_with_wrong_messagetype() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::Response.validator();
        let status = validator.validate(&attributes);

        assert!(status.is_failed());
        assert_eq!(status.code_as_int(), UCode::InvalidArgument as i32);
        assert!(status.message().contains("Wrong Attribute Type"));
        assert!(status.message().contains("Missing Sink"));
        assert!(status.message().contains("Missing correlation UUID"));
    }

    #[test]
    fn test_validating_request_containing_hint_and_token() {
        let attributes = UAttributesBuilder::new(
            UUIDv8Factory::new().create(),
            UMessageType::Publish,
            UPriority::Low,
        )
        .with_hint(USerializationHint::Json)
        .with_token("null".to_string())
        .build()
        .unwrap();

        let validator: Box<dyn UAttributesValidator> = Validators::get_validator(&attributes);
        let status = validator.validate(&attributes);

        assert!(status.is_success());
        assert_eq!(status.message(), "ok");
    }
}
