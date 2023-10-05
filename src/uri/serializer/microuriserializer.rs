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

use crate::uri::datamodel::uauthority::UAuthority;
use crate::uri::datamodel::uentity::UEntity;
use crate::uri::datamodel::uresource::UResource;
use crate::uri::datamodel::uuri::UUri;
use crate::uri::serializer::uriserializer::UriSerializer;

use byteorder::{BigEndian, WriteBytesExt};
use std::io::Cursor;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// UUri Serializer that serializes a UUri to byte[] (micro format) per
///  https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc
pub struct MicroUriSerializer;

impl UriSerializer<Vec<u8>> for MicroUriSerializer {
    /// Build a Micro-URI `Vec<u8>` using a passed `UUri` object.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI data object.
    ///
    /// # Returns
    ///
    /// Returns the short form uProtocol URI as a `Vec<u8>` from a `UUri` data object.
    ///    #[allow(arithmetic_overflow)]
    fn serialize(uri: &UUri) -> Vec<u8> {
        if uri.is_empty() || uri.entity.id.is_none() || uri.resource.id.is_none() {
            return vec![];
        }

        let mut cursor = Cursor::new(Vec::new());
        let address = uri.authority.inet_address;

        // UP_VERSION
        cursor.write_u8(0x1).unwrap();

        // TYPE
        if uri.authority.is_local() {
            cursor.write_u8(0x0).unwrap();
        } else {
            cursor
                .write_u8(match address.unwrap() {
                    IpAddr::V4(_) => 1,
                    IpAddr::V6(_) => 2,
                })
                .unwrap();
        }

        // URESOURCE_ID
        cursor
            .write_u16::<BigEndian>(uri.resource.id.unwrap())
            .unwrap();

        // UAUTHORITY_ADDRESS
        if !uri.authority.is_local() {
            match address.unwrap() {
                IpAddr::V4(addr) => cursor.write_all(&addr.octets()).unwrap(),
                IpAddr::V6(addr) => cursor.write_all(&addr.octets()).unwrap(),
            }
        }

        // UENTITY_ID
        cursor
            .write_u16::<BigEndian>(uri.entity.id.unwrap())
            .unwrap();

        // UENTITY_VERSION
        if let Some(version) = uri.entity.version.clone() {
            let parts: Vec<&str> = version.split('.').collect();
            if parts.len() > 1 {
                let mut major = parts[0].parse::<u8>().unwrap() << 3;
                let mut minor = (parts[1].parse::<u16>().unwrap() >> 8) as u8;

                major += minor;
                minor = parts[1].parse::<u16>().unwrap() as u8;

                cursor.write_u8(major).unwrap();
                cursor.write_u8(minor).unwrap();
            } else {
                let major = parts[0].parse::<u8>().unwrap() << 3;
                cursor.write_u8(major).unwrap();
                cursor.write_u8(0).unwrap();
            }
        } else {
            cursor.write_u16::<BigEndian>(std::i16::MAX as u16).unwrap();
        }
        cursor.into_inner()
    }

    /// Creates a `UUri` data object from a uProtocol micro URI.
    ///
    /// # Arguments
    ///
    /// * `micro_uri` - A byte slice representing the uProtocol micro URI.
    ///
    /// # Returns
    ///
    /// Returns a `UUri` data object.
    fn deserialize(micro_uri: Vec<u8>) -> UUri {
        if micro_uri.is_empty() || micro_uri.len() < 8 {
            return UUri::EMPTY;
        }

        // Need to be version 1
        if micro_uri[0] != 0x1 {
            return UUri::EMPTY;
        }

        let u_resource_id = u16::from_be_bytes(micro_uri[2..4].try_into().unwrap());

        let address_type = AddressType::try_from(micro_uri[1] as i32)
            .expect("Address type decoding should have worked");

        let mut index = 4;
        let authority: UAuthority;

        match address_type {
            AddressType::IPv4 => {
                let slice: [u8; 4] = micro_uri[index..index + 4]
                    .try_into()
                    .expect("Wrong slice length");

                let address = IpAddr::V4(Ipv4Addr::from(slice));
                authority = UAuthority::remote_inet(address);

                index += 4;
            }
            AddressType::IPv6 => {
                let slice: [u8; 16] = micro_uri[index..index + 16]
                    .try_into()
                    .expect("Wrong slice length");

                let address = IpAddr::V6(Ipv6Addr::from(slice));
                authority = UAuthority::remote_inet(address);

                index += 16;
            }
            AddressType::Local => {
                authority = UAuthority::LOCAL;
            }
        }

        let ue_id = u16::from_be_bytes(micro_uri[index..index + 2].try_into().unwrap());
        index += 2;
        let ue_version = u16::from_be_bytes(micro_uri[index..index + 2].try_into().unwrap());

        let mut ue_version_string = (ue_version >> 11).to_string();
        if ue_version & 0x7FF != 0 {
            ue_version_string.push_str(&format!(".{}", ue_version & 0x7FF));
        }

        UUri::new(
            Some(authority),
            Some(UEntity::micro_format(ue_version_string, ue_id)),
            Some(UResource::micro_format(u_resource_id)),
        )
    }
}

