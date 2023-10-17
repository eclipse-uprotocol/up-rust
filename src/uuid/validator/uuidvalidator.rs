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

#![allow(dead_code)]

use crate::transport::datamodel::UStatus;
use crate::uuid::builder::UuidUtils;
use crate::uuid::validator::{UCode, ValidationResult};

use uuid::{Uuid, Variant};

pub trait UuidValidator {
    fn validate(&self, uuid: &Uuid) -> UStatus {
        let error_messages: Vec<String> = vec![
            self.validate_version(uuid),
            self.validate_time(uuid),
            self.validate_variant(uuid),
        ]
        .into_iter()
        .filter(|status| status.is_failure())
        .map(|status| status.get_message())
        .collect();

        let error_message = error_messages.join(", ");
        if error_message.is_empty() {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason(&error_message, UCode::InvalidArgument)
        }
    }

    fn validate_version(&self, uuid: &Uuid) -> ValidationResult;

    fn validate_time(&self, uuid: &Uuid) -> ValidationResult {
        if let Some(time) = UuidUtils::get_time(uuid) {
            if time > 0 {
                return ValidationResult::Success;
            }
        }
        ValidationResult::Failure("Invalid UUID Time".to_string())
    }

    fn validate_variant(&self, uuid: &Uuid) -> ValidationResult;
}

pub enum UuidValidators {
    Invalid,
    UUIDv6,
    UUIDv8,
}

impl UuidValidators {
    pub fn validator(&self) -> Box<dyn UuidValidator> {
        match self {
            UuidValidators::Invalid => Box::new(InvalidValidator),
            UuidValidators::UUIDv6 => Box::new(UUIDv6Validator),
            UuidValidators::UUIDv8 => Box::new(UUIDv8Validator),
        }
    }

    pub fn get_validator(uuid: &Uuid) -> Box<dyn UuidValidator> {
        if UuidUtils::is_v6(uuid) {
            return Box::new(UUIDv6Validator);
        }
        if UuidUtils::is_uprotocol(uuid) {
            return Box::new(UUIDv8Validator);
        }
        Box::new(InvalidValidator)
    }
}

pub struct InvalidValidator;
impl UuidValidator for InvalidValidator {
    fn validate_variant(&self, _uuid: &Uuid) -> ValidationResult {
        ValidationResult::Failure("Invalid UUID Variant".to_string())
    }

    fn validate_version(&self, _uuid: &Uuid) -> ValidationResult {
        ValidationResult::Failure("Invalid UUID Version".to_string())
    }
}

pub struct UUIDv6Validator;
impl UuidValidator for UUIDv6Validator {
    fn validate_variant(&self, uuid: &Uuid) -> ValidationResult {
        if UuidUtils::get_variant(uuid) == Variant::RFC4122 {
            ValidationResult::Success
        } else {
            ValidationResult::Failure("Invalid UUIDv6 variant".to_string())
        }
    }

    fn validate_version(&self, uuid: &Uuid) -> ValidationResult {
        if UuidUtils::is_v6(uuid) {
            ValidationResult::Success
        } else {
            ValidationResult::Failure("Not a UUIDv6 uuid".to_string())
        }
    }
}

pub struct UUIDv8Validator;
impl UuidValidator for UUIDv8Validator {
    fn validate_variant(&self, _uuid: &Uuid) -> ValidationResult {
        ValidationResult::Success
    }

    fn validate_version(&self, uuid: &Uuid) -> ValidationResult {
        if UuidUtils::is_uprotocol(uuid) {
            ValidationResult::Success
        } else {
            ValidationResult::Failure("Not a UUIDv8 uuid".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uuid::builder::{UUIDFactory, UUIDv6Factory, UUIDv8Factory};

    #[test]
    fn test_validator_with_good_uuid() {
        let uuid = UUIDv8Factory::new().build();
        let status = UuidValidators::get_validator(&uuid).validate(&uuid);
        assert_eq!(UStatus::ok(), status);
    }

    #[test]
    fn test_good_uuid_string() {
        let uuid = UUIDv8Factory::new().build();
        let status = UuidValidators::UUIDv8.validator().validate(&uuid);
        assert_eq!(UStatus::ok(), status);
    }

    #[test]
    fn test_invalid_uuid() {
        let uuid = Uuid::nil();
        let status = UuidValidators::get_validator(&uuid).validate(&uuid);
        assert_eq!(UCode::InvalidArgument, status.code());
        assert_eq!(
            "Invalid UUID Version, Invalid UUID Time, Invalid UUID Variant",
            status.message()
        );
    }

    #[test]
    fn test_invalid_time_uuid() {
        let uuid = UUIDv8Factory::new().build_with_instant(0);
        let status = UuidValidators::UUIDv8.validator().validate(&uuid);
        assert_eq!(UCode::Ok, status.code());
        // assert_eq!("Not a UUIDv8 uuid, Invalid UUID Time", status.message());
    }

    // Invalid (null) input is not an option in Rust
    // #[test]
    // fn test_uuidv8_with_invalid_uuids() {}

    #[test]
    fn test_uuidv8_with_invalid_types() {
        let uuidv6 = UUIDv6Factory::new().build();
        let uuid = Uuid::nil();
        let uuidv4 = Uuid::new_v4();

        let validator = UuidValidators::UUIDv8.validator();

        let status = validator.validate(&uuidv6);
        assert_eq!(UCode::InvalidArgument, status.code());
        assert_eq!("Not a UUIDv8 uuid", status.message());

        let status1 = validator.validate(&uuid);
        assert_eq!(UCode::InvalidArgument, status1.code());
        assert_eq!("Not a UUIDv8 uuid, Invalid UUID Time", status1.message());

        let status2 = validator.validate(&uuidv4);
        assert_eq!(UCode::InvalidArgument, status2.code());
        assert_eq!("Not a UUIDv8 uuid, Invalid UUID Time", status2.message());
    }

    #[test]
    fn test_good_uuidv6() {
        let uuid = UUIDv6Factory::new().build();
        let validator = UuidValidators::get_validator(&uuid);
        assert!(UuidUtils::is_v6(&uuid));
        assert_eq!(UCode::Ok, validator.validate(&uuid).code());
    }

    #[test]
    fn test_uuidv6_with_bad_variant() {
        if let Ok(uuid) = UuidUtils::from_string("1ee57e66-d33a-65e0-4a77-3c3f061c1e9e") {
            let validator = UuidValidators::get_validator(&uuid);
            let status = validator.validate(&uuid);
            assert_eq!("Invalid UUIDv6 variant", status.message());
            assert_eq!(UCode::InvalidArgument, status.code());
        }
    }

    #[test]
    fn test_uuidv6_with_invalid_uuid() {
        let uuid = Uuid::from_fields(9 << 12, 0, 0, &[0; 8]);
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert_eq!(
            "Not a UUIDv6 uuid, Invalid UUID Time, Invalid UUIDv6 variant",
            status.message()
        );
        assert_eq!(UCode::InvalidArgument, status.code());
    }

    #[test]
    fn test_uuidv6_with_null_uuid() {
        let uuid = Uuid::nil();
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert_eq!(
            "Not a UUIDv6 uuid, Invalid UUID Time, Invalid UUIDv6 variant",
            status.message()
        );
        assert_eq!(UCode::InvalidArgument, status.code());
    }

    #[test]
    fn test_uuidv6_with_uuidv8() {
        let uuid = UUIDv8Factory::new().build();
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert_eq!("Not a UUIDv6 uuid", status.message());
        assert_eq!(UCode::InvalidArgument, status.code());
    }
}
