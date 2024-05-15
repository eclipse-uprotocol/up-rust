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

use std::hash::{Hash, Hasher};
use std::str::FromStr;

pub use crate::up_core_api::uri::UUri;

const WILDCARD_AUTHORITY: &str = "*";
const WILDCARD_ENTITY_ID: u32 = 0x0000_FFFF;
const WILDCARD_ENTITY_VERSION: u32 = 0x0000_00FF;
const WILDCARD_RESOURCE_ID: u32 = 0x0000_FFFF;

const RESOURCE_ID_RESPONSE: u32 = 0;
const RESOURCE_ID_MIN_EVENT: u32 = 0x8000;

#[derive(Debug)]
pub enum UUriError {
    SerializationError(String),
    ValidationError(String),
}

impl UUriError {
    pub fn serialization_error<T>(message: T) -> UUriError
    where
        T: Into<String>,
    {
        Self::SerializationError(message.into())
    }

    pub fn validation_error<T>(message: T) -> UUriError
    where
        T: Into<String>,
    {
        Self::ValidationError(message.into())
    }
}

impl std::fmt::Display for UUriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(e) => f.write_fmt(format_args!("Serialization error: {}", e)),
            Self::ValidationError(e) => f.write_fmt(format_args!("Validation error: {}", e)),
        }
    }
}

impl std::error::Error for UUriError {}

impl From<&UUri> for String {
    /// Serializes a UUri to a URI string.
    ///
    /// # Arguments
    ///
    /// * `uri` - The UUri to serialize.
    ///
    /// # Returns
    ///
    /// The output of [`UUri::to_uri`] without inlcuding the uProtocol scheme.
    fn from(uri: &UUri) -> Self {
        UUri::to_uri(uri, false)
    }
}

impl FromStr for UUri {
    type Err = UUriError;

    /// Attempts to serialize a `String` into a `UUri`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `String` to be converted into a `UUri`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `UUri` representation of the URI or a `SerializationError`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     authority_name: "VIN.vehicles".to_string(),
    ///     ue_id: 0x000A_8000,
    ///     ue_version_major: 0x02,
    ///     resource_id: 0x0000_1a50,
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = UUri::from_str("//VIN.vehicles/A8000/2/1A50").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        if uri.is_empty() {
            return Err(UUriError::serialization_error("URI is empty"));
        }

        let uri_to_parse = uri.find(':').map_or_else(
            || Ok(uri.replace('\\', "/")),
            |_index| {
                // strip leading scheme definition (`up`) up to and including `:`
                uri.strip_prefix("up:")
                    .ok_or(UUriError::serialization_error(
                        "uProtocol URI must use 'up' scheme",
                    ))
                    .map(|s| s.to_string())
                // if uri.starts_with("up:") {
                //     // strip leading scheme definition (`up`) up to and including `:`
                //     Ok(uri[3..].to_string())
                // } else {
                //     Err(UUriError::serialization_error(
                //         "uProtocol URI must use 'up' scheme",
                //     ))
                // }
            },
        )?;
        let is_local: bool = !uri_to_parse.starts_with("//");
        let uri_parts = uri_to_parse.split('/').collect::<Vec<&str>>();

        let mut authority_name: String = String::default();
        #[allow(unused_assignments)]
        let mut entity: String = String::default();
        #[allow(unused_assignments)]
        let mut version: String = String::default();
        #[allow(unused_assignments)]
        let mut resource: String = String::default();

        if is_local {
            if uri_parts.len() != 4 {
                return Err(UUriError::serialization_error(
                    "Local URI must contain entity ID, entity version and resource ID",
                ));
            }
            entity = uri_parts[1].to_string();
            version = uri_parts[2].to_string();
            resource = uri_parts[3].to_string();
        } else if uri_parts.len() != 6 {
            return Err(UUriError::serialization_error(
                "Remote URI must contain authority, entity ID, entity version and resource ID",
            ));
        } else if uri_parts[2].trim().is_empty() {
            return Err(UUriError::serialization_error(
                "Remote URI must contain non-empty authority",
            ));
        } else {
            authority_name = uri_parts[2].to_string();
            entity = uri_parts[3].to_string();
            version = uri_parts[4].to_string();
            resource = uri_parts[5].to_string();
        }
        if entity.is_empty() {
            return Err(UUriError::serialization_error(
                "URI must contain non-empty entity ID",
            ));
        }
        if version.is_empty() {
            return Err(UUriError::serialization_error(
                "URI must contain non-empty entity version",
            ));
        }
        if resource.is_empty() {
            return Err(UUriError::serialization_error(
                "URI must contain non-empty resource ID",
            ));
        }
        let ue_id = u32::from_str_radix(&entity, 16).map_err(|e| {
            UUriError::serialization_error(format!("Cannot parse entity ID: {}", e))
        })?;
        let ue_version_major = u8::from_str_radix(&version, 16).map_err(|e| {
            UUriError::serialization_error(format!("Cannot parse entity version: {}", e))
        })?;
        let resource_id = u16::from_str_radix(&resource, 16).map_err(|e| {
            UUriError::serialization_error(format!("Cannot parse resource ID: {}", e))
        })?;

