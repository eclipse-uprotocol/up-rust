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

use std::convert::TryFrom;

const PUBLISH: &str = "pub.v1";
const REQUEST: &str = "req.v1";
const RESPONSE: &str = "res.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UMessageType {
    Publish,
    Request,
    Response,
}

impl Default for UMessageType {
    fn default() -> Self {
        Self::Publish
    }
}

impl From<i32> for UMessageType {
    /// Creates a `UMessageType` variant from a numeric value.
    ///
    /// # Arguments
    ///
    /// * `value` - A numeric representation of a message type.
    ///
    /// # Returns
    ///
    /// Returns the `UMessageType` variant corresponding to the numeric value.
    /// Defaults to `UMessageType::Publish` if the value doesn't correspond to any known variant.
    ///
    /// TODO would a try_from be more appropriate here?
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Publish,
            1 => Self::Request,
            2 => Self::Response,
            _ => Self::default(),
        }
    }
}

impl TryFrom<&str> for UMessageType {
    type Error = ();

    /// Tries to create a `UMessageType` variant from a string slice.
    ///
    /// # Arguments
    ///
    /// * `value` - A string slice representation of a message type.
    ///
    /// # Returns
    ///
    /// Returns `Ok` with the `UMessageType` variant if it matches any known variant, otherwise returns `Err`.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            PUBLISH => Ok(Self::Publish),
            REQUEST => Ok(Self::Request),
            RESPONSE => Ok(Self::Response),
            _ => Err(()),
        }
    }
}

impl From<UMessageType> for i32 {
    /// Converts a `UMessageType` into its corresponding numeric value.
    ///
    /// # Arguments
    ///
    /// * `message_type` - An instance of `UMessageType`.
    ///
    /// # Returns
    ///
    /// Returns an `i32` representing the numerical value of the given `UMessageType`.
    fn from(message_type: UMessageType) -> i32 {
        match message_type {
            UMessageType::Publish => 0,
            UMessageType::Request => 1,
            UMessageType::Response => 2,
        }
    }
}

impl From<UMessageType> for &'static str {
    /// Converts a `UMessageType` into its corresponding string representation.
    ///
    /// # Arguments
    ///
    /// * `message_type` - An instance of `UMessageType`.
    ///
    /// # Returns
    ///
    /// Returns a `&'static str` representing the string representation of the given `UMessageType`.
    fn from(message_type: UMessageType) -> &'static str {
        match message_type {
            UMessageType::Publish => PUBLISH,
            UMessageType::Request => REQUEST,
            UMessageType::Response => RESPONSE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_umessage_type_from_number() {
        assert_eq!(UMessageType::from(0), UMessageType::Publish);
        assert_eq!(UMessageType::from(1), UMessageType::Request);
        assert_eq!(UMessageType::from(2), UMessageType::Response);
    }

    #[test]
    fn test_find_umessage_type_from_number_that_does_not_exist() {
        assert_eq!(UMessageType::from(-42), UMessageType::Publish);
    }

    #[test]
    fn test_find_umessage_type_from_string() {
        assert_eq!(
            UMessageType::try_from("pub.v1").unwrap(),
            UMessageType::Publish
        );
        assert_eq!(
            UMessageType::try_from("req.v1").unwrap(),
            UMessageType::Request
        );
        assert_eq!(
            UMessageType::try_from("res.v1").unwrap(),
            UMessageType::Response
        );
    }

    #[test]
    fn test_find_umessage_type_from_string_that_does_not_exist() {
        assert!(UMessageType::try_from("BOOM").is_err());
    }
}
