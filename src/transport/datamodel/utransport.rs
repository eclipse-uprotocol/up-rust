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

use crate::uprotocol::{UMessage, UStatus, UUri};

/// `UTransport` is the uP-L1 interface that provides a common API for uE developers to send and receive messages.
///
/// Implementations of `UTransport` contain the details for connecting to the underlying transport technology and
/// sending `UMessage` using the configured technology. For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
#[async_trait]
pub trait UTransport {
    /// Sends a message over the transport.
    ///
    /// This function asynchronously sends a message over the network transport.
    /// It's designed to handle the transmission of a message object to a designated recipient.
    ///
    /// # Arguments
    /// * `message` - The message to be sent. This encapsulates the data intended for transmission.
    ///
    /// # Returns
    /// On success, returns `Ok(())`. If the message sending fails, it returns `Err(UStatus)`,
    /// where `UStatus` contains a `UCode` indicating the specific error or failure reason.
    async fn send(&self, message: UMessage) -> Result<(), UStatus>;

    /// Receives a message from the transport.
    ///
    /// This function asynchronously receives a message from the network transport based on the provided topic.
    /// It's designed to handle the reception of a message object from the network.
    ///
    /// # Arguments
    /// * `topic` - The topic from which to receive messages.
    ///
    /// # Returns
    /// On success, returns the received message wrapped in `Ok(_)`.
    /// If receiving the message fails, it returns `Err(UStatus)`,
    /// where `UStatus` contains a `UCode` indicating the specific error or failure reason.
    async fn receive(&self, topic: UUri) -> Result<UMessage, UStatus>;

    /// Registers a listener to be called asynchronously when a message is received for the specified topic.
    ///
    /// # Arguments
    /// * `topic` - Resolved topic uri indicating the topic for which the listener is registered.
    /// * `listener` - A boxed closure (or function pointer) that takes `Result<UMessage, UStatus>` as an argument and returns nothing.
    ///                The closure is executed to process the data or handle the error for the topic.
    ///                It must be `Send`, `Sync` and `'static` to allow transfer across threads and a stable lifetime.
    ///
    /// # Returns
    /// On success, returns a `String` identifier that can be used for unregistering the listener later.
    /// On failure, returns `Err(UStatus)` with failure information.
    async fn register_listener(
        &self,
        topic: UUri,
        listener: Box<dyn Fn(Result<UMessage, UStatus>) + Send + Sync + 'static>,
    ) -> Result<String, UStatus>;

    /// Unregister a listener for a given topic. Messages arriving on this topic will no longer be processed
    /// by this listener.
    ///
    /// # Arguments
    /// * `topic` - Resolved topic uri where the listener was registered originally.
    /// * `listener` - Identifier of the listener that should be unregistered.
    ///
    /// # Returns
    /// Returns () on success, otherwise an Err(UStatus) with failure information.
    async fn unregister_listener(&self, topic: UUri, listener: &str) -> Result<(), UStatus>;
}
