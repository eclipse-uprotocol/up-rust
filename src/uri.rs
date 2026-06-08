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

/*!
Implementation of the [uProtocol URI (UUri) data model](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/basics/uri.adoc).
A UUri represents a uProtocol resource identifier and is used in various places across the uProtocol specification, e.g., to identify the source and destination of messages or to identify resources that are being accessed via RPC calls.
*/

// [impl->dsn~uri-data-model-naming~1]
// [impl->req~uri-data-model-proto~1]

use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::LazyLock;

use uriparse::{Authority, URIReference};

pub(crate) const WILDCARD_AUTHORITY: &str = "*";
pub(crate) const WILDCARD_ENTITY_INSTANCE: u32 = 0xFFFF_0000;
pub(crate) const WILDCARD_ENTITY_TYPE: u32 = 0x0000_FFFF;
pub(crate) const WILDCARD_ENTITY_VERSION: u8 = 0xFF;
pub(crate) const WILDCARD_RESOURCE_ID: u16 = 0xFFFF;

pub(crate) const RESOURCE_ID_RESPONSE: u16 = 0x0000;
pub(crate) const RESOURCE_ID_MIN_EVENT: u16 = 0x8000;

const AUTHORITY_NAME_MAX_LENGTH: usize = 128;
static AUTHORITY_NAME_PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
    let regex = format!(r"^[a-z0-9\-._~]{{0,{}}}$", AUTHORITY_NAME_MAX_LENGTH);
    regex::Regex::new(&regex).unwrap()
});

type AuthorityNameString = String;

/// An error indicating a problem with creating or parsing a UUri.
#[derive(Debug)]
pub enum UUriError {
    /// Indicates that a given URI string cannot be parsed into a UUri due to invalid formatting or content.
    SerializationError(String),
    /// Indicates that a given URI does not comply with the UUri specification.
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
            Self::SerializationError(e) => f.write_fmt(format_args!("Serialization error: {e}")),
            Self::ValidationError(e) => f.write_fmt(format_args!("Validation error: {e}")),
        }
    }
}

impl std::error::Error for UUriError {}

/// A URI that represents a uProtocol resource identifier.
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UUri {
    authority_name: AuthorityNameString,
    ue_id: u32,
    ue_version_major: u8,
    resource_id: u16,
}

impl std::fmt::Display for UUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", UUri::to_uri(self, true))
    }
}

// [impl->req~uri-serialization~1]
impl From<&UUri> for String {
    /// Serializes a uProtocol URI to a URI string.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI to serialize. Note that the given URI is **not** validated before serialization.
    ///   In particular, the URI's version and resource ID length are not checked to be within limits.
    ///
    /// # Returns
    ///
    /// The output of [`UUri::to_uri`] without including the uProtocol scheme.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("vin.vehicles", 0x0000_800A, 0x02, 0x0000_1a50).unwrap();
    /// let uri_string = String::from(&uuri);
    /// assert_eq!(uri_string, "//vin.vehicles/800A/2/1A50");
    /// ````
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
    /// let uri = UUri::try_from_parts("vin.vehicles", 0x000A_8000, 0x02, 0x0000_1a50).unwrap();
    /// let uri_from = UUri::from_str("//vin.vehicles/A8000/2/1A50").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-scheme~1]
    // [impl->dsn~uri-host-only~2]
    // [impl->dsn~uri-authority-mapping~1]
    // [impl->dsn~uri-path-mapping~2]
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
        let authority_name = parsed_uri.authority().map_or(
            Ok(AuthorityNameString::default()),
            Self::verify_parsed_authority,
        )?;

