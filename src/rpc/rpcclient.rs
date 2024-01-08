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

use crate::rpc::rpcmapper::RpcMapperError;
use crate::uprotocol::{UAttributes, UPayload, UUri};

pub type RpcClientResult = Result<UPayload, RpcMapperError>;

/// `RpcClient` serves as an interface to be used by code generators for uProtocol services defined in
/// `.proto` files, such as the core uProtocol services found in
/// [uProtocol Core API](https://github.com/eclipse-uprotocol/uprotocol-core-api).
///
/// The trait provides a clean contract for all transports to implement, enabling support for RPC on their platforms.
/// Every platform MUST implement this trait.
///
/// For more details, please refer to the
/// [RpcClient Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l2/README.adoc).
#[async_trait]
pub trait RpcClient {
    /// Support for RPC method invocation.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to invoke the method on.
    /// * `payload` - The payload to send.
    /// * `attributes` - The attributes to send.
    ///
    /// # Returns
    ///
    /// Returns a Future with the result or error.
    async fn invoke_method(
        &self,
        topic: UUri,
        payload: UPayload,
        attributes: UAttributes,
    ) -> RpcClientResult;
}
