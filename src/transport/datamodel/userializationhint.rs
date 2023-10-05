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
use std::fmt;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum USerializationHint {
    #[default]
    Unknown,
    Protobuf,
    Json,
    SomeIp,
    Raw,
}

impl USerializationHint {
    // Serialization hint is unknown
    const UNKNOWN: &'static str = "";

    // serialized com.google.protobuf.Any type
    const PROTOBUF: &'static str = "application/x-protobuf";

    // data is a UTF-8 string containing a JSON structure
    const JSON: &'static str = "application/json";

    // data is SOME/IP something
    const SOMEIP: &'static str = "application/x-someip";

    // Raw binary data that has not been serialized
    const RAW: &'static str = "application/octet-stream";

    pub fn hint_number(&self) -> i32 {
        match self {
            Self::Unknown => 0,
            Self::Protobuf => 1,
            Self::Json => 2,
            Self::SomeIp => 3,
            Self::Raw => 4,
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Unknown => Self::UNKNOWN,
            Self::Protobuf => Self::PROTOBUF,
            Self::Json => Self::JSON,
            Self::SomeIp => Self::SOMEIP,
            Self::Raw => Self::RAW,
        }
    }

    fn all_hints() -> Vec<Self> {
        vec![
            Self::Unknown,
            Self::Protobuf,
            Self::Json,
            Self::SomeIp,
            Self::Raw,
        ]
    }
}

impl fmt::Display for USerializationHint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.mime_type())
    }
}

impl From<i32> for USerializationHint {
    /// Create a [`USerializationHint`] from the given numeric value.
    /// If there is no matching `USerializationHint`, the default (`Unknown`) is returned.
    ///
    /// # Arguments
    ///
    /// * `value` - A 32-bit integer that holds the hint number to match.
    ///
    /// # Returns
    ///
    /// Returns the [`USerializationHint`] matching the numeric value or the default if not found.
    fn from(value: i32) -> Self {
        Self::all_hints()
            .into_iter()
            .find(|hint| hint.hint_number() == value)
            .unwrap_or_default()
    }
}

impl TryFrom<&str> for USerializationHint {
    type Error = ();

    /// Try to create a [`USerializationHint`] from the given string value.
    ///
    /// # Arguments
    ///
    /// * `value` - A string slice that holds the MIME type to match.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the [`USerializationHint`] matching the given string value,
    /// or a `None` wrapped in an `Err` if no match is found.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::all_hints()
            .into_iter()
            .find(|hint| hint.mime_type() == value)
            .ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_userializationhint_from_number() {
        assert_eq!(USerializationHint::from(0), USerializationHint::Unknown);
        assert_eq!(USerializationHint::from(1), USerializationHint::Protobuf);
        assert_eq!(USerializationHint::from(2), USerializationHint::Json);
        assert_eq!(USerializationHint::from(3), USerializationHint::SomeIp);
        assert_eq!(USerializationHint::from(4), USerializationHint::Raw);
    }

    #[test]
    fn test_find_userializationhint_from_number_that_does_not_exist() {
        assert_eq!(USerializationHint::from(-42), USerializationHint::Unknown);
    }

    #[test]
    fn test_find_userializationhint_from_string() {
        assert_eq!(
            USerializationHint::try_from("").unwrap(),
            USerializationHint::Unknown
        );
        assert_eq!(
            USerializationHint::try_from("application/x-protobuf").unwrap(),
            USerializationHint::Protobuf
        );
        assert_eq!(
            USerializationHint::try_from("application/json").unwrap(),
            USerializationHint::Json
        );
        assert_eq!(
            USerializationHint::try_from("application/x-someip").unwrap(),
            USerializationHint::SomeIp
        );
        assert_eq!(
            USerializationHint::try_from("application/octet-stream").unwrap(),
            USerializationHint::Raw
        );
    }

    #[test]
    fn test_find_userializationhint_from_string_that_does_not_exist() {
        assert!(USerializationHint::try_from("BOOM").is_err());
    }
}
