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
use crate::{UListener, UStatus, UUri};

use super::{CallOptions, UPayload};

/// An error indicating a problem with sending a notification to another uEntity.
// [impl->req~up-language-comm-api~1]
#[derive(Debug)]
pub enum NotificationError {
    /// Indicates that the given message cannot be sent because it is not a [valid Notification message](crate::NotificationValidator).
    InvalidArgument(String),
    /// Indicates an unspecific error that occurred at the Transport Layer while trying to send a notification.
    NotifyError(UStatus),
}

#[cfg(not(tarpaulin_include))]
impl Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationError::InvalidArgument(s) => f.write_str(s.as_str()),
            NotificationError::NotifyError(s) => {
                f.write_fmt(format_args!("failed to send notification: {}", s))
            }
        }
    }
}

impl Error for NotificationError {}

/// A client for sending Notification messages to a uEntity.
///
/// Please refer to the
/// [Communication Layer API Specifications](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l2/api.adoc).
// [impl->req~up-language-comm-api~1]
#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Sends a notification to a uEntity.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - The (local) resource identifier representing the origin of the notification.
    /// * `destination` - A URI representing the uEntity that the notification should be sent to.
    /// * `call_options` - Options to include in the notification message.
    /// * `payload` - The payload to include in the notification message.
    ///
    /// # Errors
    ///
    /// Returns an error if the given message is not a valid
    /// [uProtocol Notification message](`crate::NotificationValidator`).
    async fn notify(
        &self,
        resource_id: u16,
        destination: &UUri,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<(), NotificationError>;

    /// Starts listening to a notification topic.
    ///
    /// More than one handler can be registered for the same topic.
    /// The same handler can be registered for multiple topics.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to listen to. The topic must not contain any wildcards.
    /// * `listener` - The handler to invoke for each notification that has been sent on the topic.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be registered.
    async fn start_listening(
        &self,
        topic: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError>;

    /// Deregisters a previously [registered handler](`Self::start_listening`) for listening to notifications.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic that the handler had been registered for.
    /// * `listener` - The handler to unregister.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be unregistered.
    async fn stop_listening(
        &self,
        topic: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError>;
}
