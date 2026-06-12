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

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use crate::{communication::SubscriptionStatus, UStatus, UUri};

#[cfg(all(feature = "up-l2-rpc-client", feature = "up-core-types"))]
mod usubscription_client;
#[cfg(all(feature = "up-l2-rpc-client", feature = "up-core-types"))]
pub use usubscription_client::RpcClientUSubscription;

/// The uEntity (type) identifier of the uSubscription service.
pub const USUBSCRIPTION_TYPE_ID: u32 = 0x0000_0000;
/// The (latest) major version of the uSubscription service.
pub const USUBSCRIPTION_VERSION_MAJOR: u8 = 0x03;
/// The resource identifier of uSubscription's _subscribe_ operation.
pub const RESOURCE_ID_SUBSCRIBE: u16 = 0x0001;
/// The resource identifier of uSubscription's _unsubscribe_ operation.
pub const RESOURCE_ID_UNSUBSCRIBE: u16 = 0x0002;
/// The resource identifier of uSubscription's _fetch subscriptions_ operation.
pub const RESOURCE_ID_FETCH_SUBSCRIPTIONS: u16 = 0x0003;
/// The resource identifier of uSubscription's _register for notifications_ operation.
pub const RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS: u16 = 0x0006;
/// The resource identifier of uSubscription's _unregister for notifications_ operation.
pub const RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS: u16 = 0x0007;
/// The resource identifier of uSubscription's _fetch subscribers_ operation.
pub const RESOURCE_ID_FETCH_SUBSCRIBERS: u16 = 0x0008;
/// The resource identifier of uSubscription's _reset_ operation.
pub const RESOURCE_ID_RESET: u16 = 0x0009;

/// The resource identifier of uSubscription's _subscription change_ topic.
pub const RESOURCE_ID_SUBSCRIPTION_CHANGE: u16 = 0x8000;

#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub struct SubscriptionInfo {
    topic: UUri,
    subscriber: UUri,
    status: SubscriptionStatus,
    expiration: Option<u64>,
    min_sample_period: Option<u32>,
}

impl SubscriptionInfo {
    /// Creates a new info object.
    ///
    /// # Arguments
    /// * `topic` - The topic of the subscription.
    /// * `subscriber` - The uEntity that has established the subscription.
    /// * `status` - The status of the subscription.
    /// * `expiration` - The point in time at which the subscription expires (milliseconds since Unix epoch).
    ///   If not specified, the subscription is valid until explicitly unsubscribed.
    /// * `min_sample_period` - The minimum duration (in seconds) between two events that should be maintained
    ///   for remote only topics. Device dispatchers (i.e. streamers) use this attribute to reduce the publication
    ///   rates of events sent between devices.
    ///   This attribute is commonly used for mobile/cloud components subscribing to vehicle topics that are published
    ///   at a high rate. If the desired sampling period set by the subscriber is lower than the original
    ///   publisher's publication period, the attribute is ignored. If not specified, the sampling period is set
    ///   by the publisher.
    #[must_use]
    pub fn new(
        topic: UUri,
        subscriber: UUri,
        status: SubscriptionStatus,
        expiration: Option<u64>,
        min_sample_period: Option<u32>,
    ) -> Self {
        Self {
            topic,
            subscriber,
            status,
            expiration,
            min_sample_period,
        }
    }

    #[must_use]
    pub fn topic(&self) -> &UUri {
        &self.topic
    }

    #[must_use]
    pub fn subscriber(&self) -> &UUri {
        &self.subscriber
    }

    #[must_use]
    pub fn status(&self) -> &SubscriptionStatus {
        &self.status
    }

    #[must_use]
    pub fn expiration(&self) -> &Option<u64> {
        &self.expiration
    }

    #[must_use]
    pub fn min_sample_period(&self) -> &Option<u32> {
        &self.min_sample_period
    }

    /// Checks for a specific subscription status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{communication::SubscriptionStatus, UUri};
    /// use up_rust::core::usubscription::SubscriptionInfo;
    ///
    /// let subscription_info = SubscriptionInfo::new(
    ///     UUri::try_from("/A100/1/9000").unwrap(),
    ///     UUri::try_from("//subscriber/ABCD/1/0").unwrap(),
    ///     SubscriptionStatus::Subscribed,
    ///     None,
    ///     None,
    /// );
    /// assert!(subscription_info.has_status(SubscriptionStatus::Subscribed));
    /// assert!(!subscription_info.has_status(SubscriptionStatus::Unsubscribed));
    /// ```
    #[must_use]
    pub fn has_status(&self, state: SubscriptionStatus) -> bool {
        self.status == state
    }
}

