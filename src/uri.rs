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
use std::io::Write;
use std::str::FromStr;

use bytes::{Buf, BufMut};
use regex::Regex;

pub use crate::up_core_api::uri::{uauthority::Number, UAuthority, UEntity, UResource, UUri};

mod uauthority;
pub use uauthority::AddressType;
mod uentity;
mod uresource;

mod uresourcebuilder;
pub use uresourcebuilder::UResourceBuilder;
mod uurivalidator;
pub use uurivalidator::UriValidator;

const LOCAL_MICRO_URI_LENGTH: usize = 8; // local micro URI length
const IPV4_MICRO_URI_LENGTH: usize = 12; // IPv4 micro URI length
const IPV6_MICRO_URI_LENGTH: usize = 24; // IPv6 micro URI length
const UP_VERSION: u8 = 0x1; // UP version

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

impl TryFrom<&UUri> for String {
    type Error = UUriError;

    /// Attempts to serialize a `UUri` into a `String`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `UUri` to be converted into a `String`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `String` representation of the URI or a `SerializationError`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAuthority, UEntity, UResource, UUri};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         name: "example.com".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         name: "rpc".to_string(),
    ///         instance: Some("raise".to_string()),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     authority: None.into(),
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = String::try_from(&uri).unwrap();
    /// assert_eq!("/example.com//rpc.raise", uri_from);
    /// ````
    fn try_from(uri: &UUri) -> Result<Self, Self::Error> {
        if UriValidator::is_empty(uri) {
            return Err(UUriError::serialization_error("URI is empty"));
        }

        let mut output = String::default();
        if let Some(authority) = uri.authority.as_ref() {
            output.push_str(UUri::build_authority_part_of_uri(authority).as_str());
        }
        output.push('/');
        if let Some(entity) = uri.entity.as_ref() {
            output.push_str(UUri::build_entity_part_of_uri(entity).as_str());
        }
        output.push_str(UUri::build_resource_part_of_uri(uri).as_str());

        // remove trailing slashes
        Ok(Regex::new(r"/+$")
            .unwrap()
            .replace_all(&output, "")
            .into_owned())
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
    /// use up_rust::{UAuthority, UEntity, UResource, UUri};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         name: "example.com".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         name: "rpc".to_string(),
    ///         instance: Some("raise".to_string()),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     authority: None.into(),
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = UUri::from_str("/example.com//rpc.raise").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        if uri.is_empty() {
            return Err(UUriError::serialization_error("URI is empty"));
        }

        // strip leading scheme definition (`up`) up to and including `:`
        let uri = if let Some(index) = uri.find(':') {
            uri[index + 1..].to_string()
        } else {
            uri.replace('\\', "/")
        };
        let is_local: bool = !uri.starts_with("//");
        let uri_parts = Self::pattern_split(&uri, "/");

        if uri_parts.len() < 2 {
            return Err(UUriError::serialization_error(
                "URI missing UEntity or UResource",
            ));
        }

        #[allow(unused_assignments)]
        let mut name: String = String::default();
        let mut version: String = String::default();
        let mut resource: Option<UResource> = None;
        let mut authority: Option<UAuthority> = None;

        if is_local {
            name = uri_parts[1].to_string();
            if uri_parts.len() > 2 {
                version = uri_parts[2].to_string();
            }
            if uri_parts.len() > 3 {
                resource = Some(UResource::from(uri_parts[3].as_str()));
            }
        } else {
            if uri_parts.len() > 2 {
                if uri_parts[2].trim().is_empty() {
                    return Err(UUriError::serialization_error(
                        "Remote URI missing UAuthority",
                    ));
                }
                authority = Some(UAuthority {
                    name: Some(uri_parts[2].clone()),
                    ..Default::default()
                });
            }
            if uri_parts.len() > 3 {
                name = uri_parts[3].to_string();
                if uri_parts.len() > 4 {
                    version = uri_parts[4].to_string();
                }
                if uri_parts.len() > 5 {
                    resource = Some(UResource::from(uri_parts[5].as_str()));
                }
            } else {
                return Ok(UUri {
                    authority: authority.into(),
                    ..Default::default()
                });
            }
        }

        // Compatibility note: in the Java SDK, UEntity versions are 'int', therefore default to 0. For some reason,
        // UUris with a 0 version, the version is not properly serialized back to a string (0 is omitted). Anyways,
        // we handle this properly. There either is a version, or there is not.
        let mut ve: Option<u32> = None;
        if !version.is_empty() {
            if let Ok(version) = version.parse::<u32>() {
                ve = Some(version);
            } else {
                return Err(UUriError::serialization_error(format!(
                    "Could not parse version number - expected an unsigned integer, got {}",
                    version
                )));
            }
        }

        let entity = UEntity {
            name,
            version_major: ve,
            ..Default::default()
        };

        Ok(UUri {
            entity: Some(entity).into(),
            authority: authority.into(),
            resource: resource.into(),
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
    /// use up_rust::{UAuthority, UEntity, UResource, UUri};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         name: "example.com".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         name: "rpc".to_string(),
    ///         instance: Some("raise".to_string()),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     authority: None.into(),
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = UUri::try_from("/example.com//rpc.raise").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        UUri::from_str(uri)
    }
}

impl TryFrom<&UUri> for Vec<u8> {
    type Error = UUriError;

    /// Serializes a `UUri` into a `Vec<u8>` following the Micro-URI specifications.
    ///
    /// # Parameters
    /// * `uri`: A reference to the `UUri` data object.
    ///
    /// # Returns
    /// A `Vec<u8>` representing the serialized `UUri`.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` noting which which portion failed Micro Uri validation
    /// or another error which occurred during serialization.
    ///
    /// # Examples
    ///
    /// ## Example which passes the Micro Uri validation
    ///
    /// ```
    /// use up_rust::{UEntity, UUri, UResource};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         id: Some(19999),
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///     ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let uprotocol_uri = Vec::try_from(&uri);
    /// assert!(uprotocol_uri.is_ok());
    /// let expected_uri_bytes = vec![0x01, 0x00, 0x75, 0x2F, 0x4E, 0x1F, 0xFE, 0x00];
    /// assert_eq!(uprotocol_uri.unwrap(), expected_uri_bytes);
    /// ```
    ///
    /// ## Example which fails the Micro Uri validation due to UEntity ID being > 16 bits
    ///
    /// ```
    /// use up_rust::{UEntity, UUri, UResource};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         id: Some(0x10000), // <- note that we've exceeded the allotted 16 bits
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///     ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let uprotocol_uri = Vec::try_from(&uri);
    /// assert!(uprotocol_uri.is_err());
    /// ```
    fn try_from(uri: &UUri) -> Result<Self, Self::Error> {
        if let Err(validation_error) = UriValidator::validate_micro_form(uri) {
            let error_message =
                format!("Failed to validate micro URI format: {}", validation_error);
            return Err(UUriError::serialization_error(error_message));
        }

        let mut buf = vec![];

        // UP_VERSION
        buf.put_u8(UP_VERSION);

        // ADDRESS_TYPE
        let address_type: AddressType = match uri.authority.as_ref() {
            Some(authority) => AddressType::try_from(authority)?,
            _ => AddressType::Local,
        };
        buf.put_u8(address_type.value());

        // URESOURCE_ID
        if let Some(id) = uri.resource.as_ref().and_then(|resource| resource.id) {
            buf.write_all(&[(id >> 8) as u8])
                .map_err(|e| UUriError::serialization_error(e.to_string()))?;
            buf.write_all(&[id as u8])
                .map_err(|e| UUriError::serialization_error(e.to_string()))?;
        }

        // UENTITY_ID
        if let Some(id) = uri.entity.as_ref().and_then(|entity| entity.id) {
            buf.write_all(&[(id >> 8) as u8])
                .map_err(|e| UUriError::serialization_error(e.to_string()))?;
            buf.write_all(&[id as u8])
                .map_err(|e| UUriError::serialization_error(e.to_string()))?;
        }

        // UENTITY_VERSION
        let version = uri
            .entity
            .as_ref()
            .and_then(|entity| entity.version_major)
            .unwrap_or(0);
        buf.put_u8(version as u8);

        // UNUSED
        buf.put_u8(0);

        // UAUTHORITY
        if let Some(authority) = uri.authority.as_ref() {
            buf.put(Vec::try_from(authority)?.as_slice());
        }

        Ok(buf)
    }
}

impl TryFrom<&[u8]> for UUri {
    type Error = UUriError;

    /// Creates a `UUri` data object from a uProtocol micro URI.
    ///
    /// # Arguments
    ///
    /// * `micro_uri` - A byte slice representing the uProtocol micro URI.
    ///
    /// # Returns
    ///
    /// Returns a `UUri` data object.
    fn try_from(micro_uri: &[u8]) -> Result<Self, Self::Error> {
        if micro_uri.len() < LOCAL_MICRO_URI_LENGTH {
            return Err(UUriError::serialization_error(
                "URI is empty or not in micro form",
            ));
        }

        let mut buf: &[u8] = micro_uri;
        // Need to be version 1
        if buf.get_u8() != UP_VERSION {
            return Err(UUriError::serialization_error(format!(
                "URI is not of expected uProtocol version {}",
                UP_VERSION
            )));
        }
        let address_type = AddressType::try_from(buf.get_u8())?;

        match address_type {
            AddressType::Local => {
                if micro_uri.len() != LOCAL_MICRO_URI_LENGTH {
                    return Err(UUriError::serialization_error("Invalid micro URI length"));
                }
            }
            AddressType::IPv4 => {
                if micro_uri.len() != IPV4_MICRO_URI_LENGTH {
                    return Err(UUriError::serialization_error("Invalid micro URI length"));
                }
            }
            AddressType::IPv6 => {
                if micro_uri.len() != IPV6_MICRO_URI_LENGTH {
                    return Err(UUriError::serialization_error("Invalid micro URI length"));
                }
            }
            AddressType::ID => {
                // we cannot perform any reasonable check at this point because we do not
                // know the (variable) length of the authority ID yet
            }
        }

        // RESOURCE
        let uresource_id = u32::from(buf.get_u16());
        let resource = Some(UResourceBuilder::from_id(uresource_id));

        // UENTITY
        let ue_id = buf.get_u16();
        let ue_version = u32::from(buf.get_u8());
        let entity = Some(UEntity {
            id: Some(ue_id.into()),
            version_major: Some(ue_version),
            ..Default::default()
        });

        // skip unused byte
        buf.advance(1);

        // Calculate uAuthority
        let authority = match address_type {
            AddressType::IPv4 => Some(UAuthority {
                number: Some(Number::Ip(buf.copy_to_bytes(4).to_vec())),
                ..Default::default()
            }),
            AddressType::IPv6 => Some(UAuthority {
                number: Some(Number::Ip(buf.copy_to_bytes(16).to_vec())),
                ..Default::default()
            }),
            AddressType::ID => {
                let len = buf.get_u8() as usize;
                Some(UAuthority {
                    number: Some(Number::Id(buf.copy_to_bytes(len).to_vec())),
                    ..Default::default()
                })
            }
            AddressType::Local => None,
        };

        Ok(UUri {
            authority: authority.into(),
            entity: entity.into(),
            resource: resource.into(),
            ..Default::default()
        })
    }
}

impl TryFrom<Vec<u8>> for UUri {
    type Error = UUriError;

    /// Creates a `UUri` data object from a uProtocol micro URI.
    ///
    /// # Arguments
    ///
    /// * `micro_uri` - A byte vec representing the uProtocol micro URI.
    ///
    /// # Returns
    ///
    /// Returns a `UUri` data object.
    fn try_from(micro_uri: Vec<u8>) -> Result<Self, Self::Error> {
        UUri::try_from(micro_uri.as_slice())
    }
}

impl Hash for UUri {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.authority.hash(state);
        self.entity.hash(state);
        self.resource.hash(state);
    }
}

impl Eq for UUri {}

impl UUri {
    /// Builds a fully resolved `UUri` from the serialized long format and the serialized micro format.
    ///
    /// # Arguments
    /// * `long_uri` - uri serialized as a string.
    /// * `micro_uri` - uri serialized as a byte slice.
    ///
    /// # Returns
    /// If successful, returns an UUri object serialized from the input formats. Returns `SerializationError` if either of the input uris
    /// are empty, in case the deserialization fails, or if the resulting uri cannot be resolved.
    pub fn build_resolved(long_uri: &str, micro_uri: &[u8]) -> Result<UUri, UUriError> {
        if long_uri.is_empty() {
            return Err(UUriError::serialization_error("Long URI is empty"));
        }
        if micro_uri.is_empty() {
            return Err(UUriError::serialization_error("Micro URI is empty"));
        }

        let long_uri_parsed = UUri::from_str(long_uri)?;
        let micro_uri_parsed = UUri::try_from(micro_uri)?;

        let mut auth = match micro_uri_parsed.authority.into_option() {
            Some(value) => value,
            None => {
                return Err(UUriError::serialization_error(
                    "Micro URI is missing UAuthority",
                ))
            }
        };
        let mut ue = match micro_uri_parsed.entity.into_option() {
            Some(value) => value,
            None => {
                return Err(UUriError::serialization_error(
                    "Micro URI is missing UEntity",
                ))
            }
        };
        let mut ure = match long_uri_parsed.resource.into_option() {
            Some(value) => value,
            None => {
                return Err(UUriError::serialization_error(
                    "Long URI is missing UResource",
                ))
            }
        };

        if let Some(authority) = long_uri_parsed.authority.as_ref() {
            if let Some(name) = authority.get_name() {
                auth.name = Some(name.to_owned());
            }
        }
        if let Some(entity) = long_uri_parsed.entity.as_ref() {
            ue.name = entity.name.clone();
        }
        if let Some(resource) = micro_uri_parsed.resource.as_ref() {
            ure.id = resource.id;
        }

        let uri = UUri {
            authority: Some(auth).into(),
            entity: Some(ue).into(),
            resource: Some(ure).into(),
            ..Default::default()
        };

        if UriValidator::is_resolved(&uri) {
            Ok(uri)
        } else {
            Err(UUriError::serialization_error(format!(
                "Could not resolve uri {:?}",
                uri
            )))
        }
    }

