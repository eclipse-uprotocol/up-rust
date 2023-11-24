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

use std::str::FromStr;
use uuid::{Error, Uuid, Variant};

use crate::uprotocol::Uuid as uproto_Uuid;

pub struct UuidUtils;

impl UuidUtils {
    /// Converts the UUID to a String
    ///
    /// # Returns
    ///
    /// * `String` - The String representation of the UUID
    pub fn to_string(uuid: &uproto_Uuid) -> String {
        uuid.to_string()
    }

    /// Converts the UUID to a byte array
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - The byte array representation of the UUID
    pub fn to_bytes(uuid: &uproto_Uuid) -> Vec<u8> {
        Uuid::from(uuid.clone()).as_bytes().to_vec()
    }

    /// Converts a byte array to a UUID
    ///
    /// # Arguments
    ///
    /// * `bytes` - A byte array representing a UUID
    ///
    /// # Returns
    ///
    /// * `Result<Uuid, Error>` - UUID object built from the byte array
    pub fn from_bytes(bytes: &[u8; 16]) -> Result<uproto_Uuid, Error> {
        Ok(Uuid::from_bytes(*bytes).into())
    }

    /// Creates a UUID from a string
    ///
    /// # Arguments
    ///
    /// * `string` - The string representation of the UUID
    ///
    /// # Returns
    ///
    /// * `Result<Uuid, Error>` - The UUID object representation of the string
    pub fn from_string(uuid_str: &str) -> Result<uproto_Uuid, Error> {
        match Uuid::from_str(uuid_str) {
            Ok(uuid) => Ok(uuid.into()),
            Err(err) => Err(err),
        }
    }

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
        Version::from_value(Uuid::from(uuid.clone()).get_version_num())
    }

    /// Verify uuid is either v6 or v8
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `bool` - True if is UUID version 6 or 8
    pub fn is_uuid(uuid: &uproto_Uuid) -> bool {
        UuidUtils::is_uprotocol(uuid) || UuidUtils::is_v6(uuid)
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
        Uuid::from(uuid.clone()).get_variant() == Variant::RFC4122
    }

    /// Verifies if the version is a formal UUIDv8 uProtocol ID
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `bool` - True if the UUID is a formal UUIDv8 uProtocol ID
    pub fn is_uprotocol(uuid: &uproto_Uuid) -> bool {
        matches!(UuidUtils::get_version(uuid), Some(o) if o == Version::VersionUprotocol)
    }

    /// Verifies if the version is UUID version 6
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `bool` - True if the UUID is version 6
    pub fn is_v6(uuid: &uproto_Uuid) -> bool {
        matches!(UuidUtils::get_version(uuid), Some(o) if o == Version::VersionTimeOrdered)
    }

    /// Returns the number of milliseconds since Unix epoch for the provided UUID
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `Option<u64>` - The number of milliseconds since Unix epoch if the UUID version is supported, otherwise None.
    pub fn get_time(uuid: &uproto_Uuid) -> Option<u64> {
        if let Some(version) = UuidUtils::get_version(uuid) {
            match version {
                Version::VersionTimeOrdered => {
                    let time = Uuid::from(uuid.clone())
                        .get_timestamp()
                        .unwrap()
                        .to_rfc4122();
                    Some(time.0)
                }
                Version::VersionUprotocol => {
                    let uuid = Uuid::from(uuid.clone());
                    let uuid_bytes = uuid.as_bytes();
                    // Re-assemble the original msb (u64)
                    let msb = u64::from_le_bytes(uuid_bytes[..8].try_into().unwrap());
                    let time = msb >> 16;
                    Some(time)
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// An unknown version.
    VersionUnknown = 0,
    /// The randomly or pseudo-randomly generated version specified in RFC-4122.
    VersionRandomBased = 4,
    /// The time-ordered version with gregorian epoch proposed by Peabody and Davis.
    VersionTimeOrdered = 6,
    /// The custom or free-form version proposed by Peabody and Davis.
    VersionUprotocol = 8,
}

impl Version {
    /// Get the `Version` from the passed integer representation of the version.
    /// Returns `None` if the value is not a valid version.
    pub fn from_value(value: usize) -> Option<Self> {
        match value {
            0 => Some(Version::VersionUnknown),
            4 => Some(Version::VersionRandomBased),
            6 => Some(Version::VersionTimeOrdered),
            8 => Some(Version::VersionUprotocol),
            _ => None,
        }
    }

    /// Returns the integer representation of the version.
    pub fn value(self) -> usize {
        self as usize
    }
}
