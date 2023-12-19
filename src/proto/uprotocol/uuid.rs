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
use uuid::Uuid;

use crate::uprotocol::Uuid as uproto_Uuid;
use crate::uuid::builder::UuidConversionError;

impl uproto_Uuid {
    /// Returns a string representation of this UUID as defined by
    /// [RFC 4122, Section 3](https://www.rfc-editor.org/rfc/rfc4122.html#section-3).
    pub fn to_hyphenated_string(&self) -> String {
        Uuid::from(self).as_hyphenated().to_string()
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
        uproto_Uuid {
            msb: pair.0,
            lsb: pair.1,
        }
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

        let uuid = Uuid::from(uproto_Uuid { msb: hi, lsb: lo });
        assert_eq!(uuid.as_u64_pair(), (hi, lo));

        let uuid = Uuid::from(&uproto_Uuid { msb: hi, lsb: lo });
        assert_eq!(uuid.as_u64_pair(), (hi, lo));
    }

    #[test]
    fn test_into_string() {
        let hi = 0xa1a2a3a4b1b2c1c2u64;
        let lo = 0xd1d2d3d4d5d6d7d8u64;
        let hyphenated_string = "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8".to_string();

        let uuid_string = String::from(uproto_Uuid { msb: hi, lsb: lo });
        assert_eq!(uuid_string, hyphenated_string);

        let uuid_string = String::from(&uproto_Uuid { msb: hi, lsb: lo });
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
        let uuid = uproto_Uuid { msb: hi, lsb: lo };

        let uuid_as_bytes: [u8; 16] = (&uuid).into();
        assert_eq!(uuid_as_bytes, bytes);

        let uuid_as_bytes: [u8; 16] = uuid.into();
        assert_eq!(uuid_as_bytes, bytes);
    }
}
