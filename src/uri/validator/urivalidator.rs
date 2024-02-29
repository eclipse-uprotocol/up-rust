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

use crate::uprotocol::{UAuthority, UUri};
use crate::uri::validator::ValidationError;

/// Struct to encapsulate Uri validation logic.
pub struct UriValidator;

impl UriValidator {
    /// Validates a `UUri` to ensure that it has at least a name for the uEntity.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to validate.
    ///
    /// # Returns
    /// Returns `ValidationResult` containing a success or a failure with the error message.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the following cases:
    ///
    /// - If the `UUri` is empty. The error message in this case will be "Uri is empty", indicating that the URI does not contain any data and is therefore considered invalid.
    /// - If the `UUri` is supposed to be remote (as indicated by the presence of an authority component) but fails the `is_remote` validation check. The error message will be "Uri is remote missing uAuthority", suggesting that the URI lacks a necessary authority component for remote URIs.
    /// - If the `UUri` is missing the name of the `uSoftware Entity` or the name is present but empty. The error message for this scenario will be "Uri is missing uSoftware Entity name", indicating that a critical component of the URI is absent or not properly specified.
    pub fn validate(uri: &UUri) -> Result<(), ValidationError> {
        if Self::is_empty(uri) {
            return Err(ValidationError::new("Uri is empty"));
        }
        if uri.authority.is_some() && !Self::is_remote(uri) {
            return Err(ValidationError::new("Uri is remote missing uAuthority"));
        }
        if uri
            .entity
            .as_ref()
            .map_or(true, |entity| entity.name.trim().is_empty())
        {
            return Err(ValidationError::new("Uri is missing uSoftware Entity name"));
        }
        Ok(())
    }

    /// Validates a `UUri` that is meant to be used as an RPC method URI.
    /// Used in Request sink values and Response source values.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to validate.
    ///
    /// # Returns
    /// Returns `ValidationResult` containing a success or a failure with the error message.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the following cases:
    ///
    /// - If the `UUri` fails the basic validation checks performed by `Self::validate`. The error message will detail the specific issue with the `UUri` as identified by the `Self::validate` method.
    /// - If the `UUri` is not recognized as a valid RPC method URI. The error message in this case will be "Invalid RPC method uri. Uri should be the method to be called, or method from response", indicating that the `UUri` does not conform to the expected format or designation for RPC method URIs.
    pub fn validate_rpc_method(uri: &UUri) -> Result<(), ValidationError> {
        Self::validate(uri)?;
        if !Self::is_rpc_method(uri) {
            return Err(ValidationError::new("Invalid RPC method uri. Uri should be the method to be called, or method from response"));
        }
        Ok(())
    }

    /// Validates a `UUri` that is meant to be used as an RPC response URI.
    /// This is used in Request source values and Response sink values.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `UUri` instance to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` containing either a success or a failure, along with the corresponding error message.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the following cases:
    ///
    /// - If the `UUri` fails the basic validation checks performed by `Self::validate`. The error message will contain details about what aspect of the `UUri` was invalid, as determined by the validation logic in `Self::validate`.
    /// - If the `UUri` is not of the correct type to be used as an RPC response. In this case, the error message will be "Invalid RPC response type", indicating that the `UUri` does not meet the specific criteria for RPC response URIs.
    pub fn validate_rpc_response(uri: &UUri) -> Result<(), ValidationError> {
        Self::validate(uri)?;
        if !Self::is_rpc_response(uri) {
            return Err(ValidationError::new("Invalid RPC response type"));
        }
        Ok(())
    }

    /// Indicates whether this `UUri` is empty, meaning it does not contain authority, entity, and resource.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check for emptiness.
    ///
    /// # Returns
    /// Returns `true` if this `UUri` is an empty container and has no valuable information for building uProtocol sinks or sources.
    pub fn is_empty(uri: &UUri) -> bool {
        uri.authority.is_none() && uri.entity.is_none() && uri.resource.is_none()
    }

