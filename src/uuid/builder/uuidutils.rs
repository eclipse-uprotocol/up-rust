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

use prost::Message;
use std::str::FromStr;
use uuid::{Uuid, Variant};

use crate::uprotocol::Uuid as uproto_Uuid;

#[derive(Debug)]
pub struct UuidConversionError {
    message: String,
}

impl UuidConversionError {
    pub fn new(message: &str) -> UuidConversionError {
        UuidConversionError {
            message: message.to_string(),
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

impl TryFrom<&[u8; 16]> for uproto_Uuid {
    type Error = UuidConversionError;

    fn try_from(bytes: &[u8; 16]) -> Result<Self, Self::Error> {
        Ok(Uuid::from_bytes(*bytes).into())
    }
}

impl TryFrom<String> for uproto_Uuid {
    type Error = UuidConversionError;

    fn try_from(uuid_str: String) -> Result<Self, Self::Error> {
        match Uuid::from_str(&uuid_str) {
            Ok(uuid) => Ok(uuid.into()),
            Err(err) => Err(UuidConversionError::new(&err.to_string())),
        }
    }
}

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
        let mut v = uuid.msb.encode_to_vec();
        v.extend(uuid.lsb.encode_to_vec());
        v
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
        matches!(UuidUtils::get_version(uuid), Some(o) if o == Version::TimeOrdered)
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
                Version::TimeOrdered => Uuid::from(uuid.clone())
                    .get_timestamp()
                    .map(|time| time.to_rfc4122().0),
                Version::Uprotocol => {
                    let uuid = Uuid::from(uuid.clone());
                    // Re-assemble the original msb (u64)
                    let uuid_bytes: &[u8; 16] = uuid.as_bytes();
                    if let Ok(arr) = uuid_bytes[..8].try_into() {
                        let msb = u64::from_le_bytes(arr);
                        let time = msb >> 16;
                        Some(time)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
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
