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

use std::fmt;
use std::str::FromStr;

/// Enumeration for the core types of uProtocol CloudEvents.
#[derive(Debug, Eq, PartialEq)]
pub enum UCloudEventType {
    PUBLISH,
    FILE,
    REQUEST,
    RESPONSE,
}

impl fmt::Display for UCloudEventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PUBLISH => write!(f, "pub.v1"),
            Self::FILE => write!(f, "file.v1"),
            Self::REQUEST => write!(f, "req.v1"),
            Self::RESPONSE => write!(f, "res.v1"),
        }
    }
}

/// Convert a `&str` into an `Option<UCloudEventType>`.
///
/// # Arguments
///
/// * `s` - The `&str` value of the `UCloudEventType`.
///
/// # Returns
///
/// * `Option<UCloudEventType>` - Returns the `UCloudEventType` associated with the provided `&str`, or `None` if no such event type exists.
impl FromStr for UCloudEventType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pub.v1" => Ok(Self::PUBLISH),
            "file.v1" => Ok(Self::FILE),
            "req.v1" => Ok(Self::REQUEST),
            "res.v1" => Ok(Self::RESPONSE),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_for_publish() {
        let ucloud_event_type = UCloudEventType::PUBLISH;
        assert_eq!("pub.v1", ucloud_event_type.to_string());
    }

    #[test]
    fn test_type_for_file() {
        let ucloud_event_type = UCloudEventType::FILE;
        assert_eq!("file.v1", ucloud_event_type.to_string());
    }

    #[test]
    fn test_type_for_request() {
        let ucloud_event_type = UCloudEventType::REQUEST;
        assert_eq!("req.v1", ucloud_event_type.to_string());
    }

    #[test]
    fn test_type_for_response() {
        let ucloud_event_type = UCloudEventType::RESPONSE;
        assert_eq!("res.v1", ucloud_event_type.to_string());
    }

    #[test]
    fn test_parse_publish_event_type_from_string() {
        let type_str = "pub.v1";
        assert_eq!(
            UCloudEventType::from_str(type_str),
            Ok(UCloudEventType::PUBLISH)
        );
    }

    #[test]
    fn test_parse_file_event_type_from_string() {
        let type_str = "file.v1";
        assert_eq!(
            UCloudEventType::from_str(type_str),
            Ok(UCloudEventType::FILE)
        );
    }

    #[test]
    fn test_parse_request_event_type_from_string() {
        let type_str = "req.v1";
        assert_eq!(
            UCloudEventType::from_str(type_str),
            Ok(UCloudEventType::REQUEST)
        );
    }

    #[test]
    fn test_parse_response_event_type_from_string() {
        let type_str = "res.v1";
        assert_eq!(
            UCloudEventType::from_str(type_str),
            Ok(UCloudEventType::RESPONSE)
        );
    }

    #[test]
    fn test_parse_unknown_event_type_from_string() {
        let type_str = "unknown.v1";
        assert!(UCloudEventType::from_str(type_str).is_err());
    }
}
