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

// use regex::Regex;

use regex::Regex;

use crate::uprotocol::{UAuthority, UEntity, UResource, UUri};
use crate::uri::serializer::{SerializationError, UriSerializer};
use crate::uri::validator::UriValidator;

/// `UriSerializer` that serializes a `UUri` to a string (long format) per
/// <https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc>
pub struct LongUriSerializer;

impl UriSerializer<String> for LongUriSerializer {
    fn serialize(uri: &UUri) -> Result<String, SerializationError> {
        if UriValidator::is_empty(uri) {
            return Err(SerializationError::new("URI is empty"));
        }

        let mut output = String::default();
        if let Some(authority) = uri.authority.as_ref() {
            output.push_str(&Self::build_authority_part_of_uri(authority));
        }
        output.push('/');
        if let Some(entity) = uri.entity.as_ref() {
            output.push_str(&Self::build_entity_part_of_uri(entity));
        }
        output.push_str(&Self::build_resource_part_of_uri(uri));

        // remove trailing slashes
        Ok(Regex::new(r"/+$")
            .unwrap()
            .replace_all(&output, "")
            .into_owned())
    }

    /// Create an  URI data object from a uProtocol string (long or short).
    /// This function supports both long and short uProtocol URIs.
    ///
    /// # Arguments
    ///
    /// * `uprotocol_uri` - The uProtocol URI string to be parsed.
    ///
    /// # Returns
    ///
    /// Returns an `UUri` data object created from the given uProtocol URI string.
    fn deserialize(uprotocol_uri: String) -> Result<UUri, SerializationError> {
        if uprotocol_uri.is_empty() {
            return Err(SerializationError::new("URI is empty"));
        }

        let uri = if let Some(index) = uprotocol_uri.find(':') {
            uprotocol_uri[index + 1..].to_string()
        } else {
            uprotocol_uri.replace('\\', "/")
        };
        let is_local: bool = !uri.starts_with("//");
        let uri_parts = Self::java_split(&uri, "/");

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

impl LongUriSerializer {
    /// Creates the resource part of the uProtocol URI from a `UUri` object.
    ///
    /// # Parameters
    ///
    /// - `uri`: A `UResource` object representing a resource manipulated by a service, such as a Door.
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

    // This function is meant to replicate the behavior of the Java
    // `String[] java.lang.String.split(String regex)` method.
    fn java_split(input: &str, pattern: &str) -> Vec<String> {
        // let re = Regex::new(pattern).unwrap();
        // let mut result: Vec<String> = re
        //     .split(input)
        //     .map(std::string::ToString::to_string)
        //     .collect();
        let mut result: Vec<String> = input
            .split(pattern)
            .map(std::string::ToString::to_string)
            .collect();

        // Remove trailing empty strings, to emulate Java's behavior
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

    use crate::uri::builder::resourcebuilder::UResourceBuilder;

    #[test]
    fn test_using_the_serializers() {
        let entity = UEntity {
            name: "hartley".into(),
            ..Default::default()
        };
        let resource = UResourceBuilder::for_rpc_request(Some("raise".into()), None);
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uristr = LongUriSerializer::serialize(&uri);
        assert_eq!("/hartley//rpc.raise", uristr.as_ref().unwrap());
        let uri2 = LongUriSerializer::deserialize(uristr.unwrap());
        assert_eq!(uri, uri2.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_when_is_empty_string() {
        let uri_result = LongUriSerializer::deserialize(String::default());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is empty");

        let uristr = LongUriSerializer::serialize(&UUri::default());
        assert!(uristr.is_err());
        assert_eq!(uristr.unwrap_err().to_string(), "URI is empty");
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_slash() {
        let uri_result = LongUriSerializer::deserialize("/".into());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is invalid");
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_double_slash() {
        let uri_result = LongUriSerializer::deserialize("//".to_string());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is invalid");
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_3_slash_and_something() {
        let uri_result = LongUriSerializer::deserialize("///body.access".to_string());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is invalid");
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_4_slash_and_something() {
        let uri_result = LongUriSerializer::deserialize("////body.access".to_string());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is invalid");
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_5_slash_and_something() {
        let uri_result = LongUriSerializer::deserialize("/////body.access".to_string());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is invalid");
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_6_slash_and_something() {
        let uri_result = LongUriSerializer::deserialize("//////body.access".to_string());
        assert!(uri_result.is_err());
        assert_eq!(uri_result.unwrap_err().to_string(), "URI is invalid");
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version() {
        let uri_result = LongUriSerializer::deserialize("/body.access".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version() {
        let uri_result = LongUriSerializer::deserialize("/body.access/1".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_name_only() {
        let uri_result = LongUriSerializer::deserialize("/body.access//door".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_name_only() {
        let uri_result = LongUriSerializer::deserialize("/body.access/1/door".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_with_instance() {
        let uri_result =
            LongUriSerializer::deserialize("/body.access//door.front_left".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_with_message() {
        let uri_result =
            LongUriSerializer::deserialize("/body.access/1/door.front_left".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_with_instance_and_message(
    ) {
        let uri_result =
            LongUriSerializer::deserialize("/body.access//door.front_left#Door".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_some());
        assert_eq!(
            "Door",
            uri.resource.as_ref().unwrap().message.as_ref().unwrap()
        );
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_with_instance_and_message(
    ) {
        let uri_result =
            LongUriSerializer::deserialize("/body.access/1/door.front_left#Door".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_some());
        assert_eq!(
            "Door",
            uri.resource.as_ref().unwrap().message.as_ref().unwrap()
        );
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_local_service_no_version() {
        let uri_result = LongUriSerializer::deserialize("/petapp//rpc.response".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("petapp", uri.entity.as_ref().unwrap().name);
        assert_eq!("rpc", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.as_ref().is_some());
        assert_eq!(
            "response",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_local_service_with_version() {
        let uri_result = LongUriSerializer::deserialize("/petapp/1/rpc.response".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert_eq!("petapp", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!("rpc", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "response",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_only_device_and_cloud_domain() {
        let uri_result = LongUriSerializer::deserialize("//VCU.MY_CAR_VIN".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version() {
        let uri_result = LongUriSerializer::deserialize("//VCU.MY_CAR_VIN/body.access".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert!(uri.entity.get_or_default().version_major.is_none());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version() {
        let uri_result =
            LongUriSerializer::deserialize("//VCU.MY_CAR_VIN/body.access/1".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert_eq!(Some(1), uri.entity.get_or_default().version_major);
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_name_only() {
        let uri_result =
            LongUriSerializer::deserialize("//VCU.MY_CAR_VIN/body.access//door".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert!(uri.entity.get_or_default().version_major.is_none());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.get_or_default().name);
        assert!(uri.resource.get_instance().is_none());
        assert!(uri.resource.get_message().is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_and_instance_no_message(
    ) {
        let uri_result = LongUriSerializer::deserialize(
            "//VCU.MY_CAR_VIN/body.access//door.front_left".to_string(),
        );
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert!(uri.entity.get_or_default().version_major.is_none());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.get_or_default().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(Some("front_left"), uri.resource.get_instance());
        assert!(uri.resource.get_message().is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_and_instance_no_message(
    ) {
        let uri_result = LongUriSerializer::deserialize(
            "//VCU.MY_CAR_VIN/body.access/1/door.front_left".to_string(),
        );
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert_eq!(Some(1), uri.entity.get_or_default().version_major);
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.get_or_default().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(Some("front_left"), uri.resource.get_instance());
        assert!(uri.resource.get_message().is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_and_instance_and_message(
    ) {
        let uri_result = LongUriSerializer::deserialize(
            "//VCU.MY_CAR_VIN/body.access//door.front_left#Door".to_string(),
        );
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert!(uri.entity.get_or_default().version_major.is_none());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.get_or_default().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(Some("front_left"), uri.resource.get_instance());
        assert_eq!(Some("Door"), uri.resource.get_message());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_and_instance_and_message(
    ) {
        let uri_result = LongUriSerializer::deserialize(
            "//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door".to_string(),
        );
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(
            Some("VCU.MY_CAR_VIN"),
            uri.authority.get_or_default().get_name()
        );
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.get_or_default().name);
        assert_eq!(Some(1), uri.entity.get_or_default().version_major);
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.get_or_default().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(Some("front_left"), uri.resource.get_instance());
        assert_eq!(Some("Door"), uri.resource.get_message());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_remote_service_no_version() {
        let uri_result =
            LongUriSerializer::deserialize("//bo.cloud/petapp//rpc.response".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(Some("bo.cloud"), uri.authority.get_or_default().get_name());
        assert!(uri.entity.is_some());
        assert_eq!("petapp", uri.entity.get_or_default().name);
        assert!(uri.entity.get_or_default().version_major.is_none());
        assert!(uri.resource.is_some());
        assert_eq!("rpc", uri.resource.get_or_default().name);
        assert_eq!(Some("response"), uri.resource.get_instance());
        assert!(uri.resource.get_message().is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_remote_service_with_version() {
        let uri_result =
            LongUriSerializer::deserialize("//bo.cloud/petapp/1/rpc.response".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert_eq!(Some("bo.cloud"), uri.authority.get_or_default().get_name());
        assert!(uri.entity.is_some());
        assert_eq!("petapp", uri.entity.get_or_default().name);
        assert_eq!(Some(1), uri.entity.get_or_default().version_major);
        assert!(uri.resource.is_some());
        assert_eq!("rpc", uri.resource.get_or_default().name);
        assert_eq!(Some("response"), uri.resource.get_instance());
        assert!(uri.resource.get_message().is_none());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_is_empty() {
        let uprotocol_uri = LongUriSerializer::serialize(&UUri::default());
        assert!(uprotocol_uri.is_err());
        assert_eq!(uprotocol_uri.unwrap_err().to_string(), "URI is empty");
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_empty_use() {
        let entity = UEntity::default();
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(UAuthority::default()).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/////door", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version() {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: None.into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/body.access", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version() {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource::default();
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/body.access/1", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version_with_resource(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/body.access//door", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version_with_resource(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/body.access/1/door", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version_with_resource_with_instance_no_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/body.access//door.front_left", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version_with_resource_with_instance_no_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/body.access/1/door.front_left", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version_with_resource_with_instance_with_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "/body.access//door.front_left#Door",
            &uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version_with_resource_with_instance_with_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "/body.access/1/door.front_left#Door",
            &uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version() {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: None.into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("//vcu.my_car_vin/body.access", &uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version() {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: None.into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("//vcu.my_car_vin/body.access/1", uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version_with_resource(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door",
            uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("//vcu.my_car_vin/body.access//door", uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version_with_resource_with_instance_no_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door.front_left",
            uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource_with_instance_no_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "//vcu.my_car_vin/body.access//door.front_left",
            uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version_with_resource_with_instance_and_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door.front_left#Door",
            uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource_with_instance_and_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "//vcu.my_car_vin/body.access//door.front_left#Door",
            uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_build_protocol_uri_for_source_part_of_rpc_request_where_source_is_local() {
        let entity = UEntity {
            name: "petapp".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "rpc".into(),
            instance: Some("response".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!("/petapp/1/rpc.response", uprotocol_uri.unwrap());
    }

    #[test]
    fn test_build_protocol_uri_from_parts_when_uri_has_remote_authority_service_and_version_with_resource(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            name: Some(String::from("vcu.my_car_vin")),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door",
            uprotocol_uri.unwrap()
        );
    }

    #[test]
    fn test_parse_local_protocol_uri_with_custom_scheme() {
        let uri_result =
            LongUriSerializer::deserialize("custom:/body.access//door.front_left#Door".to_string());
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(!UriValidator::is_remote(&uri));
        assert!(uri.authority.as_ref().is_none());
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_some());
        assert_eq!(
            "Door",
            uri.resource.as_ref().unwrap().message.as_ref().unwrap()
        );
    }

    #[test]
    fn test_parse_remote_protocol_uri_with_custom_scheme() {
        let uri = "custom://vcu.vin/body.access//door.front_left#Door".to_string();
        let uri2 = "//vcu.vin/body.access//door.front_left#Door".to_string();

        let uri_result = LongUriSerializer::deserialize(uri);
        assert!(uri_result.is_ok());
        let uri = uri_result.unwrap();
        assert!(UriValidator::is_remote(&uri));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.is_some());
        if let Some(name) = UAuthority::get_name(uri.authority.as_ref().unwrap()) {
            assert_eq!("vcu.vin", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_some());
        assert_eq!(
            "Door",
            uri.resource.as_ref().unwrap().message.as_ref().unwrap()
        );
        let uri3 = LongUriSerializer::serialize(&uri);
        assert_eq!(uri2, uri3.unwrap());
    }

    #[test]
    fn test_deserialize_long_and_micro_passing_empty_long_uri_empty_byte_array() {
        let uri = LongUriSerializer::build_resolved("", &[]);
        assert!(uri.is_some());
        let uri2 = LongUriSerializer::serialize(&uri.unwrap());
        assert!(uri2.is_err());
        assert_eq!(uri2.unwrap_err().to_string(), "URI is empty");
    }
}