/// The type of address used for Micro URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressType {
    Local,
    IPv4,
    IPv6,
}

impl AddressType {
    pub fn value(&self) -> i32 {
        match *self {
            AddressType::Local => 0,
            AddressType::IPv4 => 1,
            AddressType::IPv6 => 2,
        }
    }

    pub fn from(value: i32) -> Option<AddressType> {
        match value {
            0 => Some(AddressType::Local),
            1 => Some(AddressType::IPv4),
            2 => Some(AddressType::IPv6),
            _ => None,
        }
    }
}

impl TryFrom<i32> for AddressType {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        AddressType::from(value).ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn test_empty() {
        let uri = UUri::EMPTY;
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert_eq!(0, uprotocol_uri.len());
    }

    #[test]
    fn test_build_micro_uri_from_remote_uri_missing_uentity() {
        let ipv6_address = "2001:db8:85a3:0:0:8a2e:370:7334";
        let address: std::net::IpAddr = ipv6_address.parse().unwrap();

        let authority = UAuthority::remote_inet(address);

        let uprotocol_uri = MicroUriSerializer::serialize(&UUri::new(
            Some(authority),
            Some(UEntity::EMPTY),
            Some(UResource::EMPTY),
        ));

        assert_eq!(0, uprotocol_uri.len());
    }

