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
use std::time::{Duration, SystemTime};

use async_trait::async_trait;

use crate::{
    communication::{CallOptions, RpcClient},
    core::usubscription::{
        usubscription_uri, FetchSubscriptionsRequest, FetchSubscriptionsResponse, SubscribeRequest,
        SubscribeResponse, SubscriptionInfo, SubscriptionStatus, USubscription, UnsubscribeRequest,
        RESOURCE_ID_FETCH_SUBSCRIPTIONS, RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_RESET,
        RESOURCE_ID_SUBSCRIBE, RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_UNSUBSCRIBE,
    },
    up_core_api::usubscription::{
        NotificationsResponse as NotificationResponseProto, ResetResponse as ResetResponseProto,
        UnsubscribeResponse as UnsubscribeResponseProto,
    },
    UCode, UStatus, UUri,
};

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

#[async_trait]
impl USubscription for RpcClientUSubscription {
    async fn subscribe(
        &self,
        topic: &UUri,
        expiration: Option<SystemTime>,
        sample_period: Option<Duration>,
    ) -> Result<SubscriptionStatus, UStatus> {
        // [impl->dsn~usubscription-unsubscribe-valid-topic-uuris~1]
        topic.verify_event().map_err(|e| {
            UStatus::fail_with_code(
                UCode::InvalidArgument,
                format!("topic URI is not a valid event source: {e}"),
            )
        })?;
        let subscription_request = SubscribeRequest {
            topic: topic.clone(),
            expiration,
            sample_period,
        };

        Ok(self
            .rpc_client
            .invoke_proto_method::<_, SubscribeResponse>(
                usubscription_uri(RESOURCE_ID_SUBSCRIBE),
                Self::default_call_options(),
                subscription_request,
            )
            .await?
            .status)
    }

    async fn unsubscribe(&self, topic: &UUri) -> Result<(), UStatus> {
        let unsubscribe_request = UnsubscribeRequest {
            topic: topic.clone(),
        };
        self.rpc_client
            .invoke_proto_method::<_, UnsubscribeResponseProto>(
                usubscription_uri(RESOURCE_ID_UNSUBSCRIBE),
                Self::default_call_options(),
                unsubscribe_request,
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }

    async fn fetch_subscriptions(
        &self,
        topic_filter: Option<UUri>,
        subscriber_filter: Option<UUri>,
    ) -> Result<Vec<SubscriptionInfo>, UStatus> {
        Ok(self
            .rpc_client
            .invoke_proto_method::<_, FetchSubscriptionsResponse>(
                usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIPTIONS),
                Self::default_call_options(),
                FetchSubscriptionsRequest {
                    topic_filter,
                    subscriber_filter,
                },
            )
            .await?
            .subscriptions)
    }

