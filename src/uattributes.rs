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

mod uattributesvalidator;
mod upayloadformat;
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
    /// Checks if these are the attributes for a Publish message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_publish());
    /// ```
    pub fn is_publish(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_PUBLISH)
    }

    /// Checks if these are the attributes for an RPC Request message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_request());
    /// ```
    pub fn is_request(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_REQUEST)
    }

    /// Checks if these are the attributes for an RPC Response message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_RESPONSE.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_response());
    /// ```
    pub fn is_response(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_RESPONSE)
    }

    /// Checks if these are the attributes for a Notification message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_notification());
    /// ```
    pub fn is_notification(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_NOTIFICATION)
    }
}
