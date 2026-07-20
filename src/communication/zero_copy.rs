/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
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

//! Zero-copy communication-layer facade.
//!
//! This module exposes the L2 operations that preserve zero-copy semantics:
//! stable/no-zero publish and subscription with the transport's receive lease
//! type delivered directly to the listener. The facade does not adapt or copy
//! received payloads, so selected-wire transports retain their typed
//! [`UWireRx`](crate::UWireRx) decode surface.
//!
//! RPC server and request handling remain at the [`UZeroCopyTransport`] layer
//! because their non-copying shape is tied to the transport receive lease type.
//! Any future convenience that copies out of a receive lease must use a
//! `copying` name and must not claim no-copy behavior.

use std::sync::Arc;

use crate::{
    communication::{CallOptions, PubSubError},
    payload::loan::LoanPayload,
    LocalUriProvider, UFrameMetadata, UMessageBuilder, UZeroCopyTransport, UZeroCopyTransportExt,
};
#[cfg(feature = "zero-copy-transport")]
use crate::{
    payload::{
        stable::{
            InitializedStablePayload, StableContainerPayload, StablePayloadInit,
            StablePayloadInitContext,
        },
        UWireError,
    },
    UZeroCopyUninitTransport, UZeroCopyUninitTransportExt,
};
use crate::{wire::UWirePayload, wire_transport::UHasWire};

/// *Role: the up-L2 subscribe surface over a **selected-wire zero-copy
/// transport** (experimental) — see the [trait map](crate::guide::trait_map).*
///
/// Mirrors the owned-frame [`Subscriber`](crate::communication::owned::Subscriber):
/// the uSubscription service is informed first, and the zero-copy listener is
/// registered only when the service reports the subscription active or
/// pending. Received payloads are delivered as the transport's lease type —
/// for a selected-wire transport that is [`UWireRx`](crate::UWireRx), whose
/// [`decode_payload`](crate::UWireRx::decode_payload) reads the typed value in
/// place.
#[cfg(feature = "usubscription")]
pub struct Subscriber<T>
where
    T: UZeroCopyTransport + ?Sized,
{
    transport: Arc<T>,
    usubscription: Arc<dyn crate::core::usubscription::USubscription>,
}

#[cfg(feature = "usubscription")]
impl<T> core::fmt::Debug for Subscriber<T>
where
    T: UZeroCopyTransport + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Subscriber").finish_non_exhaustive()
    }
}

#[cfg(feature = "usubscription")]
impl<T> Subscriber<T>
where
    T: UZeroCopyTransport + ?Sized,
{
    /// Creates a subscriber over a zero-copy transport and a uSubscription
    /// service client.
    #[must_use]
    pub fn new(
        transport: Arc<T>,
        usubscription: Arc<dyn crate::core::usubscription::USubscription>,
    ) -> Self {
        Self {
            transport,
            usubscription,
        }
    }

    /// Subscribes to a topic: informs the uSubscription service first, then
    /// registers the zero-copy listener on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the topic is not a valid subscription filter, if
    /// the uSubscription service rejects the subscription or reports a state
    /// other than active/pending, or if listener registration fails.
    pub async fn subscribe(
        &self,
        topic: &crate::UUri,
        listener: Arc<dyn crate::UZeroCopyListener<T::Rx>>,
    ) -> Result<(), crate::communication::RegistrationError> {
        use crate::communication::{RegistrationError, SubscriptionStatus};
        crate::communication::validate_listener_topic(topic)?;
        let state = self
            .usubscription
            .subscribe(topic, None, None)
            .await
            .map_err(|status| RegistrationError::Unknown(Box::new(status)))?;
        if state != SubscriptionStatus::Subscribed && state != SubscriptionStatus::SubscribePending
        {
            return Err(RegistrationError::Unknown(Box::new(
                crate::UStatus::fail_with_code(
                    crate::UCode::FailedPrecondition,
                    format!("uSubscription service returned {state:?}"),
                ),
            )));
        }
        self.transport
            .register_zero_copy_listener(topic, None, listener)
            .await
            .map_err(|status| RegistrationError::Unknown(Box::new(status)))
    }

    /// Unsubscribes from a topic: informs the uSubscription service, then
    /// unregisters the zero-copy listener.
    ///
    /// # Errors
    ///
    /// Returns an error if the topic is not a valid subscription filter, or if
    /// the service call or listener unregistration fails.
    pub async fn unsubscribe(
        &self,
        topic: &crate::UUri,
        listener: Arc<dyn crate::UZeroCopyListener<T::Rx>>,
    ) -> Result<(), crate::communication::RegistrationError> {
        use crate::communication::RegistrationError;
        crate::communication::validate_listener_topic(topic)?;
        self.usubscription
            .unsubscribe(topic)
            .await
            .map_err(|status| RegistrationError::Unknown(Box::new(status)))?;
        self.transport
            .unregister_zero_copy_listener(topic, None, listener)
            .await
            .map_err(|status| RegistrationError::Unknown(Box::new(status)))
    }
}