/// Potential reasons for resetting the uSubscription service.
#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum ResetReason {
    Unspecified,
    FactoryReset,
    CorruptedData,
}

/// Gets a UUri referring to one of the local uSubscription service's resources.
///
/// # Examples
///
/// ```rust
/// use up_rust::core::usubscription;
///
/// let uuri = usubscription::usubscription_uri(usubscription::RESOURCE_ID_SUBSCRIBE);
/// assert_eq!(uuri.resource_id(), 0x0001);
/// ```
#[must_use]
pub fn usubscription_uri(resource_id: u16) -> UUri {
    UUri::try_from_parts(
        "",
        USUBSCRIPTION_TYPE_ID,
        USUBSCRIPTION_VERSION_MAJOR,
        resource_id,
    )
    .unwrap()
}

/// The uProtocol Application Layer client interface to the uSubscription service.
///
/// Please refer to the [uSubscription service specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l3/usubscription/v3/README.adoc)
/// for details.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait USubscription: Send + Sync {
    /// Subscribes to a topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to subscribe to.
    /// * `expiration` - The point in time at which the subscription expires (seconds since Unix epoch).
    ///   If not specified, the subscription is valid until explicitly unsubscribed.
    /// * `min_sample_period` - The minimum time between two events that should be maintained for remote
    ///   only topics. Device dispatchers (i.e. streamers) use this attribute to reduce the publication rates of
    ///   events sent between devices.
    ///   This attribute is commonly used for mobile/cloud components subscribing to vehicle topics that are published
    ///   at a high rate. If the desired sampling period set by the subscriber is lower than the original publisher's
    ///   publication period, the attribute is ignored.
    ///   If not specified, the sampling period is set by the publisher.
    ///
    /// # Returns
    ///
    /// * The outcome of the attempt to establish the subscription.
    async fn subscribe(
        &self,
        topic: &UUri,
        expiration: Option<u64>,
        min_sample_period: Option<u32>,
    ) -> Result<SubscriptionStatus, UStatus>;

    /// Unsubscribes from a topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to unsubscribe from.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt has failed.
    async fn unsubscribe(&self, topic: &UUri) -> Result<(), UStatus>;

    /// Gets all (currently) active subscriptions for a given topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to fetch subscriptions for.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt to retrieve the subscriptions has failed.
    async fn fetch_subscriptions_by_topic(
        &self,
        topic: &UUri,
    ) -> Result<Vec<SubscriptionInfo>, UStatus>;

    /// Gets a uEntity's (currently) active subscriptions.
    ///
    /// # Parameters
    ///
    /// * `subscriber` - The uEntity to get the subscriptions for.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt to retrieve the subscriptions has failed.
    async fn fetch_subscriptions_by_subscriber(
        &self,
        subscriber: &UUri,
    ) -> Result<Vec<SubscriptionInfo>, UStatus>;

    /// Registers for notifications about changes subscription status for a given topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to receive changes to subscription status for.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt to register for notifications has failed.
    async fn register_for_notifications(&self, topic: &UUri) -> Result<(), UStatus>;

    /// Unregisters from notifications about changes subscription status for a given topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to no longer receive changes to subscription status for.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt to unregister from notifications has failed.
    async fn unregister_for_notifications(&self, topic: &UUri) -> Result<(), UStatus>;

    /// Fetches a list of subscribers that are currently subscribed to a given topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic.
    ///
    /// # Returns
    ///
    /// a list of URIs representing the uEntities that are subscribed to the given topic.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt to fetch subscribers has failed.
    async fn fetch_subscribers(&self, topic: &UUri) -> Result<Vec<UUri>, UStatus>;

    /// Flushes all stored subscription information, including any persistently stored subscriptions.
    ///
    /// # Parameters
    ///
    /// * `reason` - The reason for the reset.
    /// * `message` - An optional human-readable message providing additional context about the reset.
    /// * `before` - An optional timestamp (milliseconds since Unix epoch). All subscriptions created before
    ///   this timestamp will be removed.
    ///
    /// # Errors
    ///
    /// returns an error if the attempt to reset has failed.
    async fn reset(
        &self,
        reason: ResetReason,
        message: Option<String>,
        before: Option<u64>,
    ) -> Result<(), UStatus>;
}
