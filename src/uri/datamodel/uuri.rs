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

#![allow(unused)]

use crate::uri::datamodel::uauthority::UAuthority;
use crate::uri::datamodel::uentity::UEntity;
use crate::uri::datamodel::uresource::UResource;

use regex::Regex;
use std::fmt;
use std::net::IpAddr;

/// `UUri` is a data representation of a URI (Uniform Resource Identifier).
///
/// This struct is used to represent the source and sink (destination) parts of the Packet CloudEvent.
/// URIs are used as a method to uniquely identify devices, services, and resources on the network.
/// Defining a common URI for the system allows applications and/or services to publish and discover each other,
/// as well as maintain a database/repository of microservices in various vehicles.
///
/// # Example
///
/// ```ignore
/// //<device>.<domain>/<service>/<version>/<resource>#<message>
/// ```
///
/// # Components
///
/// - `scheme`: The scheme component of the URI, for example, `http`, `ftp`, `file`, etc.
/// - `authority`: The authority component of the URI, typically the domain name or IP address.
/// - `entity`: The entity component represents the service and its version.
/// - `resource`: The resource component represents the specific resource within the service.
#[derive(Default, PartialEq, Clone)]
pub struct UUri {
    // pub scheme: String,
    pub authority: UAuthority,
    pub entity: UEntity,
    pub resource: UResource,
    pub id: Option<u32>,
}

impl UUri {
    // pub const SCHEME: &'static str = "up:";

    /// An empty `UUri` instance.
    ///
    /// This is used as a replacement for None values, and doesn't contain any information.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uuri::UUri;
    /// let empty_uri = UUri::EMPTY;
    /// ```
    pub const EMPTY: UUri = UUri {
        // scheme: String::new(),
        authority: UAuthority::EMPTY,
        entity: UEntity::EMPTY,
        resource: UResource::EMPTY,
        id: None,
    };

    /// Create a full URI.
    ///
    /// The `UUri` struct represents a URI in the Ultiverse system.
    ///
    /// # Arguments
    ///
    /// * `authority` - The `UAuthority` represents the deployment location of a specific Software Entity in the Ultiverse.
    /// * `entity` - The `UEntity` is the Software Entity in the role of a service or an application.
    /// * `resource` - The `UResource` is something that is manipulated by a service, such as a Door.
    ///
    /// # Returns
    ///
    /// This function returns a `UUri` instance.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// use uprotocol_sdk::uri::datamodel::uentity::UEntity;
    /// use uprotocol_sdk::uri::datamodel::uresource::UResource;
    /// use uprotocol_sdk::uri::datamodel::uuri::UUri;
    ///
    /// let authority = UAuthority::remote_device_domain("VCU".to_string(), "MY_VIN".to_string());
    /// let entity = UEntity::new("body.access".to_string(), Some("1".to_string()), None, false);
    /// let resource = UResource::new("door".to_string(), Some("front_left".to_string()), None, None, false);
    ///
    /// let uri = UUri::new(Some(authority), Some(entity), Some(resource));
    /// ```
    pub fn new(
        authority: Option<UAuthority>,
        entity: Option<UEntity>,
        resource: Option<UResource>,
    ) -> Self {
        UUri {
            // scheme: String::from(Self::SCHEME),
            authority: authority.unwrap_or(UAuthority::EMPTY),
            entity: entity.unwrap_or(UEntity::EMPTY),
            resource: resource.unwrap_or(UResource::EMPTY),
            id: None,
        }
    }

    /// Creates a UUri for an RPC Response using the given Authority and Entity information.
    ///
    /// # Arguments
    ///
    /// * `authority` - The `UAuthority` represents the deployment location of a specific Software Entity.
    /// * `entity` - The `UEntity` provides information about the Software Entity.
    ///
    /// # Returns
    ///
    /// Returns a `UUri` constructed for an RPC Response.
    pub fn rpc_response(authority: UAuthority, entity: UEntity) -> Self {
        Self::new(Some(authority), Some(entity), Some(UResource::response()))
    }

    /// Determines whether this `UUri` is an empty container without any valuable information
    /// for building uProtocol sinks or sources.
    ///
    /// Returns `true` if this `UUri` is an empty container without any valuable information
    /// for building uProtocol sinks or sources.
    pub fn is_empty(&self) -> bool {
        self.authority.is_local() && self.entity.is_empty() && self.resource.is_empty()
    }

    /// Returns `true` if the URI contains both names and numeric representations of the names.
    ///
    /// # Returns
    ///
    /// Returns `true` if the URI contains both names and numeric representations of the names.
    pub fn is_resolved(&self) -> bool {
        self.authority.is_resolved() && self.entity.is_resolved() && self.resource.is_resolved()
    }

    /// Checks if the `UEntity` and `UResource` contain Long form URI information (names).
    ///
    /// # Returns
    ///
    /// Returns `true` if the `UEntity` and `UResource` contain Long form URI information (names).
    pub fn is_long_form(&self) -> bool {
        self.is_resolved() || self.entity.is_long_form() && self.resource.is_long_form()
    }
}

