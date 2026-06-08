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

use std::sync::Arc;

use async_trait::async_trait;
use protobuf::well_known_types::timestamp::Timestamp;

use crate::{
    communication::{CallOptions, RpcClient, SubscriptionStatus},
    core::usubscription::{
        usubscription_uri, ResetReason, SubscriptionInfo, USubscription,
        RESOURCE_ID_FETCH_SUBSCRIBERS, RESOURCE_ID_FETCH_SUBSCRIPTIONS,
        RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_RESET, RESOURCE_ID_SUBSCRIBE,
        RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_UNSUBSCRIBE,
    },
    up_core_api::usubscription::{
        FetchSubscribersResponse, FetchSubscriptionsRequest, FetchSubscriptionsResponse,
        NotificationsResponse, ResetResponse, SubscribeAttributes, Subscription,
        SubscriptionRequest, SubscriptionResponse, UnsubscribeRequest, UnsubscribeResponse, Update,
    },
    UCode, UStatus, UUri,
};

fn unix_epoch_millis_as_protobuf_timestamp(
    millis: Option<u64>,
) -> Result<Option<Timestamp>, UStatus> {
    if let Some(milliseconds) = millis {
        let seconds = milliseconds
            .checked_div(1000)
            .ok_or_else(|| {
                UStatus::fail_with_code(UCode::InvalidArgument, "timestamp out of range")
            })
            .and_then(|s| {
                i64::try_from(s).map_err(|_| {
                    UStatus::fail_with_code(UCode::InvalidArgument, "timestamp out of range")
                })
            })?;
        let nanos = (milliseconds % 1000)
            .checked_mul(1_000_000)
            .ok_or_else(|| {
                UStatus::fail_with_code(UCode::InvalidArgument, "timestamp out of range")
            })
            .and_then(|s| {
                i32::try_from(s).map_err(|_| {
                    UStatus::fail_with_code(UCode::InvalidArgument, "timestamp out of range")
                })
            })?;
        Ok(Some(Timestamp {
            seconds,
            nanos,
            ..Default::default()
        }))
    } else {
        Ok(None)
    }
}

fn protobuf_timestamp_as_unix_epoch_milliseconds(
    ts: Option<&Timestamp>,
) -> Result<Option<u64>, UStatus> {
    if let Some(ts) = ts {
        let err = || {
            UStatus::fail_with_code(
                UCode::InvalidArgument,
                "invalid timestamp: seconds value out of range",
            )
        };
        u64::try_from(ts.seconds)
            .ok()
            .and_then(|s| s.checked_mul(1000))
            .and_then(|ms| ms.checked_add(ts.nanos as u64 / 1_000_000))
            .ok_or_else(err)
            .map(Some)
    } else {
        Ok(None)
    }
}

impl TryFrom<&Subscription> for SubscriptionInfo {
    type Error = UStatus;

    fn try_from(subscription_proto: &Subscription) -> Result<Self, Self::Error> {
        let topic = subscription_proto
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
        let subscriber = subscription_proto
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
        let status = subscription_proto
            .status
            .as_ref()
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "status missing",
            ))
            .and_then(SubscriptionStatus::try_from)?;
        subscription_proto
            .attributes
            .as_ref()
            .ok_or_else(|| UStatus::fail_with_code(UCode::InvalidArgument, "missing attributes"))
            .and_then(|attributes| {
                let expiration =
                    protobuf_timestamp_as_unix_epoch_milliseconds(attributes.expire.as_ref())?;
                Ok(SubscriptionInfo::new(
                    topic,
                    subscriber,
                    status,
                    expiration,
                    attributes.sample_period_ms,
                ))
            })
    }
}

