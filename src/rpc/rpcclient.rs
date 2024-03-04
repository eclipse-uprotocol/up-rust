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

use async_trait::async_trait;

use crate::{CallOptions, RpcMapperError, UMessage, UPayload, UUri};

pub type RpcClientResult = Result<UMessage, RpcMapperError>;

/// `RpcClient` is an interface used by code generators for uProtocol services defined in `.proto` files such as
/// the core uProtocol services found in [uProtocol Core API](https://github.com/eclipse-uprotocol/uprotocol-core-api).
///
/// The trait provides a clean contract for mapping a RPC requiest to a response. For more details please refer to the
/// [RpcClient Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l2/README.adoc).
#[async_trait]
pub trait RpcClient {
    /// Invokes a method on a remote service asynchronously.
    ///
    /// This function is an API for clients to send an RPC request and receive a response.
    /// The client specifies the method to be invoked using the `method` parameter,
    /// which is the URI of the method. The `request` contains the request message, and
    /// `options` includes various metadata and settings for the method invocation.
    ///
    /// # Arguments
    ///
    /// * `method` - The URI of the method to be invoked. For example, in long form: "/example.hello_world/1/rpc.SayHello".
    /// * `request` - The request message to be sent to the server.
    /// * `options` - Call options for the RPC method invocation, as specified by `CallOptions`.
    ///
    /// # Returns
    ///
    /// Returns a `RpcClientResult` which contains the response message.
    /// If the invocation fails, it contains a `UStatus` detailing the failure reason.
    async fn invoke_method(
        &self,
        method: UUri,
        request: UPayload,
        options: CallOptions,
    ) -> RpcClientResult;
}
