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

use crate::uri::datamodel::{UAuthority, UEntity, UResource, UUri};
use crate::uri::serializer::uriserializer::UriSerializer;

const SCHEME: &str = "s:";

/// UUri Serializer that serializes a UUri to a short format per
/// <https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc>
pub struct ShortUriSerializer;

impl UriSerializer<String> for ShortUriSerializer {
    /// Serializes a `UUri` object into its short URI format.
    ///
    /// # Arguments
    ///
    /// * `uri` - A `UUri` object to be serialized to the short URI format.
    ///
    /// # Returns
    ///
    /// The short URI formatted string of the supplied `UUri` that can be used as a
    /// sink or a source in a uProtocol publish communication.
    fn serialize(uri: &UUri) -> String {
        if uri.is_empty() {
            return String::from("");
        }

        let mut sb = String::from(SCHEME);

        sb.push_str(&Self::build_authority_part_of_uri(&uri.authority));

        if uri.authority.is_marked_remote() {
            sb.push('/');
        }

        if uri.entity.is_empty() {
            return sb;
        }

        sb.push_str(&Self::build_software_entity_part_of_uri(&uri.entity));
        sb.push_str(&Self::build_resource_part_of_uri(&uri.resource));

        return sb.trim_end_matches('/').to_string();
    }

    /// Deserializes a short-formatted string into a `UUri` object.
    ///
    /// # Arguments
    ///
    /// * `uri` - A short format uProtocol URI string.
    ///
    /// # Returns
    ///
    /// A `UUri` data object constructed from the provided string.
    fn deserialize(uri: String) -> UUri {
        if uri.is_empty() || !uri.contains(SCHEME) {
            return UUri::EMPTY;
        }

        let uri = uri
            .split_at(uri.find(':').unwrap_or(0) + 1)
            .1
            .replace('\\', "/");
        let is_local = !uri.starts_with("//");

        let uri_parts: Vec<&str> = uri.split('/').collect();

        if uri_parts.len() <= 2 {
            return if is_local {
                UUri::EMPTY
            } else {
                UUri::new(
                    Some(UAuthority::long_remote("".to_string(), "".to_string())),
                    Some(UEntity::EMPTY),
                    Some(UResource::EMPTY),
                )
            };
        }

        let authority_parts: Vec<&str> = uri_parts[2].split('.').collect();
        let device = authority_parts[0];
        let domain = authority_parts[1..].join(".");

        let u_authority = UAuthority::long_remote(device.to_string(), domain.to_string());

        let use_name = if uri_parts.len() > 3 {
            uri_parts[3]
        } else {
            return UUri::new(
                Some(u_authority),
                Some(UEntity::EMPTY),
                Some(UResource::EMPTY),
            );
        };

        let use_version = if uri_parts.len() > 4 {
            uri_parts[4]
        } else {
            ""
        };

        let u_resource = if uri_parts.len() > 5 {
            match uri_parts[5].parse::<u16>() {
                Ok(id) => UResource::micro_format(id),
                Err(_) => {
                    return UUri::EMPTY;
                }
            }
        } else {
            UResource::EMPTY
        };

        let mut parsed_version: Option<u32> = None;
        if !use_version.trim().is_empty() {
            match use_version.parse::<u32>() {
                Ok(parsed) => {
                    parsed_version = Some(parsed);
                }
                Err(_) => {
                    return UUri::EMPTY;
                }
            }
        }

        let mut parsed_id: Option<u16> = None;
        if !use_name.trim().is_empty() {
            match use_name.parse::<u16>() {
                Ok(parsed) => {
                    parsed_id = Some(parsed);
                }
                Err(_) => {
                    return UUri::EMPTY;
                }
            }
        }

        UUri::new(
            Some(u_authority),
            Some(UEntity::micro_format(parsed_id, parsed_version)),
            Some(u_resource),
        )
    }
}

impl ShortUriSerializer {
    fn build_resource_part_of_uri(u_resource: &UResource) -> String {
        if u_resource.is_empty() || !u_resource.is_micro_form() {
            return String::from("");
        }

        let mut sb = String::from("/");
        if let Some(id) = u_resource.id {
            sb.push_str(&id.to_string());
        }

        sb
    }

