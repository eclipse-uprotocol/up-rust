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
use std::time::{Duration, SystemTime};

use crate::{communication::SubscriptionStatus, UCode, UStatus, UUri};

mod usubscription_client;
pub use usubscription_client::RpcClientUSubscription;
mod usubscription_server;
pub use usubscription_server::{
    extract_usubscription_request, pack_usubscription_response, USubscriptionRequest,
    USubscriptionResponse,
};
mod usubscription_proto;

// [impl->req~usubscription-uentity_id~1]
pub const USUBSCRIPTION_TYPE_ID: u16 = 0x0000_0000;
/// The (latest) major version of the uSubscription service.
pub const USUBSCRIPTION_VERSION_MAJOR: u8 = 0x04;
// [impl->req~usubscription-subscribe-method_id~1]
pub const RESOURCE_ID_SUBSCRIBE: u16 = 0x0001;
// [impl->req~usubscription-unsubscribe-method_id~1]
pub const RESOURCE_ID_UNSUBSCRIBE: u16 = 0x0002;
// [impl->req~usubscription-fetch-subscriptions-method_id~1]
pub const RESOURCE_ID_FETCH_SUBSCRIPTIONS: u16 = 0x0003;
// [impl->req~usubscription-register-notifications-method_id~1]
pub const RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS: u16 = 0x0004;
// [impl->req~usubscription-unregister-notifications-method_id~1]
pub const RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS: u16 = 0x0005;
// [impl->req~usubscription-reset-method_id~1]
pub const RESOURCE_ID_RESET: u16 = 0x0006;

// [impl->req~usubscription-change-notification-resource~1]
pub const RESOURCE_ID_SUBSCRIPTION_CHANGE: u16 = 0x8000;

/// Information about a client-topic subscription.
///
/// This struct represents the subscription metadata maintained by the
/// uSubscription service, including the topic, subscriber, current
/// [`SubscriptionStatus`], and optional delivery constraints.
#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub struct SubscriptionInfo {
    topic: UUri,
    subscriber: UUri,
    status: SubscriptionStatus,
    expiration: Option<SystemTime>,
    min_sample_period: Option<Duration>,
}

impl SubscriptionInfo {
    /// Creates a new subscription info object.
    ///
    /// # Arguments
    /// * `topic` - The topic of the subscription.
    /// * `subscriber` - The uEntity that has established the subscription.
    /// * `status` - The status of the subscription.
    /// * `expiration` - The point in time at which the subscription expires.
    ///   If not specified, the subscription is valid until explicitly unsubscribed.
    /// * `min_sample_period` - The minimum duration between two events that should be maintained
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
        expiration: Option<SystemTime>,
        min_sample_period: Option<Duration>,
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
    pub fn expiration(&self) -> &Option<SystemTime> {
        &self.expiration
    }

