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

use crate::transport::datamodel::{UListener, UStatus};
use crate::uprotocol::{UAttributes, UEntity, UPayload, UUri};

/// `UTransport` is the uP-L1 interface that provides a common API for uE developers to send and receive messages.
///
/// Implementations of `UTransport` contain the details for connecting to the underlying transport technology and
/// sending `UMessage` using the configured technology. For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
pub trait UTransport {
    /// Authenticates with the underlying transport layer that the `uEntity` passed
    /// matches the transport specific identity. This method requires a resolved `UUri`.
    ///
    /// # Arguments
    /// * `uEntity` - Resolved `UEntity`.
    ///
    /// # Returns
    /// Returns `OKSTATUS` if authentication was successful, `FAILSTATUS` if the calling `uE`
    /// is not authenticated.
    fn authenticate(&self, entity: UEntity) -> UStatus;

    /// Transmits `UPayload` to the topic using the attributes defined in `UTransportAttributes`.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` topic to send the payload to.
    /// * `payload` - Actual payload.
    /// * `attributes` - Additional transport attributes.
    ///
    /// # Returns
    /// Returns `OKSTATUS` if the payload has been successfully sent (ACK'ed), otherwise returns
    /// `FAILSTATUS` with the appropriate failure.
    fn send(&self, topic: UUri, payload: UPayload, attributes: UAttributes) -> UStatus;

    /// Registers a listener to be called when `UPayload` is received for the specific topic.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` for where the message arrived via the underlying transport technology.
    /// * `listener` - The method to execute to process the data for the topic.
    ///
    /// # Returns
    /// Returns `OKSTATUS` if the listener is registered correctly, otherwise returns `FAILSTATUS`
    /// with the appropriate failure.
    fn register_listener(&self, topic: UUri, listener: dyn UListener) -> UStatus;

    /// Unregister a listener for a given topic. Messages arriving on this topic will no longer be processed
    /// by this listener.
    ///
    /// # Arguments
    /// * `topic` - Resolved `UUri` for where the listener was registered to receive messages from.
    /// * `listener` - The method to execute to process the data for the topic.
    ///
    /// # Returns
    /// Returns `OKSTATUS` if the listener is unregistered correctly, otherwise returns `FAILSTATUS`
    /// with the appropriate failure.
    fn unregister_listener(&self, topic: UUri, listener: dyn UListener) -> UStatus;
}
