/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Stable selected-wire communication roles over zero-copy transport loans.
//!
//! Send paths construct [`UFrameMetadata`] directly and initialize application
//! payloads in transport-owned storage. Receive paths expose the selected-wire
//! lease so callers can borrow validated stable payloads without copying them.

use core::time::Duration;
use std::sync::Arc;

use crate::communication::{
    CallOptions, NotificationError, PubSubError, RegistrationError, ServiceInvocationError,
};
use crate::frame::metadata::UFrameMetadataBuilder;
use crate::{
    FramePriority, InitializedStablePayload, LocalUriProvider, NativePayloadIdentity,
    NativePrefixFrameMetadataCodec, StableContainerPayload, StableContainerWireFormat,
    StablePayload, StablePayloadInit, UFrameMetadata, UFrameView, USelectedWireStablePayloadInit,
    UUri, UUID,
};
use crate::{
    UHasWire, UWireRx, UWireTransport, UZeroCopyListener, UZeroCopyTransportCore,
    UZeroCopyTransportImpl, UZeroCopyUninitTransportCore,
};

/// Canonical stable-container transport used by the zero-copy L2 roles.
pub type StableTransport<TCore> =
    UWireTransport<TCore, StableContainerWireFormat, NativePrefixFrameMetadataCodec>;

/// Selected-wire receive lease exposed by the zero-copy L2 roles.
pub type StableRx<TCore> = UWireRx<
    <TCore as UZeroCopyTransportCore>::Rx,
    StableContainerWireFormat,
    NativePrefixFrameMetadataCodec,
>;

fn payload_identity<T: StablePayload>(
    transport: &impl UHasWire,
) -> Result<NativePayloadIdentity, crate::UWireError> {
    let profile = transport
        .native_profile()
        .ok_or(crate::NativeProfileError::InvalidProfile(
            "native communication role requires a configured route agreement",
        ))?;
    StableContainerPayload::<T>::identity(profile)
}

fn apply_options(
    mut builder: UFrameMetadataBuilder,
    call_options: CallOptions,
    include_token: bool,
) -> UFrameMetadataBuilder {
    let (ttl, message_id, token, priority) = call_options.into_parts();
    if ttl != 0 {
        builder = builder.with_ttl(Duration::from_millis(u64::from(ttl)));
    }
    if let Some(message_id) = message_id {
        builder = builder.with_id(message_id);
    }
    if include_token {
        if let Some(token) = token {
            builder = builder.with_token(token);
        }
    }
    if let Some(priority) = priority {
        builder = builder.with_priority(FramePriority::from_legacy_priority(priority));
    }
    builder
}

fn listener_source_wildcard() -> UUri {
    UUri::try_from_parts("*", u32::MAX, u8::MAX, u16::MAX)
        .expect("valid zero-copy listener wildcard")
}

/// Front door for stable selected-wire zero-copy communication roles.
pub struct Endpoint<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<StableTransport<TCore>>,
    uri_provider: Arc<P>,
}

impl<TCore, P> core::fmt::Debug for Endpoint<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("Endpoint").finish_non_exhaustive()
    }
}

impl<TCore, P> Endpoint<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Creates an endpoint over a stable selected-wire transport.
    #[must_use]
    pub fn new(transport: Arc<StableTransport<TCore>>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }

    /// Creates the stable zero-copy publish role.
    #[must_use]
    pub fn publisher(&self) -> Publisher<TCore, P> {
        Publisher::new(self.transport.clone(), self.uri_provider.clone())
    }

    /// Creates the stable zero-copy notification role.
    #[must_use]
    pub fn notifier(&self) -> Notifier<TCore, P> {
        Notifier::new(self.transport.clone(), self.uri_provider.clone())
    }

    /// Creates the stable zero-copy RPC client role.
    #[must_use]
    pub fn rpc_client(&self) -> RpcClient<TCore, P> {
        RpcClient::new(self.transport.clone(), self.uri_provider.clone())
    }

    /// Creates the stable zero-copy RPC server registration role.
    #[must_use]
    pub fn rpc_server(&self) -> RpcServer<TCore, P> {
        RpcServer::new(self.transport.clone(), self.uri_provider.clone())
    }

    /// Creates the stable zero-copy subscriber role.
    #[cfg(feature = "usubscription")]
    #[must_use]
    pub fn subscriber(
        &self,
        usubscription: Arc<dyn crate::core::usubscription::USubscription>,
    ) -> Subscriber<TCore> {
        Subscriber::new(self.transport.clone(), usubscription)
    }
}

