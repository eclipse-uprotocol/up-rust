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

use rand::RngCore;
use std::time::{Duration, SystemTime};
use std::{hash::Hash, str::FromStr};

pub use crate::up_core_api::uuid::UUID;

use uuid_simd::{AsciiCase, Out};

const BITMASK_VERSION: u64 = 0b1111 << 12;
const VERSION_7: u64 = 0b0111 << 12;
const BITMASK_VARIANT: u64 = 0b11 << 62;
const VARIANT_RFC4122: u64 = 0b10 << 62;

fn is_correct_version(msb: u64) -> bool {
    msb & BITMASK_VERSION == VERSION_7
}

fn is_correct_variant(lsb: u64) -> bool {
    lsb & BITMASK_VARIANT == VARIANT_RFC4122
}

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
    /// Creates a new UUID from a byte array.
    ///
    /// # Arguments
    ///
    /// `bytes` - the byte array
    ///
    /// # Returns
    ///
    /// a uProtocol [`UUID`] with the given timestamp and random values.
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes contain an invalid version and/or variant identifier.
    pub(crate) fn from_bytes(bytes: &[u8; 16]) -> Result<Self, UuidConversionError> {
        let mut msb = [0_u8; 8];
        let mut lsb = [0_u8; 8];
        msb.copy_from_slice(&bytes[..8]);
        lsb.copy_from_slice(&bytes[8..]);
        Self::from_u64_pair(u64::from_be_bytes(msb), u64::from_be_bytes(lsb))
    }

    /// Creates a new UUID from a high/low value pair.
    ///
    /// NOTE: This function does *not* check if the given bytes represent a [valid uProtocol UUID](Self::is_uprotocol_uuid).
    ///       It should therefore only be used in cases where the bytes passed in are known to be valid.
    ///
    /// # Arguments
    ///
    /// `msb` - the most significant 8 bytes
    /// `lsb` - the least significant 8 bytes
    ///
    /// # Returns
    ///
    /// a uProtocol [`UUID`] with the given timestamp and random values.
    pub(crate) fn from_bytes_unchecked(msb: [u8; 8], lsb: [u8; 8]) -> Self {
        UUID {
            msb: u64::from_be_bytes(msb),
            lsb: u64::from_be_bytes(lsb),
            ..Default::default()
        }
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
    /// a uProtocol [`UUID`] with the given timestamp and random values.
    ///
    /// # Errors
    ///
    /// Returns an error if the given bytes contain an invalid version and/or variant identifier.
    // [impl->dsn~uuid-spec~1]
    pub(crate) fn from_u64_pair(msb: u64, lsb: u64) -> Result<Self, UuidConversionError> {
        if !is_correct_version(msb) {
            return Err(UuidConversionError::new("not a v7 UUID"));
        }
        if !is_correct_variant(lsb) {
            return Err(UuidConversionError::new("not an RFC4122 UUID"));
        }
        Ok(UUID {
            msb,
            lsb,
            ..Default::default()
        })
    }

    // [impl->dsn~uuid-spec~1]
    pub(crate) fn build_for_timestamp(duration_since_unix_epoch: Duration) -> UUID {
        let timestamp_millis = u64::try_from(duration_since_unix_epoch.as_millis())
            .expect("system time is set to a time too far in the future");
        // fill upper 48 bits with timestamp
        let mut msb = (timestamp_millis << 16).to_be_bytes();
        // fill remaining bits with random bits
        rand::rng().fill_bytes(&mut msb[6..]);
        // set version (7)
        msb[6] = msb[6] & 0b00001111 | 0b01110000;

        let mut lsb = [0u8; 8];
        // fill lsb with random bits
        rand::rng().fill_bytes(&mut lsb);
        // set variant (RFC4122)
        lsb[0] = lsb[0] & 0b00111111 | 0b10000000;
        Self::from_bytes_unchecked(msb, lsb)
    }

    /// Creates a new UUID that can be used for uProtocol messages.
    ///
    /// # Panics
    ///
    /// if the system clock is set to an instant before the UNIX Epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use up_rust::UUID;
    ///
    /// let uuid = UUID::build();
    /// assert!(uuid.is_uprotocol_uuid());
    /// ```
    // [impl->dsn~uuid-spec~1]
    // [utest->dsn~uuid-spec~1]
    pub fn build() -> UUID {
        let duration_since_unix_epoch = SystemTime::UNIX_EPOCH
            .elapsed()
            .expect("current system time is set to a point in time before UNIX Epoch");
        Self::build_for_timestamp(duration_since_unix_epoch)
    }

    /// Serializes this UUID to a hyphenated string as defined by
    /// [RFC 4122, Section 3](https://www.rfc-editor.org/rfc/rfc4122.html#section-3)
    /// using lower case characters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUID;
    ///
    /// // timestamp = 1, ver = 0b0111
    /// let msb = 0x0000000000017000_u64;
    /// // variant = 0b10, random = 0x0010101010101a1a
    /// let lsb = 0x8010101010101a1a_u64;
    /// let uuid = UUID { msb, lsb, ..Default::default() };
    /// assert_eq!(uuid.to_hyphenated_string(), "00000000-0001-7000-8010-101010101a1a");
    /// ```
    // [impl->req~uuid-hex-and-dash~1]
    pub fn to_hyphenated_string(&self) -> String {
        let mut bytes = [0_u8; 16];
        bytes[..8].clone_from_slice(self.msb.to_be_bytes().as_slice());
        bytes[8..].clone_from_slice(self.lsb.to_be_bytes().as_slice());
        let mut out_bytes = [0_u8; 36];
        let out =
            uuid_simd::format_hyphenated(&bytes, Out::from_mut(&mut out_bytes), AsciiCase::Lower);
        String::from_utf8(out.to_vec()).unwrap()
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
    /// use up_rust::UUID;
    ///
    /// // timestamp = 0x018D548EA8E0 (Monday, 29 January 2024, 9:30:52 AM GMT)
    /// // ver = 0b0111
    /// let msb = 0x018D548EA8E07000u64;
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
    // [impl->dsn~uuid-spec~1]
    // [utest->dsn~uuid-spec~1]
    pub fn get_time(&self) -> Option<u64> {
        if self.is_uprotocol_uuid() {
            // the timestamp is contained in the 48 most significant bits
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
    /// use up_rust::UUID;
    ///
    /// // timestamp = 1, ver = 0b0111
    /// let msb = 0x0000000000017000u64;
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
    /// // timestamp = 1, ver = 0b0111
    /// let msb = 0x0000000000017000u64;
    /// // (invalid) variant = 0b01
    /// let lsb = 0x4000000000000000u64;
    /// assert!(!UUID { msb, lsb, ..Default::default() }.is_uprotocol_uuid());
    /// ```
    // [impl->dsn~uuid-spec~1]
    // [utest->dsn~uuid-spec~1]
    pub fn is_uprotocol_uuid(&self) -> bool {
        is_correct_version(self.msb) && is_correct_variant(self.lsb)
    }
}

impl Eq for UUID {}

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
    ///   [RFC 4122, Section 3](https://www.rfc-editor.org/rfc/rfc4122.html#section-3), or
    /// * if the bytes encoded in the string contain an invalid version and/or variant identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUID;
    ///
    /// // parsing a valid uProtocol UUID succeeds
    /// let parsing_attempt = "00000000-0001-7000-8010-101010101a1A".parse::<UUID>();
    /// assert!(parsing_attempt.is_ok());
    /// let uuid = parsing_attempt.unwrap();
    /// assert!(uuid.is_uprotocol_uuid());
    /// assert_eq!(uuid.msb, 0x0000000000017000_u64);
    /// assert_eq!(uuid.lsb, 0x8010101010101a1a_u64);
    ///
    /// // parsing an invalid UUID fails
    /// assert!("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8"
    ///     .parse::<UUID>()
    ///     .is_err());
    ///
    /// // parsing a string that is not a UUID fails
    /// assert!("this-is-not-a-UUID"
    ///     .parse::<UUID>()
    ///     .is_err());
    /// ```
    // [impl->req~uuid-hex-and-dash~1]
    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        let mut uuid = [0u8; 16];
        uuid_simd::parse_hyphenated(uuid_str.as_bytes(), Out::from_mut(&mut uuid))
            .map_err(|err| UuidConversionError::new(err.to_string()))
            .and_then(|bytes| UUID::from_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    // [utest->dsn~uuid-spec~1]
    // [utest->req~uuid-type~1]
    #[test]
    fn test_from_u64_pair() {
        // timestamp = 1, ver = 0b0111
        let msb = 0x0000000000017000_u64;
        // variant = 0b10
        let lsb = 0x8000000000000000_u64;
        let conversion_attempt = UUID::from_u64_pair(msb, lsb);
        assert!(conversion_attempt.is_ok_and(|uuid| {
            uuid.is_uprotocol_uuid()
                && uuid.get_time() == Some(0x1_u64)
                && uuid.msb == msb
                && uuid.lsb == lsb
        }));

        // timestamp = 1, (invalid) ver = 0b0000
        let msb = 0x0000000000010000_u64;
        // variant= 0b10
        let lsb = 0x80000000000000ab_u64;
        assert!(UUID::from_u64_pair(msb, lsb).is_err());

        // timestamp = 1, ver = 0b0111
        let msb = 0x0000000000017000_u64;
        // (invalid) variant= 0b00
        let lsb = 0x00000000000000ab_u64;
        assert!(UUID::from_u64_pair(msb, lsb).is_err());
    }

    // [utest->dsn~uuid-spec~1]
    #[test]
    fn test_from_bytes() {
        // timestamp = 1, ver = 0b0111, variant = 0b10
        let bytes: [u8; 16] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let conversion_attempt = UUID::from_bytes(&bytes);
        assert!(conversion_attempt.is_ok());
        let uuid = conversion_attempt.unwrap();
        assert!(uuid.is_uprotocol_uuid());
        assert_eq!(uuid.get_time(), Some(0x1_u64));
    }

    #[test]
    // [utest->req~uuid-hex-and-dash~1]
    fn test_into_string() {
        // timestamp = 1, ver = 0b0111
        let msb = 0x0000000000017000_u64;
        // variant = 0b10, random = 0x0010101010101a1a
        let lsb = 0x8010101010101a1a_u64;
        let uuid = UUID {
            msb,
            lsb,
            ..Default::default()
        };

        assert_eq!(String::from(&uuid), "00000000-0001-7000-8010-101010101a1a");
        assert_eq!(String::from(uuid), "00000000-0001-7000-8010-101010101a1a");
    }
}
