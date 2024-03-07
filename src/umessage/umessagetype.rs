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

use protobuf::Enum;

pub use crate::up_core_api::uattributes::UMessageType;

const MESSAGE_TYPE_PUBLISH: &str = "pub.v1";
const MESSAGE_TYPE_REQUEST: &str = "req.v1";
const MESSAGE_TYPE_RESPONSE: &str = "res.v1";
const MESSAGE_TYPE_UNSPECIFIED: &str = "unspec.v1";

impl UMessageType {
    /// Gets this message type's integer code as a [`String`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UMessageType;
    ///
    /// assert_eq!(UMessageType::UMESSAGE_TYPE_PUBLISH.to_i32_string(), "1");
    /// ```
    pub fn to_i32_string(&self) -> String {
        self.value().to_string()
    }

    /// Gets the message type for an integer encoded as a string.
    ///
    /// # Returns
    ///
    /// [`UMessageType::UMESSAGE_TYPE_UNSPECIFIED`] if the string cannot be parsed
    /// into an [`i32`] or the integer does not match any of the supported message types.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UMessageType;
    ///
    /// let message_type = UMessageType::from_i32_string("1");
    /// assert_eq!(message_type, UMessageType::UMESSAGE_TYPE_PUBLISH);
    ///
    /// let message_type = UMessageType::from_i32_string("16");
    /// assert_eq!(message_type, UMessageType::UMESSAGE_TYPE_UNSPECIFIED);
    ///
    /// let message_type = UMessageType::from_i32_string("foo.bar");
    /// assert_eq!(message_type, UMessageType::UMESSAGE_TYPE_UNSPECIFIED);
    /// ```
    pub fn from_i32_string<S: Into<String>>(value: S) -> Self {
        if let Ok(code) = value.into().parse::<i32>() {
            UMessageType::from_i32(code).unwrap_or(UMessageType::UMESSAGE_TYPE_UNSPECIFIED)
        } else {
            UMessageType::UMESSAGE_TYPE_UNSPECIFIED
        }
    }

    /// Gets this message type's string identifier.
    ///
    /// # Returns
    ///
    /// A stable identifier which can be used to encode a message type and decode it
    /// again using [`UMessageType::from_type_string`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UMessageType;
    ///
    /// let identifier = UMessageType::UMESSAGE_TYPE_PUBLISH.to_type_string();
    /// assert_eq!(identifier, "pub.v1");
    ///
    /// let message_type = UMessageType::from_type_string(identifier);
    /// assert_eq!(message_type, UMessageType::UMESSAGE_TYPE_PUBLISH);
    /// ```
    pub fn to_type_string(&self) -> &'static str {
        match self {
            UMessageType::UMESSAGE_TYPE_PUBLISH => MESSAGE_TYPE_PUBLISH,
            UMessageType::UMESSAGE_TYPE_REQUEST => MESSAGE_TYPE_REQUEST,
            UMessageType::UMESSAGE_TYPE_RESPONSE => MESSAGE_TYPE_RESPONSE,
            UMessageType::UMESSAGE_TYPE_UNSPECIFIED => MESSAGE_TYPE_UNSPECIFIED,
        }
    }

    /// Gets the message type for a string identifier.
    ///
    /// # Returns
    ///
    /// [`UMessageType::UMESSAGE_TYPE_UNSPECIFIED`] if the identifier does not match
    /// any of the supported message types.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UMessageType;
    ///
    /// let message_type = UMessageType::from_type_string("pub.v1");
    /// assert_eq!(message_type, UMessageType::UMESSAGE_TYPE_PUBLISH);
    ///
    /// let message_type = UMessageType::from_type_string("foo.bar");
    /// assert_eq!(message_type, UMessageType::UMESSAGE_TYPE_UNSPECIFIED);
    /// ```
    pub fn from_type_string<S: Into<String>>(value: S) -> Self {
        let type_string = value.into();
        match type_string.as_str() {
            MESSAGE_TYPE_PUBLISH => UMessageType::UMESSAGE_TYPE_PUBLISH,
            MESSAGE_TYPE_REQUEST => UMessageType::UMESSAGE_TYPE_REQUEST,
            MESSAGE_TYPE_RESPONSE => UMessageType::UMESSAGE_TYPE_RESPONSE,
            _ => UMessageType::UMESSAGE_TYPE_UNSPECIFIED,
        }
    }
}
