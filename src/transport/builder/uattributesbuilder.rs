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

use crate::uprotocol::{UAttributes, UMessageType, UPriority, UUri, Uuid};
use crate::uuid::builder::UUIDv8Builder;

/// Builder for easy construction of the `UAttributes` object.
pub struct UAttributesBuilder {
    id: Uuid,
    message_type: UMessageType,
    priority: UPriority,
    ttl: Option<i32>,
    token: Option<String>,
    sink: Option<UUri>,
    plevel: Option<i32>,
    commstatus: Option<i32>,
    reqid: Option<Uuid>,
}

impl UAttributesBuilder {
    /// Constructs a `UAttributesBuilder` for a publish message.
    ///
    /// # Arguments
    /// * `priority` - The priority of the message.
    ///
    /// # Returns
    /// Returns a `UAttributesBuilder` instance with the configured priority.
    pub fn publish(priority: UPriority) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UmessageTypePublish,
            priority,
            ttl: None,
            token: None,
            sink: None,
            plevel: None,
            commstatus: None,
            reqid: None,
        }
    }

    /// Constructs a `UAttributesBuilder` for a notification message.
    ///
    /// # Arguments
    /// * `priority` - The priority of the message.
    /// * `sink` - The destination URI.
    ///
    /// # Returns
    /// Returns a `UAttributesBuilder` instance with the configured priority and sink.
    pub fn notification(priority: UPriority, sink: UUri) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UmessageTypePublish,
            priority,
            ttl: None,
            token: None,
            sink: Some(sink),
            plevel: None,
            commstatus: None,
            reqid: None,
        }
    }

    /// Constructs a `UAttributesBuilder` for a request message.
    ///
    /// # Arguments
    /// * `priority` - The priority of the message.
    /// * `sink` - The destination URI.
    /// * `ttl` - The time to live in milliseconds.
    ///
    /// # Returns
    /// Returns a `UAttributesBuilder` instance with the configured priority, sink, and ttl.
    pub fn request(priority: UPriority, sink: UUri, ttl: i32) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UmessageTypeRequest,
            priority,
            ttl: Some(ttl),
            token: None,
            sink: Some(sink),
            plevel: None,
            commstatus: None,
            reqid: None,
        }
    }

    /// Constructs a `UAttributesBuilder` for a response message.
    ///
    /// # Arguments
    /// * `priority` - The priority of the message.
    /// * `sink` - The destination URI.
    /// * `reqid` - The original request UUID used to correlate the response to the request.
    ///
    /// # Returns
    /// Returns a `UAttributesBuilder` instance with the configured priority, sink, and reqid.
    pub fn response(priority: UPriority, sink: UUri, reqid: Uuid) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UmessageTypeResponse,
            priority,
            ttl: None,
            token: None,
            sink: Some(sink),
            plevel: None,
            commstatus: None,
            reqid: Some(reqid),
        }
    }

    /// Adds the time to live in milliseconds.
    ///
    /// # Arguments
    /// * `ttl` - The time to live in milliseconds.
    ///
    /// # Returns
    /// Returns the `UAttributesBuilder` instance with the configured ttl.
    #[must_use]
    pub fn with_ttl(&mut self, ttl: u32) -> &mut UAttributesBuilder {
        self.ttl = Some(i32::try_from(ttl).unwrap_or(i32::MAX));
        self
    }

    /// Adds the authorization token used for TAP.
    ///
    /// # Arguments
    /// * `token` - The authorization token used for TAP.
    ///
    /// # Returns
    /// Returns the `UAttributesBuilder` instance with the configured token.
    #[must_use]
    pub fn with_token<T>(&mut self, token: T) -> &mut UAttributesBuilder
    where
        T: Into<String>,
    {
        self.token = Some(token.into());
        self
    }

    /// Adds the explicit destination URI.
    ///
    /// # Arguments
    /// * `sink` - The explicit destination URI.
    ///
    /// # Returns
    /// Returns the `UAttributesBuilder` instance with the configured sink.
    #[must_use]
    pub fn with_sink(&mut self, sink: UUri) -> &mut UAttributesBuilder {
        self.sink = Some(sink);
        self
    }

    /// Adds the permission level of the message.
    ///
    /// # Arguments
    /// * `plevel` - The permission level of the message.
    ///
    /// # Returns
    /// Returns the `UAttributesBuilder` instance with the configured permission level.
    #[must_use]
    pub fn with_permission_level(&mut self, plevel: u32) -> &mut UAttributesBuilder {
        self.plevel = Some(i32::try_from(plevel).unwrap_or(i32::MAX));
        self
    }

    /// Adds the communication status of the message.
    ///
    /// # Arguments
    /// * `commstatus` - The communication status of the message.
    ///
    /// # Returns
    /// Returns the `UAttributesBuilder` instance with the configured communication status.
    #[must_use]
    pub fn with_commstatus(&mut self, commstatus: i32) -> &mut UAttributesBuilder {
        self.commstatus = Some(commstatus);
        self
    }

    /// Adds the request ID.
    ///
    /// # Arguments
    /// * `reqid` - The request ID.
    ///
    /// # Returns
    /// Returns the `UAttributesBuilder` instance with the configured request ID.
    #[must_use]
    pub fn with_reqid(&mut self, reqid: Uuid) -> &mut UAttributesBuilder {
        self.reqid = Some(reqid);
        self
    }

    /// Constructs the `UAttributes` from the builder.
    ///
    /// # Returns
    /// Returns a constructed `UAttributes` instance based on the builder's configuration.
    pub fn build(&self) -> UAttributes {
        UAttributes {
            id: Some(self.id.clone()),
            r#type: self.message_type.into(),
            priority: self.priority.into(),
            ttl: self.ttl,
            token: self.token.clone(),
            sink: self.sink.clone(),
            permission_level: self.plevel,
            commstatus: self.commstatus,
            reqid: self.reqid.clone(),
        }
    }
}