impl TryFrom<&Update> for SubscriptionInfo {
    type Error = UStatus;
    fn try_from(update_proto: &Update) -> Result<Self, Self::Error> {
        let topic = update_proto
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
        let subscriber = update_proto
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
        let status = update_proto
            .status
            .as_ref()
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "status missing",
            ))
            .and_then(SubscriptionStatus::try_from)?;
        let attribs = update_proto.attributes.get_or_default();
        let expiration = protobuf_timestamp_as_unix_epoch_milliseconds(attribs.expire.as_ref())?;
        Ok(SubscriptionInfo::new(
            topic,
            subscriber,
            status,
            expiration,
            attribs.sample_period_ms,
        ))
    }
}

impl From<ResetReason> for crate::up_core_api::usubscription::reset_request::reason::Code {
    fn from(reason: ResetReason) -> Self {
        match reason {
            ResetReason::Unspecified => Self::UNSPECIFIED,
            ResetReason::FactoryReset => Self::FACTORY_RESET,
            ResetReason::CorruptedData => Self::CORRUPTED_DATA,
        }
    }
}

/// A [`USubscription`] client implementation for invoking operations of a local USubscription service.
///
/// The client requires an [`RpcClient`] for performing the remote procedure calls.
pub struct RpcClientUSubscription {
    rpc_client: Arc<dyn RpcClient>,
}

impl RpcClientUSubscription {
    /// Creates a new Notifier for a given transport.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - The client to use for performing the remote procedure calls on the USubscription service.
    pub fn new(rpc_client: Arc<dyn RpcClient>) -> Self {
        RpcClientUSubscription { rpc_client }
    }

    fn default_call_options() -> CallOptions {
        CallOptions::for_rpc_request(5_000, None, None, None)
    }
}

impl RpcClientUSubscription {
    async fn fetch_subscriptions(
        &self,
        fetch_subscriptions_request: FetchSubscriptionsRequest,
    ) -> Result<Vec<SubscriptionInfo>, UStatus> {
        let response = self
            .rpc_client
            .invoke_proto_method::<_, FetchSubscriptionsResponse>(
                usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIPTIONS),
                Self::default_call_options(),
                fetch_subscriptions_request,
            )
            .await
            .map_err(UStatus::from)?;
        let mut result = Vec::new();
        for subscription in &response.subscriptions {
            let info = SubscriptionInfo::try_from(subscription)?;
            result.push(info);
        }
        Ok(result)
    }
}

#[async_trait]
impl USubscription for RpcClientUSubscription {
    async fn subscribe(
        &self,
        topic: &UUri,
        expiration: Option<u64>, // millis since Unix Epoch
        min_sample_period: Option<u32>,
    ) -> Result<SubscriptionStatus, UStatus> {
        let subscription_request = SubscriptionRequest {
            topic: Some(topic.into()).into(),
            attributes: match (expiration, min_sample_period) {
                (None, None) => None.into(),
                _ => Some(SubscribeAttributes {
                    expire: unix_epoch_millis_as_protobuf_timestamp(expiration)?.into(),
                    sample_period_ms: min_sample_period,
                    ..Default::default()
                })
                .into(),
            },
            ..Default::default()
        };
        self.rpc_client
            .invoke_proto_method::<_, SubscriptionResponse>(
                usubscription_uri(RESOURCE_ID_SUBSCRIBE),
                Self::default_call_options(),
                subscription_request,
            )
            .await
            .and_then(|response| {
                Ok(response.status.as_ref().map_or_else(
                    || {
                        Err(UStatus::fail_with_code(
                            UCode::InvalidArgument,
                            "uSubscription returned invalid response: no subscription status",
                        ))
                    },
                    SubscriptionStatus::try_from,
                )?)
            })
            .map_err(UStatus::from)
    }