        Ok(UUri {
            authority_name,
            ue_id,
            ue_version_major: ue_version_major as u32,
            resource_id: resource_id as u32,
            ..Default::default()
        })
    }
}

impl TryFrom<String> for UUri {
    type Error = UUriError;

    /// Attempts to serialize a `String` into a `UUri`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `String` to be converted into a `UUri`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `UUri` representation of the URI or a `SerializationError`.
    fn try_from(uri: String) -> Result<Self, Self::Error> {
        UUri::from_str(uri.as_str())
    }
}

impl TryFrom<&str> for UUri {
    type Error = UUriError;

    /// Attempts to serialize a `String` into a `UUri`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `String` to be converted into a `UUri`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `UUri` representation of the URI or a `SerializationError`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     authority_name: "".to_string(),
    ///     ue_id: 0x001A_8000,
    ///     ue_version_major: 0x02,
    ///     resource_id: 0x0000_1a50,
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = UUri::try_from("/1A8000/2/1A50").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        UUri::from_str(uri)
    }
}

impl Hash for UUri {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.authority_name.hash(state);
        self.ue_id.hash(state);
        self.ue_version_major.hash(state);
        self.resource_id.hash(state);
    }
}

impl Eq for UUri {}

impl UUri {
    /// Serializes this UUri to a URI string.
    ///
    /// # Arguments
    ///
    /// * `include_scheme` - Indicates whether to include the uProtocol scheme (`up`) in the URI.
    ///
    /// # Returns
    ///
    /// The URI as defined by the [uProtocol Specification](https://github.com/eclipse-uprotocol/up-spec).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri {
    ///     authority_name: String::from("VIN.vehicles"),
    ///     ue_id: 0x0000_800A,
    ///     ue_version_major: 0x02,
    ///     resource_id: 0x0000_1a50,
    ///     ..Default::default()
    /// };
    ///
    /// let uri_string = uuri.to_uri(true);
    /// assert_eq!(uri_string, "up://VIN.vehicles/800A/2/1A50");
    /// ````
    pub fn to_uri(&self, include_scheme: bool) -> String {
        let mut output = String::default();
        if include_scheme {
            output.push_str("up:");
        }
        if !self.authority_name.is_empty() {
            output.push_str("//");
            output.push_str(&self.authority_name);
        }
        let uri = format!(
            "/{:X}/{:X}/{:X}",
            self.ue_id, self.ue_version_major, self.resource_id
        );
        output.push_str(&uri);
        output
    }

    /// Verifies that this UUri does not contain any wildcards.
    ///
    /// # Errors
    ///
    /// Returns an error if any of this UUri's properties contain a wildcard value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     authority_name: String::from("VIN.vehicles"),
    ///     ue_id: 0x0000_2310,
    ///     ue_version_major: 0x03,
    ///     resource_id: 0xa000,
    ///     ..Default::default()
    /// };
    /// assert!(uri.verify_no_wildcards().is_ok());
    /// ```
    pub fn verify_no_wildcards(&self) -> Result<(), UUriError> {
        if self.authority_name == WILDCARD_AUTHORITY {
            Err(UUriError::validation_error(format!(
                "Authority must not contain wildcard character [{}]",
                WILDCARD_AUTHORITY
            )))
        } else if self.ue_id & WILDCARD_ENTITY_ID == WILDCARD_ENTITY_ID {
            Err(UUriError::validation_error(format!(
                "Entity ID must not be set to wildcard value [{:#X}]",
                WILDCARD_ENTITY_ID
            )))
        } else if self.ue_version_major == WILDCARD_ENTITY_VERSION {
            Err(UUriError::validation_error(format!(
                "Entity version must not be set to wildcard value [{:#X}]",
                WILDCARD_ENTITY_VERSION
            )))
        } else if self.resource_id == WILDCARD_RESOURCE_ID {
            Err(UUriError::validation_error(format!(
                "Resource ID must not be set to wildcard value [{:#X}]",
                WILDCARD_RESOURCE_ID
            )))
        } else {
            Ok(())
        }
    }

    /// Checks if this UUri refers to a service method.
    ///
    /// Returns `true` if 0 < resource ID < 0x8000.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     resource_id: 0x7FFF,
    ///     ..Default::default()
    /// };
    /// assert!(uri.is_rpc_method());
    /// ```
    pub fn is_rpc_method(&self) -> bool {
        self.resource_id > RESOURCE_ID_RESPONSE && self.resource_id < RESOURCE_ID_MIN_EVENT
    }

    /// Verifies that this UUri refers to a service method.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::is_rpc_method`] fails or
    /// the UUri [contains any wildcards](Self::verify_no_wildcards).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     resource_id: 0x8000,
    ///     ..Default::default()
    /// };
    /// assert!(uri.verify_rpc_method().is_err());
    ///
    /// let uri = UUri {
    ///     resource_id: 0x0,
    ///     ..Default::default()
    /// };
    /// assert!(uri.verify_rpc_method().is_err());
    /// ```
    pub fn verify_rpc_method(&self) -> Result<(), UUriError> {
        if !self.is_rpc_method() {
            Err(UUriError::validation_error(format!(
                "Resource ID must be a value from ]{:#X}, {:#X}[",
                RESOURCE_ID_RESPONSE, RESOURCE_ID_MIN_EVENT
            )))
        } else {
            self.verify_no_wildcards()
        }
    }