/// *Role: the up-L2 publish and subscribe surface over a **selected-wire
/// zero-copy transport** — typed payloads use transport loans and receive
/// leases directly, with role ergonomics. See the
/// [guide](crate::guide::applications::communication).*
///
/// Front door for zero-copy communication-layer clients.
pub struct Endpoint<T, P>
where
    T: UZeroCopyTransport + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<T>,
    uri_provider: Arc<P>,
}

impl<T, P> core::fmt::Debug for Endpoint<T, P>
where
    T: UZeroCopyTransport + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Endpoint").finish_non_exhaustive()
    }
}

impl<T, P> Endpoint<T, P>
where
    T: UZeroCopyTransport + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    /// Creates a zero-copy communication endpoint.
    #[must_use]
    pub fn new(transport: Arc<T>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }

    /// Creates the subscribe role over this endpoint's transport, using the
    /// given uSubscription service client.
    #[cfg(feature = "usubscription")]
    #[must_use]
    pub fn subscriber(
        &self,
        usubscription: Arc<dyn crate::core::usubscription::USubscription>,
    ) -> Subscriber<T> {
        Subscriber::new(self.transport.clone(), usubscription)
    }

    /// Creates the publish role over this endpoint's transport and identity.
    #[must_use]
    pub fn publisher(&self) -> Publisher<T, P> {
        Publisher::new(self.transport.clone(), self.uri_provider.clone())
    }
}

/// Publisher implemented over a selected-wire zero-copy transport.
pub struct Publisher<T, P>
where
    T: UZeroCopyTransport + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<T>,
    uri_provider: Arc<P>,
}

impl<T, P> core::fmt::Debug for Publisher<T, P>
where
    T: UZeroCopyTransport + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Publisher").finish_non_exhaustive()
    }
}

impl<T, P> Publisher<T, P>
where
    T: UZeroCopyTransport + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    /// Creates a publisher over a zero-copy transport.
    #[must_use]
    pub fn new(transport: Arc<T>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }

    fn build_metadata(
        &self,
        resource_id: u16,
        call_options: CallOptions,
    ) -> Result<UFrameMetadata, PubSubError> {
        let mut builder = UMessageBuilder::publish(self.uri_provider.get_resource_uri(resource_id));
        builder.with_ttl(call_options.ttl());
        if let Some(message_id) = call_options.message_id() {
            builder.with_message_id(message_id.clone());
        }
        if let Some(priority) = call_options.priority() {
            builder.with_priority(priority);
        }
        let message = builder.build().map_err(|error| {
            PubSubError::InvalidArgument(format!(
                "failed to create zero-copy Publish message metadata from parameters: {error}"
            ))
        })?;
        crate::frame::metadata::try_project_umessage_to_frame_metadata(&message).map_err(|error| {
            PubSubError::InvalidArgument(format!(
                "failed to create zero-copy Publish frame metadata from parameters: {error}"
            ))
        })
    }
}

