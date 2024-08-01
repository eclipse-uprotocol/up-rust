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
#[cfg(test)]
use mockall::automock;

use crate::communication::RegistrationError;
use crate::core::usubscription::SubscriptionStatus;
use crate::{UListener, UStatus, UUri};

use super::{CallOptions, UPayload};

/// An error indicating a problem with publishing a message to a topic.
// [impl->req~up-language-comm-api~1]
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
            PubSubError::InvalidArgument(s) => f.write_str(s.as_str()),
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
// [impl->req~up-language-comm-api~1]
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Publishes a message to a topic.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - The (local) resource ID of the topic to publish to.
    /// * `call_options` - Options to include in the published message.
    /// * `payload` - Payload to include in the published message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message could not be published.
    async fn publish(
        &self,
        resource_id: u16,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<(), PubSubError>;
}

// [impl->req~up-language-comm-api~1]
#[cfg_attr(test, automock)]
pub trait SubscriptionChangeHandler: Send + Sync {
    /// Invoked for each update to the subscription status for a given topic.
    ///
    /// Implementations must not block the current thread.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic for which the subscription status has changed.
    /// * `status` - The new status of the subscription.
    fn on_subscription_change(&self, topic: UUri, new_status: SubscriptionStatus);
}

/// A client for subscribing to topics.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
// [impl->req~up-language-comm-api~1]
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Registers a handler to invoke for messages that have been published to a given topic.
    ///
    /// More than one handler can be registered for the same topic.
    /// The same handler can be registered for multiple topics.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to subscribe to. The topic must not contain any wildcards.
    /// * `handler` - The handler to invoke for each message that has been published to the topic.
    /// * `subscription_change_handler` - A handler to invoke for any subscription state changes for
    ///                                   the given topic.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be registered.
    async fn subscribe(
        &self,
        topic: &UUri,
        handler: Arc<dyn UListener>,
        subscription_change_handler: Option<Arc<dyn SubscriptionChangeHandler>>,
    ) -> Result<(), RegistrationError>;

    /// Unregisters a previously [registered handler](`Self::subscribe`).
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic that the handler had been registered for.
    /// * `handler` - The handler to unregister.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be unregistered.
    async fn unsubscribe(
        &self,
        topic: &UUri,
        handler: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError>;
}
