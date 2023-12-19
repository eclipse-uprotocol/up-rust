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

use uuid::Variant;

use crate::uprotocol::Uuid as uproto_Uuid;
use crate::uuid::builder::UuidUtils;
use crate::uuid::validator::ValidationError;

/// Validator for uProtocol UUIDs.
pub struct UUIDValidator;

impl UUIDValidator {
    /// Validates the given UUID.
    ///
    /// # Arguments
    /// * `uuid` - The UUID to validate.
    ///
    /// # Returns
    /// Returns `Ok(())` if the given UUID meets the formal requirements defined by the
    /// [uProtocol spec](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/v1.5.0/basics/uuid.adoc#2-specification).
    /// Otherwise, returns `Err(ValidationError)` with a concatenated message of all validation errors.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if one or more of the following validations fail:
    /// - Version validation fails as per `validate_version`.
    /// - Time component validation fails as per `validate_time`.
    /// - Variant validation fails as per `validate_variant`.
    /// The error message will contain details of all failed validations, concatenated together.
    pub fn validate(uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        let error_message = vec![
            Self::validate_version(uuid),
            Self::validate_time(uuid),
            Self::validate_variant(uuid),
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

    /// Validates the time component of a UUID.
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
    pub fn validate_time(uuid: &uproto_Uuid) -> Result<(), ValidationError> {
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
    pub fn validate_variant(uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        let variant = uuid::Uuid::from(uuid).get_variant();
        if variant == Variant::RFC4122 {
            Ok(())
        } else {
            Err(ValidationError::new("UUID has unsupported variant"))
        }
    }

    /// Validates the version of a UUID.
    ///
    /// # Arguments
    /// * `uuid` - The UUID whose version is to be validated.
    ///
    /// # Returns
    /// Returns `Ok(())` if the UUID is version `8`. Otherwise, returns `Err(ValidationError)`.
    pub fn validate_version(uuid: &uproto_Uuid) -> Result<(), ValidationError> {
        if UuidUtils::is_uprotocol(uuid) {
            Ok(())
        } else {
            Err(ValidationError::new("Not a v8 UUID"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uuid::builder::UUIDv8Builder;

    #[test]
    fn test_validator_with_matching_uuid() {
        let uuid = UUIDv8Builder::new().build();
        let status = UUIDValidator::validate(&uuid);
        assert!(status.is_ok());
    }

    #[test]
    fn test_uuid_with_invalid_time() {
        let uuid = UUIDv8Builder::new().build_with_instant(0);
        let status = UUIDValidator::validate_time(&uuid);
        // millis since UNIX epoch must be > 0 (why?)
        assert!(status.is_err());
        let status = UUIDValidator::validate(&uuid);
        assert!(status.is_err());
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID Time"));
    }

    #[test]
    fn test_uuid_with_invalid_type() {
        let uuid = uproto_Uuid::from(uuid::Uuid::new_v4());
        let status = UUIDValidator::validate_version(&uuid);
        assert!(status.is_err());
        let status = UUIDValidator::validate(&uuid);
        assert!(status.is_err());
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Not a v8 UUID"));
    }

    #[test]
    fn test_uuid_with_invalid_variant() {
        let bytes: [u8; 16] = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];
        let uuid_with_wrong_variant = uuid::Builder::from_custom_bytes(bytes)
            .with_variant(uuid::Variant::NCS)
            .into_uuid();
        let uuid = uproto_Uuid::from(uuid_with_wrong_variant);
        let status = UUIDValidator::validate_variant(&uuid);
        assert!(status.is_err());
        let status = UUIDValidator::validate(&uuid);
        assert!(status.is_err());
        assert!(status
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("UUID has unsupported variant"));
    }
}
