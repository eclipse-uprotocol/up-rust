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
use std::sync::mpsc::Sender;

use crate::uprotocol::{UAttributes, UEntity, UMessage, UPayload, UStatus, UUri, Uuid};

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

    /// Registers a listener to be called when `UPayload` is received for the specific topic.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` for where the message arrived via the underlying transport technology.
    /// * `listener` - The method to execute to process the data for the topic.
    ///
    /// # Returns
    /// Returns a Uuid on success that can be used for unregistering later, otherwise an Err(UStatus) with the appropriate failure information.
    async fn register_listener(
        &self,
        topic: UUri,
        listener: Sender<UMessage>,
    ) -> Result<Uuid, UStatus>;

    /// Unregister a listener for a given topic. Messages arriving on this topic will no longer be processed
    /// by this listener.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` for where the listener was registered to receive messages from.
    /// * `listener` - A Uuid to identify the listener that should be unregistered.
    ///
    /// # Returns
    /// Returns () on success, otherwise an Err(UStatus) with the appropriate failure information.
    async fn unregister_listener(&self, topic: UUri, listener: Uuid) -> Result<(), UStatus>;
}
