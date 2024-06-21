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

use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;
use protobuf::Message;

use crate::communication::RegistrationError;
use crate::{UCode, UStatus, UUri};

use super::{CallOptions, UPayload};

/// An error indicating a problem with publishing a message to a topic.
#[derive(Debug)]
pub enum ServiceInvocationError {
    /// Indicates that a request's time-to-live (TTL) has expired.
    ///
    /// Note that this only means that the reply to the request has not been received in time. The request
    /// may still have been processed by the (remote) service provider.
    DeadlineExceeded,
    /// Indicates that the request cannot be processed because some of its parameters are not as expected.
    InvalidArgument(String),
    /// Indicates an unspecific error that occurred at the Transport Layer while trying to publish a message.
    RpcError(UStatus),
}

impl From<UStatus> for ServiceInvocationError {
    fn from(value: UStatus) -> Self {
        match value.code.enum_value() {
            Ok(UCode::DEADLINE_EXCEEDED) => ServiceInvocationError::DeadlineExceeded,
            Ok(UCode::INVALID_ARGUMENT) => {
                ServiceInvocationError::InvalidArgument(value.get_message())
            }
            _ => ServiceInvocationError::RpcError(value),
        }
    }
}

impl Display for ServiceInvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceInvocationError::DeadlineExceeded => f.write_str("request timed out"),
            ServiceInvocationError::InvalidArgument(s) => f.write_str(s.as_str()),
            ServiceInvocationError::RpcError(s) => {
                f.write_fmt(format_args!("failed to send invoke method: {}", s))
            }
        }
    }
}

impl Error for ServiceInvocationError {}

/// A client for invoking RPC methods.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
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
    /// * `proto_message` - The protobuf `Message` to include in the request message.
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
        proto_message: T,
    ) -> Result<R, ServiceInvocationError>
    where
        T: Message,
        R: Message,
    {
        let payload = UPayload::try_from_protobuf(proto_message)
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
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Invokes a method with given input parameters.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - The resource identifier of the method to invoke.
    /// * `payload` - The raw payload that contains the input data for the method.
    ///
    /// # Returns
    ///
    /// the output data generated by the method.
    ///
    /// # Errors
    ///
    /// Returns an error if the method request could not be processed successfully.
    async fn invoke_method(
        &self,
        resource_id: u16,
        payload: UPayload,
    ) -> Result<Option<UPayload>, ServiceInvocationError>;
}

/// A server for exposing RPC endpoints.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
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

    /// Unregisters a previously [registered endpoint](Self::register_endpoint).
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