    /// Creates the resrouce part of the uProtocol URI from a `UUri` object representing a service or an application.
    ///
    /// # Parameters
    ///
    /// - `uri`: A `UURi` object that represents a service or an application.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the resource part of the uProtocol URI.
    fn build_resource_part_of_uri(uri: &UUri) -> String {
        let mut output = String::default();

        if let Some(resource) = uri.resource.as_ref() {
            output.push('/');
            output.push_str(&resource.name);

            if let Some(instance) = &resource.instance {
                output.push('.');
                output.push_str(instance);
            }
            if let Some(message) = &resource.message {
                output.push('#');
                output.push_str(message);
            }
        }

        output
    }

    /// Creates the service part of the uProtocol URI from a `UEntity` object representing a service or an application.
    ///
    /// # Parameters
    ///
    /// - `entity`: A `UEntity` object that represents a service or an application.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the service part of the uProtocol URI.
    fn build_entity_part_of_uri(entity: &UEntity) -> String {
        let mut output = String::from(entity.name.trim());
        output.push('/');

        if let Some(version) = entity.version_major {
            output.push_str(&version.to_string());
        }

        output
    }

    /// Creates the authority part of the uProtocol URI from an authority object.
    ///
    /// # Arguments
    /// * `authority` - Represents the deployment location of a specific Software Entity.
    ///
    /// # Returns
    /// Returns the `String` representation of the `Authority` in the uProtocol URI.
    fn build_authority_part_of_uri(authority: &UAuthority) -> String {
        let mut output = String::from("//");
        if let Some(name) = authority.name.as_ref() {
            output.push_str(name.as_str());
        }
        output
    }