impl fmt::Display for UUri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.is_empty() {
            // let mut uri: String = self.scheme.clone();
            let mut uri: String = String::new();

            uri.push_str(&self.authority.to_string());

            if self.authority.is_remote() {
                uri.push('/');
            }
            if self.entity.is_empty() {
                return write!(f, "{}", uri);
            } else {
                uri.push_str(&self.entity.to_string());
                // uri.push_str("/");
            }
            if !self.resource.is_empty() {
                uri.push_str(&self.resource.to_string());
            }

            let re = Regex::new(r"/+$").unwrap();
            write!(f, "{}", re.replace_all(&uri, ""))
        } else {
            // write!(f, "{}", self.scheme)
            write!(f, "")
        }
    }
}

impl fmt::Debug for UUri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UUri {{ UAuthority: {:?}, UEntity: {:?}, UResource: {:?} }}",
            self.authority, self.entity, self.resource
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::IpAddr::{V4, V6};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_to_string() {
        let u_authority_local = UAuthority::LOCAL;
        let u_authority_remote =
            UAuthority::remote_device_domain("VCU".to_string(), "MY_VIN".to_string());
        let u_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let u_resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );

        let uri = UUri::new(
            Some(u_authority_local),
            Some(u_entity.clone()),
            Some(u_resource.clone()),
        );
        let expected = "UUri { UAuthority: UAuthority { device: '', domain: '', address: '', marked_remote: false }, UEntity: UEntity { name: 'body.access', version: '1', id: 'unknown', marked_resolved: 'false' }, UResource: UResource { name: 'door', instance: 'front_left', message: '', id: 'unknown' } }";
        let s_uri = format!("{:?}", uri);
        assert_eq!(expected, s_uri);

        let uri_remote = UUri::new(
            Some(u_authority_remote.clone()),
            Some(u_entity.clone()),
            Some(u_resource.clone()),
        );
        let expected_remote = "UUri { UAuthority: UAuthority { device: 'vcu', domain: 'my_vin', address: '', marked_remote: true }, UEntity: UEntity { name: 'body.access', version: '1', id: 'unknown', marked_resolved: 'false' }, UResource: UResource { name: 'door', instance: 'front_left', message: '', id: 'unknown' } }";
        let s_uri_remote = format!("{:?}", uri_remote);
        assert_eq!(expected_remote, s_uri_remote);

        let uri2 = UUri::new(
            Some(u_authority_remote),
            Some(u_entity),
            Some(UResource::EMPTY),
        );
        let expected_uri2 = "UUri { UAuthority: UAuthority { device: 'vcu', domain: 'my_vin', address: '', marked_remote: true }, UEntity: UEntity { name: 'body.access', version: '1', id: 'unknown', marked_resolved: 'false' }, UResource: UResource { name: '', instance: '', message: '', id: 'unknown' } }";
        let s_uri2 = format!("{:?}", uri2);
        assert_eq!(expected_uri2, s_uri2);
    }

    #[test]
    fn test_create_full_local_uri() {
        let u_authority = UAuthority::LOCAL;
        let use_entity = UEntity::long_format("body.access".to_string(), None);
        let u_resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );

        let uri = UUri::new(
            Some(u_authority.clone()),
            Some(use_entity.clone()),
            Some(u_resource.clone()),
        );

        assert_eq!(u_authority, uri.authority);
        assert_eq!(use_entity, uri.entity);
        assert_eq!(u_resource, uri.resource);
    }

    #[test]
    fn test_create_full_remote_uri() {
        let u_authority = UAuthority::remote_device_domain("VCU".to_string(), "MY_VIN".to_string());
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let u_resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );

        let uri = UUri::new(
            Some(u_authority.clone()),
            Some(use_entity.clone()),
            Some(u_resource.clone()),
        );

        assert_eq!(u_authority, uri.authority);
        assert_eq!(use_entity, uri.entity);
        assert_eq!(u_resource, uri.resource);
    }

    #[test]
    fn test_create_uri_no_message_with_constructor() {
        let u_authority = UAuthority::remote_device_domain("VCU".to_string(), "MY_VIN".to_string());
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let u_resource = UResource::long_format("door".to_string());

        let uri = UUri::new(
            Some(u_authority.clone()),
            Some(use_entity.clone()),
            Some(u_resource.clone()),
        );

        assert_eq!(u_authority, uri.authority);
        assert_eq!(use_entity, uri.entity);
        assert_eq!(u_resource, uri.resource);
    }

    #[test]
    fn test_create_uri_null_authority() {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let u_resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );

        let uri = UUri::new(None, Some(use_entity.clone()), Some(u_resource.clone()));

        assert_eq!(UAuthority::EMPTY, uri.authority);
    }

    #[test]
    fn test_create_uri_null_use() {
        let u_authority = UAuthority::remote_device_domain("VCU".to_string(), "MY_VIN".to_string());
        let u_resource = UResource::long_format_with_instance(
            "door".to_string(),
            None,
            "front_left".to_string(),
        );

        let uri = UUri::new(Some(u_authority.clone()), None, Some(u_resource.clone()));

        assert_eq!(UEntity::EMPTY, uri.entity);
    }

    #[test]
    fn test_create_uri_null_u_resource() {
        let u_authority = UAuthority::remote_device_domain("VCU".to_string(), "MY_VIN".to_string());
        let u_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let u_resource = UResource::EMPTY;

        let uri = UUri::new(
            Some(u_authority.clone()),
            Some(u_entity.clone()),
            Some(u_resource.clone()),
        );

        assert_eq!(UResource::EMPTY, uri.resource);
    }

    #[test]
    fn test_create_empty_using_empty() {
        let uri = UUri::EMPTY;

        assert!(uri.authority.is_local());
        assert!(uri.entity.is_empty());
        assert!(uri.resource.is_empty());
    }

    #[test]
    fn test_is_empty() {
        let uri = UUri::EMPTY;
        assert!(uri.is_empty());

        let authority = UAuthority::EMPTY;
        let entity = UEntity::EMPTY;
        let resource = UResource::EMPTY;

        let uri2 = UUri::new(Some(authority), Some(entity), Some(resource));
        assert!(uri2.is_empty());
    }

    #[test]
    fn test_is_resolved_and_is_long_form() {
        let uri = UUri::EMPTY;
        assert!(!uri.is_resolved());
        assert!(!uri.is_long_form());

        let uri2 = UUri::new(
            Some(UAuthority::LOCAL),
            Some(UEntity::long_format("Hartley".to_string(), None)),
            Some(UResource::for_rpc_request(Some("Raise".to_string()), None)),
        );
        assert!(!uri2.is_resolved());
        assert!(uri2.is_long_form());

        let uri3 = UUri::new(
            Some(UAuthority::LOCAL),
            Some(UEntity::long_format("Hartley".to_string(), None)),
            Some(UResource::new(
                "Raise".to_string(),
                Some("Salary".into()),
                Some("Bonus".into()),
                Some(1),
                false,
            )),
        );
        assert!(!uri3.is_resolved());
        assert!(uri3.is_long_form());

        let uri4 = UUri::new(
            Some(UAuthority::LOCAL),
            Some(UEntity::new("Hartley".to_string(), None, Some(2), false)),
            Some(UResource::new(
                "Raise".to_string(),
                Some("Salary".into()),
                Some("Bonus".into()),
                Some(1),
                false,
            )),
        );
        assert!(uri4.is_resolved());
        assert!(uri4.is_long_form());

        let uri5 = UUri::new(
            Some(UAuthority::remote_device_domain(
                "vcu".to_string(),
                "vin".to_string(),
            )),
            Some(UEntity::long_format("Hartley".to_string(), None)),
            Some(UResource::for_rpc_request(Some("Raise".to_string()), None)),
        );
        assert!(!uri5.is_resolved());
        assert!(uri5.is_long_form());

        let uri6 = UUri::new(
            Some(UAuthority::remote_device_domain(
                "vcu".to_string(),
                "vin".to_string(),
            )),
            Some(UEntity::long_format("Hartley".to_string(), None)),
            Some(UResource::new(
                "Raise".to_string(),
                Some("Salary".into()),
                Some("Bonus".into()),
                Some(1),
                false,
            )),
        );
        assert!(!uri6.is_resolved());
        assert!(uri6.is_long_form());

        // There is probably a mistake in the Java test here - it's identical to #6
        let uri7 = uri6.clone();
        assert!(!uri7.is_resolved());
        assert!(uri7.is_long_form());

        let uri8 = UUri::new(
            Some(UAuthority::remote(
                Some("vcu".to_string()),
                Some("vin".to_string()),
                Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))),
            )),
            Some(UEntity::long_format("Hartley".to_string(), None)),
            Some(UResource::for_rpc_request(Some("Raise".to_string()), None)),
        );
        assert!(!uri8.is_resolved());
        assert!(uri8.is_long_form());

        let uri9 = UUri::new(
            Some(UAuthority::remote(
                Some("vcu".to_string()),
                Some("vin".to_string()),
                Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))),
            )),
            Some(UEntity::long_format("Hartley".to_string(), None)),
            Some(UResource::new(
                "Raise".to_string(),
                Some("Salary".into()),
                Some("Bonus".into()),
                Some(1),
                false,
            )),
        );
        assert!(!uri9.is_resolved());
        assert!(uri9.is_long_form());

        let uri10 = UUri::new(
            Some(UAuthority::remote(
                Some("vcu".to_string()),
                Some("vin".to_string()),
                Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))),
            )),
            Some(UEntity::new("Hartley".to_string(), None, Some(2), false)),
            Some(UResource::new(
                "Raise".to_string(),
                Some("Salary".into()),
                Some("Bonus".into()),
                Some(1),
                false,
            )),
        );
        assert!(uri10.is_resolved());
        assert!(uri10.is_long_form());
    }
}