        let path_segments = parsed_uri.path().segments();
        match path_segments {
            [entity, version, resource] => {
                if entity.is_empty() {
                    return Err(UUriError::serialization_error(
                        "URI must contain non-empty entity ID",
                    ));
                }
                let ue_id = u32::from_str_radix(entity, 16).map_err(|e| {
                    UUriError::serialization_error(format!("Cannot parse entity ID: {e}"))
                })?;
                if version.is_empty() {
                    return Err(UUriError::serialization_error(
                        "URI must contain non-empty entity version",
                    ));
                }
                let ue_version_major = u8::from_str_radix(version, 16).map_err(|e| {
                    UUriError::serialization_error(format!("Cannot parse entity version: {e}"))
                })?;
                if resource.is_empty() {
                    return Err(UUriError::serialization_error(
                        "URI must contain non-empty resource ID",
                    ));
                }
                let resource_id = u16::from_str_radix(resource, 16).map_err(|e| {
                    UUriError::serialization_error(format!("Cannot parse resource ID: {e}"))
                })?;

                Ok(UUri {
                    authority_name,
                    ue_id,
                    ue_version_major,
                    resource_id,
                })
            }
            _ => Err(UUriError::serialization_error(
                "uProtocol URI must contain entity ID, entity version and resource ID",
            )),
        }
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri::try_from_parts("", 0x001A_8000, 0x02, 0x1a50).unwrap();
    /// let uri_from = UUri::try_from("/1A8000/2/1A50".to_string()).unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
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
    /// let uri = UUri::try_from_parts("", 0x001A_8000, 0x02, 0x1a50).unwrap();
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

impl PartialEq<str> for UUri {
    fn eq(&self, other: &str) -> bool {
        match UUri::from_str(other) {
            Ok(other_uri) => self.eq(&other_uri),
            Err(_) => false,
        }
    }
}

impl PartialEq<&str> for UUri {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for UUri {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

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
    /// let uuri = UUri::try_from_parts("vin.vehicles", 0x0000_800A, 0x02, 0x1a50).unwrap();
    /// let uri_string = uuri.to_uri(true);
    /// assert_eq!(uri_string, "up://vin.vehicles/800A/2/1A50");
    /// ````
    // [impl->dsn~uri-authority-mapping~1]
    // [impl->dsn~uri-path-mapping~2]
    // [impl->req~uri-serialization~1]
    #[must_use]
    pub fn to_uri(&self, include_scheme: bool) -> String {
        let mut output = String::default();
        if include_scheme {
            output.push_str("up:");
        }
        if !self.authority_name.is_empty() {
            output.push_str("//");
            output.push_str(self.authority_name.as_str());
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
    /// assert!(UUri::try_from_parts("vin", 0x0000_5a6b, 0x01, 0x0001).is_ok());
    /// ```
    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-host-only~2]
    pub fn try_from_parts(
        authority: &str,
        entity_id: u32,
        entity_version: u8,
        resource_id: u16,
    ) -> Result<Self, UUriError> {
        let authority_name = Self::verify_authority_name(authority)?;
        Ok(UUri {
            authority_name,
            ue_id: entity_id,
            ue_version_major: entity_version,
            resource_id,
        })
    }

    /// Gets a pattern URI that [matches](UUri::matches) any given candidate URI.
    #[must_use]
    pub fn any() -> Self {
        Self::any_with_resource_id(WILDCARD_RESOURCE_ID)
    }

    /// Gets a pattern URI that [matches](UUri::matches) any given candidate URI with a specific resource ID.
    #[must_use]
    pub fn any_with_resource_id(resource_id: u16) -> Self {
        UUri {
            authority_name: AuthorityNameString::from(WILDCARD_AUTHORITY),
            ue_id: WILDCARD_ENTITY_INSTANCE | WILDCARD_ENTITY_TYPE,
            ue_version_major: WILDCARD_ENTITY_VERSION,
            resource_id,
        }
    }

    /// Gets the authority name part from this uProtocol URI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri::try_from_parts("my-vehicle", 0x10101234, 0x01, 0x9a10).unwrap();
    /// assert_eq!(uri.authority_name(), "my-vehicle");
    /// ```
    #[must_use]
    pub fn authority_name(&self) -> &str {
        self.authority_name.as_str()
    }

    // Gets the uEntity type identifier part from this uProtocol URI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri::try_from_parts("my-vehicle", 0x10101234, 0x01, 0x9a10).unwrap();
    /// assert_eq!(uri.uentity_type_id(), 0x1234);
    /// ```
    #[must_use]
    pub fn uentity_type_id(&self) -> u16 {
        (self.ue_id & WILDCARD_ENTITY_TYPE) as u16
    }

    // Gets the uEntity instance identifier part from this uProtocol URI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri::try_from_parts("my-vehicle", 0x10101234, 0x01, 0x9a10).unwrap();
    /// assert_eq!(uri.uentity_instance_id(), 0x1010);
    /// ```
    #[must_use]
    pub fn uentity_instance_id(&self) -> u16 {
        ((self.ue_id & WILDCARD_ENTITY_INSTANCE) >> 16) as u16
    }

    // Gets the major version part from this uProtocol URI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri::try_from_parts("my-vehicle", 0x10101234, 0x01, 0x9a10).unwrap();
    /// assert_eq!(uri.uentity_major_version(), 0x01);
    /// ```
    #[must_use]
    pub fn uentity_major_version(&self) -> u8 {
        self.ue_version_major
    }

    // Gets the resource identifier part from this uProtocol URI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uri = UUri::try_from_parts("my-vehicle", 0x10101234, 0x01, 0x9a10).unwrap();
    /// assert_eq!(uri.resource_id(), 0x9a10);
    /// ```
    #[must_use]
    pub fn resource_id(&self) -> u16 {
        self.resource_id
    }

    #[must_use]
    pub fn clone_with_resource_id(&self, resource_id: u16) -> Self {
        UUri {
            authority_name: self.authority_name.clone(),
            ue_id: self.ue_id,
            ue_version_major: self.ue_version_major,
            resource_id,
        }
    }

    /// Verifies that the given authority name complies with the UUri specification.
    ///
    /// # Arguments
    ///
    /// * `authority` - The authority name to verify.
    ///
    /// # Errors
    ///
    /// Returns an error if the authority name is invalid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// assert!(UUri::verify_authority("my.vin").is_ok());
    /// ```
    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-host-only~2]
    pub fn verify_authority(authority: &str) -> Result<(), UUriError> {
        Self::verify_authority_name(authority).map(|_| ())
    }

    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-host-only~2]
    pub(crate) fn verify_authority_name(authority: &str) -> Result<AuthorityNameString, UUriError> {
        Authority::try_from(authority)
            .map_err(|e| UUriError::validation_error(format!("invalid authority: {e}")))
            .and_then(|auth| Self::verify_parsed_authority(&auth))
    }

