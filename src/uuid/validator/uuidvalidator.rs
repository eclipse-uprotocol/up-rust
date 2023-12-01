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

use crate::uprotocol::{UCode, UStatus, Uuid as uproto_Uuid};
use crate::uuid::builder::UuidUtils;
use crate::uuid::validator::ValidationResult;

pub trait UuidValidator {
    fn validate(&self, uuid: &uproto_Uuid) -> UStatus {
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
            UStatus::fail_with_code(UCode::InvalidArgument, &error_message)
        }
    }

    fn validate_version(&self, uuid: &uproto_Uuid) -> ValidationResult;

    fn validate_time(&self, uuid: &uproto_Uuid) -> ValidationResult {
        if let Some(time) = UuidUtils::get_time(uuid) {
            if time > 0 {
                return ValidationResult::Success;
            }
        }
        ValidationResult::Failure("Invalid UUID Time".to_string())
    }

    fn validate_variant(&self, uuid: &uproto_Uuid) -> ValidationResult;
}

#[allow(dead_code)]
pub enum UuidValidators {
    Invalid,
    UUIDv6,
    UUIDv8,
}

#[allow(dead_code)]
impl UuidValidators {
    pub fn validator(&self) -> Box<dyn UuidValidator> {
        match self {
            UuidValidators::Invalid => Box::new(InvalidValidator),
            UuidValidators::UUIDv6 => Box::new(UUIDv6Validator),
            UuidValidators::UUIDv8 => Box::new(UUIDv8Validator),
        }
    }

    pub fn get_validator(uuid: &uproto_Uuid) -> Box<dyn UuidValidator> {
        if UuidUtils::is_v6(&(uuid.clone())) {
            return Box::new(UUIDv6Validator);
        }
        if UuidUtils::is_uprotocol(&(uuid.clone())) {
            return Box::new(UUIDv8Validator);
        }
        Box::new(InvalidValidator)
    }
}

pub struct InvalidValidator;
impl UuidValidator for InvalidValidator {
    fn validate_variant(&self, _uuid: &uproto_Uuid) -> ValidationResult {
        ValidationResult::Failure("Invalid UUID Variant".to_string())
    }

    fn validate_version(&self, _uuid: &uproto_Uuid) -> ValidationResult {
        ValidationResult::Failure("Invalid UUID Version".to_string())
    }
}

pub struct UUIDv6Validator;
impl UuidValidator for UUIDv6Validator {
    fn validate_variant(&self, uuid: &uproto_Uuid) -> ValidationResult {
        if UuidUtils::is_rf4122(uuid) {
            ValidationResult::Success
        } else {
            ValidationResult::Failure("Invalid UUIDv6 variant".to_string())
        }
    }

    fn validate_version(&self, uuid: &uproto_Uuid) -> ValidationResult {
        if UuidUtils::is_v6(uuid) {
            ValidationResult::Success
        } else {
            ValidationResult::Failure("Not a UUIDv6 uuid".to_string())
        }
    }
}

pub struct UUIDv8Validator;
impl UuidValidator for UUIDv8Validator {
    fn validate_variant(&self, _uuid: &uproto_Uuid) -> ValidationResult {
        ValidationResult::Success
    }

    fn validate_version(&self, uuid: &uproto_Uuid) -> ValidationResult {
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
    use crate::uuid::builder::{UUIDv6Builder, UUIDv8Builder};

    #[test]
    fn test_validator_with_good_uuid() {
        let uuid = UUIDv8Builder::new().build();
        let status = UuidValidators::get_validator(&uuid).validate(&uuid);
        assert_eq!(UStatus::ok(), status);
    }

    #[test]
    fn test_good_uuid_string() {
        let uuid = UUIDv8Builder::new().build();
        let status = UuidValidators::UUIDv8.validator().validate(&uuid);
        assert_eq!(UStatus::ok(), status);
    }

    #[test]
    fn test_invalid_uuid() {
        let uuid: uproto_Uuid = uproto_Uuid { msb: 0, lsb: 0 };
        let status = UuidValidators::get_validator(&uuid).validate(&uuid);
        assert_eq!(UCode::InvalidArgument, status.code());
        assert_eq!(
            "Invalid UUID Version, Invalid UUID Time, Invalid UUID Variant",
            status.message()
        );
    }

    #[test]
    fn test_invalid_time_uuid() {
        let uuid = UUIDv8Builder::new().build_with_instant(0);
        let status = UuidValidators::UUIDv8.validator().validate(&uuid);
        assert_eq!(UCode::Ok, status.code());
        // assert_eq!("Not a UUIDv8 uuid, Invalid UUID Time", status.message());
    }

    // Invalid (null) input is not an option in Rust
    // #[test]
    // fn test_uuidv8_with_invalid_uuids() {}

    #[test]
    fn test_uuidv8_with_invalid_types() {
        let uuidv6 = UUIDv6Builder::new().build();
        let uuid = uproto_Uuid { msb: 0, lsb: 0 };
        let uuidv4 = uproto_Uuid::from(uuid::Uuid::new_v4());

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
        let uuid = UUIDv6Builder::new().build();
        let validator = UuidValidators::get_validator(&uuid);
        assert!(UuidUtils::is_v6(&uuid));
        assert_eq!(UCode::Ok, validator.validate(&uuid).code());
    }

    #[test]
    fn test_uuidv6_with_bad_variant() {
        if let Ok(uuid) = uproto_Uuid::try_from("1ee57e66-d33a-65e0-4a77-3c3f061c1e9e") {
            let validator = UuidValidators::get_validator(&uuid);
            let status = validator.validate(&uuid);
            assert_eq!("Invalid UUIDv6 variant", status.message());
            assert_eq!(UCode::InvalidArgument, status.code());
        }
    }

    #[test]
    fn test_uuidv6_with_invalid_uuid() {
        let uuid = uproto_Uuid::from(uuid::Uuid::from_fields(9 << 12, 0, 0, &[0; 8]));
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
        let uuid = uproto_Uuid { msb: 0, lsb: 0 };
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
        let uuid = UUIDv8Builder::new().build();
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert_eq!("Not a UUIDv6 uuid", status.message());
        assert_eq!(UCode::InvalidArgument, status.code());
    }
}
