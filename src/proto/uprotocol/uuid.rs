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
use uuid::{Uuid, Variant, Version};

use crate::uprotocol::uuid::UUID as uproto_Uuid;

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

impl uproto_Uuid {
    pub fn from_u64_pair(high: u64, low: u64) -> Self {
        uproto_Uuid {
            msb: high,
            lsb: low,
            ..Default::default()
        }
    }

    /// Returns a string representation of this UUID as defined by
    /// [RFC 4122, Section 3](https://www.rfc-editor.org/rfc/rfc4122.html#section-3).
    pub fn to_hyphenated_string(&self) -> String {
        Uuid::from(self).as_hyphenated().to_string()
    }

    fn try_get_time(uuid: &uuid::Uuid) -> Result<u64, String> {
        match uuid.get_version() {
            Some(Version::Custom) => {
                // the timstamp is contained in the 48 most significant bits
                let msb = uuid.as_u64_pair().0;
                Ok(msb >> 16)
            }
            _ => Err("not a uProtocol UUID".to_string()),
        }
    }

    /// Returns the point in time that this UUID has been created at.
    ///
    /// # Returns
    ///
    /// The number of milliseconds since UNIX EPOCH if this UUID is a uProtocol UUID, `None` otherwise.
    pub fn get_time(&self) -> Option<u64> {
        let uuid = uuid::Uuid::from(self);
        uproto_Uuid::try_get_time(&uuid).ok()
    }

    /// Checks if this is a valid uProtocol UUID.
    ///
    /// # Returns
    ///
    /// `true` if this UUID meets the formal requirements defined by the
    /// [uProtocol spec](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/v1.5.0/basics/uuid.adoc#2-specification).
    pub fn is_uprotocol_uuid(&self) -> bool {
        let uuid = uuid::Uuid::from(self);

        if !matches!(uuid.get_version(), Some(Version::Custom)) {
            return false;
        }

        if uuid.get_variant() != Variant::RFC4122 {
            return false;
        }

        true
    }
}

impl From<uproto_Uuid> for Uuid {
    fn from(value: uproto_Uuid) -> Self {
        Self::from(&value)
    }
}

impl From<&uproto_Uuid> for Uuid {
    fn from(value: &uproto_Uuid) -> Self {
        Uuid::from_u64_pair(value.msb, value.lsb)
    }
}

impl From<Uuid> for uproto_Uuid {
    fn from(value: Uuid) -> Self {
        Self::from(&value)
    }
}

impl From<&Uuid> for uproto_Uuid {
    fn from(value: &Uuid) -> Self {
        let pair = value.as_u64_pair();
        uproto_Uuid::from_u64_pair(pair.0, pair.1)
    }
}

impl From<uproto_Uuid> for String {
    fn from(value: uproto_Uuid) -> Self {
        Self::from(&value)
    }
}

impl From<&uproto_Uuid> for String {
    fn from(value: &uproto_Uuid) -> Self {
        value.to_hyphenated_string()
    }
}

impl FromStr for uproto_Uuid {
    type Err = UuidConversionError;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        match Uuid::from_str(uuid_str) {
            Ok(uuid) => Ok(uuid.into()),
            Err(err) => Err(UuidConversionError::new(err.to_string())),
        }
    }
}

impl From<uproto_Uuid> for [u8; 16] {
    fn from(value: uproto_Uuid) -> Self {
        Self::from(&value)
    }
}

impl From<&uproto_Uuid> for [u8; 16] {
    fn from(value: &uproto_Uuid) -> Self {
        *Uuid::from(value).as_bytes()
    }
}

impl From<[u8; 16]> for uproto_Uuid {
    fn from(value: [u8; 16]) -> Self {
        Self::from(&value)
    }
}

impl From<&[u8; 16]> for uproto_Uuid {
    fn from(value: &[u8; 16]) -> Self {
        Uuid::from_bytes(*value).into()
    }
}

impl TryFrom<String> for uproto_Uuid {
    type Error = UuidConversionError;

    fn try_from(uuid_str: String) -> Result<Self, Self::Error> {
        uuid_str.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_uuid() {
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;

        let uuid: uproto_Uuid = Uuid::from_u64_pair(hi, lo).into();
        assert_eq!(uuid.msb, hi);
        assert_eq!(uuid.lsb, lo);

        let uuid = uproto_Uuid::from(&Uuid::from_u64_pair(hi, lo));
        assert_eq!(uuid.msb, hi);
        assert_eq!(uuid.lsb, lo);
    }

    #[test]
    fn test_into_uuid() {
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;

        let uuid = Uuid::from(uproto_Uuid::from_u64_pair(hi, lo));
        assert_eq!(uuid.as_u64_pair(), (hi, lo));

        let uuid = Uuid::from(&uproto_Uuid::from_u64_pair(hi, lo));
        assert_eq!(uuid.as_u64_pair(), (hi, lo));
    }

    #[test]
    fn test_into_string() {
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;
        let hyphenated_string = "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8".to_string();

        let uuid_string = String::from(uproto_Uuid::from_u64_pair(hi, lo));
        assert_eq!(uuid_string, hyphenated_string);

        let uuid_string = String::from(&uproto_Uuid::from_u64_pair(hi, lo));
        assert_eq!(uuid_string, hyphenated_string);
    }

    #[test]
    fn test_parse_string() {
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;
        let uuid: uproto_Uuid = "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8".parse().unwrap();
        assert_eq!(uuid.msb, hi);
        assert_eq!(uuid.lsb, lo);
    }

    #[test]
    fn test_from_bytes() {
        let bytes: [u8; 16] = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;

        let uuid = uproto_Uuid::from(bytes);
        assert_eq!(uuid.msb, hi);
        assert_eq!(uuid.lsb, lo);

        let uuid = uproto_Uuid::from(&bytes);
        assert_eq!(uuid.msb, hi);
        assert_eq!(uuid.lsb, lo);
    }

    #[test]
    fn test_into_bytes() {
        let bytes: [u8; 16] = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;
        let uuid = uproto_Uuid::from_u64_pair(hi, lo);

        let uuid_as_bytes: [u8; 16] = (&uuid).into();
        assert_eq!(uuid_as_bytes, bytes);

        let uuid_as_bytes: [u8; 16] = uuid.into();
        assert_eq!(uuid_as_bytes, bytes);
    }

    #[test]
    fn test_is_uprotocol_uuid_succeeds() {
        // timestamp = 1, ver = 0b1000
        let hi = 0x0000000000018000u64;
        // variant = 0b10
        let lo = 0x8000000000000000u64;
        let uuid = uproto_Uuid::from_u64_pair(hi, lo);
        assert!(uuid.is_uprotocol_uuid());
    }

    #[test]
    fn test_is_uprotocol_uuid_fails_for_invalid_version() {
        // timestamp = 1, ver = 0b1100
        let hi = 0x000000000001C000u64;
        // variant = 0b10
        let lo = 0x8000000000000000u64;
        let uuid = uproto_Uuid::from_u64_pair(hi, lo);
        assert!(!uuid.is_uprotocol_uuid());
    }

    #[test]
    fn test_is_uprotocol_uuid_fails_for_invalid_variant() {
        // timestamp = 1, ver = 0b1000
        let hi = 0x0000000000018000u64;
        // variant = 0b01
        let lo = 0x4000000000000000u64;
        let uuid = uproto_Uuid::from_u64_pair(hi, lo);
        assert!(!uuid.is_uprotocol_uuid());
    }
}
