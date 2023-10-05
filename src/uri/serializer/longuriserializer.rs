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

use crate::uri::datamodel::uauthority::UAuthority;
use crate::uri::datamodel::uentity::UEntity;
use crate::uri::datamodel::uresource::UResource;
use crate::uri::datamodel::uuri::UUri;
use crate::uri::serializer::uriserializer::UriSerializer;

/// UUri Serializer that serializes a UUri to a string (long format) per
///  https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc
pub struct LongUriSerializer;

impl UriSerializer<String> for LongUriSerializer {
    fn serialize(uri: &UUri) -> String {
        if uri.is_empty() {
            return "".to_string();
        }

        let mut uristring = String::new();
        uristring.push_str(&Self::build_authority_part_of_uri(&uri.authority));

        if uri.authority.is_marked_remote() {
            uristring.push('/');
        }

        if uri.entity.is_empty() {
            return uristring;
        }
        uristring.push_str(&Self::build_entity_part_of_uri(&uri.entity));
        uristring.push_str(&Self::build_resource_part_of_uri(&uri.resource));

        let re = Regex::new(r"/+$").unwrap();
        let uristring = re.replace_all(&uristring, "").to_string();

        uristring
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
            return UUri::EMPTY;
        }

        let mut uri: String = String::from(&uprotocol_uri);

        if let Some(index) = uprotocol_uri.find(':') {
            uri = uprotocol_uri[index + 1..].to_string();
        }
        // in the original code, this is in an else path with the if above... not sure about that
        uri = uri.replace('\\', "/");

        let is_local: bool = !uri.starts_with("//");
        let uri_parts = Self::java_split(&uri, "/");

        if uri_parts.len() < 2 {
            if is_local {
                return UUri::EMPTY;
            } else {
                return UUri::new(
                    Some(UAuthority::remote_device_domain(
                        "".to_string(),
                        "".to_string(),
                    )),
                    Some(UEntity::EMPTY),
                    Some(UResource::EMPTY),
                );
            }
        }

        let use_name: String;
        let mut use_version: Option<String> = None;
        let authority: UAuthority;
        let mut resource: UResource = UResource::EMPTY;

        if is_local {
            authority = UAuthority::LOCAL;
            use_name = uri_parts[1].to_string();
            if uri_parts.len() > 2 && !uri_parts[2].to_string().trim().is_empty() {
                use_version = Some(uri_parts[2].to_string());
            }
            if uri_parts.len() > 3 {
                resource = Self::parse_resource(&uri_parts[3]);
            }
        } else {
            let authority_parts: Vec<&str> = uri_parts[2].split('.').collect();
            let device: String = authority_parts[0].to_string();
            let mut domain: String = "".to_string();

            if authority_parts.len() > 1 {
                domain = authority_parts[1..].join(".");
            }
            authority = UAuthority::remote_device_domain(device, domain);

            if uri_parts.len() > 3 {
                use_name = uri_parts[3].to_string();
            } else {
                return UUri::new(
                    Some(authority),
                    Some(UEntity::EMPTY),
                    Some(UResource::EMPTY),
                );
            }
            if uri_parts.len() > 4 && !uri_parts[4].to_string().trim().is_empty() {
                use_version = Some(uri_parts[4].to_string());
            }
            if uri_parts.len() > 5 {
                resource = Self::parse_resource(&uri_parts[5]);
            }
        }

        UUri::new(
            Some(authority),
            Some(UEntity::long_format(use_name, use_version)),
            Some(resource),
        )
    }
}

impl LongUriSerializer {
    /// Creates the resource part of the uProtocol URI from a `UResource` object.
    ///
    /// # Parameters
    ///
    /// - `resource`: A `UResource` object representing a resource manipulated by a service, such as a Door.
    /// - `short_uri`: A boolean flag indicating whether to create a short-form URI.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the resource part of the uProtocol URI.
    fn build_resource_part_of_uri(resource: &UResource) -> String {
        if resource.is_empty() {
            return "".to_string();
        }

        let mut uri: String = String::from("/");
        uri.push_str(&resource.name);

        if let Some(instance) = &resource.instance {
            uri.push('.');
            uri.push_str(instance);
        }
        if let Some(message) = &resource.message {
            uri.push('#');
            uri.push_str(message);
        }

        uri
    }

