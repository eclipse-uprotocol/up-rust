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

use crate::{UUri, UUID};

mod uattributesvalidator;
mod upriority;

pub use uattributesvalidator::*;
pub use upriority::*;

pub use crate::up_core_api::uattributes::*;

#[derive(Debug)]
pub enum UAttributesError {
    ValidationError(String),
    ParsingError(String),
}

impl UAttributesError {
    pub fn validation_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ValidationError(message.into())
    }

    pub fn parsing_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ParsingError(message.into())
    }
}

impl std::fmt::Display for UAttributesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationError(e) => f.write_fmt(format_args!("Validation failure: {}", e)),
            Self::ParsingError(e) => f.write_fmt(format_args!("Parsing error: {}", e)),
        }
    }
}

impl std::error::Error for UAttributesError {}

impl UAttributes {
    /// Creates attributes representing an RPC request message.
    ///
    /// The message's priority will be set to [`UPriority::UPRIORITY_CS4`].
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message's identifier. This identifier will also be used as the correlation ID
    ///                  in RPC response messages created by the service being invoked.
    /// * `method` - The URI identifying the method to invoke.
    /// * `reply_to_address` - The URI that the sender of the request expects to receive the response message at.
    /// * `options` - Additional options relevant for the request, like time-to-live or access token.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType, UPriority, UUIDBuilder, UUri};

    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let message_id = UUIDBuilder::build();
    /// let method_to_invoke = UUri::try_from("my-vehicle/1004F3B/3/B42")?;
    /// let reply_to_address = UUri::try_from("my-cloud/A/1/0")?;
    /// let token = Some("my_token".to_string());
    /// let ttl = Some(5_000);
    /// let attributes = UAttributes::request(message_id.clone(), method_to_invoke.clone(), reply_to_address.clone(), None, token, ttl);
    /// assert_eq!(attributes.type_, UMessageType::UMESSAGE_TYPE_REQUEST.into());
    /// assert_eq!(attributes.id, Some(message_id).into());
    /// assert_eq!(attributes.priority, UPriority::UPRIORITY_CS4.into());
    /// assert_eq!(attributes.source, Some(reply_to_address).into());
    /// assert_eq!(attributes.sink, Some(method_to_invoke).into());
    /// assert_eq!(attributes.ttl, Some(5000));
    /// assert_eq!(attributes.token, Some("my_token".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn request(
        message_id: UUID,
        method: UUri,
        reply_to_address: UUri,
        priority: Option<UPriority>,
        token: Option<String>,
        ttl: Option<u32>,
    ) -> Self {
        Self {
            type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
            id: Some(message_id).into(),
            priority: priority.unwrap_or(UPriority::UPRIORITY_CS4).into(),
            source: Some(reply_to_address).into(),
            sink: Some(method).into(),
            ttl,
            token,
            ..Default::default()
        }
    }
}
