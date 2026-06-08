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

// [impl->dsn~communication-layer-impl-default~1]

use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Deref,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    communication::{
        Notifier, RegistrationError, Subscriber, SubscriptionChangeHandler, SubscriptionStatus,
    },
    core::usubscription::{self, SubscriptionInfo, USubscription},
    UListener, UMessage, UStatus, UTransport, UUri,
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
            return Err(RegistrationError::Unknown(Box::from(
                UStatus::fail_with_code(
                    crate::UCode::Internal,
                    "failed to acquire write lock for handler map",
                ),
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
                RegistrationError::Unknown(Box::from(UStatus::fail_with_code(
                    crate::UCode::Internal,
                    "failed to acquire write lock for handler map",
                )))
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
                RegistrationError::Unknown(Box::from(UStatus::fail_with_code(
                    crate::UCode::Internal,
                    "failed to acquire write lock for handler map",
                )))
            })
            .map(|mut handlers| {
                handlers.clear();
            })
    }

    #[cfg(test)]
    fn has_handler(&self, topic: &UUri) -> bool {
        self.subscription_change_handlers
            .read()
            .is_ok_and(|handlers| handlers.contains_key(topic))
    }
}

#[async_trait]
impl UListener for SubscriptionChangeListener {
    async fn on_receive(&self, msg: UMessage) {
        if !msg.is_notification() {
            return;
        }
        let Ok(subscription_update) =
            msg.extract_protobuf::<crate::up_core_api::usubscription::Update>()
        else {
            debug!("ignoring notification that does not contain subscription update");
            return;
        };
        let Ok(subscription_info) = SubscriptionInfo::try_from(&subscription_update) else {
            debug!("ignoring notification that does not contain valid subscription update");
            return;
        };
        let topic = subscription_info.topic().to_owned();

        let Ok(handlers) = self.subscription_change_handlers.read() else {
            return;
        };
        if let Some(handler) = handlers.get(&topic) {
            handler.on_subscription_change(topic, subscription_info.status().to_owned());
        }
    }
}

/// A [`Subscriber`] which keeps all information about registered subscription change handlers in memory.
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
pub struct InMemorySubscriber<T, S, N> {
    transport: Arc<T>,
    usubscription: Arc<S>,
    notifier: Arc<N>,
    subscription_change_listener: Arc<SubscriptionChangeListener>,
}

#[cfg(feature = "up-l2-notifier")]
impl<T: UTransport + 'static, P: crate::LocalUriProvider + 'static>
    InMemorySubscriber<
        T,
        crate::core::usubscription::RpcClientUSubscription,
        crate::communication::SimpleNotifier<T, P>,
    >
{
    /// Creates a new Subscriber for a given transport.
    ///
    /// The subscriber keeps track of subscription change handlers in memory only.
    /// This function uses the given transport to create an [`crate::core::usubscription::RpcClientUSubscription`]
    /// and a [`crate::communication::SimpleNotifier`] and then delegate to [`Self::for_clients`]
    /// to create the Subscriber.
    ///
    /// # Errors
    ///
    /// Returns an error if the Notifier cannot register a listener for notifications from the USubscription service.
    pub async fn new(transport: Arc<T>, uri_provider: Arc<P>) -> Result<Self, RegistrationError> {
        let rpc_client =
            crate::communication::InMemoryRpcClient::new(transport.clone(), uri_provider.clone())
                .await
                .map(Arc::new)?;
        let usubscription_client = Arc::new(
            crate::core::usubscription::RpcClientUSubscription::new(rpc_client),
        );
        let notifier = Arc::new(crate::communication::SimpleNotifier::new(
            transport.clone(),
            uri_provider.clone(),
        ));
        Self::for_clients(transport, usubscription_client, notifier).await
    }
}

