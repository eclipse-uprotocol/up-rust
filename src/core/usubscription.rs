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

use crate::{
    up_core_api::usubscription::{
        reset_request::reason::Code, subscription_status::State,
        SubscriptionStatus as SubscriptionStatusProto,
    },
    UCode, UStatus, UUri,
};

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
pub enum SubscriptionStatus {
    Unsubscribed = State::UNSUBSCRIBED as isize,
    SubscribePending = State::SUBSCRIBE_PENDING as isize,
    Subscribed = State::SUBSCRIBED as isize,
    UnsubscribePending = State::UNSUBSCRIBE_PENDING as isize,
}

impl From<&SubscriptionStatusProto> for SubscriptionStatus {
    fn from(status: &SubscriptionStatusProto) -> Self {
        let state = status.state.enum_value_or_default();
        match state {
            State::UNSUBSCRIBED => SubscriptionStatus::Unsubscribed,
            State::SUBSCRIBE_PENDING => SubscriptionStatus::SubscribePending,
            State::SUBSCRIBED => SubscriptionStatus::Subscribed,
            State::UNSUBSCRIBE_PENDING => SubscriptionStatus::UnsubscribePending,
        }
    }
}

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
    /// use up_rust::UUri;
    /// use up_rust::core::usubscription::{SubscriptionInfo, SubscriptionStatus};
    ///
    /// let subscription_info = SubscriptionInfo::new(
    ///     UUri::try_from("/A100/1/9000").unwrap(),
    ///     UUri::try_from("//subscriber/ABCD/1/0").unwrap(),
    ///     SubscriptionStatus::Subscribed,
    ///     None,
    ///     None,
    /// );
    /// assert!(subscription_info.has_status(SubscriptionStatus::Subscribed));
    /// ```
    #[must_use]
    pub fn has_status(&self, state: SubscriptionStatus) -> bool {
        self.status == state
    }
}

impl TryFrom<&crate::up_core_api::usubscription::Subscription> for SubscriptionInfo {
    type Error = UStatus;
    fn try_from(
        subscription: &crate::up_core_api::usubscription::Subscription,
    ) -> Result<Self, Self::Error> {
        let topic = subscription
            .topic
            .as_ref()
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "topic missing",
            ))
            .and_then(|t| {
                UUri::try_from(t)
                    .map_err(|_| UStatus::fail_with_code(UCode::InvalidArgument, "invalid topic"))
            })?;
        let subscriber = subscription
            .subscriber
            .as_ref()
            .and_then(|s| s.uri.as_ref())
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "subscriber missing",
            ))
            .and_then(|s| {
                UUri::try_from(s).map_err(|_| {
                    UStatus::fail_with_code(UCode::InvalidArgument, "invalid subscriber")
                })
            })?;
        let status = subscription
            .status
            .as_ref()
            .map(SubscriptionStatus::from)
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "status missing",
            ))?;
        subscription
            .attributes
            .as_ref()
            .ok_or_else(|| UStatus::fail_with_code(UCode::InvalidArgument, "missing attributes"))
            .map(|attributes| {
                let expiration = attributes.expire.as_ref().map(|ts| ts.seconds as u64);
                SubscriptionInfo::new(
                    topic,
                    subscriber,
                    status,
                    expiration,
                    attributes.sample_period_ms,
                )
            })
    }
}

impl TryFrom<&crate::up_core_api::usubscription::Update> for SubscriptionInfo {
    type Error = UStatus;
    fn try_from(update: &crate::up_core_api::usubscription::Update) -> Result<Self, Self::Error> {
        let topic = update
            .topic
            .as_ref()
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "topic missing",
            ))
            .and_then(|t| {
                UUri::try_from(t)
                    .map_err(|_| UStatus::fail_with_code(UCode::InvalidArgument, "invalid topic"))
            })?;
        let subscriber = update
            .subscriber
            .as_ref()
            .and_then(|s| s.uri.as_ref())
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "subscriber missing",
            ))
            .and_then(|s| {
                UUri::try_from(s).map_err(|_| {
                    UStatus::fail_with_code(UCode::InvalidArgument, "invalid subscriber")
                })
            })?;
        let status =
            update
                .status
                .as_ref()
                .map(SubscriptionStatus::from)
                .ok_or(UStatus::fail_with_code(
                    UCode::InvalidArgument,
                    "status missing",
                ))?;
        let attribs = update.attributes.get_or_default();
        let expiration = attribs.expire.as_ref().map(|ts| ts.seconds as u64);
        Ok(SubscriptionInfo::new(
            topic,
            subscriber,
            status,
            expiration,
            attribs.sample_period_ms,
        ))
    }
}

#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum ResetReason {
    Unspecified = Code::UNSPECIFIED as isize,
    FactoryReset = Code::FACTORY_RESET as isize,
    CorruptedData = Code::CORRUPTED_DATA as isize,
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
    /// * `before` - An optional timestamp (seconds since Unix epoch). All subscriptions created before
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
