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

use crate::transport::datamodel::umessagetype::UMessageType;
use crate::transport::datamodel::upriority::UPriority;
use crate::transport::datamodel::userializationhint::USerializationHint;
use crate::uri::datamodel::uuri::UUri;

use std::option::Option;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct UAttributes {
    // Required Attributes
    pub id: Uuid,                   // Unique identifier for the message
    pub message_type: UMessageType, // Message type
    pub priority: UPriority,        // Message priority

    // Optional Attributes
    pub ttl: Option<u32>,                 // Time to live in milliseconds
    pub token: Option<String>,            // Authorization token used for TAP
    pub hint: Option<USerializationHint>, // Hint regarding the bytes contained within the UPayload
    pub sink: Option<UUri>,               // Explicit destination URI
    pub plevel: Option<u32>,              // Permission Level
    pub commstatus: Option<i32>,          // Communication Status
    pub reqid: Option<Uuid>,              // Request ID
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
    hint: Option<USerializationHint>,
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

    pub fn for_rpc_request(id: Uuid, sink: UUri) -> UAttributesBuilder {
        UAttributesBuilder {
            id,
            sink: Some(sink),
            ..Default::default()
        }
    }

    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub fn with_hint(mut self, hint: USerializationHint) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn with_sink(mut self, sink: UUri) -> Self {
        self.sink = Some(sink);
        self
    }

    pub fn with_plevel(mut self, plevel: u32) -> Self {
        self.plevel = Some(plevel);
        self
    }

    pub fn with_commstatus(mut self, commstatus: i32) -> Self {
        self.commstatus = Some(commstatus);
        self
    }

    pub fn with_reqid(mut self, reqid: Uuid) -> Self {
        self.reqid = Some(reqid);
        self
    }

    pub fn build(self) -> Option<UAttributes> {
        Some(UAttributes {
            id: self.id,
            message_type: self.message_type,
            priority: self.priority,
            ttl: self.ttl,
            token: self.token,
            hint: self.hint,
            sink: self.sink,
            plevel: self.plevel,
            commstatus: self.commstatus,
            reqid: self.reqid,
        })
    }
}
