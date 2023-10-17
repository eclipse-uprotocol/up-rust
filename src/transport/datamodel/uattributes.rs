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

use crate::transport::datamodel::UMessageType;
use crate::transport::datamodel::UPriority;
use crate::uri::datamodel::UUri;

use std::option::Option;
use uuid::Uuid;

/// When sending data over uTransport, the basic API for sending uses a source topic and the `UPayload` as the data.
/// Additional information about the message is held in the `UAttributes` struct.
///
/// `UAttributes` holds this additional information along with methods to better understand the message sent. It defines
/// the payload and provides options for configuring attributes like time to live, priority, security tokens, and more.
///
/// The payload defined by `UAttributes` can represent different types of message payloads:
/// - A simple published payload with a state change,
/// - A payload representing an RPC request,
/// - A payload representing an RPC response.
#[derive(Debug, Clone, Default)]
pub struct UAttributes {
    pub id: Uuid,                   // Unique identifier for the message
    pub message_type: UMessageType, // Message type
    pub priority: UPriority,        // Message priority

    pub ttl: Option<u32>,        // Time to live in milliseconds
    pub token: Option<String>,   // Authorization token used for TAP
    pub sink: Option<UUri>,      // Explicit destination URI
    pub plevel: Option<u32>,     // Permission Level
    pub commstatus: Option<i32>, // Communication Status
    pub reqid: Option<Uuid>,     // Request ID
}

impl UAttributes {
    pub fn builder() -> UAttributesBuilder {
        UAttributesBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct UAttributesBuilder {
    id: Uuid,
    message_type: UMessageType,
    priority: UPriority,
    ttl: Option<u32>,
    token: Option<String>,
    sink: Option<UUri>,
    plevel: Option<u32>,
    commstatus: Option<i32>,
    reqid: Option<Uuid>,
}

impl UAttributesBuilder {
    pub fn new(id: Uuid, message_type: UMessageType, priority: UPriority) -> UAttributesBuilder {
        UAttributesBuilder {
            id,
            message_type,
            priority,
            ..Default::default()
        }
    }

    /// Function to create a base `UAttributes` for an RPC request.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the RPC request message.
    /// * `sink` - `UUri` describing the exact RPC command.
    ///
    /// # Returns
    ///
    /// A base `UAttributesBuilder` that can be further customized to build an RPC request.
    pub fn for_rpc_request(id: Uuid, sink: UUri) -> UAttributesBuilder {
        UAttributesBuilder {
            id,
            sink: Some(sink),
            ..Default::default()
        }
    }

    /// Sets the time to live (TTL) for the event, representing how long this event should be retained after it was generated, measured in milliseconds.
    ///
    /// Events without this attribute (or where the value is 0) should never timeout.
    ///
    /// # Arguments
    ///
    /// * `ttl` - Time to live in milliseconds for the event.
    ///
    /// # Returns
    ///
    /// An instance of `Self` with the TTL set.
    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Sets the OAuth2 access token to perform the access request as defined in the request message.
    ///
    /// # Arguments
    ///
    /// * `token` - The OAuth2 access token string.
    ///
    /// # Returns
    ///
    /// An instance of `Self` with the token set.
    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    /// Sets an explicit destination URI, primarily used in notifications and RPC messages.
    ///
    /// # Arguments
    ///
    /// * `sink` - The explicit destination URI.
    ///
    /// # Returns
    ///
    /// An instance of `Self` with the sink set.
    pub fn with_sink(mut self, sink: UUri) -> Self {
        self.sink = Some(sink);
        self
    }

    /// Sets the permission level of the message.
    ///
    /// # Arguments
    ///
    /// * `plevel` - The permission level for the message.
    ///
    /// # Returns
    ///
    /// An instance of `Self` with the permission level set.
    pub fn with_plevel(mut self, plevel: u32) -> Self {
        self.plevel = Some(plevel);
        self
    }

    /// Sets the communication status of the message.
    ///
    /// # Arguments
    ///
    /// * `commstatus` - The communication status indicating any potential errors from the platform.
    ///
    /// # Returns
    ///
    /// An instance of `Self` with the communication status set.
    pub fn with_commstatus(mut self, commstatus: i32) -> Self {
        self.commstatus = Some(commstatus);
        self
    }

    /// Sets the `reqid` used to indicate a response for a specific request.
    ///
    /// # Arguments
    ///
    /// * `reqid` - The unique identifier indicating that this is a response for a specific request.
    ///
    /// # Returns
    ///
    /// An instance of `Self` with the request ID set.
    pub fn with_reqid(mut self, reqid: Uuid) -> Self {
        self.reqid = Some(reqid);
        self
    }

    /// Constructs and returns a `UAttributes` instance based on the current state of the builder.
    ///
    /// # Returns
    ///
    /// An `UAttributes` instance.
    pub fn build(self) -> UAttributes {
        UAttributes {
            id: self.id,
            message_type: self.message_type,
            priority: self.priority,
            ttl: self.ttl,
            token: self.token,
            sink: self.sink,
            plevel: self.plevel,
            commstatus: self.commstatus,
            reqid: self.reqid,
        }
    }
}
