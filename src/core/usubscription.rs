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

pub use crate::up_core_api::usubscription::{
    subscription_status::State, EventDeliveryConfig, FetchSubscribersRequest,
    FetchSubscriptionsRequest, FetchSubscriptionsResponse, NotificationsRequest,
    SubscribeAttributes, SubscriberInfo, SubscriptionRequest, SubscriptionResponse,
    SubscriptionStatus, UnsubscribeRequest, Update,
};

use crate::UStatus;
use async_trait::async_trait;

/// `USubscription` is the uP-L3 client interface to the uSubscription service.
///
/// A client would use a concrete implementation of `USubscription` typically to subscribe to
/// a topic of interest and then unsubscribe when finished.
///
/// Implementations of `USubscription` can be transport-specific to allow for flexibility and optimizations.
///
/// # Examples
///
/// ## Typical usage
///
/// ```
/// # use async_trait::async_trait;
/// # use std::future::Future;
/// # use up_rust::{UMessage, UStatus, UUri, UListener};
/// #
/// # mod up_client_foo {
/// #     use std::sync::Arc;
/// #     use up_rust::{UTransport, UListener, UStatus, UMessage, UUri};
/// #     use async_trait::async_trait;
/// #     pub struct UPClientFoo;
/// #
/// #     #[async_trait]
/// #     impl UTransport for UPClientFoo {
/// #         async fn send(&self, message: UMessage) -> Result<(), UStatus> {
/// #             todo!()
/// #         }
/// #
/// #         async fn receive(&self, source_filter: &UUri, sink_filter: Option<&UUri>) -> Result<UMessage, UStatus> {
/// #             todo!()
/// #         }
/// #
/// #         async fn register_listener(&self, source_filter: &UUri, sink_filter: Option<&UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus> {
/// #             Ok(())
/// #         }
/// #
/// #         async fn unregister_listener(&self, source_filter: &UUri, sink_filter: Option<&UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus> {
/// #             Ok(())
/// #         }
/// #     }
/// #
/// #     impl UPClientFoo {
/// #         pub fn new() -> Self {
/// #             Self
/// #         }
/// #     }
/// # }
/// #
/// # mod usubscription_foo {
/// #     use async_trait::async_trait;
/// #     use protobuf::EnumOrUnknown;
/// #     use up_rust::{UStatus, UCode,
/// #         core::usubscription::{USubscription, FetchSubscribersRequest, FetchSubscriptionsRequest,
/// #                               FetchSubscriptionsResponse, NotificationsRequest, SubscriptionRequest,
/// #                               SubscriptionResponse, UnsubscribeRequest, SubscriptionStatus, State,
/// #                               EventDeliveryConfig},
/// #     };
/// #
/// #     pub struct USubscriptionFoo;
/// #
/// #     #[async_trait]
/// #     impl USubscription for USubscriptionFoo {
/// #         async fn subscribe(&self, subscription_request: SubscriptionRequest) -> Result<SubscriptionResponse, UStatus> {
/// #             let subscription_status = SubscriptionStatus {
/// #                 state: EnumOrUnknown::from(State::SUBSCRIBED),
/// #                 code: EnumOrUnknown::from(UCode::OK),
/// #                 message: "Subscription success".to_string(),
/// #                 ..Default::default()
/// #             };
/// #
/// #             let event_delivery_config = EventDeliveryConfig {
/// #                 id: "SUBSCRIPTION_TOPIC".to_string(),
/// #                 type_: "Foo/Vehicle/EventHubs".to_string(),
/// #                 attributes: Default::default(),
/// #                 ..Default::default()
/// #             };
/// #
/// #             let subscription_response = SubscriptionResponse {
/// #                 status: Some(subscription_status).into(),
/// #                 config: Default::default(),
/// #                 topic: Default::default(),
/// #                 ..Default::default()
/// #             };
/// #
/// #             Ok(subscription_response)
/// #         }
/// #
/// #         async fn unsubscribe(&self, unsubscribe_request: UnsubscribeRequest) -> Result<(), UStatus> {
/// #             Ok(())
/// #         }
/// #
/// #         async fn fetch_subscriptions(&self, fetch_subscriptions_request: FetchSubscriptionsRequest) -> Result<FetchSubscriptionsResponse, UStatus> {
/// #             todo!()
/// #         }
/// #
/// #         async fn register_for_notifications(&self, notifications_request: NotificationsRequest) -> Result<(), UStatus> {
/// #             todo!()
/// #         }
/// #
/// #         async fn unregister_for_notifications(&self, notifications_request: NotificationsRequest) -> Result<(), UStatus> {
/// #             todo!()
/// #         }
/// #
/// #         async fn fetch_subscribers(&self, fetch_subscribers_request: FetchSubscribersRequest) -> Result<FetchSubscriptionsResponse, UStatus> {
/// #             todo!()
/// #         }
/// #     }
/// #
/// #     impl USubscriptionFoo {
/// #         pub fn new() -> Self {
/// #             Self
/// #         }
/// #     }
/// # }
/// #
/// # #[derive(Clone)]
/// # pub struct MyListener;
/// #
/// # #[async_trait]
/// # impl UListener for MyListener {
/// #     async fn on_receive(&self, msg: UMessage) {
/// #         todo!()
/// #     }
/// #
/// #     async fn on_error(&self, err: UStatus) {
/// #         todo!()
/// #     }
/// #
/// # }
/// #
/// # impl MyListener {
/// #     pub fn new() -> Self {
/// #         Self
/// #     }
/// # }
/// #
/// # #[async_std::main]
/// # pub async fn main() -> Result<(), UStatus> {
/// #
/// # let my_uuri = Default::default();
/// #
/// # let door_uuri = UUri {
/// #     authority_name: "device_a".to_string(),
/// #     ue_id: 0x0000_0001,
/// #     ue_version_major: 0x01,
/// #     resource_id: 0x0002,
/// #     ..Default::default()
/// # };
/// #
/// # let my_listener = Arc::new(MyListener::new());
/// #
/// # let up_client = up_client_foo::UPClientFoo::new();
/// #
/// use std::sync::Arc;
/// use up_rust::{UCode, UTransport,
///     core::usubscription::{USubscription, UnsubscribeRequest, SubscribeAttributes,
///                           SubscriberInfo, SubscriptionRequest, SubscriptionResponse},
/// };
///
/// let usub = usubscription_foo::USubscriptionFoo::new();
///
/// let subscriber_info = SubscriberInfo {
///     uri: my_uuri,
///     ..Default::default()
/// };
///
/// let subscribe_attributes = SubscribeAttributes {
///     sample_period_ms: Some(5000), // we want to hear about this every 5 seconds
///     ..Default::default()
/// };
///
/// let subscription_request = SubscriptionRequest {
///     topic: Some(door_uuri.clone()).into(),
///     subscriber: Some(subscriber_info.clone()).into(),
///     attributes: Some(subscribe_attributes).into(),
///     ..Default::default()
/// };
///
/// let subscription_response = usub.subscribe(subscription_request).await?;
/// let success_code = subscription_response.status.code.enum_value_or(UCode::UNKNOWN);
/// if success_code == UCode::OK {
///     let register_success = up_client.register_listener(&door_uuri, None, my_listener.clone()).await;
/// } else {
///     match success_code {
///         UCode::NOT_FOUND => { /* handle topic not found */ }
///         _ => { /* handle other error conditions */ }
///     }
/// }
/// // sometime later when done with this topic
/// let unsubscribe_request = UnsubscribeRequest {
///     topic: Some(door_uuri.clone()).into(),
///     subscriber: Some(subscriber_info.clone()).into(),
///     ..Default::default()
/// };
/// let unsubscribe_result = usub.unsubscribe(unsubscribe_request).await?;
/// let unregister_success = up_client.register_listener(&door_uuri, None, my_listener.clone()).await;
/// #
/// # Ok(())
/// # }
/// ```
///
/// For more information, please refer to the [uProtocol Specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l3/usubscription/v3/README.adoc)
/// and [uProtocol APIs](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-core-api/uprotocol/core/usubscription/v3/usubscription.proto)
#[async_trait]
pub trait USubscription: Send + Sync {
    /// Subscribe to a topic, using a [`SubscriptionRequest`]
    ///
    /// # Parameters
    ///
    /// * `subscription_request` - A request to subscribe
    ///
    /// # Returns
    ///
    /// * [`SubscriptionResponse`] detailing if subscription was successful with other metadata
    async fn subscribe(
        &self,
        subscription_request: SubscriptionRequest,
    ) -> Result<SubscriptionResponse, UStatus>;

