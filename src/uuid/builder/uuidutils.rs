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

use uuid::{Uuid, Variant};

use crate::uprotocol::Uuid as uproto_Uuid;

#[derive(Debug)]
pub struct UuidConversionError {
    message: String,
}

impl UuidConversionError {
    pub fn new<T: Into<String>>(message: T) -> UuidConversionError {
        UuidConversionError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for UuidConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error converting Uuid: {}", self.message)
    }
}

impl std::error::Error for UuidConversionError {}

pub struct UuidUtils;

impl UuidUtils {
    /// Fetches the UUID version
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `uuid::Version` - The version of the UUID
    pub fn get_version(uuid: &uproto_Uuid) -> Option<Version> {
        Version::from_value(Uuid::from(uuid).get_version_num())
    }

    /// Verify if uuid is of variant RFC4122
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `bool` - True if UUID has variant RFC4122
    pub fn is_rf4122(uuid: &uproto_Uuid) -> bool {
        Uuid::from(uuid).get_variant() == Variant::RFC4122
    }

    /// Verifies if the version is a formal `UUIDv8` uProtocol ID
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `bool` - True if the UUID is a formal `UUIDv8` uProtocol ID
    pub fn is_uprotocol(uuid: &uproto_Uuid) -> bool {
        matches!(UuidUtils::get_version(uuid), Some(o) if o == Version::Uprotocol)
    }

    /// Returns the point in time that a UUID has been created at.
    ///
    /// # Arguments
    ///
    /// * `uuid` - The UUID.
    ///
    /// # Returns
    ///
    /// The number of milliseconds since UNIX EPOCH if the given UUID is a uProtocol UUID, `None` otherwise.
    pub fn get_time(uprotocol_uuid: &uproto_Uuid) -> Option<u64> {
        let uuid = uuid::Uuid::from(uprotocol_uuid);
        match Version::from_value(uuid.get_version_num()) {
            Some(Version::Uprotocol) => {
                // the timstamp is contained in the 48 most significant bits
                let msb = uuid.as_u64_pair().0;
                Some(msb >> 16)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Version {
    /// An unknown version.
    Unknown = 0,
    /// The randomly or pseudo-randomly generated version specified in RFC-4122.
    RandomBased = 4,
    /// The time-ordered version with gregorian epoch proposed by Peabody and Davis.
    TimeOrdered = 6,
    /// The custom or free-form version proposed by Peabody and Davis.
    Uprotocol = 8,
}

impl Version {
    /// Get the `Version` from the passed integer representation of the version.
    /// Returns `None` if the value is not a valid version.
    pub fn from_value(value: usize) -> Option<Self> {
        match value {
            0 => Some(Version::Unknown),
            4 => Some(Version::RandomBased),
            6 => Some(Version::TimeOrdered),
            8 => Some(Version::Uprotocol),
            _ => None,
        }
    }

    /// Returns the integer representation of the version.
    pub fn value(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uuid::builder::UUIDv8Builder;

    #[test]
    fn test_get_time() {
        let instant = 0x18C684468F8u64; // Thu, 14 Dec 2023 12:19:23 GMT
        let uprotocol_uuid = UUIDv8Builder::new().build_with_instant(instant);
        assert_eq!(UuidUtils::get_time(&uprotocol_uuid).unwrap(), instant);
    }

    #[test]
    fn test_is_uprotocol() {
        let uprotocol_uuid = UUIDv8Builder::new().build();
        assert!(UuidUtils::is_uprotocol(&uprotocol_uuid));
        assert!(UuidUtils::is_rf4122(&uprotocol_uuid));
    }

    #[test]
    fn test_get_version() {
        let uprotocol_uuid = UUIDv8Builder::new().build();
        assert_eq!(
            UuidUtils::get_version(&uprotocol_uuid).unwrap(),
            Version::Uprotocol
        );
    }
}
