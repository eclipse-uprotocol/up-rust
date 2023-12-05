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

use crate::uprotocol::Uuid as uproto_Uuid;
use crate::uuid::builder::UuidUtils;
use crate::uuid::validator::ValidationError;

/// UUID Validator trait
///
/// Provides methods for validating different aspects of a UUID.
pub trait UuidValidator {
    /// Validates the given UUID.
    ///
    /// # Arguments
    /// * `uuid` - The UUID to validate.
    ///
    /// # Returns
    /// Returns `Ok(())` if all validations pass. Otherwise, returns `Err(ValidationError)` with a concatenated message of all validation errors.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if one or more of the following validations fail:
    /// - Version validation fails as per `validate_version`.
    /// - Time component validation fails as per `validate_time`.
    /// - Variant validation fails as per `validate_variant`.
    /// The error message will contain details of all failed validations, concatenated together.
    fn validate(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        let error_message = vec![
            self.validate_version(uuid),
            self.validate_time(uuid),
            self.validate_variant(uuid),
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

    /// Validates the version of the UUID.
    ///
    /// # Arguments
    /// * `uuid` - The UUID whose version is to be validated.
    ///
    /// # Returns
    /// Returns `Ok(())` if the version is valid. Otherwise, returns `Err(ValidationError)`.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if the UUID's version does not meet the required criteria. The specific requirements are defined in the implementation of this method.
    fn validate_version(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError>;

    /// Validates the time component of the UUID.
    ///
    /// # Arguments
    /// * `uuid` - The UUID whose time component is to be validated.
    ///
    /// # Returns
    /// Returns `Ok(())` if the time component is valid. Otherwise, returns `Err(ValidationError)` with "Invalid UUID Time".
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` with "Invalid UUID Time" if the UUID's time component is invalid or not properly formatted.
    fn validate_time(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        if let Some(time) = UuidUtils::get_time(uuid) {
            if time > 0 {
                return Ok(());
            }
        }
        Err(ValidationError::new("Invalid UUID Time"))
    }

    /// Validates the variant of the UUID.
    ///
    /// # Arguments
    /// * `uuid` - The UUID whose variant is to be validated.
    ///
    /// # Returns
    /// Returns `Ok(())` if the variant is valid. Otherwise, returns `Err(ValidationError)`.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if the UUID's variant does not meet the required criteria. The specific criteria for validation are defined in the implementation of this method.
    fn validate_variant(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError>;
}

/// Enum representing different types of UUID validators.
#[allow(dead_code)]
pub enum UuidValidators {
    Invalid,
    UUIDv6,
    UUIDv8,
}

/// Implementation of methods for `UuidValidators`.
#[allow(dead_code)]
impl UuidValidators {
    /// Returns a `UuidValidator` based on the type of the validator.
    ///
    /// # Returns
    ///
    /// Returns a boxed `UuidValidator`.
    pub fn validator(&self) -> Box<dyn UuidValidator> {
        match self {
            UuidValidators::Invalid => Box::new(InvalidValidator),
            UuidValidators::UUIDv6 => Box::new(UUIDv6Validator),
            UuidValidators::UUIDv8 => Box::new(UUIDv8Validator),
        }
    }

    /// Determines the appropriate `UuidValidator` for a given UUID.
    ///
    /// # Arguments
    ///
    /// * `uuid` - The UUID for which to determine the validator.
    ///
    /// # Returns
    ///
    /// Returns a boxed `UuidValidator` appropriate for the given UUID.
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
    fn validate_variant(&self, _uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        Err(ValidationError::new("Invalid UUID Variant"))
    }

    fn validate_version(&self, _uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        Err(ValidationError::new("Invalid UUID Version"))
    }
}

/// Validator for UUID version 6.
pub struct UUIDv6Validator;

/// `UuidValidator` implementation for `UUIDv6Validator`.
impl UuidValidator for UUIDv6Validator {
    fn validate_variant(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        if UuidUtils::is_rf4122(uuid) {
            Ok(())
        } else {
            Err(ValidationError::new("Invalid UUIDv6 variant"))
        }
    }

    fn validate_version(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        if UuidUtils::is_v6(uuid) {
            Ok(())
        } else {
            Err(ValidationError::new("Not a UUIDv6 uuid"))
        }
    }
}

/// Validator for UUID version 8.
pub struct UUIDv8Validator;

/// `UuidValidator` implementation for `UUIDv8Validator`.
impl UuidValidator for UUIDv8Validator {
    fn validate_variant(&self, _uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        Ok(())
    }

    fn validate_version(&self, uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        if UuidUtils::is_uprotocol(uuid) {
            Ok(())
        } else {
            Err(ValidationError::new("Not a UUIDv8 uuid"))
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
        assert!(status.is_ok());
    }

    #[test]
    fn test_good_uuid_string() {
        let uuid = UUIDv8Builder::new().build();
        let status = UuidValidators::UUIDv8.validator().validate(&uuid);
        assert!(status.is_ok());
    }

    #[test]
    fn test_invalid_uuid() {
        let uuid: uproto_Uuid = uproto_Uuid { msb: 0, lsb: 0 };
        let status = UuidValidators::get_validator(&uuid).validate(&uuid);
        assert!(status.is_err());
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Version"));
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Time"));
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Variant"));
    }

    #[test]
    fn test_invalid_time_uuid() {
        let uuid = UUIDv8Builder::new().build_with_instant(0);
        let status = UuidValidators::UUIDv8.validator().validate(&uuid);
        assert!(status.is_ok());
    }

    #[test]
    fn test_uuidv8_with_invalid_types() {
        let uuidv6 = UUIDv6Builder::new().build();
        let uuid = uproto_Uuid { msb: 0, lsb: 0 };
        let uuidv4 = uproto_Uuid::from(uuid::Uuid::new_v4());

        let validator = UuidValidators::UUIDv8.validator();

        let status = validator.validate(&uuidv6);
        assert!(status.is_err());
        assert_eq!(
            status.as_ref().unwrap_err().to_string(),
            "Not a UUIDv8 uuid"
        );

        let status1 = validator.validate(&uuid);
        assert!(status.is_err());
        assert!(status1
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Not a UUIDv8 uuid"));
        assert!(status1
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Time"));

        let status2 = validator.validate(&uuidv4);
        assert!(status.is_err());
        assert!(status2
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Not a UUIDv8 uuid"));
        assert!(status2
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Time"));
    }

    #[test]
    fn test_good_uuidv6() {
        let uuid = UUIDv6Builder::new().build();
        let validator = UuidValidators::get_validator(&uuid);
        assert!(UuidUtils::is_v6(&uuid));
        let status = validator.validate(&uuid);
        assert!(status.is_ok());
    }

    #[test]
    fn test_uuidv6_with_bad_variant() {
        if let Ok(uuid) = uproto_Uuid::try_from("1ee57e66-d33a-65e0-4a77-3c3f061c1e9e") {
            let validator = UuidValidators::get_validator(&uuid);
            let status = validator.validate(&uuid);
            assert!(status.is_err());
            assert_eq!(status.unwrap_err().to_string(), "Invalid UUIDv6 variant");
        }
    }

    #[test]
    fn test_uuidv6_with_invalid_uuid() {
        let uuid = uproto_Uuid::from(uuid::Uuid::from_fields(9 << 12, 0, 0, &[0; 8]));
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert!(status.is_err());
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Not a UUIDv6 uuid"));
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Time"));
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUIDv6 variant"));
    }

    #[test]
    fn test_uuidv6_with_null_uuid() {
        let uuid = uproto_Uuid { msb: 0, lsb: 0 };
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert!(status.is_err());
        {
            let uuid = uproto_Uuid::from(uuid::Uuid::from_fields(9 << 12, 0, 0, &[0; 8]));
            let validator = UuidValidators::UUIDv6.validator();
            let status = validator.validate(&uuid);
            assert!(status.is_err());
            assert!(status
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("Not a UUIDv6 uuid"));
            assert!(status
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("Invalid UUID Time"));
            assert!(status
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("Invalid UUIDv6 variant"));
        }
    }

    #[test]
    fn test_uuidv6_with_uuidv8() {
        let uuid = UUIDv8Builder::new().build();
        let validator = UuidValidators::UUIDv6.validator();
        let status = validator.validate(&uuid);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Not a UUIDv6 uuid");
    }
}