    /// Creates the service part of the uProtocol URI from a `UEntity` object representing a service or an application.
    ///
    /// # Parameters
    ///
    /// - `entity`: A `UEntity` object that represents a service or an application.
    /// - `short_uri`: A boolean flag indicating whether to create a short-form URI.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the service part of the uProtocol URI.
    fn build_entity_part_of_uri(entity: &UEntity) -> String {
        let mut uri: String = String::from(entity.name.trim());
        uri.push('/');

        if let Some(version) = &entity.version {
            uri.push_str(version);
        }

        uri
    }

    /// Creates the authority part of the uProtocol URI from a `UAuthority` object.
    ///
    /// # Parameters
    ///
    /// - `authority`: A `UAuthority` object that represents the deployment location of a specific
    ///   Software Entity in the Ultiverse.
    /// - `short_uri`: A boolean flag indicating whether to create a short-form URI.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the authority part of the uProtocol URI.
    fn build_authority_part_of_uri(authority: &UAuthority) -> String {
        if authority.is_local() {
            return "/".to_string();
        }

        let mut uri: String = String::from("//");

        if let Some(device) = &authority.device {
            uri.push_str(device);

            if authority.domain.is_some() {
                uri.push('.');
            }
        }
        if let Some(domain) = &authority.domain {
            uri.push_str(domain);
        }

        uri
    }

    // TODO - this might be a from() in UResource...
    fn parse_resource(resource_string: &str) -> UResource {
        let parts: Vec<&str> = resource_string.split('#').collect();
        let name_and_instance: String = parts[0].to_string();
        let name_and_instance_parts: Vec<&str> = name_and_instance.split('.').collect();
        let resource_name: String = name_and_instance_parts[0].to_string();
        let resource_instance: String = if name_and_instance_parts.len() > 1 {
            name_and_instance_parts[1].to_string()
        } else {
            "".to_string()
        };
        let resource_message: String = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            "".to_string()
        };

        UResource::new(
            resource_name,
            Some(resource_instance),
            Some(resource_message),
            None,
            false,
        )
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

