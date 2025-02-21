/********************************************************************************
 * Copyright (c) 2024 Contributors to the Eclipse Foundation
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

use std::sync::Arc;
use thiserror::Error;

use async_trait::async_trait;
use protobuf::MessageFull;

use crate::communication::RegistrationError;
use crate::{UAttributes, UCode, UStatus, UUri};

use super::{CallOptions, UPayload};

/// An error indicating a problem with invoking a (remote) service operation.
// [impl->req~up-language-comm-api~1]
#[derive(Clone, Error, Debug)]
pub enum ServiceInvocationError {
    /// Indicates that the calling uE requested to add/create something that already exists.
    #[error("entity already exists: {0}")]
    AlreadyExists(String),
    /// Indicates that a request's time-to-live (TTL) has expired.
    ///
    /// Note that this only means that the reply to the request has not been received in time. The request
    /// may still have been processed by the (remote) service provider.
    #[error("request timed out")]
    DeadlineExceeded,
    /// Indicates that the service provider is in a state that prevents it from handling the request.
    #[error("failed precondition: {0}")]
    FailedPrecondition(String),
    /// Indicates that a serious but unspecified internal error has occurred while sending/processing the request.
    #[error("internal error: {0}")]
    Internal(String),
    /// Indicates that the request cannot be processed because some of its parameters are invalid, e.g. not properly formatted.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// Indicates that the requested entity was not found.
    #[error("no such entity: {0}")]
    NotFound(String),
    /// Indicates that the calling uE is authenticated but does not have the required authority to invoke the method.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// Indicates that some of the resources required for processing the request have been exhausted, e.g. disk space, number of API calls.
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),
    /// Indicates an unspecific error that occurred at the Transport Layer while trying to publish a message.
    #[error("unknown error: {0}")]
    RpcError(UStatus),
    /// Indicates that the calling uE could not be authenticated properly.
    #[error("unauthenticated")]
    Unauthenticated,
    /// Indicates that some of the resources required for processing the request are currently unavailable.
    #[error("resource unavailable: {0}")]
    Unavailable(String),
    /// Indicates that part or all of the invoked operation has not been implemented yet.
    #[error("unimplemented: {0}")]
    Unimplemented(String),
}

impl From<UStatus> for ServiceInvocationError {
    fn from(value: UStatus) -> Self {
        match value.code.enum_value() {
            Ok(UCode::ALREADY_EXISTS) => ServiceInvocationError::AlreadyExists(value.get_message()),
            Ok(UCode::DEADLINE_EXCEEDED) => ServiceInvocationError::DeadlineExceeded,
            Ok(UCode::FAILED_PRECONDITION) => {
                ServiceInvocationError::FailedPrecondition(value.get_message())
            }
            Ok(UCode::INTERNAL) => ServiceInvocationError::Internal(value.get_message()),
            Ok(UCode::INVALID_ARGUMENT) => {
                ServiceInvocationError::InvalidArgument(value.get_message())
            }
            Ok(UCode::NOT_FOUND) => ServiceInvocationError::NotFound(value.get_message()),
            Ok(UCode::PERMISSION_DENIED) => {
                ServiceInvocationError::PermissionDenied(value.get_message())
            }
            Ok(UCode::RESOURCE_EXHAUSTED) => {
                ServiceInvocationError::ResourceExhausted(value.get_message())
            }
            Ok(UCode::UNAUTHENTICATED) => ServiceInvocationError::Unauthenticated,
            Ok(UCode::UNAVAILABLE) => ServiceInvocationError::Unavailable(value.get_message()),
            Ok(UCode::UNIMPLEMENTED) => ServiceInvocationError::Unimplemented(value.get_message()),
            _ => ServiceInvocationError::RpcError(value),
        }
    }
}

impl From<ServiceInvocationError> for UStatus {
    fn from(value: ServiceInvocationError) -> Self {
        match value {
            ServiceInvocationError::AlreadyExists(msg) => {
                UStatus::fail_with_code(UCode::ALREADY_EXISTS, msg)
            }
            ServiceInvocationError::DeadlineExceeded => {
                UStatus::fail_with_code(UCode::DEADLINE_EXCEEDED, "request timed out")
            }
            ServiceInvocationError::FailedPrecondition(msg) => {
                UStatus::fail_with_code(UCode::FAILED_PRECONDITION, msg)
            }
            ServiceInvocationError::Internal(msg) => UStatus::fail_with_code(UCode::INTERNAL, msg),
            ServiceInvocationError::InvalidArgument(msg) => {
                UStatus::fail_with_code(UCode::INVALID_ARGUMENT, msg)
            }
            ServiceInvocationError::NotFound(msg) => UStatus::fail_with_code(UCode::NOT_FOUND, msg),
            ServiceInvocationError::PermissionDenied(msg) => {
                UStatus::fail_with_code(UCode::PERMISSION_DENIED, msg)
            }
            ServiceInvocationError::ResourceExhausted(msg) => {
                UStatus::fail_with_code(UCode::RESOURCE_EXHAUSTED, msg)
            }
            ServiceInvocationError::Unauthenticated => {
                UStatus::fail_with_code(UCode::UNAUTHENTICATED, "client must authenticate")
            }
            ServiceInvocationError::Unavailable(msg) => {
                UStatus::fail_with_code(UCode::UNAVAILABLE, msg)
            }
            ServiceInvocationError::Unimplemented(msg) => {
                UStatus::fail_with_code(UCode::UNIMPLEMENTED, msg)
            }
            _ => UStatus::fail_with_code(UCode::UNKNOWN, "unknown"),
        }
    }
}

/// A client for performing Remote Procedure Calls (RPC) on (other) uEntities.
///
/// Please refer to the
/// [Communication Layer API specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc)
/// for details.
// [impl->req~up-language-comm-api~1]
#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
#[async_trait]
pub trait RpcClient: Send + Sync {
    /// Invokes a method on a service.
    ///
    /// # Arguments
    ///
    /// * `method` - The URI representing the method to invoke.
    /// * `call_options` - Options to include in the request message.
    /// * `payload` - The (optional) payload to include in the request message.
    ///
    /// # Returns
    ///
    /// The payload returned by the service operation.
    ///
    /// # Errors
    ///
    /// Returns an error if invocation fails or the given arguments cannot be turned into a valid RPC Request message.
    async fn invoke_method(
        &self,
        method: UUri,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError>;
}

impl dyn RpcClient {
    /// Invokes a method on a service using and returning proto-generated `Message` objects.
    ///
    /// # Arguments
    ///
    /// * `method` - The URI representing the method to invoke.
    /// * `call_options` - Options to include in the request message.
    /// * `request_message` - The protobuf `Message` to include in the request message.
    ///
    /// # Returns
    ///
    /// The payload returned by the service operation as a protobuf `Message`.
    ///
    /// # Errors
    ///
    /// Returns an error if invocation fails, the given arguments cannot be turned into a valid RPC Request message,
    /// result protobuf deserialization fails, or result payload is empty.
    pub async fn invoke_proto_method<T, R>(
        &self,
        method: UUri,
        call_options: CallOptions,
        request_message: T,
    ) -> Result<R, ServiceInvocationError>
    where
        T: MessageFull,
        R: MessageFull,
    {
        let payload = UPayload::try_from_protobuf(request_message)
            .map_err(|e| ServiceInvocationError::InvalidArgument(e.to_string()))?;

        let result = self
            .invoke_method(method, call_options, Some(payload))
            .await?;

        if let Some(result) = result {
            UPayload::extract_protobuf::<R>(&result)
                .map_err(|e| ServiceInvocationError::InvalidArgument(e.to_string()))
        } else {
            Err(ServiceInvocationError::InvalidArgument(
                "No payload".to_string(),
            ))
        }
    }
}

/// A handler for processing incoming RPC requests.
///
// [impl->req~up-language-comm-api~1]
#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handles a request to invoke a method with given input parameters.
    ///
    /// Implementations MUST NOT block the calling thread. Long running
    /// computations should be performed on a separate worker thread, yielding
    /// on the calling thread.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - The resource identifier of the method to invoke.
    /// * `message_attributes` - Any metadata that is associated with the request message.
    /// * `request_payload` - The raw payload that contains the input data for the method.
    ///
    /// # Returns
    ///
    /// the output data generated by the method.
    ///
    /// # Errors
    ///
    /// Returns an error if the request could not be processed successfully.
    async fn handle_request(
        &self,
        resource_id: u16,
        message_attributes: &UAttributes,
        request_payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError>;
}

/// A server for exposing Remote Procedure Call (RPC) endpoints.
///
/// Please refer to the
/// [Communication Layer API specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc)
/// for details.
// [impl->req~up-language-comm-api~1]
#[async_trait]
pub trait RpcServer {
    /// Registers an endpoint for RPC requests.
    ///
    /// Note that only a single endpoint can be registered for a given resource ID.
    /// However, the same request handler can be registered for multiple endpoints.
    ///
    /// # Arguments
    ///
    /// * `origin_filter` - A pattern defining origin addresses to accept requests from. If `None`, requests
    ///                     will be accepted from all sources.
    /// * `resource_id` - The resource identifier of the (local) method to accept requests for.
    /// * `request_handler` - The handler to invoke for each incoming request.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be registered or if a listener has already been registered
    /// for the given resource ID.
    async fn register_endpoint(
        &self,
        origin_filter: Option<&UUri>,
        resource_id: u16,
        request_handler: Arc<dyn RequestHandler>,
    ) -> Result<(), RegistrationError>;

    /// Deregisters a previously [registered endpoint](Self::register_endpoint).
    ///
    /// # Arguments
    ///
    /// * `origin_filter` - The origin pattern that the endpoint had been registered for.
    /// * `resource_id` - The (local) resource identifier that the endpoint had been registered for.
    /// * `request_handler` - The handler to unregister.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be unregistered.
    async fn unregister_endpoint(
        &self,
        origin_filter: Option<&UUri>,
        resource_id: u16,
        request_handler: Arc<dyn RequestHandler>,
    ) -> Result<(), RegistrationError>;
}

#[cfg(any(test, feature = "test-util"))]
mockall::mock! {
    /// This extra struct is necessary in order to comply with mockall's requirements regarding the parameter lifetimes
    /// see <https://github.com/asomers/mockall/issues/571>
    pub RpcServerImpl {
        pub async fn do_register_endpoint<'a>(&'a self, origin_filter: Option<&'a UUri>, resource_id: u16, request_handler: Arc<dyn RequestHandler>) -> Result<(), RegistrationError>;
        pub async fn do_unregister_endpoint<'a>(&'a self, origin_filter: Option<&'a UUri>, resource_id: u16, request_handler: Arc<dyn RequestHandler>) -> Result<(), RegistrationError>;
    }
}

#[cfg(any(test, feature = "test-util"))]
#[async_trait]
/// This delegates the invocation of the UTransport functions to the mocked functions of the Transport struct.
/// see <https://github.com/asomers/mockall/issues/571>
impl RpcServer for MockRpcServerImpl {
    async fn register_endpoint(
        &self,
        origin_filter: Option<&UUri>,
        resource_id: u16,
        request_handler: Arc<dyn RequestHandler>,
    ) -> Result<(), RegistrationError> {
        self.do_register_endpoint(origin_filter, resource_id, request_handler)
            .await
    }
    async fn unregister_endpoint(
        &self,
        origin_filter: Option<&UUri>,
        resource_id: u16,
        request_handler: Arc<dyn RequestHandler>,
    ) -> Result<(), RegistrationError> {
        self.do_unregister_endpoint(origin_filter, resource_id, request_handler)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use protobuf::well_known_types::wrappers::StringValue;

    use crate::{communication::CallOptions, UUri};

    use super::*;

    #[tokio::test]
    async fn test_invoke_proto_method_fails_for_unexpected_return_type() {
        let mut rpc_client = MockRpcClient::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .returning(|_method, _options, _payload| {
                let error = UStatus::fail_with_code(UCode::INTERNAL, "internal error");
                let response_payload = UPayload::try_from_protobuf(error).unwrap();
                Ok(Some(response_payload))
            });
        let client: Arc<dyn RpcClient> = Arc::new(rpc_client);
        let mut request = StringValue::new();
        request.value = "hello".to_string();
        let result = client
            .invoke_proto_method::<StringValue, StringValue>(
                UUri::try_from_parts("", 0x1000, 0x01, 0x0001).unwrap(),
                CallOptions::for_rpc_request(5_000, None, None, None),
                request,
            )
            .await;
        assert!(result.is_err_and(|e| matches!(e, ServiceInvocationError::InvalidArgument(_))));
    }

    #[tokio::test]
    async fn test_invoke_proto_method_fails_for_missing_response_payload() {
        let mut rpc_client = MockRpcClient::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .return_const(Ok(None));
        let client: Arc<dyn RpcClient> = Arc::new(rpc_client);
        let mut request = StringValue::new();
        request.value = "hello".to_string();
        let result = client
            .invoke_proto_method::<StringValue, StringValue>(
                UUri::try_from_parts("", 0x1000, 0x01, 0x0001).unwrap(),
                CallOptions::for_rpc_request(5_000, None, None, None),
                request,
            )
            .await;
        assert!(result.is_err_and(|e| matches!(e, ServiceInvocationError::InvalidArgument(_))));
    }
}
