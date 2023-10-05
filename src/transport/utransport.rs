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
use crate::transport::datamodel::ulistener::UListener;
use crate::transport::datamodel::upayload::UPayload;
use crate::transport::datamodel::ustatus::UStatus;
use crate::uri::datamodel::uentity::UEntity;
use crate::uri::datamodel::uuri::UUri;

pub trait UTransport {
    /// API to register the calling uE with the underlying transport implementation.
    fn register(&self, uentity: UEntity, token: &[u8]) -> UStatus;

    /// Transmit UPayload to the topic using the attributes defined in UTransportAttributes.
    fn send(&self, topic: UUri, payload: UPayload, attributes: UAttributes) -> UStatus;

    /// Register a method that will be called when a message comes in on the specific topic.
    fn register_listener(&self, topic: UUri, listener: dyn UListener) -> UStatus;

    /// Unregister a method on a topic. Messages arriving on this topic will no longer be processed by this listener.
    fn unregister_listener(&self, topic: UUri, listener: dyn UListener) -> UStatus;
}
