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

use crate::uprotocol::u_authority::Remote;
use crate::uprotocol::{UAuthority, UEntity, UResource, UUri};
use crate::uri::serializer::uriserializer::UriSerializer;
use crate::uri::validator::UriValidator;

/// UUri Serializer that serializes a UUri to a string (long format) per
/// <https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc>
pub struct LongUriSerializer;

impl UriSerializer<String> for LongUriSerializer {
    fn serialize(uri: &UUri) -> String {
        if UriValidator::is_empty(uri) {
            return String::default();
        }

        let mut output = String::default();
        if let Some(authority) = &uri.authority {
            output.push_str(&Self::build_authority_part_of_uri(authority));
        }
        output.push('/');
        if let Some(entity) = &uri.entity {
            output.push_str(&Self::build_entity_part_of_uri(entity));
        }
        output.push_str(&Self::build_resource_part_of_uri(uri));

        Regex::new(r"/+$")
            .unwrap()
            .replace_all(&output, "")
            .into_owned()
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
    fn deserialize(uprotocol_uri: String) -> UUri {
        if uprotocol_uri.is_empty() {
            return UUri::default();
        }

        let uri = if let Some(index) = uprotocol_uri.find(':') {
            uprotocol_uri[index + 1..].to_string()
        } else {
            uprotocol_uri.replace('\\', "/")
        };
        let is_local: bool = !uri.starts_with("//");
        let uri_parts = Self::java_split(&uri, "/");

        if uri_parts.len() < 2 {
            return UUri::default();
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
                resource = Some(UResource::from(uri_parts[3].to_string()));
            }
        } else {
            if uri_parts.len() > 2 {
                if uri_parts[2].trim().is_empty() {
                    return UUri::default();
                } else {
                    authority = Some(UAuthority {
                        remote: Some(Remote::Name(uri_parts[2].to_string())),
                    });
                }
            }
            if uri_parts.len() > 3 {
                name = uri_parts[3].to_string();
                if uri_parts.len() > 4 {
                    version = uri_parts[4].to_string();
                }
                if uri_parts.len() > 5 {
                    resource = Some(UResource::from(uri_parts[5].to_string()));
                }
            } else {
                return UUri {
                    authority,
                    ..Default::default()
                };
            }
        }

        let mut ve: Option<u32> = None;
        if !version.is_empty() {
            if let Ok(version) = version.parse::<u32>() {
                ve = Some(version);
            }
        }

        let entity = UEntity {
            name,
            version_major: ve,
            ..Default::default()
        };

        UUri {
            entity: Some(entity),
            authority,
            resource,
        }
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

        if let Some(resource) = &uri.resource {
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
        if let Some(crate::uprotocol::u_authority::Remote::Name(name)) = &authority.remote {
            output.push_str(name);
        }
        output
    }

    // This function is meant to replicate the behavior of the Java
    // `String[] java.lang.String.split(String regex)` method.
    fn java_split(input: &str, pattern: &str) -> Vec<String> {
        let re = Regex::new(pattern).unwrap();
        let mut result: Vec<String> = re.split(input).map(|x| x.to_string()).collect();

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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uristr = LongUriSerializer::serialize(&uri);
        assert_eq!("/hartley//rpc.raise", uristr);
        let uri2 = LongUriSerializer::deserialize(uristr);
        assert_eq!(uri, uri2);
    }

    #[test]
    fn test_parse_protocol_uri_when_is_empty_string() {
        let uri = LongUriSerializer::deserialize(String::default());
        assert!(UriValidator::is_empty(&uri));

        let uristr = LongUriSerializer::serialize(&UUri::default());
        assert!(uristr.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_slash() {
        let uri = LongUriSerializer::deserialize("/".into());
        assert!(uri.authority.is_none());
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
        assert!(UriValidator::is_empty(&uri));
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_double_slash() {
        let uri = LongUriSerializer::deserialize("//".to_string());
        assert!(uri.authority.is_none());
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
        assert!(UriValidator::is_empty(&uri));
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_3_slash_and_something() {
        let uri = "///body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_none());
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
        assert!(UriValidator::is_empty(&uri));
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_4_slash_and_something() {
        let uri = "////body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
        assert!(!UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(UriValidator::is_empty(&uri));
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_5_slash_and_something() {
        let uri = "/////body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
        assert!(!UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(UriValidator::is_empty(&uri));
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_6_slash_and_something() {
        let uri = "//////body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
        assert!(!UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(UriValidator::is_empty(&uri));
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version() {
        let uri = "/body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!(0, uri.entity.as_ref().unwrap().version_minor.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version() {
        let uri = "/body.access/1".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_name_only() {
        let uri = "/body.access//door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!(0, uri.entity.as_ref().unwrap().version_minor.unwrap());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_name_only() {
        let uri = "/body.access/1/door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_with_instance() {
        let uri = "/body.access//door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
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
        let uri = "/body.access/1/door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
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
        let uri = "/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().message.as_ref().unwrap()
        );
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_with_instance_and_message(
    ) {
        let uri = "/body.access/1/door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
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
            "front_left",
            uri.resource.as_ref().unwrap().message.as_ref().unwrap()
        );
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_local_service_no_version() {
        let uri = "/petapp//rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
        assert_eq!("petapp", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
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
        let uri = "/petapp/1/rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(&uri.authority.unwrap()));
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
    fn test_parse_protocol_uri_with_remote_service_only_device_and_domain() {
        let uri = "//VCU.MY_CAR_VIN".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_only_device_and_cloud_domain() {
        let uri = "//cloud.uprotocol.example.com".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_none());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version() {
        let uri = "//VCU.MY_CAR_VIN/body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_no_version() {
        let uri = "//cloud.uprotocol.example.com/body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version() {
        let uri = "//VCU.MY_CAR_VIN/body.access/1".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_with_version() {
        let uri = "//cloud.uprotocol.example.com/body.access/1".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_name_only() {
        let uri = "//VCU.MY_CAR_VIN/body.access//door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_no_version_with_resource_name_only() {
        let uri = "//cloud.uprotocol.example.com/body.access//door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_name_only() {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_cloud_with_version_with_resource_name_only() {
        let uri = "//cloud.uprotocol.example.com/body.access/1/door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_none());
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_and_instance_no_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access//door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_and_instance_no_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri
            .authority
            .as_ref()
            .as_ref()
            .and_then(|a| a.remote.as_ref())
        {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
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
    fn test_parse_protocol_uri_with_remote_cloud_service_no_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//cloud.uprotocol.example.com/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
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
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU.MY_CAR_VIN", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
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
    fn test_parse_protocol_uri_with_remote_cloud_service_with_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//cloud.uprotocol.example.com/body.access/1/door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("cloud.uprotocol.example.com", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_some());
        assert_eq!("Door", uri.resource.unwrap().message.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_domain_with_version_with_resource_and_instance_no_message(
    ) {
        let uri = "//VCU/body.access/1/door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("VCU", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("door", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "front_left",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_remote_service_no_version() {
        let uri = "//bo.cloud/petapp//rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("bo.cloud", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("petapp", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("rpc", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "response",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.as_ref().unwrap().message.is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_remote_service_with_version() {
        let uri = "//bo.cloud/petapp/1/rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("bo.cloud", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("petapp", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
        assert!(uri.resource.is_some());
        assert_eq!("rpc", uri.resource.as_ref().unwrap().name);
        assert!(uri.resource.as_ref().unwrap().instance.is_some());
        assert_eq!(
            "response",
            uri.resource.as_ref().unwrap().instance.as_ref().unwrap()
        );
        assert!(uri.resource.unwrap().message.is_none());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_is_empty() {
        let uprotocol_uri = LongUriSerializer::serialize(&UUri::default());
        assert_eq!("", &uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_empty_use() {
        let entity = UEntity::default();
        let resource = UResource {
            name: "Door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(UAuthority::default()),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/////door", &uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version() {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: None,
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access//door", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1/door", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access//door.front_left", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1/door.front_left", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access//door.front_left#Door", &uprotocol_uri);
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1/door.front_left#Door", &uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version() {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let uri = UUri {
            entity: Some(entity),
            resource: None,
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access", &uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version() {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let uri = UUri {
            entity: Some(entity),
            resource: None,
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1", uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_cloud_authority_service_and_version() {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            remote: Some(Remote::Name("cloud.uprotocol.example.com".into())),
        };
        let uri = UUri {
            entity: Some(entity),
            resource: None,
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//cloud.uprotocol.example.com/body.access/1", uprotocol_uri);
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
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1/door", uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access//door", uprotocol_uri);
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
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door.front_left",
            uprotocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_cloud_authority_service_and_version_with_resource_with_instance_no_message(
    ) {
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let authority = UAuthority {
            remote: Some(Remote::Name("cloud.uprotocol.example.com".into())),
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };

        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//cloud.uprotocol.example.com/body.access/1/door.front_left",
            uprotocol_uri
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
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };

        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access//door.front_left",
            uprotocol_uri
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
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door.front_left#Door",
            uprotocol_uri
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
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access//door.front_left#Door",
            uprotocol_uri
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
            entity: Some(entity),
            resource: Some(resource),
            authority: None,
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/petapp/1/rpc.response", uprotocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_for_source_part_of_rpc_request_where_source_is_remote() {
        let entity = UEntity {
            name: "petapp".into(),
            ..Default::default()
        };
        let authority = UAuthority {
            remote: Some(Remote::Name("cloud.uprotocol.example.com".into())),
        };
        let resource = UResource {
            name: "rpc".into(),
            instance: Some("response".into()),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//cloud.uprotocol.example.com/petapp//rpc.response",
            uprotocol_uri
        );
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
            remote: Some(Remote::Name("vcu.my_car_vin".into())),
        };
        let resource = UResource {
            name: "door".into(),
            ..Default::default()
        };
        let uri = UUri {
            entity: Some(entity),
            resource: Some(resource),
            authority: Some(authority),
        };
        let uprotocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1/door", uprotocol_uri);
    }

    #[test]
    fn test_parse_local_protocol_uri_with_custom_scheme() {
        let uri = "custom:/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(!UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_none());
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(0, uri.entity.as_ref().unwrap().version_major.unwrap());
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
        let uri = LongUriSerializer::deserialize(uri);

        assert!(UriValidator::is_remote(uri.authority.as_ref().unwrap()));
        assert!(uri.authority.as_ref().is_some());
        assert!(uri.authority.as_ref().unwrap().remote.is_some());
        if let Some(Remote::Name(name)) = uri.authority.as_ref().and_then(|a| a.remote.as_ref()) {
            assert_eq!("vcu.vin", name);
        }
        assert!(uri.entity.is_some());
        assert_eq!("body.access", uri.entity.as_ref().unwrap().name);
        assert_eq!(1, uri.entity.as_ref().unwrap().version_major.unwrap());
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
    fn test_deserialize_long_and_micro_passing_empty_long_uri_empty_byte_array() {
        let uri = LongUriSerializer::build_resolved("", &[]);
        assert!(uri.is_some());
        assert_eq!("", LongUriSerializer::serialize(&uri.unwrap()));
    }
}
