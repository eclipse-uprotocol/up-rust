/********************************************************************************
 * Copyright (c) 2024 Contributors to the Eclipse Foundation
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

use bytes::Bytes;
pub use notification::{NotificationError, NotificationListener, Notifier};
pub use pubsub::{PubSubError, Publisher, Subscriber};
pub use rpc::{RpcClient, RpcServer, ServiceInvocationError};
use std::{error::Error, fmt::Display};

use crate::{UPayloadFormat, UPriority};

mod notification;
mod pubsub;
mod rpc;

/// An error indicating a problem with registering or unregistering a message listener.
#[derive(Debug)]
pub enum RegistrationError {
    /// Indicates that the maximum number of listeners supported by the Transport Layer implementation
    /// has already been registered.
    MaxListenersExceeded,
    /// Indicates that no listener is registered for given pattern URIs.
    NoSuchListener,
    /// Indicates that the underlying Transport Layer implementation does not support registration and
    /// notification of message handlers.
    PushDeliveryMethodNotSupported,
}

impl Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::MaxListenersExceeded => {
                f.write_str("maximum number of listeners has been reached")
            }
            RegistrationError::NoSuchListener => {
                f.write_str("no listener registered for given pattern")
            }
            RegistrationError::PushDeliveryMethodNotSupported => f.write_str(
                "the underlying transport implementation does not support the push delivery method",
            ),
        }
    }
}

impl Error for RegistrationError {}

const DEFAULT_TTL: u16 = 10_000; // 10 seconds

/// General options that clients might want to specify when sending a uProtocol message.
#[derive(Debug)]
pub struct CallOptions {
    ttl: u16,
    token: Option<String>,
    priority: Option<UPriority>,
}

impl Default for CallOptions {
    /// Creates empty options with a TTL of 10s.
    fn default() -> Self {
        CallOptions {
            ttl: DEFAULT_TTL,
            token: None,
            priority: None,
        }
    }
}

impl CallOptions {
    pub fn with_ttl(&mut self, ttl: u16) -> &mut Self {
        self.ttl = ttl;
        self
    }
    pub fn ttl(&self) -> u16 {
        self.ttl
    }
    pub fn with_token(&mut self, token: String) -> &mut Self {
        self.token = Some(token);
        self
    }
    pub fn token(&self) -> Option<String> {
        self.token.clone()
    }
    pub fn with_priority(&mut self, priority: UPriority) -> &mut Self {
        self.priority = Some(priority);
        self
    }
    pub fn priority(&self) -> Option<UPriority> {
        self.priority
    }
}

/// A wrapper around (raw) message payload data and the corresponding payload format.
#[derive(Clone)]
pub struct UPayload {
    payload_format: UPayloadFormat,
    payload: Bytes,
}

impl UPayload {
    pub fn new(payload: Bytes, payload_format: UPayloadFormat) -> Self {
        UPayload {
            payload_format,
            payload,
        }
    }

    pub fn payload_format(&self) -> UPayloadFormat {
        self.payload_format
    }
    /// Gets the payload data.
    ///
    /// Note that this consumes the payload.
    pub fn payload(self) -> Bytes {
        self.payload
    }
}