    fn build_software_entity_part_of_uri(entity: &UEntity) -> String {
        let mut sb = String::new();
        if let Some(id) = entity.id {
            sb.push_str(&id.to_string());
        }
        sb.push('/');
        if let Some(version) = &entity.version {
            sb.push_str(&version.to_string());
        }
        sb
    }

    fn build_authority_part_of_uri(authority: &UAuthority) -> String {
        if authority.is_local() {
            return String::from("/");
        }
        let mut partial_uri = String::from("//");
        if let Some(device) = &authority.device {
            partial_uri.push_str(device);
            if authority.domain.is_some() {
                partial_uri.push('.');
            }
        }
        if let Some(domain) = &authority.domain {
            partial_uri.push_str(domain);
        }
        partial_uri
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_empty_uuri() {
        let str_uri = ShortUriSerializer::serialize(&UUri::EMPTY);
        assert_eq!("", str_uri);
    }

    // There is no null in Rust
    // #[test]
    // fn test_serialize_none_uuri() {}

    #[test]
    fn test_serialize_resolved_local_uuri() {
        let u_entity = UEntity::resolved_format("hartley".to_string(), 1, 2);
        let u_resource = UResource::resolved_format(
            "salary".to_string(),
            "raise".to_string(),
            "Salary".to_string(),
            4,
        );
        let uri = UUri::new(Some(UAuthority::LOCAL), Some(u_entity), Some(u_resource));
        let str_uri = ShortUriSerializer::serialize(&uri);
        assert!(!str_uri.is_empty());
        assert_eq!("s:/2/1/4", str_uri);
    }

    #[test]
    fn test_serialize_resolved_remote_uuri() {
        let u_authority = UAuthority::long_remote("vcu".to_string(), "vin".to_string());
        let u_entity = UEntity::resolved_format("hartley".to_string(), 1, 2);
        let u_resource = UResource::resolved_format(
            "salary".to_string(),
            "raise".to_string(),
            "Salary".to_string(),
            4,
        );
        let uri = UUri::new(Some(u_authority), Some(u_entity), Some(u_resource));
        let str_uri = ShortUriSerializer::serialize(&uri);
        assert!(!str_uri.is_empty());
        assert_eq!("s://vcu.vin/2/1/4", str_uri);
    }

    #[test]
    fn test_serialize_missing_authority_names() {
        let u_authority = UAuthority::long_remote("".to_string(), "".to_string());
        let u_entity = UEntity::micro_format(Some(2), Some(1));
        let u_resource = UResource::micro_format(4);
        let uri = UUri::new(Some(u_authority), Some(u_entity), Some(u_resource));
        let str_uri = ShortUriSerializer::serialize(&uri);
        assert_eq!("s://2/1/4", str_uri);
    }

    #[test]
    fn test_serialize_empty_uentity() {
        let u_authority = UAuthority::LOCAL;
        let u_entity = UEntity::EMPTY;
        let u_resource = UResource::micro_format(4);
        let uri = UUri::new(Some(u_authority), Some(u_entity), Some(u_resource));
        let str_uri = ShortUriSerializer::serialize(&uri);
        assert_eq!("s:/", str_uri);
    }

    #[test]
    fn test_serialize_empty_uresource() {
        let u_authority = UAuthority::LOCAL;
        let u_entity = UEntity::micro_format(Some(2), Some(1));
        let u_resource = UResource::EMPTY;
        let uri = UUri::new(Some(u_authority), Some(u_entity), Some(u_resource));
        let str_uri = ShortUriSerializer::serialize(&uri);
        assert_eq!("s:/2/1", str_uri);
    }

    #[test]
    fn test_serialize_missing_uresource_id() {
        let u_authority = UAuthority::LOCAL;
        let u_entity = UEntity::micro_format(Some(2), Some(1));
        let u_resource = UResource::long_format("raise".to_string());
        let uri = UUri::new(Some(u_authority), Some(u_entity), Some(u_resource));
        let str_uri = ShortUriSerializer::serialize(&uri);
        assert_eq!("s:/2/1", str_uri);
    }

    // No null in Rust
    // #[test]
    // fn test_deserialize_null_string() {}

    #[test]
    fn test_deserialize_empty_string() {
        let uri = ShortUriSerializer::deserialize("".to_string());
        assert_eq!(UUri::EMPTY, uri);
    }

    #[test]
    fn test_deserialize_string_with_no_slashes() {
        let uri = ShortUriSerializer::deserialize("abc".to_string());
        assert_eq!(UUri::EMPTY, uri);
    }

    #[test]
    fn test_deserialize_string_with_one_slash() {
        let uri = ShortUriSerializer::deserialize("s:/".to_string());
        assert_eq!(UUri::EMPTY, uri);
    }

    #[test]
    fn test_deserialize_string_with_two_slashes() {
        let uri = ShortUriSerializer::deserialize("s://".to_string());
        assert!(uri.is_empty());
        assert!(uri.authority.is_marked_remote());
    }

    #[test]
    fn test_deserialize_string_with_three_slashes() {
        let uri = ShortUriSerializer::deserialize("s:///".to_string());
        assert!(uri.is_empty());
        assert!(uri.authority.is_marked_remote());
    }

    #[test]
    fn test_deserialize_string_with_four_slashes() {
        let uri = ShortUriSerializer::deserialize("s:////".to_string());
        assert!(uri.is_empty());
        assert!(uri.authority.is_marked_remote());
    }

    #[test]
    fn test_deserialize_string_without_any_parts() {
        let uri = ShortUriSerializer::deserialize("s:".to_string());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_only_device_part_of_authority_and_no_entity_or_resource() {
        let uri = ShortUriSerializer::deserialize("s://vcu".to_string());
        assert!(!uri.is_empty());
        assert!(uri.authority.is_marked_remote());
        assert_eq!(Some("vcu".to_string()), uri.authority.device);
        assert_eq!(None, uri.authority.domain);
        assert!(uri.entity.is_empty());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_authority_and_no_entity_or_resource() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin".to_string());
        assert!(!uri.is_empty());
        assert!(uri.authority.is_marked_remote());
        assert_eq!(Some("vcu".to_string()), uri.authority.device);
        assert_eq!(Some("vin".to_string()), uri.authority.domain);
        assert!(uri.entity.is_empty());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_authority_and_entity_and_no_resource() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin/2/1".to_string());
        assert!(!uri.is_empty());
        assert!(uri.authority.is_marked_remote());
        assert_eq!(Some("vcu".to_string()), uri.authority.device);
        assert_eq!(Some("vin".to_string()), uri.authority.domain);
        assert!(!uri.entity.is_empty());
        assert_eq!(Some(2), uri.entity.id);
        assert_eq!(Some(1), uri.entity.version);
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_authority_and_missing_entity_with_resource() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin///2".to_string());
        assert!(!uri.is_empty());
        assert!(uri.authority.is_marked_remote());
        assert_eq!(Some("vcu".to_string()), uri.authority.device);
        assert_eq!(Some("vin".to_string()), uri.authority.domain);
        assert!(uri.entity.is_empty());
        assert!(!uri.resource.is_empty());
        assert_eq!(Some(2), uri.resource.id);
    }

    #[test]
    fn test_deserialize_remote_string_without_scheme() {
        let uri = ShortUriSerializer::deserialize("//vcu.vin/2/1/2".to_string());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_deserialize_local_string_without_scheme() {
        let uri = ShortUriSerializer::deserialize("/2/1/2".to_string());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_deserialize_remote_string_with_uentityid_without_version() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin/2".to_string());
        assert!(!uri.is_empty());
        assert!(uri.authority.is_marked_remote());
        assert_eq!(Some("vcu".to_string()), uri.authority.device);
        assert_eq!(Some("vin".to_string()), uri.authority.domain);
        assert!(!uri.entity.is_empty());
        assert_eq!(Some(2), uri.entity.id);
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_invalid_uentity_id() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin/abc/1/2".to_string());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_invalid_uentity_version() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin/2/abc/2".to_string());
        assert!(uri.is_empty());
    }

    #[test]
    fn test_deserialize_string_with_invalid_uresource_id() {
        let uri = ShortUriSerializer::deserialize("s://vcu.vin/2/1/abc".to_string());
        assert!(uri.is_empty());
    }
}
