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
use uuid::{Error, Uuid, Variant, Version};

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
    /// * `uuid::Version` - The version of the UUID
    pub fn get_version(uuid: &Uuid) -> Option<Version> {
        uuid.get_version()
    }

    /// Fetches the UUID variant
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `uuid::Variant` - The variant of the UUID
    pub fn get_variant(uuid: &Uuid) -> Variant {
        uuid.get_variant()
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
        matches!(uuid.get_version(), Some(o) if o == Version::Custom)
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
        matches!(uuid.get_version(), Some(o) if o == Version::SortMac)
    }

    /// Returns the number of milliseconds since Unix epoch for the provided UUID
    ///
    /// # Arguments
    ///
    /// * `uuid` - A reference to the UUID object
    ///
    /// # Returns
    ///
    /// * `Result<u64, &'static str>` - The number of milliseconds since Unix epoch if the UUID version is supported, otherwise None.
    pub fn get_time(uuid: &Uuid) -> Option<u64> {
        if let Some(version) = uuid.get_version() {
            match version {
                Version::SortMac => {
                    let time = uuid.get_timestamp().unwrap().to_rfc4122();
                    Some(time.0)
                }
                Version::Custom => {
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