impl<T, P> Publisher<T, P>
where
    T: UZeroCopyTransport + UHasWire + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    /// Publishes an initialized stable payload using the transport endpoint's selected wire.
    pub async fn publish_stable<Payload>(
        &self,
        resource_id: u16,
        call_options: CallOptions,
        init: impl for<'payload> FnOnce(&'payload mut Payload) + Send,
    ) -> Result<(), PubSubError>
    where
        T::Wire: UWirePayload<Payload>,
        <T::Wire as UWirePayload<Payload>>::Codec: LoanPayload<Payload> + Send + Sync,
    {
        let metadata = self.build_metadata(resource_id, call_options)?;
        self.transport
            .send_loaned_payload::<Payload>(metadata, init)
            .await
            .map_err(Box::from)
            .map_err(PubSubError::PublishError)
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<T, P> Publisher<T, P>
where
    T: UZeroCopyUninitTransport + UHasWire + ?Sized,
    P: LocalUriProvider + ?Sized,
{
    /// Publishes a stable payload by initializing it directly in uninitialized transport storage.
    ///
    pub async fn publish_uninit_stable<Payload>(
        &self,
        resource_id: u16,
        call_options: CallOptions,
        init: impl for<'payload> FnOnce(
                StablePayloadInitContext<'payload, Payload>,
            )
                -> Result<InitializedStablePayload<'payload, Payload>, UWireError>
            + Send,
    ) -> Result<(), PubSubError>
    where
        T::Wire: UWirePayload<Payload, Codec = StableContainerPayload<Payload>>,
        Payload: StablePayloadInit + Send,
    {
        let metadata = self.build_metadata(resource_id, call_options)?;
        self.transport
            .send_uninit_stable_payload::<Payload>(metadata, init)
            .await
            .map_err(Box::from)
            .map_err(PubSubError::PublishError)
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::{
        test_support::StableTestBytes as StableBytes, InMemoryZeroCopyTransport,
        StableContainerWireFormat, StaticUriProvider, UCode, UFrameView,
        ULoanedContiguousZeroCopyRxFrame, UStatus, UTxLoanSpec, UVecRxLease, UVecTxBuffer,
        UVecUninitTxBuffer, UZeroCopyTransportImpl, UZeroCopyUninitTransportImpl,
    };

    struct SelectedStableTransport {
        inner: InMemoryZeroCopyTransport,
        wire: StableContainerWireFormat,
    }

    impl SelectedStableTransport {
        fn new() -> Self {
            Self {
                inner: InMemoryZeroCopyTransport::default(),
                wire: StableContainerWireFormat,
            }
        }

        fn sent_frames(&self) -> Vec<UVecRxLease> {
            self.inner.sent_frames()
        }
    }

    impl UHasWire for SelectedStableTransport {
        type Wire = StableContainerWireFormat;

        fn wire(&self) -> &Self::Wire {
            &self.wire
        }
    }

    #[async_trait]
    impl UZeroCopyTransportImpl for SelectedStableTransport {
        type Tx = UVecTxBuffer;
        type Rx = UVecRxLease;

        async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
            UZeroCopyTransportImpl::loan_validated_tx(&self.inner, spec).await
        }

        async fn send_validated_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
            UZeroCopyTransportImpl::send_validated_zero_copy(&self.inner, buffer).await
        }
    }

    #[async_trait]
    impl UZeroCopyUninitTransportImpl for SelectedStableTransport {
        type UninitTx = UVecUninitTxBuffer;

        async fn loan_validated_uninit_tx(
            &self,
            spec: UTxLoanSpec,
        ) -> Result<Self::UninitTx, UStatus> {
            UZeroCopyUninitTransportImpl::loan_validated_uninit_tx(&self.inner, spec).await
        }
    }

    fn uri_provider() -> Arc<StaticUriProvider> {
        Arc::new(StaticUriProvider::new("", 0x0005, 0x02).expect("uri provider"))
    }

    #[tokio::test]
    async fn publish_stable_sends_initialized_selected_wire_payload() {
        let transport = Arc::new(SelectedStableTransport::new());
        let publisher = Publisher::new(transport.clone(), uri_provider());

        publisher
            .publish_stable::<StableBytes>(
                0x9A00,
                CallOptions::for_publish(None, None, None),
                |payload| payload.bytes.copy_from_slice(b"init"),
            )
            .await
            .expect("stable publish succeeds");

        let frames = transport.sent_frames();
        assert_eq!(frames.len(), 1);
        let frame = frames.first().expect("one sent frame");
        assert_eq!(frame.payload_len(), std::mem::size_of::<StableBytes>());
        assert_eq!(
            frame.borrow_stable_payload::<StableBytes>().unwrap(),
            &StableBytes { bytes: *b"init" }
        );
    }

    #[tokio::test]
    async fn publish_uninit_stable_sends_no_zero_selected_wire_payload() {
        let transport = Arc::new(SelectedStableTransport::new());
        let publisher = Endpoint::new(transport.clone(), uri_provider()).publisher();

        publisher
            .publish_uninit_stable::<StableBytes>(
                0x9A00,
                CallOptions::for_publish(None, None, None),
                |context| context.into_init().bytes_from_array(b"zero").finish(),
            )
            .await
            .expect("uninit stable publish succeeds");

        let frames = transport.sent_frames();
        assert_eq!(frames.len(), 1);
        let frame = frames.first().expect("one sent frame");
        assert_eq!(
            frame.borrow_stable_payload::<StableBytes>().unwrap(),
            &StableBytes { bytes: *b"zero" }
        );
    }

    #[tokio::test]
    async fn publish_stable_rejects_invalid_topic() {
        let transport = Arc::new(SelectedStableTransport::new());
        let publisher = Publisher::new(transport.clone(), uri_provider());

        let result = publisher
            .publish_stable::<StableBytes>(
                0x1000,
                CallOptions::for_publish(None, None, None),
                |payload| payload.bytes.copy_from_slice(b"drop"),
            )
            .await;

        assert!(matches!(result, Err(PubSubError::InvalidArgument(_))));
        assert!(transport.sent_frames().is_empty());
    }

    struct FailingStableTransport {
        wire: StableContainerWireFormat,
    }

    impl UHasWire for FailingStableTransport {
        type Wire = StableContainerWireFormat;

        fn wire(&self) -> &Self::Wire {
            &self.wire
        }
    }

    #[async_trait]
    impl UZeroCopyTransportImpl for FailingStableTransport {
        type Tx = UVecTxBuffer;
        type Rx = UVecRxLease;

        async fn loan_validated_tx(&self, _spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
            Err(UStatus::fail_with_code(
                UCode::Unavailable,
                "transport unavailable",
            ))
        }

        async fn send_validated_zero_copy(&self, _buffer: Self::Tx) -> Result<(), UStatus> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn publish_stable_maps_transport_error() {
        let publisher = Publisher::new(
            Arc::new(FailingStableTransport {
                wire: StableContainerWireFormat,
            }),
            uri_provider(),
        );

        let result = publisher
            .publish_stable::<StableBytes>(
                0x9A00,
                CallOptions::for_publish(None, None, None),
                |payload| payload.bytes.copy_from_slice(b"fail"),
            )
            .await;

        assert!(matches!(result, Err(PubSubError::PublishError(_))));
    }
}

