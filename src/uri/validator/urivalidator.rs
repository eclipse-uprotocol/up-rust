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

use crate::uprotocol::uri::{UAuthority, UUri};
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
        !Self::is_empty(uri)
        // TODO finish this
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
                if resource.name.contains("rpc") {
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
                    .map_or(false, |instance| instance.contains("response"));

                let has_non_zero_id = resource.id.map_or(false, |id| id != 0);

                return has_valid_instance || has_non_zero_id;
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

    /// Checks if the URI contains numbers so that it can be serialized into micro format.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` to check.
    ///
    /// # Returns
    /// Returns `true` if the URI contains numbers, allowing it to be serialized into micro format.
    #[allow(clippy::missing_panics_doc)]
    pub fn is_micro_form(uri: &UUri) -> bool {
        !Self::is_empty(uri)
            && uri.entity.has_id()
            && uri.resource.has_id()
            && (uri.authority.is_none() || uri.authority.has_id() || uri.authority.has_ip())
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
    use super::*;
    use serde_json::{Error, Value};
    use std::fs;

    use crate::{
        uprotocol::uri::{UEntity, UResource},
        uri::serializer::{LongUriSerializer, UriSerializer},
    };

    #[test]
    fn test_validate_blank_uri() {
        let uri = LongUriSerializer::deserialize("".to_string());
        assert!(uri.is_err());
    }

    #[test]
    fn test_validate_uri_with_get_entity() {
        let uri = LongUriSerializer::deserialize("/hartley".to_string()).unwrap();
        let status = UriValidator::validate(&uri);
        assert!(status.is_ok());
    }

    #[test]
    fn test_validate_with_malformed_uri() {
        let uri = LongUriSerializer::deserialize("hartley".to_string());
        assert!(uri.is_err());
    }

    #[test]
    fn test_validate_with_blank_uentity_name_uri() {
        let uri = UUri::default();
        let status = UriValidator::validate(&uri);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Uri is empty");
    }

    #[test]
    fn test_validate_rpc_method_with_valid_uri() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.echo".to_string()).unwrap();
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
    }

    #[test]
    fn test_validate_rpc_response_with_valid_uri() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.response".to_string()).unwrap();
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
    }

    #[test]
    fn test_validate_rpc_response_with_rpc_type() {
        let uri = LongUriSerializer::deserialize("/hartley//dummy.wrong".to_string()).unwrap();
        let status = UriValidator::validate_rpc_response(&uri);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid RPC response type");
    }

    #[test]
    fn test_validate_rpc_response_with_invalid_rpc_response_type() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.wrong".to_string()).unwrap();
        let status = UriValidator::validate_rpc_response(&uri);
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().to_string(), "Invalid RPC response type");
    }

    #[test]
    fn test_topic_uri_with_version_when_it_is_valid_remote() {
        let uri = "//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door".to_string();
        let status = UriValidator::validate(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_no_version_when_it_is_valid_remote() {
        let uri = "//VCU.MY_CAR_VIN/body.access//door.front_left#Door".to_string();
        let status = UriValidator::validate(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_with_version_when_it_is_valid_local() {
        let uri = "/body.access/1/door.front_left#Door".to_string();
        let status = UriValidator::validate(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_topic_uri_no_version_when_it_is_valid_local() {
        let uri = "/body.access//door.front_left#Door".to_string();
        let status = UriValidator::validate(&LongUriSerializer::deserialize(uri).unwrap());
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_missing_use_remote() {
        let uri = "//VCU.myvin///door.front_left#Door".to_string();
        let status = UriValidator::validate(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_topic_uri_invalid_when_uri_is_missing_use_name_local() {
        let uri = "//VCU.myvin//1".to_string();
        let status = UriValidator::validate(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_valid_remote() {
        let uri = "//bo.cloud/petapp/1/rpc.response".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_no_version_when_it_is_valid_remote() {
        let uri = "//bo.cloud/petapp//rpc.response".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_valid_local() {
        let uri = "/petapp/1/rpc.response".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_topic_uri_no_version_when_it_is_valid_local() {
        let uri = "/petapp//rpc.response".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_rpc_topic_uri_with_version_when_it_is_not_valid_missing_rpc_response_local() {
        let uri = "/petapp/1/dog".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid RPC method uri. Uri should be the method to be called, or method from response"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid RPC method uri. Uri should be the method to be called, or method from response"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_missing_use() {
        let uri = "//VCU.myvin".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_missing_use_name_remote() {
        let uri = "/1".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid RPC method uri. Uri should be the method to be called, or method from response"
        );
    }

    #[test]
    fn test_rpc_topic_uri_invalid_when_uri_is_missing_use_name_local() {
        let uri = "//VCU.myvin//1".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_valid_remote() {
        let uri = "//VCU.myvin/body.access/1/rpc.UpdateDoor".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_no_version_when_it_is_valid_remote() {
        let uri = "//VCU.myvin/body.access//rpc.UpdateDoor".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_valid_local() {
        let uri = "/body.access/1/rpc.UpdateDoor".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_ok());
    }

    #[test]
    fn test_rpc_method_uri_no_version_when_it_is_valid_local() {
        let uri = "/body.access//rpc.UpdateDoor".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_rpc_method_uri_with_version_when_it_is_not_valid_not_rpc_method_local() {
        let uri = "/body.access//UpdateDoor".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Invalid RPC method uri. Uri should be the method to be called, or method from response"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is remote missing uAuthority"
        );
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_missing_use() {
        let uri = "//VCU.myvin".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
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
        assert_eq!(
            status.unwrap_err().to_string(),
            "Uri is missing uSoftware Entity name"
        );
    }

    #[test]
    fn test_rpc_method_uri_invalid_when_uri_is_missing_use_name_remote() {
        let uri = "//VCU.myvin//1/rpc.UpdateDoor".to_string();
        let status =
            UriValidator::validate_rpc_method(&LongUriSerializer::deserialize(uri).unwrap());
        assert!(status.is_err());
    }

    #[test]
    fn test_all_valid_uris() {
        let json_object = get_json_object().expect("Failed to parse JSON");
        if let Some(valid_uris) = json_object.get("validUris").and_then(|v| v.as_array()) {
            for uri in valid_uris {
                let uri: String = uri.as_str().unwrap_or_default().to_string();
                let uuri = LongUriSerializer::deserialize(uri).unwrap();
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
            if let Ok(uuri) = LongUriSerializer::deserialize(uri.into()) {
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
            let uuri = LongUriSerializer::deserialize(uri.to_string()).unwrap();
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
            let uuri = LongUriSerializer::deserialize(uri.to_string()).unwrap();
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
            let uuri = LongUriSerializer::deserialize(uri.to_string()).unwrap();
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
        let resource = UResource {
            name: "rpc".into(),
            id: Some(19999),
            ..Default::default()
        };
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
            let uuri = LongUriSerializer::deserialize(uri.to_string()).unwrap();
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