    fn pattern_split(input: &str, pattern: &str) -> Vec<String> {
        let mut result: Vec<String> = input
            .split(pattern)
            .map(std::string::ToString::to_string)
            .collect();

        // Remove trailing empty strings
        while let Some(last) = result.last() {
            if last.is_empty() {
                result.pop();
            } else {
                break;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    // LONG/STRING URI TESTS

    #[test_case(""; "fail for empty string")]
    #[test_case("/"; "fail for schema and slash")]
    #[test_case("//"; "fail for schema and double slash")]
    #[test_case("///body.access"; "fail for schema and 3 slash and content")]
    #[test_case("////body.access"; "fail for schema and 4 slash and content")]
    #[test_case("/////body.access"; "fail for schema and 5 slash and content")]
    #[test_case("//////body.access"; "fail for schema and 6 slash and content")]
    fn test_long_try_from_string_fail(string: &str) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_err());
    }

    #[test_case(UUri::default(); "fail for default uri")]
    fn test_long_try_from_uri_fail(uri: UUri) {
        let parsing_result = String::try_from(&uri);
        assert!(parsing_result.is_err());
    }

    #[test_case("/body.access",
        UUri { entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(), ..Default::default() };
        "succeed for local service")]
    #[test_case("/body.access/1",
        UUri { entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(), ..Default::default() };
        "succeed for local service with version")]
    #[test_case("/body.access//door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with resource name")]
    #[test_case("/body.access/1/door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with version with resource name")]
    #[test_case("/body.access//door.front_left",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with resource name with instance")]
    #[test_case("/body.access/1/door.front_left",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with version with resource name with instance")]
    #[test_case("/body.access//door.front_left#Door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with resource name with instance with message")]
    #[test_case("/body.access/1/door.front_left#Door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with version with resource name with instance with message")]
    #[test_case("/exampleapp//rpc.response",
        UUri {
            entity: Some(UEntity { name: "exampleapp".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for local rpc service uri")]
    #[test_case("/exampleapp/1/rpc.response",
        UUri {
            entity: Some(UEntity { name: "exampleapp".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for local rpc service uri with version")]
    #[test_case("//VCU.MY_CAR_VIN",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            ..Default::default()
        };
        "succeed for remote uri")]
    #[test_case("//VCU.MY_CAR_VIN/body.access",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            ..Default::default()
        };
        "succeed for remote uri with service")]
    #[test_case("//VCU.MY_CAR_VIN/body.access/1",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with version")]
    #[test_case("//VCU.MY_CAR_VIN/body.access//door",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with resource name")]
    #[test_case("//VCU.MY_CAR_VIN/body.access//door.front_left",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with resource name with instance")]
    #[test_case("//VCU.MY_CAR_VIN/body.access/1/door.front_left",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with version with resource name with instance")]
    #[test_case("//VCU.MY_CAR_VIN/body.access//door.front_left#Door",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with resource name with instance with message")]
    #[test_case("//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with version with resource name with instance with message")]
    #[test_case("//example.cloud/exampleapp//rpc.response",
        UUri {
            authority: Some(UAuthority { name: Some("example.cloud".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "exampleapp".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for remote rpc uri with service")]
    #[test_case("//example.cloud/exampleapp/1/rpc.response",
        UUri {
            authority: Some(UAuthority { name: Some("example.cloud".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "exampleapp".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for remote rpc uri with service with version")]
    fn test_long_try_from_success(string: &str, expected_uri: UUri) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_ok());
        let parsed_uri = parsing_result.unwrap();
        assert_eq!(expected_uri, parsed_uri);

        let parsing_result = String::try_from(&parsed_uri);
        assert!(parsing_result.is_ok());
        assert_eq!(string, parsing_result.unwrap());
    }

    #[test_case("custom:/body.access//door.front_left#Door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local uri with custom scheme with service with resource name with instance with message")]
    #[test_case("custom://vcu.vin/body.access//door.front_left#Door",
        UUri {
            authority: Some(UAuthority { name: Some("vcu.vin".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with custom scheme with service with resource name with instance with message")]
    fn test_long_try_from_custom_scheme_success(string: &str, expected_uri: UUri) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_ok());
        let parsed_uri = parsing_result.unwrap();
        assert_eq!(expected_uri, parsed_uri);

        let string = string.split_once(':').unwrap().1; // remove prefix up to and including ':' from the back-comparison uri, as custom schemes are ignores by UUri deserialization
        let parsing_result = String::try_from(&parsed_uri);
        assert!(parsing_result.is_ok());
        assert_eq!(string, parsing_result.unwrap());
    }

    // MICRO/VEC URI TESTS

    #[test_case(UUri::default(); "fail for default uri")]
    #[test_case(
        UUri {
            authority: Some(UAuthority { name: Some(String::from("vcu.vin")), ..Default::default() } ).into(),
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "fail for remote service without address")]
    #[test_case(
        UUri {
            entity: Some( UEntity { name: "kaputt".to_string(), ..Default::default() } ).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "fail for rpc response without ids")]
    #[test_case(
        UUri {
            entity: Some( UEntity { name: "kaputt".to_string(), id: Some(2999), version_major: Some(1), ..Default::default() } ).into(),
            ..Default::default()
        };
        "fail for local service without resource")]
    #[test_case(
        UUri {
            authority: Some( UAuthority { number: Some(Number::Ip(vec![127, 1, 23, 123, 12, 6])), ..Default::default() } ).into(),
            ..Default::default()
        };
        "fail for remote service with bad length ip address")]
    #[test_case(
        UUri {
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(0x10000), ..Default::default() } ).into(),
            ..Default::default()
        };
        "fail for service overflow in resource id")]
    #[test_case(
        UUri {
            entity: Some( UEntity { id: Some(0x10000), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(29999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "fail for service overflow in entity id")]
    #[test_case(
        UUri {
            entity: Some( UEntity { id: Some(29999), version_major: Some(0x100), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(29999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "fail for service overflow in entity version")]
    fn test_micro_try_from_uri_fail(uri: UUri) {
        let parsing_result = Vec::try_from(&uri);
        assert!(parsing_result.is_err());
    }

    #[test_case(vec![0x1, 0x0, 0x0, 0x0, 0x0]; "fail for bad microuri length")]
    #[test_case(vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]; "fail for bad microuri version")]
    #[test_case(vec![0x1, 0x5, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]; "fail for invalid microuri address type")]
    #[test_case(vec![0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]; "fail for invalid microuri length 0")]
    #[test_case(vec![0x1, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]; "fail for invalid microuri length 1")]
    #[test_case(vec![0x1, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]; "fail for invalid microuri length 2")]
    fn test_micro_try_from_vec_fail(uri: Vec<u8>) {
        let parsing_result = UUri::try_from(uri.as_slice());
        assert!(parsing_result.is_err());
    }

    #[test_case(vec![1, 0, 78, 31, 117, 47, 254, 0],
        UUri {
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "succeed for local service with id with version with resource id")]
    #[test_case(vec![1, 0, 78, 31, 117, 47, 254, 0],
        UUri {
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "succeed for remote service with id with version with resource id")]
    #[test_case(vec![1, 1, 78, 31, 117, 47, 254, 0, 10, 0, 3, 3],
        UUri {
            authority: Some(UAuthority { number: Some(Number::Ip(vec![10, 0, 3, 3])), ..Default::default() } ).into(),
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(), // id below MAX_RPC_ID=1000 -> "rpc" resource
            ..Default::default()
        };
        "succeed for remote service with ipv4 authority with entity id with version with resource id")]
    #[test_case(vec![1, 2, 78, 31, 117, 47, 254, 0, 32, 1, 13, 184, 133, 163, 0, 0, 0, 0, 138, 46, 3, 112, 115, 52],
        UUri {
            authority: Some(UAuthority { number: Some(Number::Ip(vec![32, 1, 13, 184, 133, 163, 0, 0, 0, 0, 138, 46, 3, 112, 115, 52])), ..Default::default() } ).into(),
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "succeed for remote service with ipv6 authority with entity id with version with resource id")]
    #[test_case(vec![1, 3, 78, 31, 117, 47, 254, 0, 9, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        UUri {
            authority: Some(UAuthority { number: Some(Number::Id(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09])), ..Default::default() } ).into(),
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "succeed for remote service with id authority with entity id with version with resource id")]
    #[test_case(add_many_bytes(Some(vec![1, 3, 78, 31, 117, 47, 254, 0, 129]), 129),
        UUri {
            authority: Some(UAuthority { number: Some(Number::Id(add_many_bytes(None, 129))), ..Default::default() } ).into(),
            entity: Some( UEntity { id: Some(29999), version_major: Some(254), ..Default::default() } ).into(),
            resource: Some( UResource { id: Some(19999), ..Default::default() } ).into(),
            ..Default::default()
        };
        "succeed for remote service with long authority id with entity id with version with resource id")]
    fn test_micro_try_from_success(vec: Vec<u8>, expected_uri: UUri) {
        let parsing_result = UUri::try_from(vec.as_slice());
        assert!(parsing_result.is_ok());
        let parsed_uri = parsing_result.unwrap();
        assert_eq!(expected_uri, parsed_uri);
        assert!(UriValidator::is_micro_form(&parsed_uri));

        let parsing_result = Vec::try_from(&parsed_uri);
        assert!(parsing_result.is_ok());
        assert_eq!(vec, parsing_result.unwrap());
    }

    // MISC/OTHER TESTS

    #[test]
    fn test_build_resolved_passing_empty_long_uri_empty_micro_uri() {
        let uri: Result<UUri, UUriError> = UUri::build_resolved("", &[]);
        assert!(uri.is_err());
    }

    // HELPERS

    // Add a sequence of bytes to the end of a given byte vector, or create a new vector if None start_bytes were given
    fn add_many_bytes(start_bytes: Option<Vec<u8>>, size: usize) -> Vec<u8> {
        if let Some(mut bytes) = start_bytes {
            bytes.extend((0..size).map(|i| i as u8));
            return bytes;
        }
        (0..size).map(|i| i as u8).collect()
    }
}
