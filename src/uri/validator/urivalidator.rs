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

use crate::transport::datamodel::ustatus::{UCode, UStatus};
use crate::uri::datamodel::uresource::UResource;
use crate::uri::datamodel::uuri::UUri;
use crate::uri::serializer::longuriserializer::LongUriSerializer;
use crate::uri::serializer::uriserializer::UriSerializer;

/// Struct to encapsulate Uri validation logic.
pub struct UriValidator;

impl UriValidator {
    /// Validates a `UUri` to ensure that it has at least a name for the `uEntity`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `UUri` instance to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` containing either a success or a failure, along with the corresponding error message.
    pub fn validate(uri: &UUri) -> UStatus {
        if uri.is_empty() {
            return UStatus::fail_with_msg_and_reason("Uri is empty.", UCode::InvalidArgument);
        }

        if uri.entity.name.trim().is_empty() {
            return UStatus::fail_with_msg_and_reason(
                "Uri is missing uSoftware Entity name.",
                UCode::InvalidArgument,
            );
        }
        UStatus::ok()
    }

    /// Validates a `UUri` that is meant to be used as an RPC method URI.
    /// This is used in Request sink values and Response source values.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `UUri` instance to validate.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` containing either a success or a failure, along with the corresponding error message.
    pub fn validate_rpc_method(uri: &UUri) -> UStatus {
        let status = Self::validate(uri);
        if status.is_failed() {
            return status;
        }
        let uresource = &uri.resource;
        if !uresource.is_rpc_method() {
            return UStatus::fail_with_msg_and_reason(
                "Invalid RPC method uri. Uri should be the method to be called, or method from response.",
                UCode::InvalidArgument,
            );
        }
        UStatus::ok()
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
    pub fn validate_rpc_response(uri: &UUri) -> UStatus {
        let status = Self::validate(uri);
        if status.is_failed() {
            return status;
        }

        let uresource = &uri.resource;
        if !uresource.is_rpc_method()
            || !uresource
                .instance
                .eq(&UResource::for_rpc_response().instance)
        {
            return UStatus::fail_with_msg_and_reason(
                "Invalid RPC response type.",
                UCode::InvalidArgument,
            );
        }
        UStatus::ok()
    }

    // final UResource uResource = uri.uResource();
    // if (!uResource.isRPCMethod() || !uResource.instance().equals(UResource.forRpcResponse().instance())) {
    //     return UStatus.failed("Invalid RPC response type.", Code.INVALID_ARGUMENT);
    // }

    /// Validates a long `uProtocol` URI.
    ///
    /// This function takes a URI string and validates it according to the rules defined
    /// for long `uProtocol` URIs. If the URI is valid, it returns an `Ok` status. Otherwise,
    /// it returns a `Fail` status with a corresponding error message and code.
    ///
    /// # Arguments
    ///
    /// * `uri` - A string slice that holds the long `uProtocol` URI to be validated.
    ///
    /// # Returns
    ///
    /// Returns a `UStatus` object that contains the result of the validation.
    pub fn validate_long_uuri(uri: &str) -> UStatus {
        let uuri = LongUriSerializer::deserialize(uri.to_string());
        Self::validate(&uuri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::transport::datamodel::ustatus::UCode;
    use crate::uri::datamodel::uauthority::UAuthority;
    use crate::uri::datamodel::uentity::UEntity;

    #[test]
    fn test_validate_blank_uri() {
        let uri = LongUriSerializer::deserialize("".to_string());
        let status = UriValidator::validate(&uri);
        assert!(uri.is_empty());
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Uri is empty.", status.message());
    }

    #[test]
    fn test_validate_uri_with_no_entity_name() {
        let uri = LongUriSerializer::deserialize("//".to_string());
        let status = UriValidator::validate(&uri);
        assert!(uri.is_empty());
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Uri is empty.", status.message());
    }

    #[test]
    fn test_validate_uri_with_entity() {
        let uri = LongUriSerializer::deserialize("/hartley".to_string());
        let status = UriValidator::validate(&uri);
        assert_eq!(UStatus::ok().code_as_int(), status.code_as_int());
    }

    #[test]
    fn test_validate_with_malformed_uri() {
        let uri = LongUriSerializer::deserialize("hartley".to_string());
        let status = UriValidator::validate(&uri);
        assert!(uri.is_empty());
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Uri is empty.", status.message());
    }

    #[test]
    fn test_validate_with_blank_uentity_name_uri() {
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(UEntity::EMPTY),
            Some(UResource::for_rpc_request(Some("echo".to_string()), None)),
        );
        let status = UriValidator::validate(&uri);
        assert!(!uri.is_empty());
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Uri is missing uSoftware Entity name.", status.message());
    }

    #[test]
    fn test_validate_rpc_method_with_valid_uri() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.echo".to_string());
        let status = UriValidator::validate_rpc_method(&uri);
        assert_eq!(UStatus::ok().code_as_int(), status.code_as_int());
    }

    #[test]
    fn test_validate_rpc_method_with_invalid_uri() {
        let uri = LongUriSerializer::deserialize("/hartley/echo".to_string());
        let status = UriValidator::validate_rpc_method(&uri);
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Invalid RPC method uri. Uri should be the method to be called, or method from response.", status.message());
    }

    #[test]
    fn test_validate_rpc_method_with_malformed_uri() {
        let uri = LongUriSerializer::deserialize("hartley".to_string());
        let status = UriValidator::validate_rpc_method(&uri);
        assert!(uri.is_empty());
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Uri is empty.", status.message());
    }

    #[test]
    fn test_validate_rpc_response_with_valid_uri() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.response".to_string());
        let status = UriValidator::validate_rpc_response(&uri);
        assert_eq!(UStatus::ok().code_as_int(), status.code_as_int());
    }

    #[test]
    fn test_validate_rpc_response_with_malformed_uri() {
        let uri = LongUriSerializer::deserialize("hartley".to_string());
        let status = UriValidator::validate_rpc_response(&uri);
        assert!(uri.is_empty());
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Uri is empty.", status.message());
    }

    #[test]
    fn test_validate_rpc_response_with_rpc_type() {
        let uri = LongUriSerializer::deserialize("/hartley//dummy.wrong".to_string());
        let status = UriValidator::validate_rpc_response(&uri);
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Invalid RPC response type.", status.message());
    }

    #[test]
    fn test_validate_rpc_response_with_invalid_rpc_response_type() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.wrong".to_string());
        let status = UriValidator::validate_rpc_response(&uri);
        assert_eq!(UCode::InvalidArgument as i32, status.code_as_int());
        assert_eq!("Invalid RPC response type.", status.message());
    }

    #[test]
    fn test_validate_long_uuri_with_valid_uri() {
        let uri = LongUriSerializer::deserialize("/hartley//rpc.echo".to_string());
        let status = UriValidator::validate_long_uuri(&LongUriSerializer::serialize(&uri));
        assert_eq!(UStatus::ok().code_as_int(), status.code_as_int());
    }
}