    // [impl->dsn~uri-authority-name-length~1]
    // [impl->dsn~uri-host-only~2]
    pub(crate) fn verify_parsed_authority(
        auth: &Authority,
    ) -> Result<AuthorityNameString, UUriError> {
        if auth.has_port() {
            Err(UUriError::validation_error(
                "uProtocol URI's authority must not contain port",
            ))
        } else if auth.has_username() || auth.has_password() {
            Err(UUriError::validation_error(
                "uProtocol URI's authority must not contain userinfo",
            ))
        } else {
            let verified_name = match auth.host() {
                uriparse::Host::IPv4Address(_) | uriparse::Host::IPv6Address(_) => {
                    auth.host().to_string()
                }
                uriparse::Host::RegisteredName(name) => {
                    if !WILDCARD_AUTHORITY.eq(name.as_str())
                        && !AUTHORITY_NAME_PATTERN.is_match(name.as_str())
                    {
                        return Err(UUriError::validation_error(
                            "uProtocol URI's authority contains invalid characters",
                        ));
                    }
                    name.to_string()
                }
            };
            Ok(verified_name)
        }
    }

    /// Check if an `UUri` is remote, by comparing authority fields.
    /// UUris with empty authority are considered to be local.
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
    /// let authority_a = UUri::from_str("up://authority.a/100A/1/0").unwrap();
    /// let authority_b = UUri::from_str("up://authority.b/200B/2/20").unwrap();
    /// assert!(authority_a.is_remote(&authority_b));
    ///
    /// let authority_local = UUri::from_str("up:///100A/1/0").unwrap();
    /// assert!(!authority_local.is_remote(&authority_a));
    ///
    /// let authority_wildcard = UUri::from_str("up://*/100A/1/0").unwrap();
    /// assert!(!authority_wildcard.is_remote(&authority_a));
    /// assert!(!authority_a.is_remote(&authority_wildcard));
    /// assert!(!authority_wildcard.is_remote(&authority_wildcard));
    /// ````
    #[must_use]
    pub fn is_remote(&self, other_uri: &UUri) -> bool {
        self.is_remote_authority(other_uri.authority_name.as_str())
    }

