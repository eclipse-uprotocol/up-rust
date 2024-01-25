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

use bytes::{Buf, BufMut};
use std::io::Write;

use crate::uprotocol::uri::{UAuthority, UEntity, UUri};
use crate::uri::builder::resourcebuilder::UResourceBuilder;
use crate::uri::serializer::{SerializationError, UriSerializer};
use crate::uri::validator::UriValidator;

const LOCAL_MICRO_URI_LENGTH: usize = 8; // local micro URI length
const IPV4_MICRO_URI_LENGTH: usize = 12; // IPv4 micro URI length
const IPV6_MICRO_URI_LENGTH: usize = 24; // IPv6 micro URI length
const UP_VERSION: u8 = 0x1; // UP version

#[derive(Debug, Copy, Clone, PartialEq)]
enum AddressType {
    Local = 0,
    IPv4 = 1,
    IPv6 = 2,
    ID = 3,
}

impl AddressType {
    fn value(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for AddressType {
    type Error = SerializationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AddressType::Local),
            1 => Ok(AddressType::IPv4),
            2 => Ok(AddressType::IPv6),
            3 => Ok(AddressType::ID),
            _ => Err(SerializationError::new(format!(
                "unknown address type ID [{}]",
                value
            ))),
        }
    }
}

impl TryFrom<i32> for AddressType {
    type Error = SerializationError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if let Ok(v) = u8::try_from(value) {
            Self::try_from(v)
        } else {
            Err(SerializationError::new(format!(
                "unknown address type ID [{}]",
                value
            )))
        }
    }
}

/// `UriSerializer` that serializes a `UUri` to byte[] (micro format) per
///  <https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc>
pub struct MicroUriSerializer;

