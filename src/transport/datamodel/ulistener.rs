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

use crate::transport::datamodel::uattributes::UAttributes;
use crate::transport::datamodel::upayload::UPayload;
use crate::transport::datamodel::ustatus::UStatus;
use crate::uri::datamodel::uuri::UUri;

/// For any implementation that defines some kind of callback or function that will be called to handle incoming messages.
pub trait UListener {
    /// Method called to handle or process events.
    ///
    /// # Parameters
    ///
    /// - `topic`: The topic which is the underlying source of the message.
    /// - `payload`: The payload of the message.
    /// - `attributes`: Transportation attributes associated with the message.
    ///
    /// # Returns
    ///
    /// Returns a [`UStatus`] every time a message is received and processed.
    fn on_receive(&self, topic: UUri, payload: UPayload, attributes: UAttributes) -> UStatus;
}
