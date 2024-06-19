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

use std::{error::Error, fmt::Display, sync::Arc};

use async_trait::async_trait;

use crate::communication::RegistrationError;
use crate::{UListener, UMessage, UStatus, UUri};

/// An error indicating a problem with publishing a message to a topic.
#[derive(Debug)]
pub enum PubSubError {
    /// Indicates that the given message cannot be sent because it is not a [valid Publish message](crate::PublishValidator).
    InvalidArgument(String),
    /// Indicates an unspecific error that occurred at the Transport Layer while trying to publish a message.
    PublishError(UStatus),
}

impl Display for PubSubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PubSubError::InvalidArgument(s) => f.write_fmt(format_args!("invalid argument: {}", s)),
            PubSubError::PublishError(s) => {
                f.write_fmt(format_args!("failed to publish message: {}", s))
            }
        }
    }
}

impl Error for PubSubError {}

/// A client for publishing messages to a topic.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Publishes a message to a topic.
    ///
    /// # Errors
    ///
    /// Returns an error if the given message is not a valid
    /// [uProtocol Publish message](`crate::PublishValidator`).
    async fn publish(&self, message: UMessage) -> Result<(), PubSubError>;
}

/// A client for subscribing to topics.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Registers a handler to invoke for messages that have been published to topics matching a given pattern.
    ///
    /// More than one handler can be registered for the same pattern.
    /// The same handler can be registered for multiple patterns.
    ///
    /// # Arguments
    ///
    /// * `topic_filter` - The pattern defining the topics of interest.
    /// * `listener` - The handler to invoke for each message that has been published to a topic
    ///                [matching the given pattern](`crate::UUri::matches`).
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be registered.
    async fn subscribe(
        &self,
        topic_filter: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError>;

    /// Unregisters a previously [registered handler](`Self::subscribe`).
    ///
    /// # Arguments
    ///
    /// * `topic_filter` - The UUri pattern that the handler had been registered for.
    /// * `listener` - The handler to unregister.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be unregistered.
    async fn unsubscribe(
        &self,
        topic_filter: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError>;
}