    #[test]
    fn test_parse_uprotocol_uri_when_is_null() {
        let uri = LongUriSerializer::deserialize("".to_string());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_when_is_empty_string() {
        let uuri = LongUriSerializer::deserialize("".to_string());
        assert!(uuri.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_slash() {
        let uri = LongUriSerializer::deserialize("/".to_string());
        assert!(uri.authority.is_local());
        assert!(!uri.authority.is_remote());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_double_slash() {
        let uri = LongUriSerializer::deserialize("//".to_string());
        assert!(uri.authority.is_local());
        assert!(uri.authority.marked_remote);
        assert!(uri.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_3_slash_and_something() {
        let uri = "///body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_4_slash_and_something() {
        let uri = "////body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(uri.authority.marked_remote);
        assert!(uri.entity.name.is_empty());
        assert!(uri.entity.version.is_some());
        assert_eq!("body.access", uri.entity.version.unwrap());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_5_slash_and_something() {
        let uri = "/////body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(uri.authority.marked_remote);
        assert!(uri.entity.is_empty());
        assert_eq!("body", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("access", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_schema_and_6_slash_and_something() {
        let uri = "//////body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(uri.authority.marked_remote);
        assert!(uri.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version() {
        let uri = "/body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version() {
        let uri = "/body.access/1".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_name_only() {
        let uri = "/body.access//door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_none());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_name_only() {
        let uri = "/body.access/1/door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_none());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_with_instance() {
        let uri = "/body.access//door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_with_message() {
        let uri = "/body.access/1/door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_no_version_with_resource_with_instance_and_message(
    ) {
        let uri = "/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_some());
        assert_eq!("Door", uri.resource.message.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_with_local_service_with_version_with_resource_with_instance_and_message(
    ) {
        let uri = "/body.access/1/door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("body.access", uri.entity.name);
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_some());
        assert_eq!("Door", uri.resource.message.unwrap());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_local_service_no_version() {
        let uri = "/petapp//rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("petapp", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("rpc", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("response", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_local_service_with_version() {
        let uri = "/petapp/1/rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_local());
        assert!(!uri.authority.marked_remote);
        assert_eq!("petapp", uri.entity.name);
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("rpc", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("response", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_only_device_no_domain() {
        let uri = "//VCU".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_remote());
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_none());
        assert!(uri.entity.is_empty());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_only_device_and_domain() {
        let uri = "//VCU.MY_CAR_VIN".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_remote());
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert!(uri.entity.is_empty());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_only_device_and_cloud_domain() {
        let uri = "//cloud.uprotocol.example.com".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_remote());
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert!(uri.entity.is_empty());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version() {
        let uri = "//VCU.MY_CAR_VIN/body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.is_remote());
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_no_version() {
        let uri = "//cloud.uprotocol.example.com/body.access".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version() {
        let uri = "//VCU.MY_CAR_VIN/body.access/1".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_with_version() {
        let uri = "//cloud.uprotocol.example.com/body.access/1".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_name_only() {
        let uri = "//VCU.MY_CAR_VIN/body.access//door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_none());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_no_version_with_resource_name_only() {
        let uri = "//cloud.uprotocol.example.com/body.access//door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_none());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_name_only() {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_none());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_cloud_with_version_with_resource_name_only() {
        let uri = "//cloud.uprotocol.example.com/body.access/1/door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_none());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_and_instance_no_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access//door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_and_instance_no_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_some());
        assert_eq!("Door", uri.resource.message.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_no_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//cloud.uprotocol.example.com/body.access//door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_some());
        assert_eq!("Door", uri.resource.message.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_with_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("my_car_vin", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_some());
        assert_eq!("Door", uri.resource.message.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_cloud_service_with_version_with_resource_and_instance_and_message(
    ) {
        let uri = "//cloud.uprotocol.example.com/body.access/1/door.front_left#Door".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("cloud", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("uprotocol.example.com", uri.authority.domain.unwrap());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_some());
        assert_eq!("Door", uri.resource.message.unwrap());
    }

    #[test]
    fn test_parse_protocol_uri_with_remote_service_no_domain_with_version_with_resource_and_instance_no_message(
    ) {
        let uri = "//VCU/body.access/1/door.front_left".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("vcu", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_none());
        assert_eq!("body.access", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("door", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("front_left", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_remote_service_no_version() {
        let uri = "//bo.cloud/petapp//rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("bo", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("cloud", uri.authority.domain.unwrap());
        assert_eq!("petapp", uri.entity.name);
        assert!(uri.entity.version.is_none());
        assert_eq!("rpc", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("response", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_parse_protocol_rpc_uri_with_remote_service_with_version() {
        let uri = "//bo.cloud/petapp/1/rpc.response".to_string();
        let uri = LongUriSerializer::deserialize(uri);
        assert!(uri.authority.marked_remote);
        assert!(uri.authority.device.is_some());
        assert_eq!("bo", uri.authority.device.unwrap());
        assert!(uri.authority.domain.is_some());
        assert_eq!("cloud", uri.authority.domain.unwrap());
        assert_eq!("petapp", uri.entity.name);
        assert!(uri.entity.version.is_some());
        assert_eq!("1", uri.entity.version.unwrap());
        assert_eq!("rpc", uri.resource.name);
        assert!(uri.resource.instance.is_some());
        assert_eq!("response", uri.resource.instance.unwrap());
        assert!(uri.resource.message.is_none());
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_is_empty() {
        let u_protocol_uri = LongUriSerializer::serialize(&UUri::EMPTY);
        // !!ATTENTION!! This is a deviation from the Java uProtocol SDK - the behavior of having an EMPTY UUri instance
        // initialize with a non-empty String (""), which is a problem with how Rust deals with const definitions vs Strings.
        // There are some potential workarounds using lazy initialization etc, which I did not want to go to just for this one
        // non-relevant test case, because all of these workarounds would massible increase code complexity/reduce clarity.
        //        assert_eq!("", &u_protocol_uri);
        assert_eq!("", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_empty_use() {
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(UEntity::EMPTY),
            Some(UResource::long_format("door".to_string())),
        );
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version() {
        let use_entity = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), None);
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version() {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), None);
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version_with_resource(
    ) {
        let use_entity = UEntity::long_format("body.access".to_string(), None);
        let resource = UResource::long_format("door".to_string());
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access//door", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version_with_resource(
    ) {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let resource = UResource::long_format("door".to_string());
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1/door", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version_with_resource_with_instance_no_message(
    ) {
        let use_entity = UEntity::new("body.access".to_string(), None, None, false);
        let resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access//door.front_left", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version_with_resource_with_instance_no_message(
    ) {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1/door.front_left", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_no_version_with_resource_with_instance_with_message(
    ) {
        let use_entity = UEntity::new("body.access".to_string(), None, None, false);
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access//door.front_left#Door", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_local_authority_service_and_version_with_resource_with_instance_with_message(
    ) {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(use_entity), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("/body.access/1/door.front_left#Door", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version() {
        let use_entity = UEntity::new("body.access".to_string(), None, None, false);
        let authority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let uri = UUri::new(Some(authority), Some(use_entity), None);
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access", &u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_no_device_with_domain_with_service_no_version(
    ) {
        let use_case = UEntity::long_format("body.access".to_string(), None);
        let uauthority = UAuthority::remote_device_domain("".to_string(), "MY_CAR_VIN".to_string());
        let uresource = UResource::EMPTY;
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(uresource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//my_car_vin/body.access", u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version() {
        let use_case = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let uresource = UResource::EMPTY;
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(uresource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1", u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_cloud_authority_service_and_version() {
        let use_case = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uauthority = UAuthority::remote_device_domain(
            "cloud".to_string(),
            "uprotocol.example.com".to_string(),
        );
        let uri = UUri::new(Some(uauthority), Some(use_case), None);
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//cloud.uprotocol.example.com/body.access/1",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version_with_resource(
    ) {
        let use_case = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let resource = UResource::long_format("door".to_string());
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1/door", u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource(
    ) {
        let use_case = UEntity::new("body.access".to_string(), None, None, false);
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let resource = UResource::long_format("door".to_string());
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access//door", u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version_with_resource_with_instance_no_message(
    ) {
        let use_case = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door.front_left",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_cloud_authority_service_and_version_with_resource_with_instance_no_message(
    ) {
        let use_case = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uauthority = UAuthority::remote_device_domain(
            "cloud".to_string(),
            "uprotocol.example.com".to_string(),
        );
        let resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//cloud.uprotocol.example.com/body.access/1/door.front_left",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource_with_instance_no_message(
    ) {
        let use_case = UEntity::new("body.access".to_string(), None, None, false);
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access//door.front_left",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_and_version_with_resource_with_instance_and_message(
    ) {
        let use_case = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/door.front_left#Door",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_uri_when_uri_has_remote_authority_service_no_version_with_resource_with_instance_and_message(
    ) {
        let use_case = UEntity::new("body.access".to_string(), None, None, false);
        let uauthority =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_CAR_VIN".to_string());
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        let uri = UUri::new(Some(uauthority), Some(use_case), Some(resource));
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!(
            "//vcu.my_car_vin/body.access//door.front_left#Door",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_for_source_part_of_rpc_request_where_source_is_local() {
        let uauthority = UAuthority::LOCAL;
        let use_case = UEntity::new("petapp".to_string(), Some("1".to_string()), None, false);
        let u_protocol_uri =
            LongUriSerializer::serialize(&UUri::rpc_response(uauthority, use_case));

        assert_eq!("/petapp/1/rpc.response", u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_for_source_part_of_rpc_request_where_source_is_remote() {
        let uauthority = UAuthority::remote_device_domain(
            "cloud".to_string(),
            "uprotocol.example.com".to_string(),
        );
        let use_case = UEntity::long_format("petapp".to_string(), None);
        let u_protocol_uri =
            LongUriSerializer::serialize(&UUri::rpc_response(uauthority, use_case));
        assert_eq!(
            "//cloud.uprotocol.example.com/petapp//rpc.response",
            u_protocol_uri
        );
    }

    #[test]
    fn test_build_protocol_uri_from_parts_when_they_are_none() {
        let uauthority = None;
        let use_case = None;
        let uresource = None;
        let uri = UUri::new(uauthority, use_case, uresource);
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("", u_protocol_uri);
    }

    #[test]
    fn test_build_protocol_uri_from_parts_when_uri_has_remote_authority_service_and_version_with_resource(
    ) {
        let uauthority = Some(UAuthority::remote_device_domain(
            "VCU".to_string(),
            "MY_CAR_VIN".to_string(),
        ));
        let use_case = Some(UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        ));
        let uresource = Some(UResource::long_format("door".to_string()));
        let uri = UUri::new(uauthority, use_case, uresource);
        let u_protocol_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1/door", u_protocol_uri);
    }

    #[test]
    fn test_custom_scheme_no_scheme_empty() {
        let authority: Option<UAuthority> = None;
        let entity: Option<UEntity> = None;
        let resource: Option<UResource> = None;
        let custom_uri = UUri::new(authority, entity, resource);
        assert!(custom_uri.is_empty());
    }

    #[test]
    fn test_custom_scheme_no_scheme() {
        let authority = Some(UAuthority::remote_device_domain(
            "VCU".to_string(),
            "MY_CAR_VIN".to_string(),
        ));
        let entity = Some(UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        ));
        let resource = Some(UResource::long_format("door".to_string()));
        let uri = UUri::new(authority, entity, resource);
        let custom_uri = LongUriSerializer::serialize(&uri);
        assert_eq!("//vcu.my_car_vin/body.access/1/door", custom_uri);
    }

    #[test]
    fn test_parse_local_protocol_uri_with_custom_scheme() {
        let uri = "custom:/body.access//door.front_left#Door".to_string();
        let parsed_uri = LongUriSerializer::deserialize(uri);

        // Check if the UAuthority is local and not marked as remote
        assert!(parsed_uri.authority.is_local());
        assert!(!parsed_uri.authority.is_marked_remote());

        // Validate UEntity details
        let entity = parsed_uri.entity;
        assert_eq!("body.access", entity.name);
        assert!(entity.version.is_none());

        // Validate UResource details
        let resource = parsed_uri.resource;
        assert_eq!("door", resource.name);
        assert!(resource.instance.is_some());
        assert_eq!("front_left", resource.instance.unwrap());
        assert!(resource.message.is_some());
        assert_eq!("Door", resource.message.unwrap());
    }

    #[test]
    fn test_parse_remote_protocol_uri_with_custom_scheme() {
        let uri = "custom://vcu.vin/body.access//door.front_left#Door".to_string();
        let uri2 = "//vcu.vin/body.access//door.front_left#Door".to_string();
        let parsed_uri = LongUriSerializer::deserialize(uri);

        // Check if the UAuthority is remote and marked as remote
        assert!(!parsed_uri.authority.is_local());
        assert!(parsed_uri.authority.is_marked_remote());

        // Validate UAuthority details
        assert_eq!("vcu", parsed_uri.authority.device.as_ref().unwrap());
        assert!(parsed_uri.authority.domain.is_some());
        assert_eq!("vin", parsed_uri.authority.domain.as_ref().unwrap());

        // Validate UEntity details
        let entity = parsed_uri.clone().entity;
        assert_eq!("body.access", entity.name);
        assert!(entity.version.is_none());

        // Validate UResource details
        let resource = parsed_uri.clone().resource;
        assert_eq!("door", resource.name);
        assert!(resource.instance.is_some());
        assert_eq!("front_left", resource.instance.unwrap());
        assert!(resource.message.is_some());
        assert_eq!("Door", resource.message.unwrap());

        // Validate that the parsed URI equals uri2
        assert_eq!(uri2, parsed_uri.to_string());
    }
}