impl UriSerializer<Vec<u8>> for MicroUriSerializer {
    /// Serializes a `UUri` into a `Vec<u8>` following the Micro-URI specifications.
    ///
    /// # Parameters
    /// * `uri`: A reference to the `UUri` data object.
    ///
    /// # Returns
    /// A `Vec<u8>` representing the serialized `UUri`.
    #[allow(clippy::cast_possible_truncation)]
    fn serialize(uri: &UUri) -> Result<Vec<u8>, SerializationError> {
        if UriValidator::is_empty(uri) || !UriValidator::is_micro_form(uri) {
            return Err(SerializationError::new("URI is empty or not in micro form"));
        }

        let mut buf = vec![];
        let mut address_type = AddressType::Local;
        let mut authority_id: Option<Vec<u8>> = None;
        let mut remote_ip: Option<Vec<u8>> = None;

        // UP_VERSION
        buf.put_u8(UP_VERSION);

        // ADDRESS_TYPE
        if let Some(authority) = uri.authority.as_ref() {
            if authority.get_name().is_none() {
                address_type = AddressType::Local;
            }
            if let Some(id) = authority.get_id() {
                authority_id = Some(id.to_vec());
                address_type = AddressType::ID;
            } else if let Some(ip) = authority.get_ip() {
                match ip.len() {
                    4 => address_type = AddressType::IPv4,
                    16 => address_type = AddressType::IPv6,
                    _ => return Err(SerializationError::new("Invalid IP address")),
                }
                remote_ip = Some(ip.to_vec());
            }
        }

        buf.put_u8(address_type.value());

        // URESOURCE_ID
        if let Some(id) = uri.resource.as_ref().and_then(|resource| resource.id) {
            buf.write_all(&[(id >> 8) as u8])
                .map_err(|e| SerializationError::new(e.to_string()))?;
            buf.write_all(&[id as u8])
                .map_err(|e| SerializationError::new(e.to_string()))?;
        }

        // UENTITY_ID
        if let Some(id) = uri.entity.as_ref().and_then(|entity| entity.id) {
            buf.write_all(&[(id >> 8) as u8])
                .map_err(|e| SerializationError::new(e.to_string()))?;
            buf.write_all(&[id as u8])
                .map_err(|e| SerializationError::new(e.to_string()))?;
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
        if address_type != AddressType::Local {
            if let Some(id) = authority_id {
                buf.put_u8(id.len() as u8);
                buf.write_all(&id)
                    .map_err(|e| SerializationError::new(e.to_string()))?;
            } else if let Some(ip) = remote_ip {
                buf.write_all(&ip)
                    .map_err(|e| SerializationError::new(e.to_string()))?;
            }
        }
        Ok(buf)
    }

    /// Creates a `UUri` data object from a uProtocol micro URI.
    ///
    /// # Arguments
    ///
    /// * `micro_uri` - A byte vec representing the uProtocol micro URI.
    ///
    /// # Returns
    ///
    /// Returns a `UUri` data object.
    fn deserialize(micro_uri: Vec<u8>) -> Result<UUri, SerializationError> {
        if micro_uri.len() < LOCAL_MICRO_URI_LENGTH {
            return Err(SerializationError::new("URI is empty or not in micro form"));
        }

        let mut buf = micro_uri.as_slice();
        // Need to be version 1
        if buf.get_u8() != UP_VERSION {
            return Err(SerializationError::new(format!(
                "URI is not of expected uProtocol version {}",
                UP_VERSION
            )));
        }
        let address_type = AddressType::try_from(buf.get_u8())?;

        match address_type {
            AddressType::Local => {
                if micro_uri.len() != LOCAL_MICRO_URI_LENGTH {
                    return Err(SerializationError::new("Invalid micro URI length"));
                }
            }
            AddressType::IPv4 => {
                if micro_uri.len() != IPV4_MICRO_URI_LENGTH {
                    return Err(SerializationError::new("Invalid micro URI length"));
                }
            }
            AddressType::IPv6 => {
                if micro_uri.len() != IPV6_MICRO_URI_LENGTH {
                    return Err(SerializationError::new("Invalid micro URI length"));
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
            AddressType::IPv4 => {
                let ip4_address = buf.copy_to_bytes(4);
                Some(UAuthority {
                    ip: Some(ip4_address.into()),
                    ..Default::default()
                })
            }
            AddressType::IPv6 => {
                let ip6_address = buf.copy_to_bytes(16);
                Some(UAuthority {
                    ip: Some(ip6_address.into()),
                    ..Default::default()
                })
            }
            AddressType::ID => {
                let length = buf.get_u8();
                let authority_id = buf.copy_to_bytes(length as usize);
                Some(UAuthority {
                    id: Some(authority_id.into()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    use crate::uprotocol::uri::UResource;
    use crate::uri::builder::resourcebuilder::UResourceBuilder;

    #[test]
    fn test_empty() {
        let uri = UUri::default();
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "URI is empty or not in micro form"
        );
    }

    #[test]
    fn test_serialize_uri() {
        let uri = UUri {
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(254),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                id: Some(19999),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_ok());
        let uri2 = MicroUriSerializer::deserialize(uprotocol_uri.unwrap());
        assert!(uri2.is_ok());
        assert_eq!(uri, uri2.unwrap())
    }

    #[test]
    fn test_serialize_remote_uri_without_address() {
        let uri = UUri {
            authority: Some(UAuthority {
                name: Some(String::from("vcu.vin")),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(254),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                id: Some(19999),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "URI is empty or not in micro form"
        );
    }

    #[test]
    fn test_serialize_uri_missing_ids() {
        let uri = UUri {
            entity: Some(UEntity {
                name: "kaputt".to_string(),
                ..Default::default()
            })
            .into(),
            resource: Some(UResourceBuilder::for_rpc_response()).into(),
            ..Default::default()
        };
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "URI is empty or not in micro form"
        );
    }

    #[test]
    fn test_serialize_uri_missing_resource_ids() {
        let uri = UUri {
            entity: Some(UEntity {
                name: "kaputt".to_string(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "URI is empty or not in micro form"
        );
    }

    #[test]
    fn test_deserialize_bad_microuri_length() {
        let bad_uri: Vec<u8> = vec![0x1, 0x0, 0x0, 0x0, 0x0];
        let uprotocol_uri = MicroUriSerializer::deserialize(bad_uri);
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "URI is empty or not in micro form"
        );
    }

    #[test]
    fn test_deserialize_bad_microuri_not_version_1() {
        let bad_uri: Vec<u8> = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
        let uprotocol_uri = MicroUriSerializer::deserialize(bad_uri);
        assert!(uprotocol_uri.is_err());
    }

    #[test]
    fn test_deserialize_bad_microuri_not_valid_address_type() {
        let bad_uri: Vec<u8> = vec![0x1, 0x5, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
        let uprotocol_uri = MicroUriSerializer::deserialize(bad_uri);
        assert!(uprotocol_uri.is_err());
    }

    #[test]
    fn test_deserialize_bad_microuri_valid_address_type_invalid_length() {
        let bad_uri: Vec<u8> = vec![0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
        let uprotocol_uri = MicroUriSerializer::deserialize(bad_uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "Invalid micro URI length"
        );

        let bad_uri: Vec<u8> = vec![0x1, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
        let uprotocol_uri = MicroUriSerializer::deserialize(bad_uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "Invalid micro URI length"
        );

        let bad_uri: Vec<u8> = vec![0x1, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
        let uprotocol_uri = MicroUriSerializer::deserialize(bad_uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(
            uprotocol_uri.unwrap_err().to_string(),
            "Invalid micro URI length"
        );
    }

    #[test]
    fn test_serialize_good_ipv4_based_authority() {
        let address: Ipv4Addr = "10.0.3.3".parse().unwrap();
        let uri = UUri {
            authority: Some(UAuthority {
                ip: Some(address.octets().to_vec()),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(254),
                ..Default::default()
            })
            .into(),
            resource: Some(UResourceBuilder::for_rpc_request(None, Some(99))).into(),
            ..Default::default()
        };

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(!uprotocol_uri.as_ref().unwrap().is_empty());
        let uri2 = MicroUriSerializer::deserialize(uprotocol_uri.unwrap());
        assert!(uri2.as_ref().is_ok());
        assert!(UriValidator::is_micro_form(&uri));
        assert!(UriValidator::is_micro_form(uri2.as_ref().unwrap()));
        assert_eq!(uri.to_string(), uri2.as_ref().unwrap().to_string());
        assert_eq!(uri, uri2.unwrap());
    }

    #[test]
    fn test_serialize_good_ipv6_based_authority() {
        let address: Ipv6Addr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334".parse().unwrap();
        let uri = UUri {
            authority: Some(UAuthority {
                ip: Some(address.octets().to_vec()),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(254),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                id: Some(19999),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.as_ref().is_ok());
        assert!(!uprotocol_uri.as_ref().unwrap().is_empty());
        let uri2 = MicroUriSerializer::deserialize(uprotocol_uri.unwrap());
        assert!(uri2.as_ref().is_ok());
        assert!(UriValidator::is_micro_form(&uri));
        assert!(UriValidator::is_micro_form(uri2.as_ref().unwrap()));
        assert_eq!(uri.to_string(), uri2.as_ref().unwrap().to_string());
        assert_eq!(uri, uri2.unwrap());
    }

    #[test]
    fn test_serialize_id_based_authority() {
        let authority_id: Vec<u8> = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        let uri = UUri {
            authority: Some(UAuthority {
                id: Some(authority_id),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(254),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                id: Some(19999),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        assert!(UriValidator::is_micro_form(&uri));

        let serialization_attempt = MicroUriSerializer::serialize(&uri);
        assert!(serialization_attempt.is_ok());
        let uprotocol_uri = serialization_attempt.unwrap();
        assert!(!uprotocol_uri.is_empty());
        let deserialization_attempt = MicroUriSerializer::deserialize(uprotocol_uri);
        assert!(deserialization_attempt.is_ok());
        let uri2 = deserialization_attempt.unwrap();
        assert!(UriValidator::is_micro_form(&uri2));
        assert_eq!(uri, uri2);
    }

    #[test]
    fn test_serialize_bad_length_ip_based_authority() {
        let bad_bytes: Vec<u8> = vec![127, 1, 23, 123, 12, 6];
        let uri = UUri {
            authority: Some(UAuthority {
                ip: Some(bad_bytes),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(3),
                ..Default::default()
            })
            .into(),
            resource: Some(UResourceBuilder::for_rpc_request(None, Some(99))).into(),
            ..Default::default()
        };
        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.is_err());
        assert_eq!(uprotocol_uri.unwrap_err().to_string(), "Invalid IP address");
    }

    #[test]
    fn test_serialize_id_size_255_based_authority() {
        let size = 129;
        let bytes: Vec<u8> = (0..size).map(|i| i as u8).collect();

        let uri = UUri {
            authority: Some(UAuthority {
                id: Some(bytes),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                id: Some(29999),
                version_major: Some(254),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                id: Some(19999),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };

        let uprotocol_uri = MicroUriSerializer::serialize(&uri);
        assert!(uprotocol_uri.as_ref().is_ok());
        assert_eq!(uprotocol_uri.as_ref().unwrap().len(), 9 + size);
        let uri2 = MicroUriSerializer::deserialize(uprotocol_uri.unwrap());
        assert!(uri2.is_ok());
        assert!(UriValidator::is_micro_form(&uri));
        assert!(UriValidator::is_micro_form(uri2.as_ref().unwrap()));
        assert_eq!(uri, uri2.unwrap());
    }
}