/// Publisher that initializes stable payloads directly in transport storage.
pub struct Publisher<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<StableTransport<TCore>>,
    uri_provider: Arc<P>,
}

impl<TCore, P> core::fmt::Debug for Publisher<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("Publisher").finish_non_exhaustive()
    }
}

impl<TCore, P> Publisher<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Creates a stable zero-copy publisher.
    #[must_use]
    pub fn new(transport: Arc<StableTransport<TCore>>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }
}

impl<TCore, P> Publisher<TCore, P>
where
    TCore: UZeroCopyUninitTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Publishes a generated stable payload without an intermediate payload buffer.
    pub async fn publish_stable<T, F>(
        &self,
        resource_id: u16,
        call_options: CallOptions,
        initialize: F,
    ) -> Result<(), PubSubError>
    where
        T: StablePayload + StablePayloadInit,
        F: for<'a> FnOnce(USelectedWireStablePayloadInit<'a, T>) -> InitializedStablePayload<'a, T>
            + Send,
    {
        let metadata = apply_options(
            UFrameMetadata::publish(self.uri_provider.get_resource_uri(resource_id)),
            call_options,
            false,
        )
        .with_native_payload_identity(
            payload_identity::<T>(self.transport.as_ref())
                .map_err(|error| PubSubError::InvalidArgument(error.to_string()))?,
        )
        .build()
        .map_err(|error| PubSubError::InvalidArgument(error.to_string()))?;
        self.transport
            .send_stable_payload::<T, F>(metadata, initialize)
            .await
            .map_err(Box::new)
            .map_err(PubSubError::PublishError)
    }
}

/// Notifier that sends stable payloads and exposes zero-copy receive leases.
pub struct Notifier<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<StableTransport<TCore>>,
    uri_provider: Arc<P>,
}

impl<TCore, P> core::fmt::Debug for Notifier<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("Notifier").finish_non_exhaustive()
    }
}

impl<TCore, P> Notifier<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Creates a stable zero-copy notifier.
    #[must_use]
    pub fn new(transport: Arc<StableTransport<TCore>>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }

    /// Registers a zero-copy notification listener.
    pub async fn start_listening(
        &self,
        topic: &UUri,
        listener: Arc<dyn UZeroCopyListener<StableRx<TCore>>>,
    ) -> Result<(), RegistrationError> {
        topic
            .verify_no_wildcards()
            .map_err(|error| RegistrationError::InvalidFilter(error.to_string()))?;
        self.transport
            .register_validated_zero_copy_listener(
                topic,
                Some(&self.uri_provider.get_source_uri()),
                listener,
            )
            .await
            .map_err(RegistrationError::from)
    }

    /// Unregisters a zero-copy notification listener.
    pub async fn stop_listening(
        &self,
        topic: &UUri,
        listener: Arc<dyn UZeroCopyListener<StableRx<TCore>>>,
    ) -> Result<(), RegistrationError> {
        topic
            .verify_no_wildcards()
            .map_err(|error| RegistrationError::InvalidFilter(error.to_string()))?;
        self.transport
            .unregister_validated_zero_copy_listener(
                topic,
                Some(&self.uri_provider.get_source_uri()),
                listener,
            )
            .await
            .map_err(RegistrationError::from)
    }
}

impl<TCore, P> Notifier<TCore, P>
where
    TCore: UZeroCopyUninitTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Sends a stable notification without an intermediate payload buffer.
    pub async fn notify_stable<T, F>(
        &self,
        resource_id: u16,
        destination: &UUri,
        call_options: CallOptions,
        initialize: F,
    ) -> Result<(), NotificationError>
    where
        T: StablePayload + StablePayloadInit,
        F: for<'a> FnOnce(USelectedWireStablePayloadInit<'a, T>) -> InitializedStablePayload<'a, T>
            + Send,
    {
        let metadata = apply_options(
            UFrameMetadata::notification(
                self.uri_provider.get_resource_uri(resource_id),
                destination.clone(),
            ),
            call_options,
            false,
        )
        .with_native_payload_identity(
            payload_identity::<T>(self.transport.as_ref())
                .map_err(|error| NotificationError::InvalidArgument(error.to_string()))?,
        )
        .build()
        .map_err(|error| NotificationError::InvalidArgument(error.to_string()))?;
        self.transport
            .send_stable_payload::<T, F>(metadata, initialize)
            .await
            .map_err(Box::new)
            .map_err(NotificationError::NotifyError)
    }
}

