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

use regex::Regex;

use crate::uprotocol::uri::UUri;
use crate::uprotocol::SerializationError;
use crate::uprotocol::{UAuthority, UEntity, UResource};
use crate::uri::validator::UriValidator;

use crate::uri::serializer::{MicroUriSerializer, UriSerializer};

impl TryFrom<UUri> for String {
    type Error = SerializationError;

    fn try_from(uri: UUri) -> Result<Self, Self::Error> {
        if UriValidator::is_empty(&uri) {
            return Err(SerializationError::new("URI is empty"));
        }

        let mut output = String::default();
        if let Some(authority) = uri.authority.as_ref() {
            output.push_str(UUri::build_authority_part_of_uri(authority).as_str());
        }
        output.push('/');
        if let Some(entity) = uri.entity.as_ref() {
            output.push_str(UUri::build_entity_part_of_uri(entity).as_str());
        }
        output.push_str(UUri::build_resource_part_of_uri(&uri).as_str());

        // remove trailing slashes
        Ok(Regex::new(r"/+$")
            .unwrap()
            .replace_all(&output, "")
            .into_owned())
    }
}

impl TryFrom<&str> for UUri {
    type Error = SerializationError;

    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        if uri.is_empty() {
            return Err(SerializationError::new("URI is empty"));
        }

        let uri = if let Some(index) = uri.find(':') {
            uri[index + 1..].to_string()
        } else {
            uri.replace('\\', "/")
        };
        let is_local: bool = !uri.starts_with("//");
        let uri_parts = Self::pattern_split(&uri, "/");

        if uri_parts.len() < 2 {
            return Err(SerializationError::new("URI is invalid"));
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
                    return Err(SerializationError::new("URI is invalid"));
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
                return Err(SerializationError::new("URI is invalid"));
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

impl TryFrom<UUri> for Vec<u8> {
    type Error = SerializationError;

    fn try_from(value: UUri) -> Result<Self, Self::Error> {
        MicroUriSerializer::serialize(&value)
    }
}

impl TryFrom<Vec<u8>> for UUri {
    type Error = SerializationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        MicroUriSerializer::deserialize(value)
    }
}

impl UUri {
    /// Builds a fully resolved `UUri` from the serialized long format and the serialized micro format.
    ///
    /// # Arguments
    /// * `long_uri` - uri serialized as a string.
    /// * `micro_uri` - uri serialized as a byte slice.
    ///
    /// # Returns
    /// If successful, returns an UUri object serialized from the input formats. Returns `SerializationError` if the deserialization
    /// fails or the resulting uri cannot be resolved.
    pub fn build_resolved(long_uri: &str, micro_uri: &[u8]) -> Result<UUri, SerializationError> {
        if long_uri.is_empty() && micro_uri.is_empty() {
            return Err(SerializationError::new("Input uris are empty"));
        }

        let long_uri = UUri::try_from(long_uri)?;
        let micro_uri = UUri::try_from(micro_uri.to_vec())?;

        let mut auth = micro_uri.authority.unwrap_or_default();
        let mut ue = micro_uri.entity.unwrap_or_default();
        let mut ure = long_uri.resource.unwrap_or_default();

        if let Some(authority) = long_uri.authority.as_ref() {
            if let Some(name) = authority.get_name() {
                auth.name = Some(name.to_owned());
            }
        }
        if let Some(entity) = long_uri.entity.as_ref() {
            ue.name = entity.name.clone();
        }
        if let Some(resource) = micro_uri.resource.as_ref() {
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
            Err(SerializationError::new(format!(
                "Could not resolve uri {uri}"
            )))
        }
    }

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