    /// Checks if the URI contains both names and numeric representations of the names.
    ///
    /// This indicates that the `UUri` can be serialized to long or micro formats.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check if resolved.
    ///
    /// # Returns
    /// Returns `true` if the URI contains both names and numeric representations of the names,
    /// meaning that this `UUri` can be serialized to long or micro formats.
    pub fn is_resolved(uri: &UUri) -> bool {
        UriValidator::is_micro_form(uri) && UriValidator::is_long_form(uri)
    }

    /// Checks if the URI is of type RPC.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check if it is of type RPC method.
    ///
    /// # Returns
    /// Returns `true` if the URI is of type RPC.
    pub fn is_rpc_method(uri: &UUri) -> bool {
        if !Self::is_empty(uri) {
            if let Some(resource) = uri.resource.as_ref() {
                if resource.name == "rpc" {
                    let has_valid_instance = resource
                        .instance
                        .as_ref()
                        .map_or(false, |instance| !instance.trim().is_empty());

                    let has_non_zero_id = resource.id.map_or(false, |id| id != 0);

                    return has_valid_instance || has_non_zero_id;
                }
            }
        }
        false
    }

    /// Checks if the URI is of type RPC response.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check if it is a response for an RPC method.
    ///
    /// # Returns
    /// Returns `true` if the URI is of type RPC response.
    pub fn is_rpc_response(uri: &UUri) -> bool {
        if Self::is_rpc_method(uri) {
            if let Some(resource) = uri.resource.as_ref() {
                let has_valid_instance = resource
                    .instance
                    .as_ref()
                    .map_or(false, |instance| instance == "response");

                let has_zero_id = resource.id.map_or(false, |id| id == 0);

                return resource.name == "rpc" && has_valid_instance && has_zero_id;
            }
        }
        false
    }

    /// Checks if a `UAuthority` is of type remote
    ///
    /// # Arguments
    /// * `authority` - The `UAuthority` to check if.
    ///
    /// # Returns
    /// Returns `true` if the `UAuthority` is of type remote.
    #[allow(clippy::missing_panics_doc)]
    pub fn is_remote(uri: &UUri) -> bool {
        uri.authority
            .as_ref()
            .map_or(false, |auth| auth.get_name().is_some())
    }

