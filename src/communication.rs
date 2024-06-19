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

use std::{error::Error, fmt::Display};

pub use notification::{NotificationError, NotificationListener, Notifier};
pub use pubsub::{PubSubError, Publisher, Subscriber};
pub use rpc::{RpcClient, RpcServer, ServiceInvocationError};

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