/// RPC client that sends a stable request and returns the response lease.
pub struct RpcClient<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<StableTransport<TCore>>,
    uri_provider: Arc<P>,
}

impl<TCore, P> core::fmt::Debug for RpcClient<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("RpcClient").finish_non_exhaustive()
    }
}

impl<TCore, P> RpcClient<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Creates a stable zero-copy RPC client.
    #[must_use]
    pub fn new(transport: Arc<StableTransport<TCore>>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }
}

impl<TCore, P> RpcClient<TCore, P>
where
    TCore: UZeroCopyUninitTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Sends a stable request and returns the matching selected-wire response lease.
    pub async fn invoke_stable<T, F>(
        &self,
        method: UUri,
        call_options: CallOptions,
        initialize: F,
    ) -> Result<StableRx<TCore>, ServiceInvocationError>
    where
        T: StablePayload + StablePayloadInit,
        F: for<'a> FnOnce(USelectedWireStablePayloadInit<'a, T>) -> InitializedStablePayload<'a, T>
            + Send,
    {
        let reply_to = self.uri_provider.get_source_uri();
        let (ttl, message_id, token, priority) = call_options.into_parts();
        let message_id = message_id.unwrap_or_else(UUID::build);
        let mut builder = UFrameMetadata::request(
            method.clone(),
            reply_to.clone(),
            Duration::from_millis(u64::from(ttl)),
        )
        .with_id(message_id.clone())
        .with_native_payload_identity(
            payload_identity::<T>(self.transport.as_ref())
                .map_err(|error| ServiceInvocationError::InvalidArgument(error.to_string()))?,
        );
        if let Some(token) = token {
            builder = builder.with_token(token);
        }
        if let Some(priority) = priority {
            builder = builder.with_priority(FramePriority::from_legacy_priority(priority));
        }
        let metadata = builder
            .build()
            .map_err(|error| ServiceInvocationError::InvalidArgument(error.to_string()))?;
        self.transport
            .send_stable_payload::<T, F>(metadata, initialize)
            .await?;
        let response = self
            .transport
            .receive_validated_zero_copy(&method, Some(&reply_to))
            .await?;
        if response.metadata().reqid() != Some(&message_id) {
            return Err(ServiceInvocationError::Internal(
                "received RPC response for unexpected request ID".to_string(),
            ));
        }
        Ok(response)
    }
}

/// RPC server registration facade delivering request leases without copying.
pub struct RpcServer<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    transport: Arc<StableTransport<TCore>>,
    uri_provider: Arc<P>,
}

impl<TCore, P> core::fmt::Debug for RpcServer<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("RpcServer").finish_non_exhaustive()
    }
}

impl<TCore, P> RpcServer<TCore, P>
where
    TCore: UZeroCopyTransportCore,
    P: LocalUriProvider + ?Sized,
{
    /// Creates a stable zero-copy RPC server registration facade.
    #[must_use]
    pub fn new(transport: Arc<StableTransport<TCore>>, uri_provider: Arc<P>) -> Self {
        Self {
            transport,
            uri_provider,
        }
    }

    /// Registers a request-lease listener for one local method resource.
    pub async fn register_method(
        &self,
        resource_id: u16,
        listener: Arc<dyn UZeroCopyListener<StableRx<TCore>>>,
    ) -> Result<(), RegistrationError> {
        self.transport
            .register_validated_zero_copy_listener(
                &listener_source_wildcard(),
                Some(&self.uri_provider.get_resource_uri(resource_id)),
                listener,
            )
            .await
            .map_err(RegistrationError::from)
    }

    /// Unregisters a request-lease listener for one local method resource.
    pub async fn unregister_method(
        &self,
        resource_id: u16,
        listener: Arc<dyn UZeroCopyListener<StableRx<TCore>>>,
    ) -> Result<(), RegistrationError> {
        self.transport
            .unregister_validated_zero_copy_listener(
                &listener_source_wildcard(),
                Some(&self.uri_provider.get_resource_uri(resource_id)),
                listener,
            )
            .await
            .map_err(RegistrationError::from)
    }
}

