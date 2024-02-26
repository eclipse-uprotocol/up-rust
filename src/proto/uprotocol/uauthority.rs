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

use bytes::BufMut;

use crate::uprotocol::uri::uauthority::Number;
use crate::uprotocol::UAuthority;

pub use crate::proto::uprotocol::uuri::SerializationError;
use crate::uri::validator::ValidationError;

const REMOTE_IPV4_BYTES: usize = 4;
const REMOTE_IPV6_BYTES: usize = 16;
const REMOTE_ID_MINIMUM_BYTES: usize = 1;
const REMOTE_ID_MAXIMUM_BYTES: usize = 255;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AddressType {
    Local = 0,
    IPv4 = 1,
    IPv6 = 2,
    ID = 3,
}

impl AddressType {
    pub fn value(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for AddressType {
    type Error = SerializationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AddressType::Local),
            1 => Ok(AddressType::IPv4),
            2 => Ok(AddressType::IPv6),
            3 => Ok(AddressType::ID),
            _ => Err(SerializationError::new(format!(
                "unknown address type ID [{}]",
                value
            ))),
        }
    }
}

impl TryFrom<i32> for AddressType {
    type Error = SerializationError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if let Ok(v) = u8::try_from(value) {
            Self::try_from(v)
        } else {
            Err(SerializationError::new(format!(
                "unknown address type ID [{}]",
                value
            )))
        }
    }
}

impl TryFrom<&UAuthority> for AddressType {
    type Error = SerializationError;

    fn try_from(authority: &UAuthority) -> Result<Self, Self::Error> {
        if authority.has_id() {
            Ok(AddressType::ID)
        } else if authority.has_ip() {
            match authority.ip().len() {
                REMOTE_IPV4_BYTES => Ok(AddressType::IPv4),
                REMOTE_IPV6_BYTES => Ok(AddressType::IPv6),
                _ => return Err(SerializationError::new("Invalid IP address length")),
            }
        } else {
            Ok(AddressType::Local)
        }
    }
}

impl TryFrom<&UAuthority> for Vec<u8> {
    type Error = SerializationError;

    fn try_from(authority: &UAuthority) -> Result<Self, Self::Error> {
        authority
            .validate_micro_form()
            .map_err(|e| SerializationError::new(e.to_string()))?;

        if authority.has_id() {
            let mut buf: Vec<u8> = Vec::new();
            buf.put_u8(authority.id().len() as u8);
            buf.put(authority.id());
            Ok(buf)
        } else if authority.has_ip() {
            Ok(authority.ip().to_vec())
        } else {
            Err(SerializationError::new("No IP or ID in UAuthority"))
        }
    }
}

/// Helper functions to deal with `UAuthority::Remote` structure
impl UAuthority {
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns whether a `UAuthority` satisfies the requirements of a micro form URI
    ///
    /// # Returns
    /// Returns a `Result<(), ValidationError>` where the ValidationError will contain the reasons it failed or OK(())
    /// otherwise
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the failure case
    pub fn validate_micro_form(&self) -> Result<(), ValidationError> {
        let Some(number) = &self.number else {
            return Err(ValidationError::new(
                "Must have IP address or ID set as UAuthority for micro form. Neither are set.",
            ));
        };

        match number {
            Number::Ip(ip) => {
                if !(ip.len() == REMOTE_IPV4_BYTES || ip.len() == REMOTE_IPV6_BYTES) {
                    return Err(ValidationError::new(
                        "IP address is not IPv4 (4 bytes) or IPv6 (16 bytes)",
                    ));
                }
            }
            Number::Id(id) => {
                if !matches!(id.len(), REMOTE_ID_MINIMUM_BYTES..=REMOTE_ID_MAXIMUM_BYTES) {
                    return Err(ValidationError::new("ID doesn't fit in bytes allocated"));
                }
            }
        }
        Ok(())
    }
}
