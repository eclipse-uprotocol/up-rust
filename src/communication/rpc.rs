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

use crate::communication::RegistrationError;
use crate::{UMessage, UStatus, UUri};

/// An error indicating a problem with publishing a message to a topic.
#[derive(Debug)]
pub enum ServiceInvocationError {
    /// Indicates that the given message cannot be sent because it is not a [valid Publish message](crate::PublishValidator).
    InvalidArgument(String),
    /// Indicates an unspecific error that occurred at the Transport Layer while trying to publish a message.
    RpcError(UStatus),
}

impl Display for ServiceInvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceInvocationError::InvalidArgument(s) => {
                f.write_fmt(format_args!("invalid argument: {}", s))
            }
            ServiceInvocationError::RpcError(s) => {
                f.write_fmt(format_args!("failed to invoke service operation: {}", s))
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
    /// * `request` - The request message to be sent to the server.
    ///
    /// # Returns
    ///
    /// Returns the response message if the invocation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if invocation fails or the given message is not a valid RPC Request message.
    async fn invoke_method(&self, request: UMessage) -> Result<UMessage, ServiceInvocationError>;
}

/// A server for exposing RPC endpoints.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
#[async_trait]
pub trait RpcServer: Send + Sync {
    async fn register_endpoint(
        &self,
        source_filter: &UUri,
        method: &UUri,
        listener: Arc<dyn RpcClient>,
    ) -> Result<(), RegistrationError>;
    async fn unregister_endpoint(
        &self,
        source_filter: &UUri,
        method: &UUri,
        listener: Arc<dyn RpcClient>,
    ) -> Result<(), RegistrationError>;
}
