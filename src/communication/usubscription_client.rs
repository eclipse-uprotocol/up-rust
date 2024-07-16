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

use crate::{
    core::usubscription::{
        usubscription_uri, FetchSubscribersRequest, FetchSubscribersResponse,
        FetchSubscriptionsRequest, FetchSubscriptionsResponse, NotificationsRequest,
        NotificationsResponse, SubscriptionRequest, SubscriptionResponse, USubscription,
        UnsubscribeRequest, UnsubscribeResponse, RESOURCE_ID_FETCH_SUBSCRIBERS,
        RESOURCE_ID_FETCH_SUBSCRIPTIONS, RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS,
        RESOURCE_ID_SUBSCRIBE, RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_UNSUBSCRIBE,
    },
    UStatus,
};

use super::{CallOptions, RpcClient};

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
        subscription_request: SubscriptionRequest,
    ) -> Result<SubscriptionResponse, UStatus> {
        self.rpc_client
            .invoke_proto_method::<_, SubscriptionResponse>(
                usubscription_uri(RESOURCE_ID_SUBSCRIBE),
                Self::default_call_options(),
                subscription_request,
            )
            .await
            .map_err(UStatus::from)
    }

    async fn unsubscribe(&self, unsubscribe_request: UnsubscribeRequest) -> Result<(), UStatus> {
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

    async fn fetch_subscriptions(
        &self,
        fetch_subscriptions_request: FetchSubscriptionsRequest,
    ) -> Result<FetchSubscriptionsResponse, UStatus> {
        self.rpc_client
            .invoke_proto_method::<_, FetchSubscriptionsResponse>(
                usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIPTIONS),
                Self::default_call_options(),
                fetch_subscriptions_request,
            )
            .await
            .map_err(UStatus::from)
    }

    async fn register_for_notifications(
        &self,
        notifications_register_request: NotificationsRequest,
    ) -> Result<(), UStatus> {
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

    async fn unregister_for_notifications(
        &self,
        notifications_unregister_request: NotificationsRequest,
    ) -> Result<(), UStatus> {
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

    async fn fetch_subscribers(
        &self,
        fetch_subscribers_request: FetchSubscribersRequest,
    ) -> Result<FetchSubscribersResponse, UStatus> {
        self.rpc_client
            .invoke_proto_method::<_, FetchSubscribersResponse>(
                usubscription_uri(RESOURCE_ID_FETCH_SUBSCRIBERS),
                Self::default_call_options(),
                fetch_subscribers_request,
            )
            .await
            .map_err(UStatus::from)
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;

    use super::*;
    use crate::{
        communication::{rpc::MockRpcClient, UPayload},
        core::usubscription::{Request, SubscriptionResponse},
        UCode, UUri,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_subscribe_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let request = SubscriptionRequest {
            topic: Some(topic).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
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
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let usubscription_client = RpcClientUSubscription::new(Arc::new(rpc_client));

        assert!(usubscription_client
            .subscribe(request.clone())
            .await
            .is_err_and(|e| e.get_code() == UCode::INTERNAL));
        assert!(usubscription_client.subscribe(request).await.is_ok());
    }

    #[tokio::test]
    async fn test_unsubscribe_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let request = UnsubscribeRequest {
            topic: Some(topic).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
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
            .unsubscribe(request.clone())
            .await
            .is_err_and(|e| e.get_code() == UCode::INTERNAL));
        assert!(usubscription_client.unsubscribe(request).await.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_subscriptions_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let request = FetchSubscriptionsRequest {
            request: Some(Request::Topic(topic)),
            ..Default::default()
        };
        let expected_request = request.clone();
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
            .fetch_subscriptions(request.clone())
            .await
            .is_err_and(|e| e.get_code() == UCode::INTERNAL));
        assert!(usubscription_client
            .fetch_subscriptions(request)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_fetch_subscribers_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let request = FetchSubscribersRequest {
            topic: Some(topic).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
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
            .fetch_subscribers(request.clone())
            .await
            .is_err_and(|e| e.get_code() == UCode::INTERNAL));
        assert!(usubscription_client
            .fetch_subscribers(request)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_register_for_notifications_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let request = NotificationsRequest {
            topic: Some(topic).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
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
            .register_for_notifications(request.clone())
            .await
            .is_err_and(|e| e.get_code() == UCode::INTERNAL));
        assert!(usubscription_client
            .register_for_notifications(request)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_unregister_for_notifications_invokes_rpc_client() {
        let topic = UUri::try_from_parts("other", 0xd5a3, 0x01, 0xd3fe).unwrap();
        let request = NotificationsRequest {
            topic: Some(topic).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
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
            .unregister_for_notifications(request.clone())
            .await
            .is_err_and(|e| e.get_code() == UCode::INTERNAL));
        assert!(usubscription_client
            .unregister_for_notifications(request)
            .await
            .is_ok());
    }
}
