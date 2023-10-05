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
use uuid::{Error, Uuid};

/// Different versions of UUID
pub enum UuidVersion {
    /// Unknown version
    VersionUnknown,
    /// Time-based version with gregorian epoch specified in RFC-4122
    VersionTimeBased,
    /// DCE Security version, with embedded POSIX UIDs
    VersionDceSecurity,
    /// Name-based version specified in RFC-4122 that uses MD5 hashing
    VersionNameBasedMd5,
    /// Randomly or pseudo-randomly generated version specified in RFC-4122
    VersionRandomBased,
    /// Name-based version specified in RFC-4122 that uses SHA-1 hashing
    VersionNameBasedSha1,
    /// Time-ordered version with gregorian epoch proposed by Peabody and Davis
    VersionTimeOrdered,
    /// Time-ordered version with Unix epoch proposed by Peabody and Davis
    VersionTimeOrderedEpoch,
    /// Custom or free-form version proposed by Peabody and Davis
    VersionUProtocol,
}

impl UuidVersion {
    pub fn get_version(&self) -> u32 {
        match self {
            UuidVersion::VersionUnknown => 0,
            UuidVersion::VersionTimeBased => 1,
            UuidVersion::VersionDceSecurity => 2,
            UuidVersion::VersionNameBasedMd5 => 3,
            UuidVersion::VersionRandomBased => 4,
            UuidVersion::VersionNameBasedSha1 => 5,
            UuidVersion::VersionTimeOrdered => 6,
            UuidVersion::VersionTimeOrderedEpoch => 7,
            UuidVersion::VersionUProtocol => 8,
        }
    }

    pub fn from_version(value: u32) -> Option<UuidVersion> {
        match value {
            0 => Some(UuidVersion::VersionUnknown),
            1 => Some(UuidVersion::VersionTimeBased),
            2 => Some(UuidVersion::VersionDceSecurity),
            3 => Some(UuidVersion::VersionNameBasedMd5),
            4 => Some(UuidVersion::VersionRandomBased),
            5 => Some(UuidVersion::VersionNameBasedSha1),
            6 => Some(UuidVersion::VersionTimeOrdered),
            7 => Some(UuidVersion::VersionTimeOrderedEpoch),
            8 => Some(UuidVersion::VersionUProtocol),
            _ => None,
        }
    }
}

pub struct UuidUtils;

impl UuidUtils {
    /// Converts the UUID to a String
    ///
    /// # Returns
    ///
    /// * `String` - The String representation of the UUID
    pub fn to_string(uuid: &Uuid) -> String {
        uuid.to_string()
    }

    /// Converts the UUID to a byte array
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - The byte array representation of the UUID
    pub fn to_bytes(uuid: &Uuid) -> Vec<u8> {
        uuid.as_bytes().to_vec()
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
    pub fn from_bytes(bytes: &[u8; 16]) -> Result<Uuid, Error> {
        Ok(Uuid::from_bytes(*bytes))
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
    pub fn from_string(uuid_str: &str) -> Result<Uuid, Error> {
        Uuid::from_str(uuid_str)
    }

    /// Fetches the UUID version
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `UuidVersion` - The version of the UUID
    pub fn get_version(uuid: &Uuid) -> UuidVersion {
        let version: usize = uuid.get_version_num();
        UuidVersion::from_version(version as u32).unwrap()
    }

    /// Verifies if the UUID is either version 6 or version 8
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `bool` - True if the UUID is either version 6 or version 8
    pub fn is_valid_uuid(uuid: &Uuid) -> bool {
        UuidUtils::is_v6(uuid) | UuidUtils::is_uprotocol(uuid)
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
    pub fn is_uprotocol(uuid: &Uuid) -> bool {
        let version = UuidUtils::get_version(uuid);
        matches!(version, UuidVersion::VersionUProtocol)
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
    pub fn is_v6(uuid: &Uuid) -> bool {
        let version = UuidUtils::get_version(uuid);
        matches!(version, UuidVersion::VersionTimeOrdered)
    }

    /// Returns the number of milliseconds since Unix epoch for the provided UUID
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `Result<u64, &'static str>` - The number of milliseconds since Unix epoch if the UUID version is supported. Otherwise, returns an error string indicating that the UUID version is unsupported.
    pub fn get_time(uuid: &Uuid) -> Result<u64, &'static str> {
        let version = UuidUtils::get_version(uuid);
        match version {
            UuidVersion::VersionTimeOrdered => {
                let time = uuid.get_timestamp().unwrap().to_rfc4122();
                Ok(time.0)
            }
            UuidVersion::VersionUProtocol => {
                let uuid_bytes = uuid.as_bytes();
                // Re-assemble the original msb (u64)
                let msb = u64::from_le_bytes(uuid_bytes[..8].try_into().unwrap());
                let time = msb >> 16;
                Ok(time)
            }
            _ => Err("Unsupported UUID version"),
        }
    }
}