    async fn unsubscribe(&self, topic: &UUri) -> Result<(), UStatus> {
        let unsubscribe_request = UnsubscribeRequest {
            topic: Some(topic.into()).into(),
            ..Default::default()
        };
        self.rpc_client
            .invoke_proto_method::<_, UnsubscribeResponse>(
                usubscription_uri(RESOURCE_ID_UNSUBSCRIBE),
                Self::default_call_options(),
                unsubscribe_request,
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }

    async fn fetch_subscriptions_by_topic(
        &self,
        topic: &UUri,
    ) -> Result<Vec<SubscriptionInfo>, UStatus> {
        let fetch_subscriptions_request =
            crate::up_core_api::usubscription::FetchSubscriptionsRequest {
                request: Some(
                    crate::up_core_api::usubscription::fetch_subscriptions_request::Request::Topic(
                        topic.into(),
                    ),
                ),
                ..Default::default()
            };
        self.fetch_subscriptions(fetch_subscriptions_request).await
    }

    async fn fetch_subscriptions_by_subscriber(
        &self,
        subscriber: &UUri,
    ) -> Result<Vec<SubscriptionInfo>, UStatus> {
        let subscriber_info = crate::up_core_api::usubscription::SubscriberInfo {
            uri: Some(subscriber.into()).into(),
            ..Default::default()
        };
        let fetch_subscriptions_request =
            crate::up_core_api::usubscription::FetchSubscriptionsRequest {
                request: Some(
                    crate::up_core_api::usubscription::fetch_subscriptions_request::Request::Subscriber(subscriber_info),
                ),
                ..Default::default()
            };
        self.fetch_subscriptions(fetch_subscriptions_request).await
    }

    async fn register_for_notifications(&self, topic: &UUri) -> Result<(), UStatus> {
        let notifications_register_request =
            crate::up_core_api::usubscription::NotificationsRequest {
                topic: Some(topic.into()).into(),
                ..Default::default()
            };
        self.rpc_client
            .invoke_proto_method::<_, NotificationsResponse>(
                usubscription_uri(RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS),
                Self::default_call_options(),
                notifications_register_request,
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }

    async fn unregister_for_notifications(&self, topic: &UUri) -> Result<(), UStatus> {
        let notifications_unregister_request =
            crate::up_core_api::usubscription::NotificationsRequest {
                topic: Some(topic.into()).into(),
                ..Default::default()
            };
        self.rpc_client
            .invoke_proto_method::<_, NotificationsResponse>(
                usubscription_uri(RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS),
                Self::default_call_options(),
                notifications_unregister_request,
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }

    async fn fetch_subscribers(&self, topic: &UUri) -> Result<Vec<UUri>, UStatus> {
        let fetch_subscribers_request =
            crate::up_core_api::usubscription::FetchSubscribersRequest {
                topic: Some(topic.into()).into(),
                ..Default::default()
            };
        let response = self
            .rpc_client
            .invoke_proto_method::<_, FetchSubscribersResponse>(
                usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIBERS),
                Self::default_call_options(),
                fetch_subscribers_request,
            )
            .await?;
        let mut result = vec![];
        for subscriber_info in &response.subscribers {
            let uri = subscriber_info
                .uri
                .as_ref()
                .ok_or_else(|| {
                    UStatus::fail_with_code(
                        UCode::InvalidArgument,
                        "uSubscription returned invalid response: missing subscriber URI",
                    )
                })
                .and_then(|uri_proto| {
                    UUri::try_from(uri_proto).map_err(|_e| {
                        UStatus::fail_with_code(
                            UCode::InvalidArgument,
                            "uSubscription returned invalid response: invalid subscriber URI",
                        )
                    })
                })?;
            result.push(uri);
        }
        Ok(result)
    }

    async fn reset(
        &self,
        reason: ResetReason,
        message: Option<String>,
        before: Option<u64>, // millis since Unix Epoch
    ) -> Result<(), UStatus> {
        let before_ts = unix_epoch_millis_as_protobuf_timestamp(before)?;
        let reset_request = crate::up_core_api::usubscription::ResetRequest {
            reason: Some(crate::up_core_api::usubscription::reset_request::Reason {
                code: crate::up_core_api::usubscription::reset_request::reason::Code::from(reason)
                    .into(),
                message,
                ..Default::default()
            })
            .into(),
            before: before_ts.into(),
            ..Default::default()
        };
        self.rpc_client
            .invoke_proto_method::<_, ResetResponse>(
                usubscription_uri(RESOURCE_ID_RESET),
                Self::default_call_options(),
                reset_request,
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;

    use super::*;
    use crate::{
        communication::{MockRpcClient, UPayload},
        up_core_api::usubscription::{
            fetch_subscriptions_request::Request, FetchSubscribersRequest, NotificationsRequest,
            ResetRequest,
        },
        UCode, UUri,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_subscribe_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = SubscriptionRequest {
            topic: Some((&topic).into()).into(),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_SUBSCRIBE) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<SubscriptionRequest>()
                    .unwrap();
                request == expected_request && method == &usubscription_uri(RESOURCE_ID_SUBSCRIBE)
            })
            .returning(move |_method, _options, _payload| {
                let response = SubscriptionResponse {
                    status: Some(crate::up_core_api::usubscription::SubscriptionStatus {
                        state: crate::up_core_api::usubscription::subscription_status::State::SUBSCRIBED
                            .into(),
                        ..Default::default()
                    }).into(),
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .subscribe(&topic, None, None)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .subscribe(&topic, None, None)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_unsubscribe_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = UnsubscribeRequest {
            topic: Some((&topic).into()).into(),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_UNSUBSCRIBE) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<UnsubscribeRequest>()
                    .unwrap();
                request == expected_request && method == &usubscription_uri(RESOURCE_ID_UNSUBSCRIBE)
            })
            .returning(move |_method, _options, _payload| {
                let response = UnsubscribeResponse {
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .unsubscribe(&topic)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client.unsubscribe(&topic).await.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_subscriptions_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = FetchSubscriptionsRequest {
            request: Some(Request::Topic((&topic).into())),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIPTIONS) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<FetchSubscriptionsRequest>()
                    .unwrap();

                request == expected_request
                    && method == &usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIPTIONS)
            })
            .returning(move |_method, _options, _payload| {
                let response = FetchSubscriptionsResponse {
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .fetch_subscriptions_by_topic(&topic)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .fetch_subscriptions_by_topic(&topic)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_fetch_subscribers_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = FetchSubscribersRequest {
            topic: Some((&topic).into()).into(),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIBERS) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<FetchSubscribersRequest>()
                    .unwrap();

                request == expected_request
                    && method == &usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIBERS)
            })
            .returning(move |_method, _options, _payload| {
                let response = FetchSubscribersResponse {
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .fetch_subscribers(&topic)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client.fetch_subscribers(&topic).await.is_ok());
    }

    #[tokio::test]
    async fn test_register_for_notifications_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = NotificationsRequest {
            topic: Some((&topic).into()).into(),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS)
                    && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<NotificationsRequest>()
                    .unwrap();

                request == expected_request
                    && method == &usubscription_uri(RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS)
            })
            .returning(move |_method, _options, _payload| {
                let response = NotificationsResponse {
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .register_for_notifications(&topic)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .register_for_notifications(&topic)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_unregister_for_notifications_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = NotificationsRequest {
            topic: Some((&topic).into()).into(),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS)
                    && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<NotificationsRequest>()
                    .unwrap();

                request == expected_request
                    && method == &usubscription_uri(RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS)
            })
            .returning(move |_method, _options, _payload| {
                let response = NotificationsResponse {
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .unregister_for_notifications(&topic)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .unregister_for_notifications(&topic)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_reset_invokes_rpc_client() {
        let expected_request = ResetRequest {
            reason: Some(crate::up_core_api::usubscription::reset_request::Reason {
                code: crate::up_core_api::usubscription::reset_request::reason::Code::UNSPECIFIED
                    .into(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &usubscription_uri(RESOURCE_ID_RESET) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<ResetRequest>()
                    .unwrap();

                request == expected_request && method == &usubscription_uri(RESOURCE_ID_RESET)
            })
            .returning(move |_method, _options, _payload| {
                let response = ResetResponse {
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .reset(ResetReason::Unspecified, None, None)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .reset(ResetReason::Unspecified, None, None)
            .await
            .is_ok());
    }
}
