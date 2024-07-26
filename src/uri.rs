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

// [impl->dsn~uri-data-model-naming~1]
// [impl->req~uri-data-model-proto~1]

use std::hash::{Hash, Hasher};
use std::str::FromStr;

use uriparse::{Authority, URIReference};

pub use crate::up_core_api::uri::UUri;

pub(crate) const WILDCARD_AUTHORITY: &str = "*";
pub(crate) const WILDCARD_ENTITY_ID: u32 = 0x0000_FFFF;
pub(crate) const WILDCARD_ENTITY_VERSION: u32 = 0x0000_00FF;
pub(crate) const WILDCARD_RESOURCE_ID: u32 = 0x0000_FFFF;

pub(crate) const RESOURCE_ID_RESPONSE: u32 = 0;
pub(crate) const RESOURCE_ID_MIN_EVENT: u32 = 0x8000;

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

// [impl->req~uri-serialization~1]
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

    /// Attempts to parse a `String` into a `UUri`.
    ///
    /// As part of the parsing, the _authority_ of the URI is getting normalized. This means that all characters
    /// are converted to lowercase, no bytes that are in the unreserved character set remain percent-encoded,
    /// and all alphabetical characters in percent-encodings are converted to uppercase.
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
    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-scheme~1]
    // [impl->dsn~uri-host-only~2]
    // [impl->dsn~uri-authority-mapping~1]
    // [impl->dsn~uri-path-mapping~1]
    // [impl->req~uri-serialization~1]
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        if uri.is_empty() {
            return Err(UUriError::serialization_error("URI is empty"));
        }
        let parsed_uri = URIReference::try_from(uri)
            .map_err(|e| UUriError::serialization_error(e.to_string()))?;

        if let Some(scheme) = parsed_uri.scheme() {
            if scheme.ne("up") {
                return Err(UUriError::serialization_error(
                    "uProtocol URI must use 'up' scheme",
                ));
            }
        }
        if parsed_uri.has_query() {
            return Err(UUriError::serialization_error(
                "uProtocol URI must not contain query",
            ));
        }
        if parsed_uri.has_fragment() {
            return Err(UUriError::serialization_error(
                "uProtocol URI must not contain fragment",
            ));
        }
        let authority_name = parsed_uri
            .authority()
            .map_or(Ok(String::default()), |auth| {
                if auth.has_port() {
                    Err(UUriError::serialization_error(
                        "uProtocol URI's authority must not contain port",
                    ))
                } else if auth.has_username() || auth.has_password() {
                    Err(UUriError::serialization_error(
                        "uProtocol URI's authority must not contain userinfo",
                    ))
                } else {
                    let auth_name = auth.host().to_string();
                    if auth_name.len() <= 128 {
                        Ok(auth_name)
                    } else {
                        Err(UUriError::serialization_error(
                            "URI's authority name must not exceed 128 characters",
                        ))
                    }
                }
            })?;

        let path_segments = parsed_uri.path().segments();
        if path_segments.len() != 3 {
            return Err(UUriError::serialization_error(
                "uProtocol URI must contain entity ID, entity version and resource ID",
            ));
        }
        let entity = path_segments[0].as_str();
        if entity.is_empty() {
            return Err(UUriError::serialization_error(
                "URI must contain non-empty entity ID",
            ));
        }
        let ue_id = u32::from_str_radix(entity, 16).map_err(|e| {
            UUriError::serialization_error(format!("Cannot parse entity ID: {}", e))
        })?;
        let version = path_segments[1].as_str();
        if version.is_empty() {
            return Err(UUriError::serialization_error(
                "URI must contain non-empty entity version",
            ));
        }
        let ue_version_major = u8::from_str_radix(version, 16).map_err(|e| {
            UUriError::serialization_error(format!("Cannot parse entity version: {}", e))
        })?;
        let resource = path_segments[2].as_str();
        if resource.is_empty() {
            return Err(UUriError::serialization_error(
                "URI must contain non-empty resource ID",
            ));
        }
        let resource_id = u16::from_str_radix(resource, 16).map_err(|e| {
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

// [impl->req~uri-serialization~1]
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

// [impl->req~uri-serialization~1]
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

    /// Creates a new UUri from its parts.
    ///
    /// # Errors
    ///
    /// Returns a [`UUriError::ValidationError`] if the authority does not comply with the UUri specification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// assert!(UUri::try_from_parts("vin", 0x5a6b, 0x01, 0x0001).is_ok());
    /// ```
    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-host-only~2]
    pub fn try_from_parts(
        authority: &str,
        entity_id: u32,
        entity_version: u8,
        resource_id: u16,
    ) -> Result<Self, UUriError> {
        let auth = Authority::try_from(authority)
            .map_err(|e| UUriError::validation_error(format!("invalid authority: {}", e)))
            .and_then(|auth| {
                if auth.has_port() {
                    Err(UUriError::validation_error(
                        "uProtocol URI's authority must not contain port",
                    ))
                } else if auth.has_username() || auth.has_password() {
                    Err(UUriError::validation_error(
                        "uProtocol URI's authority must not contain userinfo",
                    ))
                } else {
                    let auth_name = auth.host().to_string();
                    if auth_name.len() <= 128 {
                        Ok(auth)
                    } else {
                        Err(UUriError::validation_error(
                            "URI's authority name must not exceed 128 characters",
                        ))
                    }
                }
            })?;
        Ok(UUri {
            authority_name: auth.host().to_string(),
            ue_id: entity_id,
            ue_version_major: entity_version as u32,
            resource_id: resource_id as u32,
            ..Default::default()
        })
    }

    /// Gets a URI that consists of wildcards only and therefore matches any URI.
    pub fn any() -> Self {
        Self::any_with_resource_id(WILDCARD_RESOURCE_ID)
    }

    /// Gets a URI that consists of wildcards and the specific resource ID.
    pub fn any_with_resource_id(resource_id: u32) -> Self {
        UUri {
            authority_name: WILDCARD_AUTHORITY.to_string(),
            ue_id: WILDCARD_ENTITY_ID,
            ue_version_major: WILDCARD_ENTITY_VERSION,
            resource_id,
            ..Default::default()
        }
    }

    /// Checks if this URI is empty.
    ///
    /// # Returns
    ///
    /// 'true' if this URI is equal to `UUri::default()`, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.eq(&UUri::default())
    }

    /// Check if an `UUri` is remote, by comparing authority fields. UUris with empty authority are
    /// considered to be local.
    ///
    /// # Returns
    ///
    /// 'true' if other_uri has a different authority than `Self`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use up_rust::UUri;
    ///
    /// let authority_a = UUri::from_str("up://Authority.A/100A/1/0").unwrap();
    /// let authority_b = UUri::from_str("up://Authority.B/200B/2/20").unwrap();
    /// assert!(authority_a.is_remote_authority(&authority_b));
    ///
    /// let authority_local = UUri::from_str("up:///100A/1/0").unwrap();
    /// assert!(!authority_local.is_remote_authority(&authority_a));
    ///
    /// let authority_wildcard = UUri::from_str("up://*/100A/1/0").unwrap();
    /// assert!(!authority_wildcard.is_remote_authority(&authority_a));
    /// assert!(!authority_a.is_remote_authority(&authority_wildcard));
    /// assert!(!authority_wildcard.is_remote_authority(&authority_wildcard));
    /// ````
    pub fn is_remote_authority(&self, other_uri: &UUri) -> bool {
        !self.authority_name.is_empty()
            && self.authority_name != WILDCARD_AUTHORITY
            && other_uri.authority_name != WILDCARD_AUTHORITY
            && self.authority_name != other_uri.authority_name
    }

    /// Checks if this UUri has an empty authority name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("", 0x9b3a, 0x01, 0x145b).unwrap();
    /// assert!(uuri.has_empty_authority());
    /// ```
    pub fn has_empty_authority(&self) -> bool {
        self.authority_name.is_empty()
    }

    /// Checks if this UUri has a wildcard authority name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("*", 0x9b3a, 0x01, 0x145b).unwrap();
    /// assert!(uuri.has_wildcard_authority());
    /// ```
    pub fn has_wildcard_authority(&self) -> bool {
        self.authority_name == WILDCARD_AUTHORITY
    }

    /// Checks if this UUri has a wildcard entity identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("vin", 0xFFFF, 0x01, 0x145b).unwrap();
    /// assert!(uuri.has_wildcard_entity_id());
    /// ```
    pub fn has_wildcard_entity_id(&self) -> bool {
        self.ue_id & WILDCARD_ENTITY_ID == WILDCARD_ENTITY_ID
    }

    /// Checks if this UUri has a wildcard major version.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("vin", 0x9b3a, 0xFF, 0x145b).unwrap();
    /// assert!(uuri.has_wildcard_version());
    /// ```
    pub fn has_wildcard_version(&self) -> bool {
        self.ue_version_major == WILDCARD_ENTITY_VERSION
    }

    /// Checks if this UUri has a wildcard entity identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("vin", 0x9b3a, 0x01, 0xFFFF).unwrap();
    /// assert!(uuri.has_wildcard_resource_id());
    /// ```
    pub fn has_wildcard_resource_id(&self) -> bool {
        self.resource_id == WILDCARD_RESOURCE_ID
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
        if self.has_wildcard_authority() {
            Err(UUriError::validation_error(format!(
                "Authority must not contain wildcard character [{}]",
                WILDCARD_AUTHORITY
            )))
        } else if self.has_wildcard_entity_id() {
            Err(UUriError::validation_error(format!(
                "Entity ID must not be set to wildcard value [{:#X}]",
                WILDCARD_ENTITY_ID
            )))
        } else if self.has_wildcard_version() {
            Err(UUriError::validation_error(format!(
                "Entity version must not be set to wildcard value [{:#X}]",
                WILDCARD_ENTITY_VERSION
            )))
        } else if self.has_wildcard_resource_id() {
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

    /// Checks if this UUri represents a destination for a Notification.
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
    /// assert!(uri.is_notification_destination());
    /// ```
    pub fn is_notification_destination(&self) -> bool {
        self.resource_id == RESOURCE_ID_RESPONSE
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

    fn matches_authority(&self, candidate: &UUri) -> bool {
        self.authority_name == WILDCARD_AUTHORITY || self.authority_name == candidate.authority_name
    }

    fn matches_entity_type(&self, candidate: &UUri) -> bool {
        self.ue_id & WILDCARD_ENTITY_ID == WILDCARD_ENTITY_ID
            || self.ue_id & WILDCARD_ENTITY_ID == candidate.ue_id & WILDCARD_ENTITY_ID
    }

    fn matches_entity_instance(&self, candidate: &UUri) -> bool {
        self.ue_id & 0xFFFF_0000 == 0x0000_0000
            || self.ue_id & 0xFFFF_0000 == candidate.ue_id & 0xFFFF_0000
    }

    fn matches_entity_version(&self, candidate: &UUri) -> bool {
        self.ue_version_major == WILDCARD_ENTITY_VERSION
            || self.ue_version_major == candidate.ue_version_major
    }

    fn matches_entity(&self, candidate: &UUri) -> bool {
        self.matches_entity_type(candidate)
            && self.matches_entity_instance(candidate)
            && self.matches_entity_version(candidate)
    }

    fn matches_resource(&self, candidate: &UUri) -> bool {
        self.resource_id == WILDCARD_RESOURCE_ID || self.resource_id == candidate.resource_id
    }

    /// Checks if a given candidate URI matches a pattern.
    ///
    /// # Returns
    ///
    /// `true` if the candiadate matches the pattern represented by this UUri.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let pattern = UUri::try_from("//VIN/A14F/3/FFFF").unwrap();
    /// let candidate = UUri::try_from("//VIN/A14F/3/B1D4").unwrap();
    /// assert!(pattern.matches(&candidate));
    /// ```
    // [impl->dsn~uri-pattern-matching~1]
    pub fn matches(&self, candidate: &UUri) -> bool {
        self.matches_authority(candidate)
            && self.matches_entity(candidate)
            && self.matches_resource(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::Message;
    use test_case::test_case;

    // [utest->req~uri-serialization~1]
    // [utest->dsn~uri-scheme~1]
    // [utest->dsn~uri-host-only~2]
    // [utest->dsn~uri-authority-mapping~1]
    // [utest->dsn~uri-path-mapping~1]
    #[test_case(""; "for empty string")]
    #[test_case("/"; "for single slash")]
    #[test_case("up:/"; "for scheme and single slash")]
    #[test_case("//"; "for double slash")]
    #[test_case("up://"; "for scheme and double slash")]
    #[test_case("custom://my-vehicle/8000/2/1"; "for unsupported scheme")]
    #[test_case("////2/1"; "for missing authority and entity")]
    #[test_case("/////1"; "for missing authority, entity and version")]
    #[test_case("up://MYVIN/1A23/1/a13?foo=bar"; "for URI with query")]
    #[test_case("up://MYVIN/1A23/1/a13#foobar"; "for URI with fragement")]
    #[test_case("up://MYVIN:1000/1A23/1/A13"; "for authority with port")]
    #[test_case("up://user:pwd@MYVIN/1A23/1/A13"; "for authority with userinfo")]
    fn test_from_string_fails(string: &str) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_err());
    }

    // [utest->req~uri-serialization~1]
    // [utest->dsn~uri-scheme~1]
    // [utest->dsn~uri-host-only~2]
    // [utest->dsn~uri-authority-mapping~1]
    // [utest->dsn~uri-path-mapping~1]
    #[test_case("UP:/8000/1/2",
        UUri {
            authority_name: String::default(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
            ..Default::default()
        };
        "for local service with version and resource")]
    #[test_case("/108000/1/2",
        UUri {
            authority_name: String::default(),
            ue_id: 0x0010_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
            ..Default::default()
        };
        "for local service instance with version and resource")]
    #[test_case("/8000/1/0",
        UUri {
            authority_name: String::default(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0000,
            ..Default::default()
        };
        "for local rpc service response")]
    #[test_case("up://VCU.MY_CAR_VIN/108000/1/2",
        UUri {
            authority_name: "VCU.MY_CAR_VIN".to_string(),
            ue_id: 0x0010_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
            ..Default::default()
        };
        "for remote uri")]
    #[test_case("//*/FFFF/FF/FFFF",
        UUri {
            authority_name: "*".to_string(),
            ue_id: 0x0000_FFFF,
            ue_version_major: 0xFF,
            resource_id: 0xFFFF,
            ..Default::default()
        };
        "for remote uri with wildcards")]
    fn test_from_string_succeeds(uri: &str, expected_uuri: UUri) {
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

    // [utest->req~uri-data-model-proto~1]
    #[test]
    fn test_protobuf_serialization() {
        let uri = UUri {
            authority_name: "MYVIN".to_string(),
            ue_id: 0x0000_1a4f,
            ue_version_major: 0x10,
            resource_id: 0xb392,
            ..Default::default()
        };
        let pb = uri.write_to_bytes().unwrap();
        let deserialized_uri = UUri::parse_from_bytes(pb.as_slice()).unwrap();
        assert_eq!(uri, deserialized_uri);
    }

    // [utest->dsn~uri-authority-name-length~1]
    #[test]
    fn test_from_str_fails_for_authority_exceeding_max_length() {
        let host_name = ['a'; 129];
        let uri = format!("//{}/A100/1/6501", host_name.iter().collect::<String>());
        assert!(UUri::from_str(&uri).is_err());

        let host_name = ['a'; 126];
        // add single percent encoded character
        // this should result in a 129 character host
        let uri = format!("//{}%42/A100/1/6501", host_name.iter().collect::<String>());
        assert!(UUri::from_str(&uri).is_err());
    }

    // [utest->dsn~uri-authority-name-length~1]
    #[test]
    fn test_try_from_parts_fails_for_authority_exceeding_max_length() {
        let authority = ['a'; 129].iter().collect::<String>();
        assert!(UUri::try_from_parts(&authority, 0xa100, 0x01, 0x6501).is_err());

        let mut authority = ['a'; 126].iter().collect::<String>();
        // add single percent encoded character
        // this should result in a 129 character host
        authority.push_str("%42");
        assert!(UUri::try_from_parts(&authority, 0xa100, 0x01, 0x6501).is_err());
    }

    // [utest->dsn~uri-host-only~2]
    #[test_case("MYVIN:1000"; "for authority with port")]
    #[test_case("user:pwd@MYVIN"; "for authority with userinfo")]
    fn test_try_from_parts_fails(authority: &str) {
        assert!(UUri::try_from_parts(authority, 0xa100, 0x01, 0x6501).is_err());
    }

    // [utest->dsn~uri-pattern-matching~1]
    #[test_case("//authority/A410/3/1003", "//authority/A410/3/1003"; "for identical URIs")]
    #[test_case("//*/A410/3/1003", "//authority/A410/3/1003"; "for pattern with wildcard authority")]
    #[test_case("//*/A410/3/1003", "/A410/3/1003"; "for pattern with wildcard authority and local candidate URI")]
    #[test_case("//authority/FFFF/3/1003", "//authority/A410/3/1003"; "for pattern with wildcard entity ID")]
    #[test_case("//authority/A410/3/1003", "//authority/2A410/3/1003"; "for pattern with wildcard entity instance")]
    #[test_case("//authority/A410/FF/1003", "//authority/A410/3/1003"; "for pattern with wildcard entity version")]
    #[test_case("//authority/A410/3/FFFF", "//authority/A410/3/1003"; "for pattern with wildcard resource")]
    fn test_matches_succeeds(pattern: &str, candidate: &str) {
        let pattern_uri =
            UUri::try_from(pattern).expect("should have been able to create pattern UUri");
        let candidate_uri =
            UUri::try_from(candidate).expect("should have been able to create candidate UUri");
        assert!(pattern_uri.matches(&candidate_uri));
    }

    // [utest->dsn~uri-pattern-matching~1]
    #[test_case("//Authority/A410/3/1003", "//authority/A410/3/1003"; "for pattern with upper case authority")]
    #[test_case("/A410/3/1003", "//authority/A410/3/1003"; "for local pattern and candidate URI with authority")]
    #[test_case("//other/A410/3/1003", "//authority/A410/3/1003"; "for pattern with different authority")]
    #[test_case("//authority/45/3/1003", "//authority/A410/3/1003"; "for pattern with different entity ID")]
    #[test_case("//authority/30A410/3/1003", "//authority/2A410/3/1003"; "for pattern with different entity instance")]
    #[test_case("//authority/A410/1/1003", "//authority/A410/3/1003"; "for pattern with different entity version")]
    #[test_case("//authority/A410/3/ABCD", "//authority/A410/3/1003"; "for pattern with different resource")]
    fn test_matches_fails(pattern: &str, candidate: &str) {
        let pattern_uri =
            UUri::try_from(pattern).expect("should have been able to create pattern UUri");
        let candidate_uri =
            UUri::try_from(candidate).expect("should have been able to create candidate UUri");
        assert!(!pattern_uri.matches(&candidate_uri));
    }
}
