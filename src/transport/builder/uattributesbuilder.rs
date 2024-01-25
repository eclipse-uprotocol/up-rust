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

use crate::uprotocol::uattributes::{UAttributes, UMessageType, UPriority};
use crate::uprotocol::uri::UUri;
use crate::uprotocol::uuid::UUID;
use crate::uuid::builder::UUIDv8Builder;

/// Builder for easy construction of the `UAttributes` object.
pub struct UAttributesBuilder {
    id: UUID,
    message_type: UMessageType,
    priority: UPriority,
    ttl: Option<i32>,
    token: Option<String>,
    sink: Option<UUri>,
    plevel: Option<i32>,
    commstatus: Option<i32>,
    reqid: Option<UUID>,
}

impl UAttributesBuilder {
    /// Gets a builder for creating a publish message.
    ///
    /// # Arguments
    ///
    /// * `priority` - The priority of the message.
    ///
    /// # Returns
    ///
    /// The builder initialized with the given values.
    pub fn publish(priority: UPriority) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UMESSAGE_TYPE_PUBLISH,
            priority,
            ttl: None,
            token: None,
            sink: None,
            plevel: None,
            commstatus: None,
            reqid: None,
        }
    }

    /// Gets a builder for creating a notification message.
    ///
    /// # Arguments
    ///
    /// * `priority` - The priority of the message.
    /// * `sink` - The destination URI.
    ///
    /// # Returns
    ///
    /// The builder initialized with the given values.
    pub fn notification(priority: UPriority, sink: UUri) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UMESSAGE_TYPE_PUBLISH,
            priority,
            ttl: None,
            token: None,
            sink: Some(sink),
            plevel: None,
            commstatus: None,
            reqid: None,
        }
    }

    /// Gets a builder for creating a request message.
    ///
    /// # Arguments
    ///
    /// * `priority` - The priority of the message.
    /// * `sink` - The destination URI.
    /// * `ttl` - The time to live in milliseconds. The value is capped at [`i32::MAX`].
    ///
    /// # Returns
    ///
    /// The builder initialized with the given values.
    pub fn request(priority: UPriority, sink: UUri, ttl: u32) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UMESSAGE_TYPE_REQUEST,
            priority,
            ttl: Some(i32::try_from(ttl).unwrap_or(i32::MAX)),
            token: None,
            sink: Some(sink),
            plevel: None,
            commstatus: None,
            reqid: None,
        }
    }

    /// Gets a builder for creating a repsonse message.
    ///
    /// # Arguments
    ///
    /// * `priority` - The priority of the message.
    /// * `sink` - The destination URI.
    /// * `reqid` - The original request UUID used to correlate the response to the request.
    ///
    /// # Returns
    ///
    /// The builder initialized with the given values.
    pub fn response(priority: UPriority, sink: UUri, reqid: UUID) -> UAttributesBuilder {
        UAttributesBuilder {
            id: UUIDv8Builder::new().build(),
            message_type: UMessageType::UMESSAGE_TYPE_RESPONSE,
            priority,
            ttl: None,
            token: None,
            sink: Some(sink),
            plevel: None,
            commstatus: None,
            reqid: Some(reqid),
        }
    }

    /// Sets the message's time-to-live.
    ///
    /// # Arguments
    ///
    /// * `ttl` - The time-to-live in milliseconds. The value is capped at [`i32::MAX`].
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn with_ttl(&mut self, ttl: u32) -> &mut UAttributesBuilder {
        self.ttl = Some(i32::try_from(ttl).unwrap_or(i32::MAX));
        self
    }

    /// Sets the message's authorization token used for TAP.
    ///
    /// # Arguments
    ///
    /// * `token` - The token.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn with_token<T>(&mut self, token: T) -> &mut UAttributesBuilder
    where
        T: Into<String>,
    {
        self.token = Some(token.into());
        self
    }

    /// Sets the message's destination URI.
    ///
    /// # Arguments
    ///
    /// * `sink` - The URI.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn with_sink(&mut self, sink: UUri) -> &mut UAttributesBuilder {
        self.sink = Some(sink);
        self
    }

    /// Sets the message's permission level.
    ///
    /// # Arguments
    ///
    /// * `plevel` - The level.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn with_permission_level(&mut self, plevel: u32) -> &mut UAttributesBuilder {
        self.plevel = Some(i32::try_from(plevel).unwrap_or(i32::MAX));
        self
    }

    /// Sets the message's communication status.
    ///
    /// # Arguments
    ///
    /// * `commstatus` - The status.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn with_commstatus(&mut self, commstatus: i32) -> &mut UAttributesBuilder {
        self.commstatus = Some(commstatus);
        self
    }

    /// Sets the messgae's request ID.
    ///
    /// # Arguments
    ///
    /// * `reqid` - The ID.
    ///
    /// # Returns
    ///
    /// The builder.
    #[must_use]
    pub fn with_reqid(&mut self, reqid: UUID) -> &mut UAttributesBuilder {
        self.reqid = Some(reqid);
        self
    }

    /// Creates the attributes based on the builder's state.
    ///
    /// # Returns
    ///
    /// The attributes.
    pub fn build(&self) -> UAttributes {
        UAttributes {
            id: Some(self.id.clone()).into(),
            type_: self.message_type.into(),
            priority: self.priority.into(),
            ttl: self.ttl,
            token: self.token.clone(),
            sink: self.sink.clone().into(),
            permission_level: self.plevel,
            commstatus: self.commstatus,
            reqid: self.reqid.clone().into(),
            ..Default::default()
        }
    }
}