    /// Check if an authority is remote compared to the authority field of the UUri.
    /// Empty authorities are considered to be local.
    ///
    /// # Returns
    ///
    /// 'true' if authority is a different than `Self.authority_name`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use up_rust::UUri;
    ///
    /// let authority_a = UUri::from_str("up://authority.a/100A/1/0").unwrap();
    /// let authority_b = "authority.b";
    /// assert!(authority_a.is_remote_authority(&authority_b));
    ///
    /// let authority_local = "";
    /// assert!(!authority_a.is_remote_authority(&authority_local));
    ///
    /// let authority_wildcard = "*";
    /// assert!(!authority_a.is_remote_authority(&authority_wildcard));
    /// ```
    #[must_use]
    pub fn is_remote_authority(&self, authority: &str) -> bool {
        !authority.is_empty()
            && !self.authority_name.is_empty()
            && !self.has_wildcard_authority()
            && authority != WILDCARD_AUTHORITY
            && self.authority_name.as_str() != authority
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
    #[must_use]
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
    #[must_use]
    pub fn has_wildcard_authority(&self) -> bool {
        self.authority_name == WILDCARD_AUTHORITY
    }

    /// Checks if this UUri has an entity identifier matching any instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("vin", 0xFFFF_0123, 0x01, 0x145b).unwrap();
    /// assert!(uuri.has_wildcard_entity_instance());
    /// ```
    #[must_use]
    pub fn has_wildcard_entity_instance(&self) -> bool {
        self.ue_id & WILDCARD_ENTITY_INSTANCE == WILDCARD_ENTITY_INSTANCE
    }

    /// Checks if this UUri has an entity identifier matching any type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UUri;
    ///
    /// let uuri = UUri::try_from_parts("vin", 0x00C0_FFFF, 0x01, 0x145b).unwrap();
    /// assert!(uuri.has_wildcard_entity_type());
    /// ```
    #[must_use]
    pub fn has_wildcard_entity_type(&self) -> bool {
        self.ue_id & WILDCARD_ENTITY_TYPE == WILDCARD_ENTITY_TYPE
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
    #[must_use]
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
    #[must_use]
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
    /// let uri = UUri::try_from_parts("vin.vehicles", 0x0000_2310, 0x03, 0xa000).unwrap();
    /// assert!(uri.verify_no_wildcards().is_ok());
    /// ```
    pub fn verify_no_wildcards(&self) -> Result<(), UUriError> {
        if self.has_wildcard_authority() {
            Err(UUriError::validation_error(format!(
                "Authority must not contain wildcard character [{WILDCARD_AUTHORITY}]"
            )))
        } else if self.has_wildcard_entity_instance() {
            Err(UUriError::validation_error(format!(
                "Entity instance ID must not be set to wildcard value [{WILDCARD_ENTITY_INSTANCE:#X}]")))
        } else if self.has_wildcard_entity_type() {
            Err(UUriError::validation_error(format!(
                "Entity type ID must not be set to wildcard value [{WILDCARD_ENTITY_TYPE:#X}]"
            )))
        } else if self.has_wildcard_version() {
            Err(UUriError::validation_error(format!(
                "Entity version must not be set to wildcard value [{WILDCARD_ENTITY_VERSION:#X}]"
            )))
        } else if self.has_wildcard_resource_id() {
            Err(UUriError::validation_error(format!(
                "Resource ID must not be set to wildcard value [{WILDCARD_RESOURCE_ID:#X}]"
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x7FFF).unwrap();
    /// assert!(uri.is_rpc_method());
    /// ```
    #[must_use]
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x8000).unwrap();
    /// assert!(uri.verify_rpc_method().is_err());
    ///
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x0000).unwrap();
    /// assert!(uri.verify_rpc_method().is_err());
    /// ```
    pub fn verify_rpc_method(&self) -> Result<(), UUriError> {
        if !self.is_rpc_method() {
            Err(UUriError::validation_error(format!(
                "Resource ID must be a value from ]{RESOURCE_ID_RESPONSE:#X}, {RESOURCE_ID_MIN_EVENT:#X}[")))
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x0000).unwrap();
    /// assert!(uri.is_notification_destination());
    /// ```
    #[must_use]
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x0000).unwrap();
    /// assert!(uri.is_rpc_response());
    /// ```
    #[must_use]
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x4001).unwrap();
    /// assert!(uri.verify_rpc_response().is_err());
    /// ```
    pub fn verify_rpc_response(&self) -> Result<(), UUriError> {
        if !self.is_rpc_response() {
            Err(UUriError::validation_error(format!(
                "Resource ID must be {RESOURCE_ID_RESPONSE:#X}"
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x8000).unwrap();
    /// assert!(uri.is_event());
    /// ```
    #[must_use]
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
    /// let uri = UUri::try_from_parts("", 0xabcd, 0x01, 0x7FFF).unwrap();
    /// assert!(uri.verify_event().is_err());
    /// ```
    pub fn verify_event(&self) -> Result<(), UUriError> {
        if !self.is_event() {
            Err(UUriError::validation_error(format!(
                "Resource ID must be >= {RESOURCE_ID_MIN_EVENT:#X}"
            )))
        } else {
            self.verify_no_wildcards()
        }
    }

    fn matches_authority(&self, candidate: &UUri) -> bool {
        self.has_wildcard_authority() || self.authority_name == candidate.authority_name
    }

    fn matches_entity_type(&self, candidate: &UUri) -> bool {
        self.has_wildcard_entity_type() || self.uentity_type_id() == candidate.uentity_type_id()
    }

    fn matches_entity_instance(&self, candidate: &UUri) -> bool {
        self.has_wildcard_entity_instance()
            || self.uentity_instance_id() == candidate.uentity_instance_id()
    }

    fn matches_entity_version(&self, candidate: &UUri) -> bool {
        self.has_wildcard_version()
            || self.uentity_major_version() == candidate.uentity_major_version()
    }

    fn matches_entity(&self, candidate: &UUri) -> bool {
        self.matches_entity_type(candidate)
            && self.matches_entity_instance(candidate)
            && self.matches_entity_version(candidate)
    }

    fn matches_resource(&self, candidate: &UUri) -> bool {
        self.has_wildcard_resource_id() || self.resource_id == candidate.resource_id
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
    /// let pattern = UUri::try_from("//vin/A14F/3/FFFF").unwrap();
    /// let candidate = UUri::try_from("//vin/A14F/3/B1D4").unwrap();
    /// assert!(pattern.matches(&candidate));
    /// ```
    // [impl->dsn~uri-pattern-matching~2]
    #[must_use]
    pub fn matches(&self, candidate: &UUri) -> bool {
        self.matches_authority(candidate)
            && self.matches_entity(candidate)
            && self.matches_resource(candidate)
    }
}

#[cfg(feature = "up-core-types")]
mod core_types_support {
    use super::*;
    use crate::up_core_api::uri::UUri as UUriProto;
    use crate::ProtobufMappable;
    use protobuf::{well_known_types::any::Any, Message};

    impl TryFrom<&UUriProto> for UUri {
        type Error = crate::UUriError;

        fn try_from(uuri_proto: &UUriProto) -> Result<Self, Self::Error> {
            let version = u8::try_from(uuri_proto.ue_version_major).map_err(|_e| {
                UUriError::validation_error(
                    "uProtocol URI's major version must be an 8 bit unsigned integer".to_string(),
                )
            })?;
            let resource_id = u16::try_from(uuri_proto.resource_id).map_err(|_e| {
                UUriError::validation_error(
                    "uProtocol URI's resource ID must be a 16 bit unsigned integer".to_string(),
                )
            })?;
            UUri::try_from_parts(
                &uuri_proto.authority_name,
                uuri_proto.ue_id,
                version,
                resource_id,
            )
        }
    }

    impl From<&UUri> for UUriProto {
        fn from(uuri: &UUri) -> Self {
            UUriProto {
                authority_name: uuri.authority_name.as_str().to_string(),
                ue_id: uuri.ue_id,
                ue_version_major: uuri.ue_version_major as u32,
                resource_id: uuri.resource_id as u32,
                ..Default::default()
            }
        }
    }

    impl ProtobufMappable for UUri {
        fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
            let uuri_proto = UUriProto::parse_from_bytes(proto).map_err(|e| {
                crate::SerializationError::new(format!("Protobuf decode error: {e}"))
            })?;
            UUri::try_from(&uuri_proto)
                .map_err(|e| crate::SerializationError::new(format!("UUri conversion error: {e}")))
        }
        fn parse_from_packed_protobuf_bytes(
            proto: &[u8],
        ) -> Result<Self, crate::SerializationError> {
            Any::parse_from_bytes(proto)
                .map_err(|err| crate::SerializationError::new(err.to_string()))
                .and_then(|any| match any.unpack::<UUriProto>() {
                    Ok(Some(uuri_proto)) => UUri::try_from(&uuri_proto)
                        .map_err(|e| crate::SerializationError::new(e.to_string())),
                    Ok(None) => Err(crate::SerializationError::new(
                        "Protobuf Any does not contain UUriProto".to_string(),
                    )),
                    Err(e) => Err(crate::SerializationError::new(format!(
                        "Protobuf Any unpack error: {e}"
                    ))),
                })
        }
        fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
            UUriProto::from(self)
                .write_to_bytes()
                .map_err(|e| crate::SerializationError::new(format!("Protobuf encode error: {e}")))
        }
        fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
            Any::pack(&UUriProto::from(self))
                .map_err(|e| crate::SerializationError::new(format!("Failed to pack UUri: {e}")))
                .and_then(|any| any.write_to_protobuf_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(UUri {
            authority_name: "vin".into(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
        },
        "//vin/8000/1/2" => true;
        "succeeds for full URI with authority")]
    #[test_case(UUri {
            authority_name: "".into(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
        },
        "up:/8000/1/2" => true;
        "succeeds for URI without authority")]
    #[test_case(UUri {
            authority_name: "vin".into(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
        },
        "//other-vin/8000/1/2" => false;
        "fails for different authority")]
    #[test_case(UUri {
            authority_name: "vin".into(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
        },
        "up://vin/18000/1/2" => false;
        "fails for different entity ID")]
    #[test_case(UUri {
            authority_name: "vin".into(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
        },
        "//vin/8000/2/2" => false;
        "fails for different version")]
    #[test_case(UUri {
            authority_name: "vin".into(),
            ue_id: 0x0000_8000,
            ue_version_major: 0x01,
            resource_id: 0x0002,
        },
        "up://vin/8000/1/5" => false;
        "fails for different resource ID")]
    #[allow(clippy::cmp_owned)]
    fn test_eq_with_str(uuri: UUri, uri_str: &str) -> bool {
        uuri == uri_str && uuri == *uri_str && uuri == String::from(uri_str)
    }

    #[test_case("//*/A100/1/1"; "for any authority")]
    #[test_case("//vin/FFFF/1/1"; "for any entity type")]
    #[test_case("//vin/FFFF0ABC/1/1"; "for any entity instance")]
    #[test_case("//vin/A100/FF/1"; "for any version")]
    #[test_case("//vin/A100/1/FFFF"; "for any resource")]
    fn test_verify_no_wildcards_fails(uri: &str) {
        let uuri = UUri::try_from(uri).expect("should have been able to deserialize URI");
        assert!(uuri.verify_no_wildcards().is_err());
    }

    // [utest->dsn~uri-authority-name-length~1]
    #[test]
    fn test_from_str_fails_for_authority_exceeding_max_length() {
        let host_name = "a".repeat(129);
        let uri = format!("//{}/A100/1/6501", host_name);
        assert!(UUri::from_str(&uri).is_err());
    }

    // [utest->dsn~uri-path-mapping~2]
    #[test]
    fn test_from_str_accepts_lowercase_hex_encoding() {
        let result = UUri::try_from("up://vin/ffff0abc/a1/bcd1");
        assert!(result.is_ok_and(|uuri| {
            uuri.authority_name == "vin"
                && uuri.ue_id == 0xFFFF0ABC
                && uuri.ue_version_major == 0xA1
                && uuri.resource_id == 0xBCD1
        }));
    }
}