    /// Unsubscribe to a topic, using an [`UnsubscribeRequest`]
    ///
    /// # Parameters
    ///
    /// * `unsubscribe_request` - A request to unsubscribe
    ///
    /// # Returns
    ///
    /// * [`UStatus`] detailing if unsubscription was successful and if not why not
    async fn unsubscribe(&self, unsubscribe_request: UnsubscribeRequest) -> Result<(), UStatus>;

    /// Fetch all subscriptions for a given topic or subscriber contained inside a [`FetchSubscriptionsRequest`]
    ///
    /// # Parameters
    ///
    /// * `fetch_subscriptions_request` - A request to fetch subscriptions given a topic or subscriber
    ///
    /// # Returns
    ///
    /// * [`FetchSubscriptionsResponse`] detailing the zero or more subscriptions' info
    async fn fetch_subscriptions(
        &self,
        fetch_subscriptions_request: FetchSubscriptionsRequest,
    ) -> Result<FetchSubscriptionsResponse, UStatus>;

    /// Register for notifications relevant to a given topic inside a [`NotificationsRequest`]
    /// changing in subscription status.
    ///
    /// # Parameters
    ///
    /// * `notifications_register_request` - A request to receive changes to subscription status
    ///
    /// # Returns
    ///
    /// * [`UStatus`] detailing if notification registration was successful and if not why not
    async fn register_for_notifications(
        &self,
        notifications_register_request: NotificationsRequest,
    ) -> Result<(), UStatus>;

    /// Unregister for notifications relevant to a given topic inside a [`NotificationsRequest`]
    /// changing in subscription status.
    ///
    /// # Parameters
    ///
    /// * `notifications_unregister_request` - A request to no longer receive changes to subscription status
    ///
    /// # Returns
    ///
    /// * [`UStatus`] detailing if notification unregistration was successful and if not why not
    async fn unregister_for_notifications(
        &self,
        notifications_unregister_request: NotificationsRequest,
    ) -> Result<(), UStatus>;

    /// Fetch a list of subscribers that are currently subscribed to a given topic in a [`FetchSubscribersRequest`]
    ///
    /// # Parameters
    ///
    /// * `fetch_subscribers_request` - Request containing topic for which we'd like all subscribers' info
    ///
    /// # Returns
    ///
    /// * [`FetchSubscriptionsResponse`] detailing subscriber info for the provided topic
    async fn fetch_subscribers(
        &self,
        fetch_subscribers_request: FetchSubscribersRequest,
    ) -> Result<FetchSubscriptionsResponse, UStatus>;
}
