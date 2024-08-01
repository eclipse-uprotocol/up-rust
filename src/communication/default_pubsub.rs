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

// [impl->req~up-language-comm-api-default-impl~1]

use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Deref,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    core::usubscription::{
        self, State, SubscriberInfo, SubscriptionRequest, USubscription, UnsubscribeRequest, Update,
    },
    LocalUriProvider, UListener, UMessage, UMessageBuilder, UStatus, UTransport, UUri,
};

use super::{
    apply_common_options, build_message, pubsub::SubscriptionChangeHandler, CallOptions,
    InMemoryRpcClient, Notifier, PubSubError, Publisher, RegistrationError, RpcClientUSubscription,
    SimpleNotifier, Subscriber, UPayload,
};

#[derive(Clone)]
struct ComparableSubscriptionChangeHandler {
    inner: Arc<dyn SubscriptionChangeHandler>,
}

impl ComparableSubscriptionChangeHandler {
    fn new(handler: Arc<dyn SubscriptionChangeHandler>) -> Self {
        ComparableSubscriptionChangeHandler {
            inner: handler.clone(),
        }
    }
}

impl Deref for ComparableSubscriptionChangeHandler {
    type Target = dyn SubscriptionChangeHandler;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl PartialEq for ComparableSubscriptionChangeHandler {
    /// Compares this handler to another handler.
    ///
    /// # Returns
    ///
    /// `true` if the pointer to the handler held by `self` is equal to the pointer held by `other`.
    /// This is consistent with the implementation of [`ComparableSubscriptionChangeHandler::hash`].
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ComparableSubscriptionChangeHandler {}

#[derive(Default)]
struct SubscriptionChangeListener {
    subscription_change_handlers: RwLock<HashMap<UUri, ComparableSubscriptionChangeHandler>>,
}

impl SubscriptionChangeListener {
    /// Adds a handler for a given topic.
    ///
    /// # Errors
    ///
    /// Returns a [`RegistrationError::AlreadyExists`] if another handler has already been registered for
    /// the given topic. Returns a [`RegistrationError::Unknown`] if the internal state could not be accessed,
    fn add_handler(
        &self,
        topic: UUri,
        subscription_change_handler: Arc<dyn SubscriptionChangeHandler>,
    ) -> Result<(), RegistrationError> {
        let Ok(mut handlers) = self.subscription_change_handlers.write() else {
            return Err(RegistrationError::Unknown(UStatus::fail_with_code(
                crate::UCode::INTERNAL,
                "failed to acquire write lock for handler map",
            )));
        };
        let handler_to_add = ComparableSubscriptionChangeHandler::new(subscription_change_handler);
        match handlers.entry(topic) {
            Entry::Vacant(entry) => {
                entry.insert(handler_to_add);
                Ok(())
            }
            Entry::Occupied(entry) => {
                if entry.get() == &handler_to_add {
                    Ok(())
                } else {
                    Err(RegistrationError::AlreadyExists)
                }
            }
        }
    }

    /// Removes the handler for a given topic.
    ///
    /// This function also succeeds if no handler is registered for the topic.
    ///
    /// # Errors
    ///
    /// Returns a [`RegistrationError::Unknown`] if the internal state could not be accessed,
    fn remove_handler(&self, topic: &UUri) -> Result<(), RegistrationError> {
        self.subscription_change_handlers
            .write()
            .map_err(|_e| {
                RegistrationError::Unknown(UStatus::fail_with_code(
                    crate::UCode::INTERNAL,
                    "failed to acquire write lock for handler map",
                ))
            })
            .map(|mut handlers| {
                handlers.remove(topic);
            })
    }

    /// Removes all handlers for all topic.
    ///
    /// # Errors
    ///
    /// Returns a [`RegistrationError::Unknown`] if the internal state could not be accessed,
    fn clear(&self) -> Result<(), RegistrationError> {
        self.subscription_change_handlers
            .write()
            .map_err(|_e| {
                RegistrationError::Unknown(UStatus::fail_with_code(
                    crate::UCode::INTERNAL,
                    "failed to acquire write lock for handler map",
                ))
            })
            .map(|mut handlers| {
                handlers.clear();
            })
    }

    #[cfg(test)]
    fn has_handler(&self, topic: &UUri) -> bool {
        self.subscription_change_handlers
            .read()
            .map_or(false, |handlers| handlers.contains_key(topic))
    }
}

#[async_trait]
impl UListener for SubscriptionChangeListener {
    async fn on_receive(&self, msg: UMessage) {
        if !msg.is_notification() {
            return;
        }
        let Ok(subscription_update) = msg.extract_protobuf::<Update>() else {
            debug!("ignoring notification that does not contain subscription update");
            return;
        };
        let Some(topic) = subscription_update.topic.as_ref() else {
            return;
        };
        let Some(status) = subscription_update.status.as_ref() else {
            return;
        };

        let Ok(handlers) = self.subscription_change_handlers.read() else {
            return;
        };
        if let Some(handler) = handlers.get(topic) {
            handler.on_subscription_change(topic.to_owned(), status.to_owned());
        }
    }
}

/// A [`Publisher`] that uses the uProtocol Transport Layer API for publishing events to topics.
pub struct SimplePublisher {
    transport: Arc<dyn UTransport>,
    uri_provider: Arc<dyn LocalUriProvider>,
}

impl SimplePublisher {
    /// Creates a new client.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for sending messages.
    /// * `uri_provider` - The service to use for creating the event messages' _sink_ address.
    pub fn new(transport: Arc<dyn UTransport>, uri_provider: Arc<dyn LocalUriProvider>) -> Self {
        SimplePublisher {
            transport,
            uri_provider,
        }
    }
}

#[async_trait]
impl Publisher for SimplePublisher {
    async fn publish(
        &self,
        resource_id: u16,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<(), PubSubError> {
        let mut builder = UMessageBuilder::publish(self.uri_provider.get_resource_uri(resource_id));
        apply_common_options(call_options, &mut builder);
        match build_message(&mut builder, payload) {
            Ok(publish_message) => self
                .transport
                .send(publish_message)
                .await
                .map_err(PubSubError::PublishError),
            Err(e) => Err(PubSubError::InvalidArgument(format!(
                "failed to create Publish message from parameters: {}",
                e
            ))),
        }
    }
}

/// A [`Subscriber`] which keeps all information about registered susbcription change handlers in memory.
///
/// The subscriber requires a (client) implementation of [`USubscription`] in order to inform the local
/// USubscription service about newly subscribed and unsubscribed topics. It also needs a [`Notifier`]
/// for receiving notifications about subscription status updates from the local USubscription service.
/// Finally, it needs a [`UTransport`] for receiving events that have been published to subscribed topics.
///
/// During [startup](`Self::for_clients`) the subscriber uses the Notifier to register a generic [`UListener`]
/// for receiving notifications from the USubscription service. The listener maintains an in-memory mapping
/// of subscribed topics to corresponding subscription change handlers.
///
/// When a client [`subscribes to a topic`](Self::subscribe), the local USubscription service is informed
/// about the new subscription and a (client provided) subscription change handler is registered with the
/// listener. When a subscription change notification arrives from the USubscription service, the corresponding
/// handler is being looked up and invoked.
pub struct InMemorySubscriber {
    transport: Arc<dyn UTransport>,
    uri_provider: Arc<dyn LocalUriProvider>,
    usubscription: Arc<dyn USubscription>,
    notifier: Arc<dyn Notifier>,
    subscription_change_listener: Arc<SubscriptionChangeListener>,
}

impl InMemorySubscriber {
    /// Creates a new Subscriber for a given transport.
    ///
    /// The subscriber keeps track of subscription change handlers in memory only.
    /// This function uses the given transport to create an [`RpcClientUSubscription`] and a [`SimpleNotifier`]
    /// and then delegate to [`Self::for_clients`] to create the Subscriber.
    ///
    /// # Errors
    ///
    /// Returns an error if the Notifier cannot register a listener for notifications from the USubscription service.
    pub async fn new(
        transport: Arc<dyn UTransport>,
        uri_provider: Arc<dyn LocalUriProvider>,
    ) -> Result<Self, RegistrationError> {
        let rpc_client = InMemoryRpcClient::new(transport.clone(), uri_provider.clone())
            .await
            .map(Arc::new)?;
        let usubscription_client = Arc::new(RpcClientUSubscription::new(rpc_client));
        let notifier = Arc::new(SimpleNotifier::new(transport.clone(), uri_provider.clone()));
        Self::for_clients(transport, uri_provider, usubscription_client, notifier).await
    }

    /// Creates a new Subscriber for given clients.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for registering the event listeners for subscribed topics.
    /// * `uri-provider` - The service to use for creating topic addresses.
    /// * `usubscription` - The client to use for interacting with the (local) USubscription service.
    /// * `notifier` - The client to use for registering the listener for subscription updates from USubscription.
    ///
    /// # Errors
    ///
    /// Returns an error if the Notifier cannot register a listener for notifications from the USubscription service.
    pub async fn for_clients(
        transport: Arc<dyn UTransport>,
        uri_provider: Arc<dyn LocalUriProvider>,
        usubscription: Arc<dyn USubscription>,
        notifier: Arc<dyn Notifier>,
    ) -> Result<Self, RegistrationError> {
        // register a generic listener for subscription updates
        // whenever a uE later tries to subscribe to a topic, it can provide an optional callback for
        // handling subscription updates for the topic it tries to subscribe to
        let subscription_change_listener = Arc::new(SubscriptionChangeListener {
            subscription_change_handlers: RwLock::new(HashMap::new()),
        });
        notifier
            .start_listening(
                &usubscription::usubscription_uri(usubscription::RESOURCE_ID_SUBSCRIPTION_CHANGE),
                subscription_change_listener.clone(),
            )
            .await?;
        Ok(InMemorySubscriber {
            transport,
            uri_provider,
            usubscription,
            notifier,
            subscription_change_listener,
        })
    }

    /// Stops this client.
    ///
    /// Clears all internal state and unregisters the listener for subscription updates from the USubscription service.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener could not be unregistered. In this case the internal state remains intact.
    pub async fn stop(&self) -> Result<(), RegistrationError> {
        self.notifier
            .stop_listening(
                &usubscription::usubscription_uri(usubscription::RESOURCE_ID_SUBSCRIPTION_CHANGE),
                self.subscription_change_listener.clone(),
            )
            .await
            .and_then(|_ok| self.subscription_change_listener.clear())
    }

    fn subscriber_info(&self) -> SubscriberInfo {
        SubscriberInfo {
            uri: Some(self.uri_provider.get_source_uri()).into(),
            ..Default::default()
        }
    }

    async fn invoke_subscribe(
        &self,
        topic: &UUri,
        subscription_change_handler: Option<Arc<dyn SubscriptionChangeHandler>>,
    ) -> Result<State, RegistrationError> {
        let subscription_request = SubscriptionRequest {
            subscriber: Some(self.subscriber_info()).into(),
            topic: Some(topic.to_owned()).into(),
            ..Default::default()
        };
        match self.usubscription.subscribe(subscription_request).await {
            Ok(response) => match response.status.state.enum_value() {
                Ok(state) if state == State::SUBSCRIBED || state == State::SUBSCRIBE_PENDING => {
                    if let Some(handler) = subscription_change_handler.clone() {
                        self.subscription_change_listener
                            .add_handler(topic.to_owned(), handler)?;
                    }
                    Ok(state)
                }
                _ => {
                    debug!(topic = %topic, "failed to subscribe to topic: {}", response.status.message);
                    Err(RegistrationError::Unknown(UStatus::fail_with_code(
                        crate::UCode::FAILED_PRECONDITION,
                        response.status.message.to_owned(),
                    )))
                }
            },
            Err(e) => {
                info!(topic = %topic, "error invoking USubscription service: {}", e);
                Err(RegistrationError::Unknown(UStatus::fail_with_code(
                    crate::UCode::INTERNAL,
                    "failed to invoke USubscription service",
                )))
            }
        }
    }

    async fn invoke_unsubscribe(&self, topic: &UUri) -> Result<(), RegistrationError> {
        let request = UnsubscribeRequest {
            subscriber: Some(self.subscriber_info()).into(),
            topic: Some(topic.to_owned()).into(),
            ..Default::default()
        };
        self.usubscription
            .unsubscribe(request)
            .await
            .map(|_| {
                let _ = self.subscription_change_listener.remove_handler(topic);
            })
            .map_err(|e| {
                info!(topic = %topic, "error invoking USubscription service: {}", e);
                RegistrationError::Unknown(UStatus::fail_with_code(
                    crate::UCode::INTERNAL,
                    "failed to invoke USubscription service",
                ))
            })
    }

    #[cfg(test)]
    fn add_subscription_change_handler(
        &self,
        topic: &UUri,
        subscription_change_handler: Arc<dyn SubscriptionChangeHandler>,
    ) -> Result<(), RegistrationError> {
        self.subscription_change_listener
            .add_handler(topic.to_owned(), subscription_change_handler)
    }

    #[cfg(test)]
    fn has_subscription_change_handler(&self, topic: &UUri) -> bool {
        self.subscription_change_listener.has_handler(topic)
    }
}

#[async_trait]
impl Subscriber for InMemorySubscriber {
    async fn subscribe(
        &self,
        topic_filter: &UUri,
        handler: Arc<dyn UListener>,
        subscription_change_handler: Option<Arc<dyn SubscriptionChangeHandler>>,
    ) -> Result<(), RegistrationError> {
        self.invoke_subscribe(topic_filter, subscription_change_handler)
            .await?;
        self.transport
            .register_listener(topic_filter, None, handler.clone())
            .await
            // When this fails, we have ended up in a situation where we
            // have successfully (logically) subscribed to the topic via the USubscriptio service
            // but we have not been able to register the listener with the local transport.
            // This means that events might start getting forwarded to the local authority which
            // are not being consumed. Apart from this inefficiency, this does not pose a real
            // problem and since we return an err, the client might be inclined to try
            // again and (eventually) succeed in registering the listener as well.
            .map_err(RegistrationError::from)
    }

    async fn unsubscribe(
        &self,
        topic: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError> {
        self.invoke_unsubscribe(topic).await?;
        self.transport
            .unregister_listener(topic, None, listener)
            .await
            // When this fails, we have ended up in a situation where we
            // have successfully (logically) unsubscribed from the topic via the USubscriptio service
            // but we have not been able to unregister the listener from the local transport.
            // This means that events originating from entities connected to a different transport
            // may no longer get forwarded to the local transport, resulting in the (still registered)
            // listener not being invoked for these events. We therefore return an error which should
            // trigger the client to try again and (eventually) succeed in unregistering the listener as well.
            .map_err(RegistrationError::from)
    }
}

#[cfg(test)]
mod tests {

    // [utest->req~up-language-comm-api-default-impl~1]

    use super::*;

    use mockall::Sequence;
    use protobuf::well_known_types::wrappers::StringValue;
    use protobuf::Enum;
    use usubscription::{MockUSubscription, SubscriptionResponse, SubscriptionStatus};

    use crate::{
        communication::{notification::MockNotifier, pubsub::MockSubscriptionChangeHandler},
        utransport::{MockLocalUriProvider, MockTransport, MockUListener},
        UAttributes, UCode, UMessageType, UPriority, UStatus, UUri, UUID,
    };

    fn new_uri_provider() -> Arc<dyn LocalUriProvider> {
        let mut mock_uri_locator = MockLocalUriProvider::new();
        mock_uri_locator
            .expect_get_resource_uri()
            .returning(|resource_id| UUri {
                ue_id: 0x0005,
                ue_version_major: 0x02,
                resource_id: resource_id as u32,
                ..Default::default()
            });
        mock_uri_locator.expect_get_source_uri().returning(|| UUri {
            ue_id: 0x0005,
            ue_version_major: 0x02,
            resource_id: 0x0000,
            ..Default::default()
        });
        Arc::new(mock_uri_locator)
    }

    fn succeding_notifier() -> Arc<dyn Notifier> {
        let mut notifier = MockNotifier::new();
        notifier
            .expect_start_listening()
            .once()
            .return_const(Ok(()));
        Arc::new(notifier)
    }

    #[tokio::test]
    async fn test_publish_fails_for_invalid_topic() {
        // GIVEN a publisher
        let uri_provider = new_uri_provider();
        let mut transport = MockTransport::new();

        transport.expect_do_send().never();
        let publisher = SimplePublisher::new(Arc::new(transport), uri_provider);

        // WHEN publishing to an invalid topic
        let options = CallOptions::for_publish(None, None, None);
        let publish_result = publisher
            // resource ID for topic must be >= 0x8000
            .publish(0x1000, options, None)
            .await;

        // THEN publishing fails with an InvalidArgument error
        assert!(publish_result.is_err_and(|e| matches!(e, PubSubError::InvalidArgument(_msg))));
    }

    #[tokio::test]
    async fn test_publish_fails_with_transport_error() {
        let message_id = UUID::build();
        // GIVEN a publisher
        let uri_provider = new_uri_provider();
        let mut transport = MockTransport::new();
        // that is not connected to the underlying messaging infrastructure
        let expected_message_id = message_id.clone();
        transport
            .expect_do_send()
            .once()
            .withf(move |msg| {
                msg.attributes.get_or_default().id.get_or_default() == &expected_message_id
            })
            .returning(|_msg| {
                Err(UStatus::fail_with_code(
                    UCode::UNAVAILABLE,
                    "transport not available",
                ))
            });
        let publisher = SimplePublisher::new(Arc::new(transport), uri_provider);

        // WHEN publishing to a valid topic
        let options = CallOptions::for_publish(None, Some(message_id), None);
        let publish_result = publisher.publish(0x9A00, options, None).await;

        // THEN publishing fails with a PublishError
        assert!(publish_result.is_err_and(|e| matches!(e, PubSubError::PublishError(_status))));
    }

    #[tokio::test]
    async fn test_publish_succeeds() {
        // GIVEN a publisher
        let uri_provider = new_uri_provider();
        let mut transport = MockTransport::new();
        let message_id = UUID::build();
        let expected_message_id = message_id.clone();

        transport
            .expect_do_send()
            .once()
            .withf(move |message| {
                let Ok(payload) = message.extract_protobuf::<StringValue>() else {
                    return false;
                };
                payload.value == *"Hello"
                    && message.is_publish()
                    && message.attributes.as_ref().map_or(false, |attribs| {
                        attribs.id.as_ref() == Some(&expected_message_id)
                            && attribs.priority.value() == UPriority::UPRIORITY_CS3.value()
                            && attribs.ttl == Some(5_000)
                    })
            })
            .returning(|_msg| Ok(()));

        let publisher = SimplePublisher::new(Arc::new(transport), uri_provider);

        // WHEN publishing some data to a valid topic
        let call_options = CallOptions::for_publish(
            Some(5_000),
            Some(message_id.clone()),
            Some(crate::UPriority::UPRIORITY_CS3),
        );
        let payload = StringValue {
            value: "Hello".to_string(),
            ..Default::default()
        };
        let publish_result = publisher
            .publish(
                0x9A00,
                call_options,
                Some(
                    UPayload::try_from_protobuf(payload)
                        .expect("should have been able to create message payload"),
                ),
            )
            .await;

        // THEN a corresponding Publish message has been sent via the transport
        assert!(publish_result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_creation_fails_when_notifier_fails_to_register_listener() {
        // GIVEN a Notifier
        let mut notifier = MockNotifier::new();
        // that is not connected to its transport
        notifier
            .expect_start_listening()
            .once()
            .return_const(Err(RegistrationError::Unknown(UStatus::fail_with_code(
                UCode::UNAVAILABLE,
                "not available",
            ))));

        // WHEN trying to create a Subscriber for this Notifier
        let creation_attempt = InMemorySubscriber::for_clients(
            Arc::new(MockTransport::new()),
            new_uri_provider(),
            Arc::new(MockUSubscription::new()),
            Arc::new(notifier),
        )
        .await;
        // THEN creation fails
        assert!(creation_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));
    }

    #[tokio::test]
    async fn test_subscriber_stop_succeeds() {
        // GIVEN a Notifier
        let mut notifier = MockNotifier::new();
        // that succeeds to stop listening to notifications
        notifier.expect_stop_listening().once().return_const(Ok(()));

        // and a Subscriber using this Notifier
        let subscription_change_listener = Arc::new(SubscriptionChangeListener::default());
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let handler = Arc::new(MockSubscriptionChangeHandler::new());
        subscription_change_listener
            .add_handler(topic.clone(), handler)
            .expect("adding a handler should have succeeded");

        let subscriber = InMemorySubscriber {
            transport: Arc::new(MockTransport::new()),
            uri_provider: new_uri_provider(),
            usubscription: Arc::new(MockUSubscription::new()),
            notifier: Arc::new(notifier),
            subscription_change_listener,
        };

        // WHEN trying to stop the Subscriber
        let stop_attempt = subscriber.stop().await;

        // THEN the attempt succeeds
        assert!(stop_attempt.is_ok_and(|_| {
            // and the subscription change handlers have been cleared
            !subscriber.has_subscription_change_handler(&topic)
        }));
    }

    #[tokio::test]
    async fn test_subscribe_fails_when_usubscription_invocation_fails() {
        // GIVEN a USubscription client
        let mut seq = Sequence::new();
        let mut usubscription_client = MockUSubscription::new();
        // that fails to perform subscription
        // due to different reasons
        usubscription_client
            .expect_subscribe()
            .once()
            .in_sequence(&mut seq)
            .return_const(Err(UStatus::fail_with_code(
                UCode::UNAVAILABLE,
                "not connected",
            )));
        usubscription_client
            .expect_subscribe()
            .once()
            .in_sequence(&mut seq)
            .return_const({
                let response = SubscriptionResponse {
                    status: Some(SubscriptionStatus {
                        state: State::UNSUBSCRIBED.into(),
                        message: "unsupported topic".to_string(),
                        ..Default::default()
                    })
                    .into(),
                    ..Default::default()
                };
                Ok(response)
            });
        usubscription_client
            .expect_subscribe()
            .once()
            .in_sequence(&mut seq)
            .return_const({
                let response = SubscriptionResponse {
                    status: Some(SubscriptionStatus {
                        message: "unknown state".to_string(),
                        ..Default::default()
                    })
                    .into(),
                    ..Default::default()
                };
                Ok(response)
            });

        // and a transport
        let mut transport = MockTransport::new();
        transport.expect_do_register_listener().never();

        // and a Subscriber using that USubscription client
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();

        // WHEN subscribing to a topic
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let mut listener = MockUListener::new();
        listener.expect_on_receive().never();
        let listener_ref = Arc::new(listener);

        let subscribe_attempt = subscriber
            .subscribe(&topic, listener_ref.clone(), None)
            .await;

        // THEN the first attempt fails
        assert!(subscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));

        let subscribe_attempt = subscriber
            .subscribe(&topic, listener_ref.clone(), None)
            .await;

        // and the second attempt fails as well
        assert!(subscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));

        let subscribe_attempt = subscriber
            .subscribe(&topic, listener_ref.clone(), None)
            .await;

        // and the third attempt fails as well
        assert!(subscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));
    }

    #[tokio::test]
    async fn test_repeated_subscribe_fails_for_different_subscription_change_handlers() {
        // GIVEN  a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that succeeds to subscribe to topics
        usubscription_client
            .expect_subscribe()
            .times(2)
            .returning(|request| {
                let response = SubscriptionResponse {
                    topic: request.topic.clone(),
                    status: Some(SubscriptionStatus {
                        state: State::SUBSCRIBED.into(),
                        ..Default::default()
                    })
                    .into(),
                    ..Default::default()
                };
                Ok(response)
            });

        // and a transport
        let mut transport = MockTransport::new();
        // that fails to register a listener
        transport
            .expect_do_register_listener()
            .once()
            .return_const(Err(UStatus::fail_with_code(
                UCode::UNAVAILABLE,
                "not connected",
            )));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();

        // WHEN subscribing to a topic
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let listener = Arc::new(MockUListener::new());
        let subscribe_attempt = subscriber
            .subscribe(
                &topic,
                listener.clone(),
                Some(Arc::new(MockSubscriptionChangeHandler::new())),
            )
            .await;

        // THEN the first attempt fails due to the transport having failed
        assert!(subscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));

        // and a second attempt using a different subscription change handler
        let subscribe_attempt = subscriber
            .subscribe(
                &topic,
                listener.clone(),
                Some(Arc::new(MockSubscriptionChangeHandler::new())),
            )
            .await;
        // fails with an ALREADY_EXISTS error
        assert!(subscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::AlreadyExists)));
    }

    #[tokio::test]
    async fn test_subscribe_succeeds_on_second_attempt() {
        let (captured_listener_tx, captured_listener_rx) = std::sync::mpsc::channel();

        // GIVEN a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that succeeds to subscribe to topics
        usubscription_client
            .expect_subscribe()
            .times(2)
            .returning(|request| {
                let response = SubscriptionResponse {
                    topic: request.topic.clone(),
                    status: Some(SubscriptionStatus {
                        state: State::SUBSCRIBED.into(),
                        ..Default::default()
                    })
                    .into(),
                    ..Default::default()
                };
                Ok(response)
            });

        // and a transport
        let mut transport = MockTransport::new();
        let mut seq = Sequence::new();
        // that first fails to register a listener
        transport
            .expect_do_register_listener()
            .once()
            .in_sequence(&mut seq)
            .return_const(Err(UStatus::fail_with_code(
                UCode::UNAVAILABLE,
                "not connected",
            )));
        // but succeeds on the second attempt
        transport
            .expect_do_register_listener()
            .once()
            .in_sequence(&mut seq)
            .returning(move |_source_filter, _sink_filter, listener| {
                captured_listener_tx
                    .send(listener)
                    .map_err(|_e| UStatus::fail("cannot capture listener"))
            });

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();

        // WHEN subscribing to a topic
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().once().return_const(());
        let listener = Arc::new(mock_listener);
        let handler = Arc::new(MockSubscriptionChangeHandler::new());
        let subscribe_attempt = subscriber
            .subscribe(&topic, listener.clone(), Some(handler.clone()))
            .await;

        // THEN the first attempt fails
        assert!(subscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));

        let subscribe_attempt = subscriber
            .subscribe(&topic, listener.clone(), Some(handler.clone()))
            .await;

        // but the second attempt succeeds
        assert!(subscribe_attempt.is_ok());

        // and the registered listener receives events that are published to the topic
        let event = UMessageBuilder::publish(topic).build().unwrap();
        let captured_listener = captured_listener_rx.recv().unwrap().to_owned();
        captured_listener.on_receive(event).await;
    }

    #[tokio::test]
    async fn test_unsubscribe_fails_for_unknown_listener() {
        // GIVEN a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that succeeds to unsubscribe from topics
        usubscription_client
            .expect_unsubscribe()
            .once()
            .return_const(Ok(()));

        // and a transport
        let mut transport = MockTransport::new();
        // which fails to unregister an unknown listener
        transport
            .expect_do_unregister_listener()
            .once()
            .return_const(Err(UStatus::fail_with_code(
                UCode::NOT_FOUND,
                "no such listener",
            )));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();

        // WHEN unsubscribing from a topic for which no listener had been registered
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let listener = Arc::new(MockUListener::new());
        let unsubscribe_attempt = subscriber.unsubscribe(&topic, listener.clone()).await;

        // THEN the the attempt fails
        assert!(unsubscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::NoSuchListener)));
    }

    #[tokio::test]
    async fn test_unsubscribe_fails_if_usubscription_invocation_fails() {
        // GIVEN a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that fails to unsubscribe from topics
        usubscription_client
            .expect_unsubscribe()
            .once()
            .return_const(Err(UStatus::fail_with_code(UCode::UNAVAILABLE, "unknown")));

        // and a transport
        let mut transport = MockTransport::new();
        // which succeeds to unregister listeners
        transport
            .expect_do_unregister_listener()
            .never()
            .return_const(Ok(()));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();
        // which already has a listener registered
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let handler = MockSubscriptionChangeHandler::new();
        subscriber
            .add_subscription_change_handler(&topic, Arc::new(handler))
            .expect("should be able to add handler");
        assert!(subscriber.has_subscription_change_handler(&topic));

        // WHEN unsubscribing from the topic
        let listener = Arc::new(MockUListener::new());
        let unsubscribe_attempt = subscriber.unsubscribe(&topic, listener).await;

        // THEN the the attempt fails
        assert!(unsubscribe_attempt.is_err_and(|e| {
            matches!(e, RegistrationError::Unknown(_))
                // and the handler is still registered
                && subscriber.has_subscription_change_handler(&topic)
        }));
    }

    #[tokio::test]
    async fn test_unsubscribe_succeeds() {
        // GIVEN a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that succeeds to unsubscribe from topics
        usubscription_client
            .expect_unsubscribe()
            .once()
            .return_const(Ok(()));

        // and a transport
        let mut transport = MockTransport::new();
        // which succeeds to unregister listeners
        transport
            .expect_do_unregister_listener()
            .once()
            .return_const(Ok(()));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();
        // which already has a listener registered
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let handler = MockSubscriptionChangeHandler::new();
        subscriber
            .add_subscription_change_handler(&topic, Arc::new(handler))
            .expect("should be able to add handler");
        assert!(subscriber.has_subscription_change_handler(&topic));

        // WHEN unsubscribing from a topic for which no listener had been registered
        let listener = Arc::new(MockUListener::new());
        let unsubscribe_attempt = subscriber.unsubscribe(&topic, listener.clone()).await;

        // THEN the the attempt succeeds
        assert!(
            unsubscribe_attempt.is_ok_and(|_| !subscriber.has_subscription_change_handler(&topic))
        );
    }

    #[tokio::test]
    async fn test_unsubscribe_succeeds_on_second_attempt() {
        // GIVEN a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that succeeds to unsubscribe from topics
        usubscription_client
            .expect_unsubscribe()
            .times(2)
            .return_const(Ok(()));

        // and a transport
        let mut transport = MockTransport::new();
        let mut seq = Sequence::new();
        // that first fails to unregister a listener
        transport
            .expect_do_unregister_listener()
            .once()
            .in_sequence(&mut seq)
            .return_const(Err(UStatus::fail_with_code(
                UCode::UNAVAILABLE,
                "not connected",
            )));
        // but succeeds on the second attempt
        transport
            .expect_do_unregister_listener()
            .once()
            .in_sequence(&mut seq)
            .return_const(Ok(()));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            new_uri_provider(),
            Arc::new(usubscription_client),
            succeding_notifier(),
        )
        .await
        .unwrap();
        // which already has a listener registered
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let handler = MockSubscriptionChangeHandler::new();
        subscriber
            .add_subscription_change_handler(&topic, Arc::new(handler))
            .expect("should be able to add handler");
        assert!(subscriber.has_subscription_change_handler(&topic));

        // WHEN unsubscribing from a topic for which a listener had been registered before
        let listener = Arc::new(MockUListener::new());
        let unsubscribe_attempt = subscriber.unsubscribe(&topic, listener.clone()).await;

        // THEN the first attempt fails
        assert!(unsubscribe_attempt.is_err_and(|e| matches!(e, RegistrationError::Unknown(_))));

        let unsubscribe_attempt = subscriber.unsubscribe(&topic, listener).await;

        // but the second attempt succeeds
        assert!(unsubscribe_attempt.is_ok_and(|_| {
            // and the handler has been removed
            !subscriber.has_subscription_change_handler(&topic)
        }));
    }

    fn message_with_wrong_type(msg_type: UMessageType) -> UMessage {
        let attributes = UAttributes {
            type_: msg_type.into(),
            ..Default::default()
        };
        UMessage {
            attributes: Some(attributes).into(),
            ..Default::default()
        }
    }

    fn notification_with_wrong_payload() -> UMessage {
        let payload = UPayload::try_from_protobuf(StringValue::new())
            .expect("should have been able to create protobuf");
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            payload_format: payload.payload_format().into(),
            ..Default::default()
        };
        UMessage {
            attributes: Some(attributes).into(),
            payload: Some(payload.payload()),
            ..Default::default()
        }
    }

    fn status_update_without_topic() -> UMessage {
        let status = SubscriptionStatus {
            state: State::SUBSCRIBED.into(),
            ..Default::default()
        };
        let update = Update {
            status: Some(status).into(),
            ..Default::default()
        };
        let payload =
            UPayload::try_from_protobuf(update).expect("should have been able to create protobuf");
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            payload_format: payload.payload_format().into(),
            ..Default::default()
        };

        UMessage {
            attributes: Some(attributes).into(),
            payload: Some(payload.payload()),
            ..Default::default()
        }
    }

    fn status_update_without_status() -> UMessage {
        let update = Update {
            topic: Some(UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap()).into(),
            ..Default::default()
        };
        let payload =
            UPayload::try_from_protobuf(update).expect("should have been able to create protobuf");
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            payload_format: payload.payload_format().into(),
            ..Default::default()
        };

        UMessage {
            attributes: Some(attributes).into(),
            payload: Some(payload.payload()),
            ..Default::default()
        }
    }

    #[test_case::test_case(message_with_wrong_type(UMessageType::UMESSAGE_TYPE_PUBLISH); "Publish messages")]
    #[test_case::test_case(message_with_wrong_type(UMessageType::UMESSAGE_TYPE_REQUEST); "Request messages")]
    #[test_case::test_case(message_with_wrong_type(UMessageType::UMESSAGE_TYPE_RESPONSE); "Response messages")]
    #[test_case::test_case(notification_with_wrong_payload(); "wrong payload")]
    #[test_case::test_case(status_update_without_topic(); "status without topic")]
    #[test_case::test_case(status_update_without_status(); "update without status")]
    #[tokio::test]
    async fn test_subscription_change_listener_ignores(notification: UMessage) {
        let listener = SubscriptionChangeListener::default();

        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let mut handler = MockSubscriptionChangeHandler::new();
        handler.expect_on_subscription_change().never();

        listener
            .add_handler(topic.clone(), Arc::new(handler))
            .expect("should have been able to register listener");
        listener.on_receive(notification).await;
    }

    #[tokio::test]
    async fn test_subscription_change_listener_invokes_handler_for_subscribed_topic() {
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let status = SubscriptionStatus {
            state: State::SUBSCRIBED.into(),
            ..Default::default()
        };
        let update = Update {
            topic: Some(topic.clone()).into(),
            status: Some(status.clone()).into(),
            ..Default::default()
        };
        let payload =
            UPayload::try_from_protobuf(update).expect("should have been able to create protobuf");
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            payload_format: payload.payload_format().into(),
            ..Default::default()
        };

        let notification = UMessage {
            attributes: Some(attributes).into(),
            payload: Some(payload.payload()),
            ..Default::default()
        };

        let expected_topic = topic.clone();
        let mut handler = MockSubscriptionChangeHandler::new();
        handler
            .expect_on_subscription_change()
            .once()
            .withf(move |topic, updated_status| {
                topic == &expected_topic && updated_status == &status
            })
            .return_const(());

        let listener = SubscriptionChangeListener::default();
        listener
            .add_handler(topic, Arc::new(handler))
            .expect("should have been able to register listener");

        listener.on_receive(notification).await;
    }
}