/// Subscriber that delivers selected-wire receive leases directly.
#[cfg(feature = "usubscription")]
pub struct Subscriber<TCore>
where
    TCore: UZeroCopyTransportCore,
{
    transport: Arc<StableTransport<TCore>>,
    usubscription: Arc<dyn crate::core::usubscription::USubscription>,
}

#[cfg(feature = "usubscription")]
impl<TCore> core::fmt::Debug for Subscriber<TCore>
where
    TCore: UZeroCopyTransportCore,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("Subscriber").finish_non_exhaustive()
    }
}

#[cfg(feature = "usubscription")]
impl<TCore> Subscriber<TCore>
where
    TCore: UZeroCopyTransportCore,
{
    /// Creates a zero-copy subscriber and binds its service client.
    #[must_use]
    pub fn new(
        transport: Arc<StableTransport<TCore>>,
        usubscription: Arc<dyn crate::core::usubscription::USubscription>,
    ) -> Self {
        Self {
            transport,
            usubscription,
        }
    }

    /// Subscribes through the service before registering the local lease listener.
    pub async fn subscribe(
        &self,
        topic: &UUri,
        listener: Arc<dyn UZeroCopyListener<StableRx<TCore>>>,
    ) -> Result<(), RegistrationError> {
        use crate::communication::SubscriptionStatus;

        super::validate_listener_topic(topic)?;
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
            .register_validated_zero_copy_listener(topic, None, listener)
            .await
            .map_err(RegistrationError::from)
    }

    /// Unsubscribes through the service before removing the local lease listener.
    pub async fn unsubscribe(
        &self,
        topic: &UUri,
        listener: Arc<dyn UZeroCopyListener<StableRx<TCore>>>,
    ) -> Result<(), RegistrationError> {
        super::validate_listener_topic(topic)?;
        self.usubscription
            .unsubscribe(topic)
            .await
            .map_err(|status| RegistrationError::Unknown(Box::new(status)))?;
        self.transport
            .unregister_validated_zero_copy_listener(topic, None, listener)
            .await
            .map_err(RegistrationError::from)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::{
        PreparedTxLoanSpec, StaticUriProvider, UEncodedRxFrame, UVecTxBuffer, UVecUninitTxBuffer,
        UWithNativePrefixWire,
    };

    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload, crate::StablePayloadInit)]
    #[stable_payload(type_name = "uprotocol.test.ZeroCopyRolePayload")]
    struct RolePayload {
        bytes: [u8; 4],
    }

    struct RawRx {
        encoded_metadata: Vec<u8>,
        payload: Vec<u8>,
    }

    impl UEncodedRxFrame for RawRx {
        type PayloadReader<'a>
            = Cursor<&'a [u8]>
        where
            Self: 'a;
        type PayloadSlices<'a>
            = std::iter::Once<&'a [u8]>
        where
            Self: 'a;

        fn encoded_metadata(&self) -> &[u8] {
            &self.encoded_metadata
        }

        fn payload_len(&self) -> usize {
            self.payload.len()
        }

        fn payload_reader(&self) -> Self::PayloadReader<'_> {
            Cursor::new(&self.payload)
        }

        fn payload_slices(&self) -> Self::PayloadSlices<'_> {
            std::iter::once(self.payload.as_slice())
        }

        fn try_contiguous_payload(&self) -> Option<&[u8]> {
            Some(&self.payload)
        }
    }

    #[derive(Default)]
    struct RecordingCore {
        prepared: Mutex<Vec<PreparedTxLoanSpec>>,
        sent: Mutex<Vec<UVecTxBuffer>>,
    }

    #[async_trait]
    impl UZeroCopyTransportCore for RecordingCore {
        type Tx = UVecTxBuffer;
        type Rx = RawRx;

        async fn loan_prepared_tx(
            &self,
            spec: PreparedTxLoanSpec,
        ) -> Result<Self::Tx, crate::UStatus> {
            self.prepared.lock().unwrap().push(spec.clone());
            UVecTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }

        async fn send_prepared_zero_copy(&self, buffer: Self::Tx) -> Result<(), crate::UStatus> {
            self.sent.lock().unwrap().push(buffer);
            Ok(())
        }
    }

    #[async_trait]
    impl UZeroCopyUninitTransportCore for RecordingCore {
        type UninitTx = UVecUninitTxBuffer;

        async fn loan_prepared_uninit_tx(
            &self,
            spec: PreparedTxLoanSpec,
        ) -> Result<Self::UninitTx, crate::UStatus> {
            self.prepared.lock().unwrap().push(spec.clone());
            UVecUninitTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }
    }

    fn provider(entity_id: u32) -> Arc<StaticUriProvider> {
        Arc::new(StaticUriProvider::new("vehicle", entity_id, 1).unwrap())
    }

    fn profile() -> crate::NativeProfileAgreement {
        let table = crate::NativeProfileTable::new([(
            crate::PayloadEncoding::from_id(0xF201).unwrap(),
            RolePayload::native_representation(),
        )])
        .unwrap();
        let profile = crate::NativeProfile::new(
            "zero-copy-role-tests",
            1,
            crate::NativeProfileMode::Table(table),
        )
        .unwrap();
        crate::NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap()
    }

    fn initialize(
        init: USelectedWireStablePayloadInit<'_, RolePayload>,
    ) -> InitializedStablePayload<'_, RolePayload> {
        init.into_initializer()
            .bytes_from_slice(b"role")
            .unwrap()
            .finish()
    }

    #[tokio::test]
    async fn send_roles_build_native_metadata_directly() {
        let transport =
            Arc::new(RecordingCore::default().into_stable_container_transport(profile()));
        let endpoint = Endpoint::new(transport.clone(), provider(0x1_0001));
        let publish_id = UUID::build();
        endpoint
            .publisher()
            .publish_stable::<RolePayload, _>(
                0x8001,
                CallOptions::for_publish(
                    Some(250),
                    Some(publish_id.clone()),
                    Some(crate::UPriority::CS2),
                ),
                initialize,
            )
            .await
            .unwrap();

        let destination = provider(0x2_0002).get_source_uri();
        endpoint
            .notifier()
            .notify_stable::<RolePayload, _>(
                0x8002,
                &destination,
                CallOptions::for_notification(None, None, None),
                initialize,
            )
            .await
            .unwrap();

        let method = provider(0x2_0002).get_resource_uri(0x00a1);
        assert!(endpoint
            .rpc_client()
            .invoke_stable::<RolePayload, _>(
                method.clone(),
                CallOptions::for_rpc_request(500, None, Some("token".into()), None),
                initialize,
            )
            .await
            .is_err());

        let prepared = transport.core().prepared.lock().unwrap();
        let publish = prepared.first().expect("publish metadata").metadata();
        assert_eq!(publish.kind(), crate::FrameMessageKind::Publish);
        assert_eq!(publish.id(), &publish_id);
        assert_eq!(publish.source().resource_id(), 0x8001);
        assert_eq!(publish.sink(), None);
        assert_eq!(publish.ttl(), Some(Duration::from_millis(250)));
        assert_eq!(
            publish.payload_encoding(),
            Some(
                &payload_identity::<RolePayload>(transport.as_ref())
                    .unwrap()
                    .encoding()
            )
        );

        let notification = prepared.get(1).expect("notification metadata").metadata();
        assert_eq!(notification.kind(), crate::FrameMessageKind::Notification);
        assert_eq!(notification.source().resource_id(), 0x8002);
        assert_eq!(notification.sink(), Some(&destination));

        let request = prepared.get(2).expect("request metadata").metadata();
        assert_eq!(request.kind(), crate::FrameMessageKind::Request);
        assert_eq!(request.source(), &provider(0x1_0001).get_source_uri());
        assert_eq!(request.sink(), Some(&method));
        assert_eq!(request.ttl(), Some(Duration::from_millis(500)));
        assert_eq!(request.token(), Some("token"));
        drop(prepared);

        let sent = transport.core().sent.lock().unwrap();
        assert_eq!(sent.len(), 3);
        assert!(sent.iter().all(|buffer| buffer.payload() == b"role"));
    }
}
