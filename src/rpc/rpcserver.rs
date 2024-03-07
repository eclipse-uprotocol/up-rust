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

use crate::{UMessage, UStatus, UUri};

pub type RpcServerListener = Box<dyn Fn(Result<UMessage, UStatus>) + Send + Sync + 'static>;

/// `RpcServer` is an interface called by uServices to register method listeners for incoming RPC requests from clients.
///
/// For more details, please refer to the [RpcServer Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/rpcserver.adoc)
#[async_trait]
pub trait RpcServer {
    /// Register a listener for a particular method URI to be notified when requests are sent against said method.
    /// Note: Only one listener is allowed to be registered per method URI.
    ///
    /// # Arguments
    /// * `method` - Resolved `UUri` indicating the method for which the listener is registered.
    /// * `listener` - A boxed closure (or function pointer) that takes `Result<UMessage, UStatus>` as an argument and returns nothing.
    ///                The closure is executed to process the data or handle the error for the method.
    ///                It must be `Send`, `Sync` and `'static` to allow transfer across threads and a stable lifetime.
    ///
    /// # Returns
    /// Asynchronously returns a `Result<String, UStatus>`.
    /// On success, returns a `String` containing an identifier that can be used for unregistering the listener later.
    /// On failure, returns `Err(UStatus)` with the appropriate failure information.
    async fn register_rpc_listener(
        &self,
        method: UUri,
        listener: RpcServerListener,
    ) -> Result<String, UStatus>;

    /// Unregister an RPC listener for a given method Uri. Messages arriving on this method
    /// will no longer be processed by this listener.
    ///
    /// # Arguments
    /// * `method` - Resolved `UUri` for where the listener was registered to receive messages from.
    /// * `listener` - Identifier of the listener that should be unregistered.
    ///
    /// # Returns
    /// Returns () on success, otherwise an Err(UStatus) with the appropriate failure information.
    async fn unregister_rpc_listener(&self, method: UUri, listener: &str) -> Result<(), UStatus>;
}