    /// Checks if the URI contains appropriate fields and numbers of the appropriate size so that it can be serialized
    /// into micro format.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check.
    ///
    /// # Returns
    /// Returns `Ok(())` if the URI contains numbers which will fit in the allotted space,
    /// allowing it to be serialized into micro format.
    ///
    /// # Errors
    ///
    /// Otherwise returns `ValidationError` containing description of error.
    ///
    /// # Examples
    ///
    /// ## `UUri` in valid micro form
    ///
    /// The specifics of what makes component of a `UUri` valid can be garnered
    /// from their impls of `valid_micro_form()`: `UAuthority`, `UEntity`, `UResource`
    ///
    /// ```
    /// use up_rust::uprotocol::{UAuthority, UUri, UEntity, UResource, uri::uauthority::Number};
    /// use up_rust::uri::validator::{UriValidator, ValidationError};
    ///
    /// let uri = UUri {
    ///     authority: Some(UAuthority {
    ///         number: Some(Number::Ip(
    ///             vec![192, 168, 1, 202],
    ///         )),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     entity: Some(UEntity {
    ///         id: Some(29999),
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let val_micro_form = UriValidator::validate_micro_form(&uri);
    /// assert!(val_micro_form.is_ok());
    /// ```
    ///
    /// ## `UAuthority` IP is incorrect format (neither IPv4, nor IPv6)
    /// ```
    /// use up_rust::uprotocol::{UAuthority, UUri, UEntity, UResource, uri::uauthority::Number};
    /// use up_rust::uri::validator::{UriValidator, ValidationError};
    ///
    /// let uri = UUri {
    ///     authority: Some(UAuthority {
    ///         number: Some(Number::Ip(
    ///             vec![127, 0, 0],        // <- note only 3 bytes, must be 4 (IPv4)
    ///         )),                         //    or 16 (IPv6)
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     entity: Some(UEntity {
    ///         id: Some(29999),
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let val_micro_form = UriValidator::validate_micro_form(&uri);
    /// assert!(val_micro_form.is_err());
    /// ```
    ///
    /// ## `UAuthority` ID is longer than maximum allowed (255 bytes)
    /// ```
    /// use up_rust::uprotocol::{UAuthority, UUri, UEntity, UResource, uri::uauthority::Number};
    /// use up_rust::uri::validator::{UriValidator, ValidationError};
    ///
    /// let uri = UUri {
    ///     authority: Some(UAuthority {
    ///         number: Some(Number::Ip(
    ///                 (0..=256) // <- note that ID will exceed 255 byte limit
    ///                 .map(|i| (i % 256) as u8)
    ///                 .collect::<Vec<u8>>())),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     entity: Some(UEntity {
    ///         id: Some(29999),
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let val_micro_form = UriValidator::validate_micro_form(&uri);
    /// assert!(val_micro_form.is_err());
    /// ```
    ///
    /// ## Overflowing `UEntity` ID's 16 bit capacity
    /// ```
    /// use up_rust::uprotocol::{UUri, UEntity, UResource, uri::uauthority::Number};
    /// use up_rust::uri::validator::{UriValidator, ValidationError};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         id: Some(0x10000), // <- exceeds allotted 16 bits
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let val_micro_form = UriValidator::validate_micro_form(&uri);
    /// assert!(val_micro_form.is_err());
    /// ```
    ///
    /// ## Overflowing `UEntity` Major Version 8 bit capacity
    /// ```
    /// use up_rust::uprotocol::{UUri, UEntity, UResource};
    /// use up_rust::uri::validator::{UriValidator, ValidationError};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         id: Some(29999),
    ///         version_major: Some(0x100), // <- exceeds allotted 8 bits
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(29999),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let val_micro_form = UriValidator::validate_micro_form(&uri);
    /// assert!(val_micro_form.is_err());
    /// ```
    ///
    /// ## Overflowing `UResource` ID's 16 bit capacity
    /// ```
    /// use up_rust::uprotocol::{UUri, UEntity, UResource};
    /// use up_rust::uri::validator::{UriValidator, ValidationError};
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         id: Some(29999),
    ///         version_major: Some(254),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         id: Some(0x10000), // <- exceeds allotted 16 bits
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     ..Default::default()
    /// };
    /// let val_micro_form = UriValidator::validate_micro_form(&uri);
    /// assert!(val_micro_form.is_err());
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn validate_micro_form(uri: &UUri) -> Result<(), ValidationError> {
        if Self::is_empty(uri) {
            Err(ValidationError::new("URI is empty"))?;
        }

        if let Some(entity) = uri.entity.as_ref() {
            if let Err(e) = entity.validate_micro_form() {
                return Err(ValidationError::new(format!("Entity: {}", e)));
            }
        } else {
            return Err(ValidationError::new("Entity: Is missing"));
        }

        if let Some(resource) = uri.resource.as_ref() {
            if let Err(e) = resource.validate_micro_form() {
                return Err(ValidationError::new(format!("Resource: {}", e)));
            }
        } else {
            return Err(ValidationError::new("Resource: Is missing"));
        }

        if let Some(authority) = uri.authority.as_ref() {
            if let Err(e) = authority.validate_micro_form() {
                return Err(ValidationError::new(format!("Authority: {}", e)));
            }
        }

        Ok(())
    }

    /// Checks if the URI contains numbers of the appropriate size so that it can be serialized into micro format.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check.
    ///
    /// # Returns
    /// Returns `true` if the URI contains numbers which will fit in the allotted space (16 bits for
    /// id), allowing it to be serialized into micro format.
    #[allow(clippy::missing_panics_doc)]
    pub fn is_micro_form(uri: &UUri) -> bool {
        Self::validate_micro_form(uri).is_ok()
    }

    /// Checks if the URI contains names so that it can be serialized into long format.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check.
    ///
    /// # Returns
    /// Returns `true` if the URI contains names, allowing it to be serialized into long format.
    pub fn is_long_form(uri: &UUri) -> bool {
        if Self::is_empty(uri) {
            return false;
        }

        let mut auth_name = String::new();
        if let Some(authority) = uri.authority.as_ref() {
            if let Some(name) = UAuthority::get_name(authority) {
                auth_name = name.to_string();
            }
        }

        let mut ent_name = String::new();
        if let Some(entity) = uri.entity.as_ref() {
            ent_name = entity.name.to_string();
        }

        let mut res_name = String::new();
        if let Some(resource) = uri.resource.as_ref() {
            res_name = resource.name.to_string();
        }

        !auth_name.is_empty() && !ent_name.trim().is_empty() && !res_name.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json::{Error, Value};
    use std::fs;

    use crate::uprotocol::{UEntity, UResource};
    use crate::uri::builder::resourcebuilder::UResourceBuilder;

    #[test]
    fn test_validate_blank_uri() {
        let uri = UUri::from_str("");
        assert!(uri.is_err());
    }

    #[test]
    fn test_validate_uri_with_get_entity() {
        let uri = UUri::from_str("/hartley").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_with_malformed_uri() {
        let uri = UUri::from_str("hartley");
        assert!(uri.is_err());
    }

    #[test]
    fn test_validate_with_blank_uentity_name_uri() {
        let uri = UUri::default();
        let status = UriValidator::validate(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_validate_rpc_method_with_valid_uri() {
        let uri = UUri::from_str("/hartley//rpc.echo").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_rpc_method_with_invalid_uri() {
        let entity = UEntity {
            name: "hartley".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "echo".into(),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(UAuthority::default()).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_validate_rpc_method_with_malformed_uri() {
        let entity = UEntity {
            name: "hartley".into(),
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(UAuthority::default()).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_validate_rpc_response_with_valid_uri() {
        let uri = UUri::from_str("/hartley//rpc.response").unwrap();
        let status = UriValidator::validate_rpc_response(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_rpc_response_with_malformed_uri() {
        let entity = UEntity {
            name: "hartley".into(),
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(UAuthority::default()).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_response(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_validate_rpc_response_with_rpc_type() {
        let uri = UUri::from_str("/hartley//dummy.wrong").unwrap();
        let status = UriValidator::validate_rpc_response(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_validate_rpc_response_with_invalid_rpc_response_type() {
        let uri = UUri::from_str("/hartley//rpc.wrong").unwrap();
        let status = UriValidator::validate_rpc_response(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_with_version_when_it_is_valid_remote() {
        let uri = UUri::from_str("//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_no_version_when_it_is_valid_remote() {
        let uri = UUri::from_str("//VCU.MY_CAR_VIN/body.access//door.front_left#Door").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_with_version_when_it_is_valid_local() {
        let uri = UUri::from_str("/body.access/1/door.front_left#Door").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_no_version_when_it_is_valid_local() {
        let uri = UUri::from_str("/body.access//door.front_left#Door").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_has_schema_only() {
        let authority = UAuthority {
            name: Some(String::from(":")),
            ..Default::default()
        };
        let entity = UEntity {
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_has_empty_use_name_local() {
        let entity = UEntity {
            name: "".into(),
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };

        let status = UriValidator::validate(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_remote_no_authority() {
        let authority = UAuthority {
            ..Default::default()
        };
        let entity = UEntity {
            name: "".into(),
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_remote_no_authority_with_use() {
        let authority = UAuthority {
            ..Default::default()
        };
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
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };
        let status = UriValidator::validate(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_missing_use_remote() {
        let uri = UUri::from_str("//VCU.myvin///door.front_left#Door").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_missing_use_name_remote() {
        let entity = UEntity {
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "door".into(),
            instance: Some("front_left".into()),
            message: Some("Door".into()),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };

        let status = UriValidator::validate(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_missing_use_name_local() {
        let uri = UUri::from_str("//VCU.myvin//1").unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_valid_remote() {
        let uri = UUri::from_str("//bo.cloud/petapp/1/rpc.response").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_no_version_when_it_is_valid_remote() {
        let uri = UUri::from_str("//bo.cloud/petapp//rpc.response").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_valid_local() {
        let uri = UUri::from_str("/petapp/1/rpc.response").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_no_version_when_it_is_valid_local() {
        let uri = UUri::from_str("/petapp//rpc.response").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_has_schema_only() {
        let authority = UAuthority {
            name: Some(String::from(":")),
            ..Default::default()
        };
        let entity = UEntity {
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_not_valid_missing_rpc_response_local() {
        let uri = UUri::from_str("/petapp/1/dog").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_not_valid_missing_rpc_response_remote() {
        let entity = UEntity {
            name: "petapp".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "dog".into(),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_remote_no_authority() {
        let authority = UAuthority {
            ..Default::default()
        };
        let entity = UEntity {
            name: "".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "".into(),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_remote_no_authority_with_use() {
        let authority = UAuthority {
            ..Default::default()
        };
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_missing_use() {
        let uri = UUri::from_str("//VCU.myvin").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_missing_use_name_remote() {
        let uri = UUri::from_str("/1").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_missing_use_name_local() {
        let uri = UUri::from_str("//VCU.myvin//1").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_valid_remote() {
        let uri = UUri::from_str("//VCU.myvin/body.access/1/rpc.UpdateDoor").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_no_version_when_it_is_valid_remote() {
        let uri = UUri::from_str("//VCU.myvin/body.access//rpc.UpdateDoor").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_valid_local() {
        let uri = UUri::from_str("/body.access/1/rpc.UpdateDoor").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_no_version_when_it_is_valid_local() {
        let uri = UUri::from_str("/body.access//rpc.UpdateDoor").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_has_schema_only() {
        let authority = UAuthority {
            name: Some(String::from(":")),
            ..Default::default()
        };
        let entity = UEntity {
            ..Default::default()
        };
        let resource = UResource {
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_not_valid_not_rpc_method_local() {
        let uri = UUri::from_str("/body.access//UpdateDoor").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_not_valid_not_rpc_method_remote() {
        let authority = UAuthority {
            ..Default::default()
        };
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "UpdateDoor".into(),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_remote_no_authority() {
        let authority = UAuthority {
            ..Default::default()
        };
        let entity = UEntity {
            name: "".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "".into(),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_remote_no_authority_with_use() {
        let authority = UAuthority {
            ..Default::default()
        };
        let entity = UEntity {
            name: "body.access".into(),
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "rpc".into(),
            instance: Some("UpdateDoor".into()),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(authority).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_remote_missing_authority_remotecase() {
        let entity = UEntity {
            name: "body.access".into(),
            ..Default::default()
        };
        let resource = UResource {
            name: "rpc".into(),
            instance: Some("UpdateDoor".into()),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: Some(UAuthority::default()).into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_missing_use() {
        let uri = UUri::from_str("//VCU.myvin").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_missing_use_name_local() {
        let entity = UEntity {
            version_major: Some(1),
            ..Default::default()
        };
        let resource = UResource {
            name: "rpc".into(),
            instance: Some("UpdateDoor".into()),
            ..Default::default()
        };
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_method(&uuri);
        assert!(status.is_err());
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_missing_use_name_remote() {
        let uri = UUri::from_str("//VCU.myvin//1/rpc.UpdateDoor").unwrap();
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(status.is_err());
    }

    #[test]
    fn test_all_valid_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        if let Some(valid_uris) = json_object.get("validUris").and_then(|v| v.as_array()) {
            for uri in valid_uris {
                let uuri = UUri::from_str(uri.as_str().unwrap_or_default()).unwrap();
                let status = UriValidator::validate(&uuri);
                assert!(status.is_ok());
            }
        }
    }

    #[test]
    fn test_all_invalid_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        let invalid_uris = json_object.get("invalidUris").unwrap().as_array().unwrap();

        for uri_object in invalid_uris {
            let uri = uri_object.get("uri").unwrap().as_str().unwrap();
            if let Ok(uuri) = UUri::from_str(uri) {
                let status = UriValidator::validate(&uuri);
                assert!(status.is_err());
                let message = uri_object.get("status_message").unwrap().as_str().unwrap();
                assert_eq!(message, status.unwrap_err().to_string());
            }
        }
    }

    #[test]
    fn test_all_valid_rpc_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        let valid_rpc_uris = json_object.get("validRpcUris").unwrap().as_array().unwrap();

        for uri in valid_rpc_uris {
            let uuri = UUri::from_str(uri.as_str().unwrap_or_default()).unwrap();
            let status = UriValidator::validate_rpc_method(&uuri);
            assert!(status.is_ok());
        }
    }

    #[test]
    fn test_all_invalid_rpc_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        let invalid_rpc_uris = json_object
            .get("invalidRpcUris")
            .unwrap()
            .as_array()
            .unwrap();

        for uri_object in invalid_rpc_uris {
            let uri = uri_object.get("uri").unwrap().as_str().unwrap();
            let uuri = UUri::from_str(uri).unwrap();
            let status = UriValidator::validate_rpc_method(&uuri);
            assert!(status.is_err());
            let message = uri_object.get("status_message").unwrap().as_str().unwrap();
            assert_eq!(message, status.unwrap_err().to_string());
        }
    }

    #[test]
    fn test_all_valid_rpc_response_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        let valid_rpc_response_uris = json_object
            .get("validRpcResponseUris")
            .unwrap()
            .as_array()
            .unwrap();

        for uri in valid_rpc_response_uris {
            let uuri = UUri::from_str(uri.as_str().unwrap_or_default()).unwrap();
            let status = UriValidator::validate_rpc_response(&uuri);
            assert!(UriValidator::is_rpc_response(&uuri));
            assert!(status.is_ok());
        }
    }

    #[test]
    fn test_valid_rpc_response_uri() {
        let entity = UEntity {
            name: "hartley".into(),
            ..Default::default()
        };
        let resource = UResourceBuilder::for_rpc_response();
        let uuri = UUri {
            entity: Some(entity).into(),
            resource: Some(resource).into(),
            authority: None.into(),
            ..Default::default()
        };

        let status = UriValidator::validate_rpc_response(&uuri);
        assert!(UriValidator::is_rpc_response(&uuri));
        assert!(status.is_ok());
    }

    #[test]
    fn test_all_invalid_rpc_response_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        let invalid_rpc_response_uris = json_object
            .get("invalidRpcResponseUris")
            .unwrap()
            .as_array()
            .unwrap();

        for uri in invalid_rpc_response_uris {
            let uuri = UUri::from_str(uri.as_str().unwrap_or_default()).unwrap();
            let status = UriValidator::validate_rpc_response(&uuri);
            assert!(status.is_err());
        }
    }

    fn get_json_object() -> Result<Value, Error> {
        let current_directory = std::env::current_dir().expect("Failed to get current directory");
        let json_path = current_directory.join("tests").join("uris.json");

        let json_string = fs::read_to_string(json_path).expect("Failed to read the JSON file");
        serde_json::from_str(&json_string)
    }
}