#[cfg(all(test, feature = "usubscription"))]
mod subscriber_tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::Subscriber;
    use crate::communication::{RegistrationError, SubscriptionStatus};
    use crate::core::usubscription::USubscription;
    use crate::{
        InMemoryZeroCopyTransport, UStatus, UTxLoanSpec, UUri, UVecRxLease, UVecTxBuffer,
        UZeroCopyListener, UZeroCopyTransportImpl,
    };

    struct RecordingZeroCopyTransport {
        inner: InMemoryZeroCopyTransport,
        lifecycle: Arc<Mutex<Vec<&'static str>>>,
    }

    impl RecordingZeroCopyTransport {
        fn new(lifecycle: Arc<Mutex<Vec<&'static str>>>) -> Self {
            Self {
                inner: InMemoryZeroCopyTransport::default(),
                lifecycle,
            }
        }

        fn record(&self, event: &'static str) {
            self.lifecycle
                .lock()
                .expect("lifecycle lock poisoned")
                .push(event);
        }
    }

    #[async_trait]
    impl UZeroCopyTransportImpl for RecordingZeroCopyTransport {
        type Tx = UVecTxBuffer;
        type Rx = UVecRxLease;

        async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
            UZeroCopyTransportImpl::loan_validated_tx(&self.inner, spec).await
        }

        async fn send_validated_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
            UZeroCopyTransportImpl::send_validated_zero_copy(&self.inner, buffer).await
        }

        async fn register_validated_zero_copy_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
        ) -> Result<(), UStatus> {
            self.record("register");
            UZeroCopyTransportImpl::register_validated_zero_copy_listener(
                &self.inner,
                source_filter,
                sink_filter,
                listener,
            )
            .await
        }

        async fn unregister_validated_zero_copy_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
        ) -> Result<(), UStatus> {
            self.record("unregister");
            UZeroCopyTransportImpl::unregister_validated_zero_copy_listener(
                &self.inner,
                source_filter,
                sink_filter,
                listener,
            )
            .await
        }
    }

    #[derive(Default)]
    struct StubUSubscription {
        lifecycle: Arc<Mutex<Vec<&'static str>>>,
        decline: bool,
    }

    #[async_trait]
    impl USubscription for StubUSubscription {
        async fn subscribe(
            &self,
            _topic: &UUri,
            _expiration: Option<u64>,
            _min_sample_period: Option<u32>,
        ) -> Result<SubscriptionStatus, UStatus> {
            self.lifecycle
                .lock()
                .expect("lifecycle lock poisoned")
                .push("subscribe");
            if self.decline {
                Ok(SubscriptionStatus::Unsubscribed)
            } else {
                Ok(SubscriptionStatus::Subscribed)
            }
        }
        async fn unsubscribe(&self, _topic: &UUri) -> Result<(), UStatus> {
            self.lifecycle
                .lock()
                .expect("lifecycle lock poisoned")
                .push("unsubscribe");
            Ok(())
        }
        async fn fetch_subscriptions_by_topic(
            &self,
            _topic: &UUri,
        ) -> Result<Vec<crate::core::usubscription::SubscriptionInfo>, UStatus> {
            Ok(Vec::new())
        }
        async fn fetch_subscriptions_by_subscriber(
            &self,
            _subscriber: &UUri,
        ) -> Result<Vec<crate::core::usubscription::SubscriptionInfo>, UStatus> {
            Ok(Vec::new())
        }
        async fn register_for_notifications(&self, _topic: &UUri) -> Result<(), UStatus> {
            Ok(())
        }
        async fn unregister_for_notifications(&self, _topic: &UUri) -> Result<(), UStatus> {
            Ok(())
        }
        async fn fetch_subscribers(&self, _topic: &UUri) -> Result<Vec<UUri>, UStatus> {
            Ok(Vec::new())
        }
        async fn reset(
            &self,
            _reason: crate::core::usubscription::ResetReason,
            _message: Option<String>,
        ) -> Result<(), UStatus> {
            Ok(())
        }
    }

    struct CountingListener(Mutex<usize>);

    #[async_trait]
    impl<Rx: crate::UZeroCopyRxLease + Send + 'static> UZeroCopyListener<Rx> for CountingListener {
        async fn on_receive_zero_copy(&self, _frame: Rx) {
            *self.0.lock().expect("count lock poisoned") += 1;
        }
    }

    fn topic() -> UUri {
        UUri::try_from_parts("demo", 0x1_0001, 1, 0x8001).expect("valid topic URI")
    }

    #[tokio::test]
    async fn subscribe_consults_the_service_before_registering() {
        let lifecycle = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Subscriber::new(
            Arc::new(RecordingZeroCopyTransport::new(lifecycle.clone())),
            Arc::new(StubUSubscription {
                lifecycle: lifecycle.clone(),
                decline: false,
            }),
        );

        subscriber
            .subscribe(&topic(), Arc::new(CountingListener(Mutex::new(0))))
            .await
            .expect("subscribe succeeds when the service reports Subscribed");

        assert_eq!(
            *lifecycle.lock().expect("lifecycle lock poisoned"),
            ["subscribe", "register"]
        );
    }

    #[tokio::test]
    async fn subscribe_fails_and_registers_nothing_when_the_service_declines() {
        let lifecycle = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Subscriber::new(
            Arc::new(RecordingZeroCopyTransport::new(lifecycle.clone())),
            Arc::new(StubUSubscription {
                lifecycle: lifecycle.clone(),
                decline: true,
            }),
        );

        subscriber
            .subscribe(&topic(), Arc::new(CountingListener(Mutex::new(0))))
            .await
            .expect_err("a declined subscription must not register a listener");

        assert_eq!(
            *lifecycle.lock().expect("lifecycle lock poisoned"),
            ["subscribe"]
        );
    }

    #[tokio::test]
    async fn unsubscribe_informs_the_service_and_unregisters() {
        let lifecycle = Arc::new(Mutex::new(Vec::new()));
        let listener = Arc::new(CountingListener(Mutex::new(0)));
        let subscriber = Subscriber::new(
            Arc::new(RecordingZeroCopyTransport::new(lifecycle.clone())),
            Arc::new(StubUSubscription {
                lifecycle: lifecycle.clone(),
                decline: false,
            }),
        );

        subscriber
            .subscribe(&topic(), listener.clone())
            .await
            .expect("subscribed");
        subscriber
            .unsubscribe(&topic(), listener)
            .await
            .expect("unsubscribe succeeds after subscribe");

        assert_eq!(
            *lifecycle.lock().expect("lifecycle lock poisoned"),
            ["subscribe", "register", "unsubscribe", "unregister"]
        );
    }

    #[tokio::test]
    async fn unsubscribe_rejects_wildcards_before_contacting_the_service() {
        let lifecycle = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Subscriber::new(
            Arc::new(RecordingZeroCopyTransport::new(lifecycle.clone())),
            Arc::new(StubUSubscription {
                lifecycle: lifecycle.clone(),
                decline: false,
            }),
        );
        let invalid_topic =
            UUri::try_from("up://my-vin/A15B/1/FFFF").expect("valid wildcard subscription filter");

        let result = subscriber
            .unsubscribe(&invalid_topic, Arc::new(CountingListener(Mutex::new(0))))
            .await;

        assert!(matches!(result, Err(RegistrationError::InvalidFilter(_))));
        assert!(lifecycle
            .lock()
            .expect("lifecycle lock poisoned")
            .is_empty());
    }
}
