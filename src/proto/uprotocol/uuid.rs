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

use std::{hash::Hash, str::FromStr};

use crate::uprotocol::UUID;

const BITMASK_VERSION: u64 = 0b1111 << 12;
pub(crate) const VERSION_CUSTOM: u64 = 0b1000 << 12;
const BITMASK_VARIANT: u64 = 0b11 << 62;
pub(crate) const VARIANT_RFC4122: u64 = 0b10 << 62;

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

impl UUID {
    /// Creates a new [`UUID`] from an existing [`uuid::Uuid`].
    ///
    /// # Returns
    ///
    /// a [`UUID`] having the same bytes as the [`uuid::Uuid`].
    ///
    /// # Errors
    ///
    /// Returns an error if the given UUID has an invalid version and/or variant identifier.
    pub(crate) fn from_uuid(uuid: &uuid::Uuid) -> Result<Self, UuidConversionError> {
        UUID::from_u64_pair(uuid.as_u64_pair().0, uuid.as_u64_pair().1)
    }

    /// Creates a new [`uuid::Uuid`] from this UUID.
    ///
    /// # Returns
    ///
    /// a [`uuid::Uuid`] having the same bytes as this UUID.
    pub(crate) fn to_uuid(&self) -> uuid::Uuid {
        uuid::Uuid::from_u64_pair(self.msb, self.lsb)
    }

    /// Creates a new UUID from a high/low value pair.
    ///
    /// # Arguments
    ///
    /// `msb` - the most significant 8 bytes
    /// `lsb` - the least significant 8 bytes
    ///
    /// # Returns
    ///
    /// a uProtocol [`UUID`] with the given timestamp, counter and random values.
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes contain an invalid version and/or variant identifier.
    pub(crate) fn from_u64_pair(msb: u64, lsb: u64) -> Result<Self, UuidConversionError> {
        if msb & BITMASK_VERSION != VERSION_CUSTOM {
            return Err(UuidConversionError::new("not a v8 UUID"));
        }
        if lsb & BITMASK_VARIANT != VARIANT_RFC4122 {
            return Err(UuidConversionError::new("not an RFC4122 UUID"));
        }
        Ok(UUID {
            msb,
            lsb,
            ..Default::default()
        })
    }

    /// Serializes this UUID to a hyphenated string as defined by
    /// [RFC 4122, Section 3](https://www.rfc-editor.org/rfc/rfc4122.html#section-3).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uprotocol::UUID;
    ///
    /// // timestamp = 1, ver = 0b1000
    /// let msb = 0x0000000000018000_u64;
    /// // variant = 0b10, random = 0x0010101010101010
    /// let lsb = 0x8010101010101010_u64;
    /// let uuid = UUID { msb, lsb, ..Default::default() };
    /// assert_eq!(uuid.to_hyphenated_string(), "00000000-0001-8000-8010-101010101010");
    /// ```
    pub fn to_hyphenated_string(&self) -> String {
        self.to_uuid().as_hyphenated().to_string()
    }

    fn is_custom_version(&self) -> bool {
        self.msb & BITMASK_VERSION == VERSION_CUSTOM
    }

    fn is_rfc_variant(&self) -> bool {
        self.lsb & BITMASK_VARIANT == VARIANT_RFC4122
    }

    /// Returns the point in time that this UUID has been created at.
    ///
    /// # Returns
    ///
    /// The number of milliseconds since UNIX EPOCH if this UUID is a uProtocol UUID,
    /// or [`Option::None`] otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uprotocol::UUID;
    ///
    /// // timestamp = 0x018D548EA8E0 (Monday, 29 January 2024, 9:30:52 AM GMT)
    /// // ver = 0b1000
    /// let msb = 0x018D548EA8E08000u64;
    /// // variant = 0b10
    /// let lsb = 0x8000000000000000u64;
    /// let creation_time = UUID { msb, lsb, ..Default::default() }.get_time();
    /// assert_eq!(creation_time.unwrap(), 0x018D548EA8E0_u64);
    ///
    /// // timestamp = 1, (invalid) ver = 0b1100
    /// let msb = 0x000000000001C000u64;
    /// // variant = 0b10
    /// let lsb = 0x8000000000000000u64;
    /// let creation_time = UUID { msb, lsb, ..Default::default() }.get_time();
    /// assert!(creation_time.is_none());
    /// ```
    pub fn get_time(&self) -> Option<u64> {
        if self.is_uprotocol_uuid() {
            // the timstamp is contained in the 48 most significant bits
            Some(self.msb >> 16)
        } else {
            None
        }
    }