    /// Checks if this UUri represents an RPC response address.
    ///
    /// Returns `true` if resource ID is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     resource_id: 0,
    ///     ..Default::default()
    /// };
    /// assert!(uri.is_rpc_response());
    /// ```
    pub fn is_rpc_response(&self) -> bool {
        self.resource_id == RESOURCE_ID_RESPONSE
    }

    /// Verifies that this UUri represents an RPC response address.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::is_rpc_response`] fails or
    /// the UUri [contains any wildcards](Self::verify_no_wildcards).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     resource_id: 0x4001,
    ///     ..Default::default()
    /// };
    /// assert!(uri.verify_rpc_response().is_err());
    /// ```
    pub fn verify_rpc_response(&self) -> Result<(), UUriError> {
        if !self.is_rpc_response() {
            Err(UUriError::validation_error(format!(
                "Resource ID must be {:#X}",
                RESOURCE_ID_RESPONSE
            )))
        } else {
            self.verify_no_wildcards()
        }
    }

    /// Checks if this UUri can be used as the source of an event.
    ///
    /// Returns `true` if resource ID >= 0x8000.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     resource_id: 0x8000,
    ///     ..Default::default()
    /// };
    /// assert!(uri.is_event());
    /// ```
    pub fn is_event(&self) -> bool {
        self.resource_id >= RESOURCE_ID_MIN_EVENT
    }

    /// Verifies that this UUri can be used as the source of an event.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::is_event`] fails or
    /// the UUri [contains any wildcards](Self::verify_no_wildcards).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri {
    ///     resource_id: 0x7FFF,
    ///     ..Default::default()
    /// };
    /// assert!(uri.verify_event().is_err());
    /// ```
    pub fn verify_event(&self) -> Result<(), UUriError> {
        if !self.is_event() {
            Err(UUriError::validation_error(format!(
                "Resource ID must be >= {:#X}",
                RESOURCE_ID_MIN_EVENT
            )))
        } else {
            self.verify_no_wildcards()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    #[test_case(""; "fail for empty string")]
    #[test_case("/"; "fail for no scheme and slash")]
    #[test_case("up:/"; "fail for scheme and slash")]
    #[test_case("//"; "fail for no scheme and double slash")]
    #[test_case("up://"; "fail for scheme and double slash")]
    #[test_case("custom://my-vehicle/8000/2/1"; "fail for unsupported scheme")]
    #[test_case("///8000/2/1"; "fail for no scheme and missing authority")]
    #[test_case("////2/1"; "fail for no scheme, missing authority and entity")]
    #[test_case("/////1"; "fail for no scheme, missing authority, entity and version")]
    fn test_try_from_string_fail(string: &str) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_err());
    }

    #[test_case("up:/8000/1/2",
        UUri {
            authority_name: String::default(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
            ..Default::default()
        };
        "succeed for local service with version and resource")]
    #[test_case("/108000/1/2",
        UUri {
            authority_name: String::default(),
            ue_id: 0x0010_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
            ..Default::default()
        };
        "succeed for local service instance with version and resource")]
    #[test_case("/8000/1/0",
        UUri {
            authority_name: String::default(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0000,
            ..Default::default()
        };
        "succeed for local rpc service response")]
    #[test_case("up://VCU.MY_CAR_VIN/108000/1/2",
        UUri {
            authority_name: "VCU.MY_CAR_VIN".to_string(),
            ue_id: 0x0010_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
            ..Default::default()
        };
        "succeed for remote uri")]
    #[test_case("//*/FFFF/FF/FFFF",
        UUri {
            authority_name: "*".to_string(),
            ue_id: 0x0000_FFFF,
            ue_version_major: 0xFF,
            resource_id: 0xFFFF,
            ..Default::default()
        };
        "succeed for remote uri with wildcards")]
    fn test_try_from_success(uri: &str, expected_uuri: UUri) {
        let parsing_result = UUri::from_str(uri);
        if parsing_result.is_err() {
            println!("error: {}", parsing_result.as_ref().unwrap_err());
        }
        assert!(parsing_result.is_ok());
        let parsed_uuri = parsing_result.unwrap();
        assert_eq!(expected_uuri, parsed_uuri);
    }

    #[test_case("//*/A100/1/1"; "for any authority")]
    #[test_case("//VIN/FFFF/1/1"; "for any entity")]
    #[test_case("//VIN/A100/FF/1"; "for any version")]
    #[test_case("//VIN/A100/1/FFFF"; "for any resource")]
    fn test_verify_no_wildcards_fails(uri: &str) {
        let uuri = UUri::try_from(uri).expect("should have been able to deserialize URI");
        assert!(uuri.verify_no_wildcards().is_err());
    }
}