    #[test]
    fn test_build_micro_uri_from_local_uri_simple_version() {
        let authority = UAuthority::LOCAL;
        let entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            Some(5),
            false,
        );
        let resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            Some(3),
            false,
        );

        let uprotocol_uri = MicroUriSerializer::serialize(&UUri::new(
            Some(authority),
            Some(entity),
            Some(resource),
        ));

        assert_eq!(8, uprotocol_uri.len());
        assert_eq!(1, uprotocol_uri[0]); // version 1
        assert_eq!(0, uprotocol_uri[1]); // local
        assert_eq!(0, uprotocol_uri[2]); // UResource ID (MSB)
        assert_eq!(3, uprotocol_uri[3]); // UResource ID (LSB)
        assert_eq!(0, uprotocol_uri[4]); // UEntity ID (MSB)
        assert_eq!(5, uprotocol_uri[5]); // UEntity ID (LSB)
        assert_eq!(1 << 3, uprotocol_uri[6]); // UEntity Version (MSB)
        assert_eq!(0, uprotocol_uri[7]); // UEntity Version (LSB)
    }

    #[test]
    fn test_build_micro_uri_from_local_uri() {
        let authority = UAuthority::LOCAL;
        let entity = UEntity::new(
            "body.access".to_string(),
            Some("1.1".to_string()),
            Some(5),
            false,
        );
        let resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            Some(3),
            false,
        );

        let uprotocol_uri = MicroUriSerializer::serialize(&UUri::new(
            Some(authority),
            Some(entity),
            Some(resource),
        ));

        assert_eq!(8, uprotocol_uri.len());
        assert_eq!(1, uprotocol_uri[0]); // version 1
        assert_eq!(0, uprotocol_uri[1]); // local
        assert_eq!(0, uprotocol_uri[2]); // UResource ID (MSB)
        assert_eq!(3, uprotocol_uri[3]); // UResource ID (LSB)
        assert_eq!(0, uprotocol_uri[4]); // UEntity ID (MSB)
        assert_eq!(5, uprotocol_uri[5]); // UEntity ID (LSB)
        assert_eq!(1 << 3, uprotocol_uri[6]); // UEntity Version (MSB)
        assert_eq!(1, uprotocol_uri[7]); // UEntity Version (LSB)
    }

    #[test]
    fn test_build_micro_uri_from_local_uri_large_minor_version() {
        let authority = UAuthority::LOCAL;
        let entity = UEntity::new(
            "body.access".to_string(),
            Some("1.599".to_string()),
            Some(5),
            false,
        );
        let resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            Some(3),
            false,
        );

        let uprotocol_uri = MicroUriSerializer::serialize(&UUri::new(
            Some(authority),
            Some(entity),
            Some(resource),
        ));

        assert_eq!(8, uprotocol_uri.len());
        assert_eq!(1, uprotocol_uri[0]); // version 1
        assert_eq!(0, uprotocol_uri[1]); // local
        assert_eq!(0, uprotocol_uri[2]); // UResource ID (MSB)
        assert_eq!(3, uprotocol_uri[3]); // UResource ID (LSB)
        assert_eq!(0, uprotocol_uri[4]); // UEntity ID (MSB)
        assert_eq!(5, uprotocol_uri[5]); // UEntity ID (LSB)
        assert_eq!(10, uprotocol_uri[6]); // UEntity Version (MSB)
        assert_eq!((599u16 & 0xff) as u8, uprotocol_uri[7]); // UEntity Version (LSB)
    }

    #[test]
    fn test_build_micro_uri_from_local_uri_no_version() {
        let uauthority = UAuthority::LOCAL;
        let entity = UEntity::new("body.access".to_string(), None, Some(5), false);
        let uresource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            Some(3),
            false,
        );

        let uuri = UUri::new(Some(uauthority), Some(entity), Some(uresource));
        let uprotocol_uri = MicroUriSerializer::serialize(&uuri);

        assert_eq!(8, uprotocol_uri.len());
        assert_eq!(1, uprotocol_uri[0]); // version 1
        assert_eq!(0, uprotocol_uri[1]); // local
        assert_eq!(0, uprotocol_uri[2]); // UResource ID (MSB)
        assert_eq!(3, uprotocol_uri[3]); // UResource ID (LSB)
        assert_eq!(0, uprotocol_uri[4]); // UEntity ID (MSB)
        assert_eq!(5, uprotocol_uri[5]); // UEntity ID (LSB)
        assert_eq!((i16::MAX >> 8) as u8, uprotocol_uri[6]); // UEntity Version (MSB)
        assert_eq!((i16::MAX & 0xff) as u8, uprotocol_uri[7]); // UEntity Version (LSB)
    }

    #[test]
    fn test_build_micro_uri_from_local_uri_then_parse_back_to_uri() {
        let uauthority = UAuthority::LOCAL;
        let use_entity = UEntity::new("".to_string(), Some("1.599".to_string()), Some(5), false);
        let uresource = UResource::micro_format(3);

        let uuri = UUri::new(
            Some(uauthority.clone()),
            Some(use_entity.clone()),
            Some(uresource.clone()),
        );
        let uprotocol_uri = MicroUriSerializer::serialize(&uuri);

        let parsed_uuri = MicroUriSerializer::deserialize(uprotocol_uri);

        assert_eq!(uauthority, parsed_uuri.authority);
        assert_eq!(use_entity, parsed_uuri.entity);
        assert_eq!(uresource, parsed_uuri.resource);
    }

    #[test]
    fn test_build_micro_uri_from_remote_ipv4_address() {
        let ipv4_address = "127.0.0.1";
        let address: IpAddr = ipv4_address.parse().unwrap();

        let authority = UAuthority::remote_inet(address);
        let entity = UEntity::new("".to_string(), None, Some(5), false);
        let resource = UResource::micro_format(3);

        let uri = UUri::new(Some(authority), Some(entity), Some(resource));

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert_eq!(12, uprotocol_uri.len()); // Check URI length
        assert_eq!(1, uprotocol_uri[0]); // version 1
        assert_eq!(AddressType::IPv4.value() as u8, uprotocol_uri[1]);
        assert_eq!(0, uprotocol_uri[2]); // UResource ID (MSB)
        assert_eq!(3, uprotocol_uri[3]); // UResource ID (LSB)

        let addr_bytes: [u8; 4] = uprotocol_uri[4..8].try_into().unwrap();
        let addr = IpAddr::from(addr_bytes);
        assert_eq!(address, addr);

        assert_eq!(0, uprotocol_uri[8]); // UEntity ID (MSB)
        assert_eq!(5, uprotocol_uri[9]); // UEntity ID (LSB)
        assert_eq!((std::i16::MAX >> 8) as u8, uprotocol_uri[10]); // UEntity Version (MSB)
        assert_eq!(std::i16::MAX as u8, uprotocol_uri[11]); // UEntity Version (LSB)
    }

    #[test]
    fn test_build_micro_uri_from_remote_ipv6_address() {
        let ipv6_address = "2001:db8:85a3:0:0:8a2e:370:7334";
        let address: IpAddr = ipv6_address.parse().unwrap();

        let authority = UAuthority::remote_inet(address);
        let entity = UEntity::new("".to_string(), None, Some(5), false);
        let resource = UResource::micro_format(3);

        let uri = UUri::new(Some(authority), Some(entity), Some(resource));

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);

        // Assuming that the IPv6 Micro URI length is 24 bytes
        assert_eq!(24, uprotocol_uri.len());
        assert_eq!(1, uprotocol_uri[0]); // version 1
        assert_eq!(AddressType::IPv6.value() as u8, uprotocol_uri[1]);
        assert_eq!(0, uprotocol_uri[2]); // UResource ID (MSB)
        assert_eq!(3, uprotocol_uri[3]); // UResource ID (LSB)

        let addr_bytes: [u8; 16] = uprotocol_uri[4..20].try_into().unwrap();
        let addr = IpAddr::from(addr_bytes);
        assert_eq!(address, addr);

        assert_eq!(0, uprotocol_uri[20]); // UEntity ID (MSB)
        assert_eq!(5, uprotocol_uri[21]); // UEntity ID (LSB)
        assert_eq!((std::i16::MAX >> 8) as u8, uprotocol_uri[22]); // UEntity Version (MSB)
        assert_eq!(std::i16::MAX as u8, uprotocol_uri[23]); // UEntity Version (LSB)
    }

    // Can't pass null/None to this function in Rust
    // #[test]
    // fn test_build_micro_uri_from_null_uri() {}

    #[test]
    fn test_build_micro_uri_from_empty_uri() {
        let empty_uri = UUri::EMPTY;
        let uprotocol_uri = MicroUriSerializer::serialize(&empty_uri);
        assert_eq!(0, uprotocol_uri.len());
    }

    #[test]
    fn test_build_micro_uri_from_uri_missing_uresource_id() {
        let authority = UAuthority::LOCAL;
        let entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            Some(5),
            false,
        );
        let resource = UResource::EMPTY;

        let uri = UUri::new(Some(authority), Some(entity), Some(resource));

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert_eq!(0, uprotocol_uri.len());
    }

    #[test]
    fn test_build_micro_uri_from_uri_invalid_address() {
        let address: IpAddr =
            IpAddr::from_str("example.com").unwrap_or(IpAddr::V4("0.0.0.0".parse().unwrap()));
        let authority = UAuthority::remote_inet(address);
        let entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            Some(5),
            false,
        );
        let resource = UResource::long_format("door".to_string());

        let uri = UUri::new(Some(authority), Some(entity), Some(resource));

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert_eq!(0, uprotocol_uri.len());
    }

    #[test]
    fn test_parse_micro_uri_from_empty_byte_array() {
        let uprotocol_uri: Vec<u8> = vec![];
        let uri = MicroUriSerializer::deserialize(uprotocol_uri);

        assert_eq!(UUri::EMPTY, uri);
    }
}
