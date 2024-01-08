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

use crate::uprotocol::{UAttributes, UEntity, UMessage, UPayload, UStatus, UUri};

/// `UTransport` is the uP-L1 interface that provides a common API for uE developers to send and receive messages.
///
/// Implementations of `UTransport` contain the details for connecting to the underlying transport technology and
/// sending `UMessage` using the configured technology. For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
#[async_trait]
pub trait UTransport {
    /// Authenticates with the underlying transport layer that the `uEntity` passed
    /// matches the transport specific identity. This method requires a resolved `UUri`.
    ///
    /// # Arguments
    /// * `uEntity` - Resolved `UEntity`.
    ///
    /// # Returns
    /// Returns () on success, otherwise an Err(UStatus) with the appropriate failure information.
    async fn authenticate(&self, entity: UEntity) -> Result<(), UStatus>;

    /// Transmits `UPayload` to the topic using the attributes defined in `UTransportAttributes`.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` topic to send the payload to.
    /// * `payload` - Actual payload.
    /// * `attributes` - Additional transport attributes.
    ///
    /// # Returns
    /// Returns () on success, otherwise an Err(UStatus) with the appropriate failure information.
    async fn send(
        &self,
        topic: UUri,
        payload: UPayload,
        attributes: UAttributes,
    ) -> Result<(), UStatus>;

    /// Registers a listener to be called asynchronously when `UMessage` is received for the specified topic.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` indicating the topic for which the listener is registered.
    /// * `listener` - A boxed closure (or function pointer) that takes `Result<UMessage, UStatus>` as an argument and returns nothing.
    ///                The closure is executed to process the data or handle the error for the topic.
    ///                It must be `Send`, `Sync` and `'static` to allow transfer across threads and a stable lifetime.
    ///
    /// # Returns
    /// Asynchronously returns a `Result<String, UStatus>`.
    /// On success, returns a `String` containing an identifier that can be used for unregistering the listener later.
    /// On failure, returns `Err(UStatus)` with the appropriate failure information.
    async fn register_listener(
        &self,
        topic: UUri,
        listener: Box<dyn Fn(Result<UMessage, UStatus>) + Send + Sync + 'static>,
    ) -> Result<String, UStatus>;

    /// Unregister a listener for a given topic. Messages arriving on this topic will no longer be processed
    /// by this listener.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` for where the listener was registered to receive messages from.
    /// * `listener` - Identifier of the listener that should be unregistered.
    ///
    /// # Returns
    /// Returns () on success, otherwise an Err(UStatus) with the appropriate failure information.
    async fn unregister_listener(&self, topic: UUri, listener: &str) -> Result<(), UStatus>;
}