    #[must_use]
    pub fn min_sample_period(&self) -> &Option<Duration> {
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

/// A request to subscribe to a topic (client/subscriber UUri from the message UAttributes envelope).
#[derive(Clone, Debug, PartialEq)]
pub struct SubscribeRequest {
    /// The topic to subscribe to.
    pub topic: UUri,
    /// The point in time at which the subscription expires.
    pub expiration: Option<SystemTime>,
    /// The minimum duration between two events (before they should be forwarded by a UStreamer).
    pub sample_period: Option<Duration>,
}

impl SubscribeRequest {
    pub fn new(
        topic: &UUri,
        expiration: Option<SystemTime>,
        sample_period: Option<Duration>,
    ) -> Result<Self, UStatus> {
        Self::validate_topic(topic)?;
        Self::validate_expiration(&expiration)?;
        Self::validate_sample_period(&sample_period)?;

        Ok(SubscribeRequest {
            topic: topic.to_owned(),
            expiration,
            sample_period,
        })
    }

    fn validate_topic(topic: &UUri) -> Result<(), UStatus> {
        topic.verify_event().map_err(|e| {
            UStatus::fail_with_code(
                UCode::InvalidArgument,
                format!("topic URI is not a valid event source: {e}"),
            )
        })?;

        Ok(())
    }

    fn validate_expiration(expiration: &Option<SystemTime>) -> Result<(), UStatus> {
        if let Some(expiration) = expiration {
            if expiration <= &SystemTime::now() {
                return Err(UStatus::fail_with_code(
                    UCode::InvalidArgument,
                    "expiration must be a timestamp in the future",
                ));
            }
        }
        Ok(())
    }

    fn validate_sample_period(sample_period: &Option<Duration>) -> Result<(), UStatus> {
        if let Some(sample_period) = sample_period {
            if sample_period.is_zero() {
                return Err(UStatus::fail_with_code(
                    UCode::InvalidArgument,
                    "sample_period must be greater than zero",
                ));
            }
        }
        Ok(())
    }
    /// Verifies that the given subscribe request's options comply with the USubscription specification.
    ///
    /// # Arguments
    ///
    /// * `subscribe_request` - The subscribe request to verify.
    ///
    /// # Errors
    ///
    /// Returns an error if the request's topic is not a valid event source, if `expiration` is set
    /// to a timestamp that is not in the future, or if `sample_period` is set to a duration of zero.
    pub fn validate(&self) -> Result<(), UStatus> {
        Self::validate_topic(&self.topic)?;
        Self::validate_expiration(&self.expiration)?;
        Self::validate_sample_period(&self.sample_period)?;
        Ok(())
    }
}

/// The response to a [`SubscribeRequest`].
#[derive(Clone, Debug, PartialEq)]
pub struct SubscribeResponse {
    /// The topic the subscription refers to.
    pub topic: UUri,
    /// The resulting status of the subscription.
    pub status: SubscriptionStatus,
}

/// A request to unsubscribe from a topic (client/unsubscriber UUri from the message UAttributes envelope).
#[derive(Clone, Debug, PartialEq)]
pub struct UnsubscribeRequest {
    /// The topic to unsubscribe from.
    pub topic: UUri,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnsubscribeResponse {}

/// A request to fetch subscription information.
#[derive(Clone, Debug, PartialEq)]
pub struct FetchSubscriptionsRequest {
    /// The topic filter to fetch subscription information for.
    pub topic_filter: Option<UUri>,
    /// The subscriber filter to fetch subscription information for.
    pub subscriber_filter: Option<UUri>,
}

/// The response to a [`FetchSubscriptionsRequest`].
#[derive(Clone, Debug, PartialEq)]
pub struct FetchSubscriptionsResponse {
    /// The topic the subscription refers to.
    pub subscriptions: Vec<SubscriptionInfo>,
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
        USUBSCRIPTION_TYPE_ID as u32,
        USUBSCRIPTION_VERSION_MAJOR,
        resource_id,
    )
    .unwrap()
}

/// The uProtocol Application Layer client interface to the uSubscription service.
///
/// Please refer to the [uSubscription service specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l3/usubscription/v4/README.adoc)
/// for details.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait USubscription: Send + Sync {
    /// Subscribes to a topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to subscribe to.
    /// * `expiration` - The point in time at which the subscription expires.
    ///   If not specified, the subscription is valid until explicitly unsubscribed.
    ///   If expiration time is set in the past, no subscription is recorded and SubscriptionStatus::Unsubscribed
    ///   is returned.
    ///   When called for a topic that the client is already subscribed to and where the expiration field value differs
    ///   from the current expiration time, the expiration field of the subscription is updated with the new value.
    ///   When called for a topic that the client is already subscribed to where the expiration field value differs
    ///   from the current expiration time and lies in the past, the subscription is unregistered and
    ///   SubscriptionStatus::Unsubscribed is returned.
    /// * `min_sample_period` - The minimum duration between two events that should be maintained
    ///   for remote only topics. Device dispatchers (i.e. streamers) use this attribute to reduce the
    ///   publication rates of events sent between devices.
    ///   This attribute is commonly used for mobile/cloud components subscribing to vehicle topics that are published
    ///   at a high rate. If the desired sampling period set by the subscriber is lower than the original publisher's
    ///   publication period, the attribute is ignored.
    ///   If not specified, the sampling period is set by the publisher.
    ///   Durations used in `min_sample_period` will be clamped to [0; u32::MAX] milliseconds.
    ///
    /// # Returns
    ///
    /// The outcome of the attempt to establish the subscription.
    async fn subscribe(
        &self,
        topic: &UUri,
        expiration: Option<SystemTime>,
        min_sample_period: Option<Duration>,
    ) -> Result<SubscriptionStatus, UStatus>;

    /// Unsubscribes this client from a topic.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic to unsubscribe from.
    ///
    /// # Errors
    ///
    /// Returns an error if the attempt to unsubscribe has failed.
    async fn unsubscribe(&self, topic: &UUri) -> Result<(), UStatus>;

    /// Gets details about subscriptions that are currently tracked by uSubscription service. Clients
    /// can provide UURIs to filter subscriptions by topic and/or subscriber. Filter UURIs may contain wildcards.
    ///
    /// Topic and subscription filters can be provided individually or in combination; uSubscription service will
    /// only return subscription information that match all of the provided filter criteria.
    ///
    /// # Parameters
    ///
    /// * `topic_filter` - Only return subscriptions where the topic is matched by this filter.
    /// * `subscriber_filter` - Only return subscriptions where the subscriber is matched by this filter.
    ///
    /// # Errors
    ///
    /// Returns an error if the attempt to retrieve the subscriptions has failed.
    async fn fetch_subscriptions(
        &self,
        topic_filter: Option<UUri>,
        subscriber_filter: Option<UUri>,
    ) -> Result<Vec<SubscriptionInfo>, UStatus>;

    /// Registers this client for notifications about changes of subscriptions managed by uSubscription service.
    ///
    /// # Errors
    ///
    /// Returns an error if the attempt to register for notifications has failed.
    async fn register_for_notifications(&self) -> Result<(), UStatus>;

    /// Unregisters this client from notifications about changes of subscriptions managed by uSubscription service.
    ///
    /// # Errors
    ///
    /// Returns an error if the attempt to unregister from notifications has failed.
    async fn unregister_for_notifications(&self) -> Result<(), UStatus>;

    /// Flushes all stored subscription information, including any persistently stored subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error if the attempt to reset has failed.
    async fn reset(&self) -> Result<(), UStatus>;
}