    async fn register_for_notifications(&self) -> Result<(), UStatus> {
        self.rpc_client
            .invoke_proto_method::<_, NotificationResponseProto>(
                usubscription_uri(RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS),
                Self::default_call_options(),
                crate::up_core_api::usubscription::NotificationsRequest::default(),
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }

    async fn unregister_for_notifications(&self) -> Result<(), UStatus> {
        self.rpc_client
            .invoke_proto_method::<_, NotificationResponseProto>(
                usubscription_uri(RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS),
                Self::default_call_options(),
                crate::up_core_api::usubscription::NotificationsRequest::default(),
            )
            .await
            .map(|_response| ())
            .map_err(UStatus::from)
    }

    async fn reset(&self) -> Result<(), UStatus> {
        self.rpc_client
            .invoke_proto_method::<_, ResetResponseProto>(
                usubscription_uri(RESOURCE_ID_RESET),
                Self::default_call_options(),
                crate::up_core_api::usubscription::ResetRequest::default(),
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
        core::usubscription::{FetchSubscriptionsRequest, SubscribeRequest},
        UCode, UUri,
    };
    use std::sync::Arc;

    // [utest->req~usubscription-subscribe-method_id~1]
    // [utest->req~usubscription-subscribe-request-signature~1]
    // [utest->req~usubscription-subscribe-response-signature~1]
    #[tokio::test]
    async fn test_subscribe_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = SubscribeRequest {
            topic: topic.clone(),
            expiration: None,
            sample_period: None,
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

        let topic_clone = topic.clone();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<SubscribeRequest>()
                    .unwrap();
                request == expected_request && method == &usubscription_uri(RESOURCE_ID_SUBSCRIBE)
            })
            .returning(move |_method, _options, _payload| {
                let response = SubscribeResponse {
                    topic: topic_clone.clone(),
                    status: SubscriptionStatus::Subscribed,
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

    // [utest->req~usubscription-unsubscribe-method_id~1]
    // [utest->req~usubscription-unsubscribe-request-signature~1]
    // [utest->req~usubscription-unsubscribe-response-signature~1]
    #[tokio::test]
    async fn test_unsubscribe_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = UnsubscribeRequest {
            topic: topic.clone(),
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
                Ok(Some(
                    UPayload::try_from_protobuf(UnsubscribeResponseProto::default()).unwrap(),
                ))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .unsubscribe(&topic)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client.unsubscribe(&topic).await.is_ok());
    }

    // [utest->req~usubscription-fetch-subscriptions-method_id~1]
    // [utest->req~usubscription-fetch-subscriptions-request-signature~1]
    // [utest->req~usubscription-fetch-subscriptions-response-signature~1]
    #[tokio::test]
    async fn test_fetch_subscriptions_invokes_rpc_client() {
        let topic_filter = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let expected_request = FetchSubscriptionsRequest {
            topic_filter: Some(topic_filter.clone()),
            subscriber_filter: None,
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
                    subscriptions: Vec::default(),
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .fetch_subscriptions(Some(topic_filter.clone()), None)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .fetch_subscriptions(Some(topic_filter), None)
            .await
            .is_ok());
    }

    // [utest->req~usubscription-register-notifications-method_id~1]
    // [utest->req~usubscription-register-notifications-request-signature~1]
    // [utest->req~usubscription-register-notifications-response-signature~1]
    #[tokio::test]
    async fn test_register_for_notifications_invokes_rpc_client() {
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
            .withf(move |method, _options, _payload| {
                method == &usubscription_uri(RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS)
            })
            .returning(move |_method, _options, _payload| {
                Ok(Some(
                    UPayload::try_from_protobuf(NotificationResponseProto::default()).unwrap(),
                ))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .register_for_notifications()
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .register_for_notifications()
            .await
            .is_ok());
    }

    // [utest->req~usubscription-unregister-notifications-method_id~1]
    // [utest->req~usubscription-unregister-notifications-request-signature~1]
    // [utest->req~usubscription-unregister-notifications-response-signature~1]
    #[tokio::test]
    async fn test_unregister_for_notifications_invokes_rpc_client() {
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
            .withf(move |method, _options, _payload| {
                method == &usubscription_uri(RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS)
            })
            .returning(move |_method, _options, _payload| {
                Ok(Some(
                    UPayload::try_from_protobuf(NotificationResponseProto::default()).unwrap(),
                ))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .unregister_for_notifications()
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client
            .unregister_for_notifications()
            .await
            .is_ok());
    }

    // [utest->req~usubscription-reset-method_id~1]
    // [utest->req~usubscription-reset-request-signature~1]
    // [utest->req~usubscription-reset-response-signature~1]
    #[tokio::test]
    async fn test_reset_invokes_rpc_client() {
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
            .withf(move |method, _options, _payload| {
                method == &usubscription_uri(RESOURCE_ID_RESET)
            })
            .returning(move |_method, _options, _payload| {
                Ok(Some(
                    UPayload::try_from_protobuf(ResetResponseProto::default()).unwrap(),
                ))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .reset()
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(usubscription_client.reset().await.is_ok());
    }
}