impl<T: UTransport, S: USubscription, N: Notifier> InMemorySubscriber<T, S, N> {
    /// Creates a new Subscriber for given clients.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for registering the event listeners for subscribed topics.
    /// * `usubscription` - The client to use for interacting with the (local) USubscription service.
    /// * `notifier` - The client to use for registering the listener for subscription updates from USubscription.
    ///
    /// # Errors
    ///
    /// Returns an error if the Notifier cannot register a listener for notifications from the USubscription service.
    pub async fn for_clients(
        transport: Arc<T>,
        usubscription: Arc<S>,
        notifier: Arc<N>,
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
            usubscription,
            notifier,
            subscription_change_listener,
        })
    }

    /// Stops this client.
    ///
    /// Clears all internal state and deregisters the listener for subscription updates from the USubscription service.
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

    async fn invoke_subscribe(
        &self,
        topic: &UUri,
        subscription_change_handler: Option<Arc<dyn SubscriptionChangeHandler>>,
    ) -> Result<SubscriptionStatus, RegistrationError> {
        match self.usubscription.subscribe(topic, None, None).await {
            Ok(state)
                if state == SubscriptionStatus::Subscribed
                    || state == SubscriptionStatus::SubscribePending =>
            {
                if let Some(handler) = subscription_change_handler.clone() {
                    self.subscription_change_listener
                        .add_handler(topic.to_owned(), handler)?;
                }
                Ok(state)
            }
            Ok(_) => {
                debug!(topic = %topic, "failed to subscribe to topic");
                Err(RegistrationError::Unknown(Box::from(
                    UStatus::fail_with_code(
                        crate::UCode::FailedPrecondition,
                        "failed to subscribe to topic",
                    ),
                )))
            }
            Err(e) => {
                info!(topic = %topic, "error invoking USubscription service: {}", e);
                Err(RegistrationError::Unknown(Box::from(
                    UStatus::fail_with_code(
                        crate::UCode::Internal,
                        "failed to invoke USubscription service",
                    ),
                )))
            }
        }
    }

    async fn invoke_unsubscribe(&self, topic: &UUri) -> Result<(), RegistrationError> {
        self.usubscription
            .unsubscribe(topic)
            .await
            .map(|_| {
                let _ = self.subscription_change_listener.remove_handler(topic);
            })
            .map_err(|e| {
                info!(topic = %topic, "error invoking USubscription service: {}", e);
                RegistrationError::Unknown(Box::from(UStatus::fail_with_code(
                    crate::UCode::Internal,
                    "failed to invoke USubscription service",
                )))
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
impl<T: UTransport, U: USubscription, N: Notifier> Subscriber for InMemorySubscriber<T, U, N> {
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
            // have successfully (logically) subscribed to the topic via the USubscription service
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
            // have successfully (logically) unsubscribed from the topic via the USubscription service
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

    // [utest->dsn~communication-layer-impl-default~1]

    use super::*;

    use mockall::Sequence;
    use protobuf::well_known_types::wrappers::StringValue;
    use usubscription::MockUSubscription;

    use crate::{
        communication::{notification::MockNotifier, pubsub::MockSubscriptionChangeHandler},
        up_core_api::usubscription::Update,
        utransport::{MockTransport, MockUListener},
        UCode, UMessageBuilder, UMessageType, UStatus, UUri, UUID,
    };

    fn succeeding_notifier() -> Arc<MockNotifier> {
        let mut notifier = MockNotifier::new();
        notifier
            .expect_start_listening()
            .once()
            .return_const(Ok(()));
        Arc::new(notifier)
    }

    #[tokio::test]
    async fn test_subscriber_creation_fails_when_notifier_fails_to_register_listener() {
        // GIVEN a Notifier
        let mut notifier = MockNotifier::new();
        // that is not connected to its transport
        notifier
            .expect_start_listening()
            .once()
            .return_const(Err(RegistrationError::Unknown(Box::from(
                UStatus::fail_with_code(UCode::Unavailable, "not available"),
            ))));

        // WHEN trying to create a Subscriber for this Notifier
        let creation_attempt = InMemorySubscriber::for_clients(
            Arc::new(MockTransport::new()),
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
            .returning(|_, _, _| Err(UStatus::fail_with_code(UCode::Unavailable, "not connected")));
        usubscription_client
            .expect_subscribe()
            .once()
            .in_sequence(&mut seq)
            .return_const(Ok(SubscriptionStatus::Unsubscribed));
        // and a transport
        let mut transport = MockTransport::new();
        transport.expect_do_register_listener().never();

        // and a Subscriber using that USubscription client
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
    }

    #[tokio::test]
    async fn test_repeated_subscribe_fails_for_different_subscription_change_handlers() {
        // GIVEN  a USubscription client
        let mut usubscription_client = MockUSubscription::new();
        // that succeeds to subscribe to topics
        usubscription_client
            .expect_subscribe()
            .times(2)
            .return_const(Ok(SubscriptionStatus::Subscribed));

        // and a transport
        let mut transport = MockTransport::new();
        // that fails to register a listener
        transport
            .expect_do_register_listener()
            .once()
            .return_const(Err(UStatus::fail_with_code(
                UCode::Unavailable,
                "not connected",
            )));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
            .return_const(Ok(SubscriptionStatus::Subscribed));

        // and a transport
        let mut transport = MockTransport::new();
        let mut seq = Sequence::new();
        // that first fails to register a listener
        transport
            .expect_do_register_listener()
            .once()
            .in_sequence(&mut seq)
            .return_const(Err(UStatus::fail_with_code(
                UCode::Unavailable,
                "not connected",
            )));
        // but succeeds on the second attempt
        transport
            .expect_do_register_listener()
            .once()
            .in_sequence(&mut seq)
            .returning(move |_source_filter, _sink_filter, listener| {
                captured_listener_tx.send(listener).map_err(|_e| {
                    UStatus::fail_with_code(UCode::Internal, "cannot capture listener")
                })
            });

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
                UCode::NotFound,
                "no such listener",
            )));

        // and a Subscriber using that USubscription client, Notifier and transport
        let subscriber = InMemorySubscriber::for_clients(
            Arc::new(transport),
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
            .return_const(Err(UStatus::fail_with_code(UCode::Unavailable, "unknown")));

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
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
                UCode::Unavailable,
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
            Arc::new(usubscription_client),
            succeeding_notifier(),
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
        match msg_type {
            UMessageType::Publish => UMessageBuilder::publish(
                UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100)
                    .expect("should have been able to create URI"),
            )
            .build()
            .expect("should have been able to create publish message"),
            UMessageType::Notification => UMessageBuilder::notification(
                UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100)
                    .expect("should have been able to create origin URI"),
                UUri::try_from_parts("other", 0x1a9b, 0x01, 0x0000)
                    .expect("should have been able to create destination URI"),
            )
            .build()
            .expect("should have been able to create notification message"),
            UMessageType::Request => UMessageBuilder::request(
                UUri::try_from_parts("other", 0x1a9a, 0x01, 0x0100)
                    .expect("should have been able to create method-to-invoke URI"),
                UUri::try_from_parts("other", 0x1a9b, 0x01, 0x0000)
                    .expect("should have been able to create reply-to-address URI"),
                5000,
            )
            .build()
            .expect("should have been able to create request message"),
            UMessageType::Response => UMessageBuilder::response(
                UUri::try_from_parts("other", 0x1a9b, 0x01, 0x0000)
                    .expect("should have been able to create reply-to-address URI"),
                UUID::build(),
                UUri::try_from_parts("other", 0x1a9a, 0x01, 0x0100)
                    .expect("should have been able to create method-to-invoke URI"),
            )
            .build()
            .expect("should have been able to create response message"),
        }
    }

    fn notification_with_wrong_payload() -> UMessage {
        UMessageBuilder::notification(
            UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100)
                .expect("should have been able to create origin URI"),
            UUri::try_from_parts("other", 0x1a9b, 0x01, 0x0000)
                .expect("should have been able to create destination URI"),
        )
        .build_with_protobuf_payload(&StringValue::new())
        .expect("should have been able to create notification with payload")
    }

    fn status_update_without_topic() -> UMessage {
        let status = crate::up_core_api::usubscription::SubscriptionStatus {
            state: crate::up_core_api::usubscription::subscription_status::State::SUBSCRIBED.into(),
            ..Default::default()
        };
        let update = Update {
            status: Some(status).into(),
            ..Default::default()
        };
        UMessageBuilder::notification(
            UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100)
                .expect("should have been able to create origin URI"),
            UUri::try_from_parts("other", 0x1a9b, 0x01, 0x0000)
                .expect("should have been able to create destination URI"),
        )
        .build_with_protobuf_payload(&update)
        .expect("should have been able to create notification with payload")
    }

    fn status_update_without_status() -> UMessage {
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100)
            .expect("should have been able to create topic URI");
        let proto_topic = crate::up_core_api::uri::UUri::from(&topic);
        let update = Update {
            topic: Some(proto_topic).into(),
            ..Default::default()
        };
        UMessageBuilder::notification(
            UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100)
                .expect("should have been able to create origin URI"),
            UUri::try_from_parts("other", 0x1a9b, 0x01, 0x0000)
                .expect("should have been able to create destination URI"),
        )
        .build_with_protobuf_payload(&update)
        .expect("should have been able to create notification with payload")
    }

    #[test_case::test_case(message_with_wrong_type(UMessageType::Publish); "Publish messages")]
    #[test_case::test_case(message_with_wrong_type(UMessageType::Request); "Request messages")]
    #[test_case::test_case(message_with_wrong_type(UMessageType::Response); "Response messages")]
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
        let subscriber = UUri::try_from_parts("local", 0x2000, 0x01, 0x0000)
            .expect("should have been able to create subscriber URI");
        let topic = UUri::try_from_parts("other", 0x1a9a, 0x01, 0x8100).unwrap();
        let status_proto = crate::up_core_api::usubscription::SubscriptionStatus {
            state: crate::up_core_api::usubscription::subscription_status::State::SUBSCRIBED.into(),
            ..Default::default()
        };
        let update_proto = Update {
            topic: Some((&topic).into()).into(),
            subscriber: Some(crate::up_core_api::usubscription::SubscriberInfo {
                uri: Some((&subscriber).into()).into(),
                ..Default::default()
            })
            .into(),
            status: Some(status_proto.clone()).into(),
            ..Default::default()
        };
        let subscription_change_notification = UMessageBuilder::notification(
            UUri::try_from_parts("local", 0x0000, 0x01, 0x8000)
                .expect("should have been able to create uSubscription service URI"),
            subscriber.clone(),
        )
        .build_with_protobuf_payload(&update_proto)
        .expect("should have been able to create notification with payload");

        let expected_topic = topic.clone();
        let mut handler = MockSubscriptionChangeHandler::new();
        handler
            .expect_on_subscription_change()
            .once()
            .withf(move |topic, updated_status| {
                topic == &expected_topic
                    && *updated_status
                        == SubscriptionStatus::try_from(&status_proto)
                            .expect("should have been able to convert status proto")
            })
            .return_const(());

        let listener = SubscriptionChangeListener::default();
        listener
            .add_handler(topic, Arc::new(handler))
            .expect("should have been able to register listener");

        listener.on_receive(subscription_change_notification).await;
    }
}