    /// Checks if this is a valid uProtocol UUID.
    ///
    /// # Returns
    ///
    /// `true` if this UUID meets the formal requirements defined by the
    /// [uProtocol spec](https://github.com/eclipse-uprotocol/uprotocol-spec).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uprotocol::UUID;
    ///
    /// // timestamp = 1, ver = 0b1000
    /// let msb = 0x0000000000018000u64;
    /// // variant = 0b10
    /// let lsb = 0x8000000000000000u64;
    /// assert!(UUID { msb, lsb, ..Default::default() }.is_uprotocol_uuid());
    ///
    /// // timestamp = 1, (invalid) ver = 0b1100
    /// let msb = 0x000000000001C000u64;
    /// // variant = 0b10
    /// let lsb = 0x8000000000000000u64;
    /// assert!(!UUID { msb, lsb, ..Default::default() }.is_uprotocol_uuid());
    ///
    /// // timestamp = 1, ver = 0b1000
    /// let msb = 0x0000000000018000u64;
    /// // (invalid) variant = 0b01
    /// let lsb = 0x4000000000000000u64;
    /// assert!(!UUID { msb, lsb, ..Default::default() }.is_uprotocol_uuid());
    /// ```
    pub fn is_uprotocol_uuid(&self) -> bool {
        self.is_custom_version() && self.is_rfc_variant()
    }
}

impl Hash for UUID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let bytes = (self.msb, self.lsb);
        bytes.hash(state)
    }
}

impl From<UUID> for String {
    fn from(value: UUID) -> Self {
        Self::from(&value)
    }
}

impl From<&UUID> for String {
    fn from(value: &UUID) -> Self {
        value.to_hyphenated_string()
    }
}

impl FromStr for UUID {
    type Err = UuidConversionError;

    /// Parses a string into a UUID.
    ///
    /// # Returns
    ///
    /// a uProtocol [`UUID`] based on the bytes encoded in the string.
    ///
    /// # Errors
    ///
    /// Returns an error
    /// * if the given string does not represent a UUID as defined by
    /// [RFC 4122, Section 3](https://www.rfc-editor.org/rfc/rfc4122.html#section-3), or
    /// * if the bytes encoded in the string contain an invalid version and/or variant identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uprotocol::UUID;
    ///
    /// // parsing a valid uProtocol UUID succeeds
    /// let parsing_attempt = "00000000-0001-8000-8010-101010101010".parse::<UUID>();
    /// assert!(parsing_attempt.is_ok());
    /// let uuid = parsing_attempt.unwrap();
    /// assert!(uuid.is_uprotocol_uuid());
    /// assert_eq!(uuid.msb, 0x0000000000018000_u64);
    /// assert_eq!(uuid.lsb, 0x8010101010101010_u64);
    ///
    /// // parsing an invalid UUID fails
    /// assert!("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8"
    ///     .parse::<UUID>()
    ///     .is_err());
    /// ```
    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::from_str(uuid_str)
            .map_err(|err| UuidConversionError::new(err.to_string()))
            .and_then(|uuid| UUID::from_uuid(&uuid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u64_pair() {
        // timestamp = 1, ver = 0b1000
        let msb = 0x0000000000018000u64;
        // variant = 0b10
        let lsb = 0x8000000000000000u64;
        let conversion_attempt = UUID::from_u64_pair(msb, lsb);
        assert!(conversion_attempt.is_ok());
        let uuid = conversion_attempt.unwrap();
        assert!(uuid.is_uprotocol_uuid());
        assert_eq!(uuid.get_time(), Some(0x1_u64));

        // timestamp = 1, (invalid) ver = 0b0000
        let msb = 0x0000000000010000u64;
        // (invalid) variant= 0b00
        let lsb = 0x00000000000000abu64;
        assert!(UUID::from_u64_pair(msb, lsb).is_err());
    }

    #[test]
    fn test_from_uuid() {
        // timestamp = 1, ver = 0b1000
        let hi = 0x0000000000018000_u64;
        // variant = 0b10
        let lo = 0x8000000000000000_u64;
        let other_uuid = uuid::Uuid::from_u64_pair(hi, lo);
        let conversion_attempt = UUID::from_uuid(&other_uuid);
        assert!(conversion_attempt.is_ok());
        let uuid = conversion_attempt.unwrap();
        assert_eq!((uuid.msb, uuid.lsb), other_uuid.as_u64_pair());

        // timestamp = 1, (invalid) ver = 0b0000
        let hi = 0x0000000000010000_u64;
        // (invalid) variant= 0b00
        let lo = 0x00000000000000ab_u64;
        let other_uuid = uuid::Uuid::from_u64_pair(hi, lo);
        assert!(UUID::from_uuid(&other_uuid).is_err());
    }

    #[test]
    fn test_to_uuid() {
        // timestamp = 1, ver = 0b1000
        let msb = 0x0000000000018000_u64;
        // variant = 0b10
        let lsb = 0x8000000000000000_u64;
        let conversion_attempt = UUID::from_u64_pair(msb, lsb);
        assert!(conversion_attempt.is_ok());
        let uuid = conversion_attempt.unwrap();
        assert!(uuid.is_uprotocol_uuid());
        let other_uuid = uuid.to_uuid();
        assert_eq!((uuid.msb, uuid.lsb), other_uuid.as_u64_pair());
    }
}
