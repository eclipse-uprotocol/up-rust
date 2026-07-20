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

use super::*;

mod zero_copy_transport_sealed {
    pub trait Sealed {}
}

#[cfg(feature = "zero-copy-transport")]
mod zero_copy_uninit_transport_sealed {
    pub trait Sealed {}
}

/// Send-side timings emitted by the hidden stable-payload diagnostic API.
#[cfg(feature = "perf-diagnostics")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UninitStableSendPhases {
    pub prepare: std::time::Duration,
    pub loan: std::time::Duration,
    pub verify: std::time::Duration,
    pub initialize: std::time::Duration,
    pub witness: std::time::Duration,
    pub commit: std::time::Duration,
    pub total: std::time::Duration,
    pub residual: std::time::Duration,
}

#[cfg(feature = "perf-diagnostics")]
impl UninitStableSendPhases {
    #[must_use]
    pub fn accounted(self) -> std::time::Duration {
        self.prepare + self.loan + self.verify + self.initialize + self.witness + self.commit
    }

    /// Measures the median back-to-back `Instant` acquisition cost.
    #[must_use]
    pub fn calibrate_clock_overhead(iterations: usize) -> std::time::Duration {
        let mut samples = Vec::with_capacity(iterations.max(1));
        for _ in 0..iterations.max(1) {
            let start = std::time::Instant::now();
            samples.push(start.elapsed());
        }
        samples.sort_unstable();
        *samples
            .get(samples.len() / 2)
            .expect("clock calibration always records at least one sample")
    }
}

#[cfg(feature = "zero-copy-transport")]
fn initialize_stable_tx_payload<B, T>(
    buffer: &mut B,
    init: impl for<'payload> FnOnce(
        StablePayloadInitContext<'payload, T>,
    ) -> Result<InitializedStablePayload<'payload, T>, UWireError>,
) -> Result<(), UStatus>
where
    B: UUninitTxBuffer,
    T: StablePayloadInit,
{
    let payload = buffer.payload_uninit_mut();
    // SAFETY: the caller obtained `buffer` from the uninitialized transport
    // loan API and verified its visible payload layout before this helper.
    let loaned = unsafe {
        LoanedPayloadUninitMut::new_unchecked(payload, PayloadLoanProvenance::OpaqueTransportLoan)
    };
    let initializer = T::init_from_uninit_payload(loaned).map_err(UStatus::from)?;
    let context = StablePayloadInitContext::new(initializer);
    let _ = init(context).map_err(UStatus::from)?;
    Ok(())
}

/// *Role: implemented by transports whose API speaks the library's frame types; users call [`UZeroCopyTransport`](crate::UZeroCopyTransport) — see the [trait map](crate::guide::trait_map).*
///
/// Semantic native-frame implementation boundary for zero-copy transports.
///
/// Implementing this trait buys the validated public [`UZeroCopyTransport`]
/// API, listener filter validation, receive-frame validation, codec extension
/// helpers, and stable-payload initialization. This seam receives validated
/// [`UFrameMetadata`]. A physical transport that should carry encoded metadata
/// bytes without semantic knowledge implements `UZeroCopyTransportCore`
/// instead and is wrapped by the selected-wire adapter.
///
/// Two operations are required: reserve a validated TX loan and commit it.
/// Pull receive and listener registration/unregistration have default
/// unsupported implementations; override only the carriage patterns the
/// technology supports. [`UZeroCopyUninitTransportImpl`] adds the optional
/// uninitialized-TX capability.
///
/// The transport must honor or reject the validated payload layout, keep TX
/// loans exclusive until commit, and keep RX leases immutable until release
/// (`req~zero-copy-alignment~1`, `req~zero-copy-loan-isolation~1`, and
/// `req~zero-copy-lease-immutability~1`). `InMemoryZeroCopyTransport` under
/// `test-util` is the semantic reference implementation.
#[async_trait]
pub trait UZeroCopyTransportImpl: Send + Sync {
    /// Transport-specific transmit loan type.
    type Tx: UTxBuffer + Send;

    /// Transport-specific receive lease type.
    type Rx: UZeroCopyRxLease + Send + 'static;

    /// Reserves transmit storage for a validated frame loan spec.
    async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus>;

    /// Commits a validated transmit loan.
    async fn send_validated_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus>;

    /// Receives one matching zero-copy frame from transports that support pull receive.
    async fn receive_validated_zero_copy(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "not implemented",
        ))
    }

    /// Registers a zero-copy listener after public filter validation.
    async fn register_validated_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "not implemented",
        ))
    }

    /// Unregisters a zero-copy listener after public filter validation.
    async fn unregister_validated_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "not implemented",
        ))
    }
}

impl<T> zero_copy_transport_sealed::Sealed for T where T: UZeroCopyTransportImpl + ?Sized {}

/// *Role: called by users of the zero-copy family; transports implement [`UZeroCopyTransportImpl`](crate::UZeroCopyTransportImpl) or the encoded core instead — see the [trait map](crate::guide::trait_map).*
///
/// The zero-copy transport capability API.
#[async_trait]
pub trait UZeroCopyTransport: zero_copy_transport_sealed::Sealed + Send + Sync {
    /// Transport-specific transmit loan type returned by [`Self::loan_tx`].
    type Tx: UTxBuffer + Send;

    /// Transport-specific receive lease type returned by pull receive and listeners.
    type Rx: UZeroCopyRxLease + Send + 'static;

    /// Reserves transmit storage for a validated frame loan spec.
    async fn loan_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus>;

    /// Commits a previously reserved transmit loan.
    async fn send_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus>;

    /// Receives one matching zero-copy frame from transports that support pull receive.
    async fn receive_zero_copy(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus>;

    /// Registers a listener for matching zero-copy receive leases.
    async fn register_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus>;

    /// Unregisters a listener for matching zero-copy receive leases.
    async fn unregister_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus>;
}

#[async_trait]
impl<T> UZeroCopyTransport for T
where
    T: UZeroCopyTransportImpl + ?Sized,
{
    type Tx = T::Tx;
    type Rx = T::Rx;

    async fn loan_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
        UZeroCopyTransportImpl::loan_validated_tx(self, spec).await
    }

    async fn send_zero_copy(&self, mut buffer: Self::Tx) -> Result<(), UStatus> {
        validate_tx_buffer_for_transport(&mut buffer)?;
        UZeroCopyTransportImpl::send_validated_zero_copy(self, buffer).await
    }

    async fn receive_zero_copy(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        verify_zero_copy_filter_criteria(source_filter, sink_filter)?;
        let frame =
            UZeroCopyTransportImpl::receive_validated_zero_copy(self, source_filter, sink_filter)
                .await?;
        validate_frame_view_for_transport(&frame)?;
        Ok(frame)
    }

    async fn register_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        verify_zero_copy_filter_criteria(source_filter, sink_filter)?;
        let key = zero_copy_listener_registration_key(
            self,
            source_filter,
            sink_filter,
            zero_copy_listener_pointer(&listener),
        );
        let (listener, inserted) = registered_zero_copy_listener(&key, listener);
        let result = UZeroCopyTransportImpl::register_validated_zero_copy_listener(
            self,
            source_filter,
            sink_filter,
            listener,
        )
        .await;
        if result.is_err() && inserted {
            ZERO_COPY_LISTENER_REGISTRY
                .lock()
                .expect("zero-copy listener registry lock poisoned")
                .remove(&key);
        }
        result
    }

    async fn unregister_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        verify_zero_copy_filter_criteria(source_filter, sink_filter)?;
        let key = zero_copy_listener_registration_key(
            self,
            source_filter,
            sink_filter,
            zero_copy_listener_pointer(&listener),
        );
        let listener = zero_copy_listener_for_unregister(&key, listener);
        let result = UZeroCopyTransportImpl::unregister_validated_zero_copy_listener(
            self,
            source_filter,
            sink_filter,
            listener,
        )
        .await;
        if result.is_ok() {
            ZERO_COPY_LISTENER_REGISTRY
                .lock()
                .expect("zero-copy listener registry lock poisoned")
                .remove(&key);
        }
        result
    }
}

/// *Role: free blanket methods on every zero-copy transport; never implemented by hand — see the [trait map](crate::guide::trait_map).*
///
/// Convenience methods for zero-copy transports with initialized TX storage.
#[async_trait]
pub trait UZeroCopyTransportExt: UZeroCopyTransport {
    /// Initializes a typed payload using the adapter's selected wire and sends it.
    ///
    /// Prefer this selected-wire helper on values produced by explicit selected-wire adapter construction.
    /// Use [`Self::send_loaned_payload_as`] only for low-level codec escape hatches.
    ///
    /// # Errors
    ///
    /// Returns an error if the selected wire does not successfully loan, encode,
    /// or send the payload through the underlying transport.
    #[cfg(feature = "selected-wire-transport-adapter")]
    async fn send_loaned_payload<T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(&'payload mut T) + Send,
    ) -> Result<(), UStatus>
    where
        Self: UHasWire,
        Self::Wire: UWirePayload<T>,
        <Self::Wire as UWirePayload<T>>::Codec: LoanPayload<T> + Send + Sync,
    {
        self.send_loaned_payload_as::<<Self::Wire as UWirePayload<T>>::Codec, T>(metadata, init)
            .await
    }

    /// Initializes a typed payload directly in a transmit loan and sends it.
    ///
    /// This is the low-level codec-selected form. Product code that already uses
    /// explicit selected-wire adapter construction should prefer
    /// `send_loaned_payload` so the
    /// selected wire supplies the payload codec.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation fails, the transport cannot loan
    /// the requested initialized layout, the codec rejects the loaned storage, or
    /// sending the committed loan fails.
    async fn send_loaned_payload_as<C, T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(&'payload mut T) + Send,
    ) -> Result<(), UStatus>
    where
        C: PayloadCodec + LoanPayload<T> + Send + Sync,
    {
        let metadata = metadata
            .with_payload_encoding(C::payload_encoding())
            .map_err(frame_metadata_error)?;
        let layout = C::loan_layout().map_err(UStatus::from)?;
        let mut buffer = self
            .loan_tx(UTxLoanSpec::payload(
                metadata,
                layout.len(),
                layout.align(),
            )?)
            .await?;
        verify_tx_buffer_payload_layout(&mut buffer, layout.len(), layout.align())?;
        {
            let payload = C::loan_payload(buffer.payload_mut()).map_err(UStatus::from)?;
            init(payload);
        }
        self.send_zero_copy(buffer).await
    }
}

impl<T> UZeroCopyTransportExt for T where T: UZeroCopyTransport + ?Sized {}

/// *Role: free blanket methods for typed initialization into uninitialized loans; never implemented by hand — see the [trait map](crate::guide::trait_map).*
///
/// Convenience methods for zero-copy transports with uninitialized TX storage.
#[cfg(feature = "zero-copy-transport")]
#[async_trait]
pub trait UZeroCopyUninitTransportExt: UZeroCopyUninitTransport {
    /// Constructs a typed payload using the adapter's selected wire and sends it.
    ///
    /// Prefer this selected-wire helper on values produced by explicit selected-wire adapter construction.
    /// Use [`Self::send_uninit_loaned_payload_as`] only for low-level codec
    /// escape hatches.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation, loaning, initialization, or send
    /// fails.
    #[cfg(feature = "selected-wire-transport-adapter")]
    async fn send_uninit_loaned_payload<T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(
                LoanedUninitPayload<'payload, T>,
            ) -> Result<LoanedInitPayload<'payload, T>, UWireError>
            + Send,
    ) -> Result<(), UStatus>
    where
        Self: UHasWire,
        Self::Wire: UWirePayload<T>,
        <Self::Wire as UWirePayload<T>>::Codec: LoanUninitPayload<T> + Send + Sync,
        T: Send,
    {
        self.send_uninit_loaned_payload_as::<<Self::Wire as UWirePayload<T>>::Codec, T>(
            metadata, init,
        )
        .await
    }

    /// Constructs a typed payload directly in uninitialized transmit storage and sends it.
    ///
    /// This is the low-level codec-selected form. Product code that already uses
    /// explicit selected-wire adapter construction should prefer
    /// `send_uninit_loaned_payload` so the
    /// selected wire supplies the payload codec.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation fails, the transport cannot loan
    /// the requested uninitialized layout, the codec rejects the loaned storage,
    /// the initializer fails, or sending the committed loan fails.
    async fn send_uninit_loaned_payload_as<C, T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(
                LoanedUninitPayload<'payload, T>,
            ) -> Result<LoanedInitPayload<'payload, T>, UWireError>
            + Send,
    ) -> Result<(), UStatus>
    where
        C: PayloadCodec + LoanUninitPayload<T> + Send + Sync,
        T: Send,
    {
        let metadata = metadata
            .with_payload_encoding(C::payload_encoding())
            .map_err(frame_metadata_error)?;
        let layout = C::loan_uninit_layout().map_err(UStatus::from)?;
        let mut buffer = self
            .loan_uninit_tx(UTxLoanSpec::payload(
                metadata,
                layout.len(),
                layout.align(),
            )?)
            .await?;
        verify_uninit_tx_buffer_payload_layout(&mut buffer, layout.len(), layout.align())?;
        {
            let payload = buffer.payload_uninit_mut();
            // SAFETY: `UZeroCopyUninitTransport::loan_uninit_tx` returned this
            // buffer as the transport loan for the validated spec. The public
            // verifier above checked that this visible range matches the request.
            let loaned = unsafe {
                LoanedPayloadUninitMut::new_unchecked(
                    payload,
                    PayloadLoanProvenance::OpaqueTransportLoan,
                )
            };
            let loaned = C::loan_uninit_payload(loaned).map_err(UStatus::from)?;
            let expected = loaned.uninit_ptr();
            let initialized = init(loaned).map_err(UStatus::from)?;
            if initialized.initialized_ptr().cast::<MaybeUninit<T>>() != expected {
                return Err(invalid_argument(
                    "initialized payload proof does not match the TX loan",
                ));
            }
        }
        // SAFETY: the initializer returned a marker tied to the same checked
        // loan slot, proving the visible payload bytes have been initialized.
        let buffer = unsafe { buffer.assume_payload_init() };
        self.send_zero_copy(buffer).await
    }

    /// Initializes a stable-container payload through the adapter's selected wire.
    ///
    /// Prefer this selected-wire helper on values produced by explicit selected-wire adapter construction.
    /// It is available only for selected wires whose uninitialized-loan codec is
    /// `StableContainerPayload<T>`.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation, stable initialization, loaning, or
    /// send fails.
    ///
    #[cfg(feature = "selected-wire-transport-adapter")]
    async fn send_uninit_stable_payload<T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(
                StablePayloadInitContext<'payload, T>,
            )
                -> Result<InitializedStablePayload<'payload, T>, UWireError>
            + Send,
    ) -> Result<(), UStatus>
    where
        Self: UHasWire,
        Self::Wire: UWirePayload<T, Codec = StableContainerPayload<T>>,
        T: StablePayloadInit + Send,
    {
        self.send_uninit_stable_payload_as::<T>(metadata, init)
            .await
    }

    /// Initializes a stable-container payload directly in uninitialized transmit storage.
    ///
    /// This is the low-level stable-container form. Product code that already
    /// uses explicit selected-wire adapter construction should prefer
    /// `send_uninit_stable_payload` so
    /// the selected wire authorizes the stable-container payload family.
    ///
    /// The initializer is generated by `#[derive(StablePayloadInit)]`; it exposes
    /// named typed setters and returns a completion token only after all required
    /// fields and generated padding gaps are initialized.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation fails, the transport cannot loan
    /// the requested stable layout, initialization fails, the completion token is
    /// not completed, or sending the committed loan fails.
    async fn send_uninit_stable_payload_as<T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(
                StablePayloadInitContext<'payload, T>,
            )
                -> Result<InitializedStablePayload<'payload, T>, UWireError>
            + Send,
    ) -> Result<(), UStatus>
    where
        T: StablePayloadInit + Send,
    {
        let metadata = metadata
            .with_payload_encoding(StableContainerPayload::<T>::encoding())
            .map_err(frame_metadata_error)?;
        let layout_len = std::mem::size_of::<T>();
        let layout_align = std::mem::align_of::<T>();
        let mut buffer = self
            .loan_uninit_tx(UTxLoanSpec::payload(metadata, layout_len, layout_align)?)
            .await?;
        verify_uninit_tx_buffer_payload_layout(&mut buffer, layout_len, layout_align)?;
        initialize_stable_tx_payload(&mut buffer, init)?;
        // SAFETY: the generated stable initializer returned a completion proof
        // for the same loan slot after all fields and padding were initialized.
        let buffer = unsafe { buffer.assume_payload_init() };
        self.send_zero_copy(buffer).await
    }

    /// Diagnostic variant of [`Self::send_uninit_stable_payload_as`].
    #[cfg(feature = "perf-diagnostics")]
    #[doc(hidden)]
    async fn send_uninit_stable_payload_as_phased<T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(
                StablePayloadInitContext<'payload, T>,
            )
                -> Result<InitializedStablePayload<'payload, T>, UWireError>
            + Send,
    ) -> Result<UninitStableSendPhases, UStatus>
    where
        T: StablePayloadInit + Send,
    {
        let total_start = std::time::Instant::now();

        let phase_start = std::time::Instant::now();
        let metadata = metadata
            .with_payload_encoding(StableContainerPayload::<T>::encoding())
            .map_err(frame_metadata_error)?;
        let layout_len = std::mem::size_of::<T>();
        let layout_align = std::mem::align_of::<T>();
        let spec = UTxLoanSpec::payload(metadata, layout_len, layout_align)?;
        let prepare = phase_start.elapsed();

        let phase_start = std::time::Instant::now();
        let mut buffer = self.loan_uninit_tx(spec).await?;
        let loan = phase_start.elapsed();

        let phase_start = std::time::Instant::now();
        verify_uninit_tx_buffer_payload_layout(&mut buffer, layout_len, layout_align)?;
        let verify = phase_start.elapsed();

        let phase_start = std::time::Instant::now();
        initialize_stable_tx_payload(&mut buffer, init)?;
        let initialize = phase_start.elapsed();

        let phase_start = std::time::Instant::now();
        // SAFETY: the shared initializer kernel returned a completion proof for
        // this same verified loan after initializing all fields and padding.
        let buffer = unsafe { buffer.assume_payload_init() };
        let witness = phase_start.elapsed();

        let phase_start = std::time::Instant::now();
        self.send_zero_copy(buffer).await?;
        let commit = phase_start.elapsed();
        let total = total_start.elapsed();
        let accounted = prepare + loan + verify + initialize + witness + commit;

        Ok(UninitStableSendPhases {
            prepare,
            loan,
            verify,
            initialize,
            witness,
            commit,
            total,
            residual: total.saturating_sub(accounted),
        })
    }

    /// Selected-wire diagnostic variant of [`Self::send_uninit_stable_payload`].
    #[cfg(all(
        feature = "perf-diagnostics",
        feature = "selected-wire-transport-adapter"
    ))]
    #[doc(hidden)]
    async fn send_uninit_stable_payload_phased<T>(
        &self,
        metadata: UFrameMetadata,
        init: impl for<'payload> FnOnce(
                StablePayloadInitContext<'payload, T>,
            )
                -> Result<InitializedStablePayload<'payload, T>, UWireError>
            + Send,
    ) -> Result<UninitStableSendPhases, UStatus>
    where
        Self: UHasWire,
        Self::Wire: UWirePayload<T, Codec = StableContainerPayload<T>>,
        T: StablePayloadInit + Send,
    {
        self.send_uninit_stable_payload_as_phased::<T>(metadata, init)
            .await
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<T> UZeroCopyUninitTransportExt for T where T: UZeroCopyUninitTransport + ?Sized {}

/// *Role: implemented by transports that can lend uninitialized storage — see the [trait map](crate::guide::trait_map).*
///
/// Implementation boundary for transports that can expose uninitialized TX payload storage.
#[async_trait]
pub trait UZeroCopyUninitTransportImpl: UZeroCopyTransportImpl {
    /// Transport-specific uninitialized transmit loan type.
    type UninitTx: UUninitTxBuffer<Initialized = Self::Tx> + Send;

    /// Reserves uninitialized transmit storage for a validated frame loan spec.
    async fn loan_validated_uninit_tx(&self, spec: UTxLoanSpec) -> Result<Self::UninitTx, UStatus>;
}

#[cfg(feature = "zero-copy-transport")]
impl<T> zero_copy_uninit_transport_sealed::Sealed for T where
    T: UZeroCopyUninitTransportImpl + ?Sized
{
}

/// *Role: called by users to obtain uninitialized loans; transports implement [`UZeroCopyUninitTransportImpl`](crate::UZeroCopyUninitTransportImpl) — see the [trait map](crate::guide::trait_map).*
///
/// Optional zero-copy capability for transports that can expose uninitialized TX payload storage.
#[cfg(feature = "zero-copy-transport")]
#[async_trait]
pub trait UZeroCopyUninitTransport:
    UZeroCopyTransport + zero_copy_uninit_transport_sealed::Sealed
{
    /// Transport-specific uninitialized transmit loan type.
    type UninitTx: UUninitTxBuffer<Initialized = Self::Tx> + Send;

    /// Reserves uninitialized transmit storage for a validated frame loan spec.
    async fn loan_uninit_tx(&self, spec: UTxLoanSpec) -> Result<Self::UninitTx, UStatus>;
}

#[cfg(feature = "zero-copy-transport")]
#[async_trait]
impl<T> UZeroCopyUninitTransport for T
where
    T: UZeroCopyUninitTransportImpl + ?Sized,
{
    type UninitTx = T::UninitTx;

    async fn loan_uninit_tx(&self, spec: UTxLoanSpec) -> Result<Self::UninitTx, UStatus> {
        UZeroCopyUninitTransportImpl::loan_validated_uninit_tx(self, spec).await
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ZeroCopyListenerRegistrationKey {
    transport: usize,
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: usize,
}

static ZERO_COPY_LISTENER_REGISTRY: LazyLock<
    Mutex<HashMap<ZeroCopyListenerRegistrationKey, Arc<dyn Any + Send + Sync>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct ValidatingZeroCopyListener<Rx>
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    listener: Arc<dyn UZeroCopyListener<Rx>>,
}

#[async_trait]
impl<Rx> UZeroCopyListener<Rx> for ValidatingZeroCopyListener<Rx>
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    async fn on_receive_zero_copy(&self, frame: Rx) {
        match validate_frame_view_for_transport(&frame) {
            Ok(()) => self.listener.on_receive_zero_copy(frame).await,
            Err(error) => {
                warn!(%error, "dropping invalid zero-copy frame before listener delivery")
            }
        }
    }
}

fn registered_zero_copy_listener<Rx>(
    key: &ZeroCopyListenerRegistrationKey,
    listener: Arc<dyn UZeroCopyListener<Rx>>,
) -> (Arc<dyn UZeroCopyListener<Rx>>, bool)
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    let mut registry = ZERO_COPY_LISTENER_REGISTRY
        .lock()
        .expect("zero-copy listener registry lock poisoned");
    if let Some(existing) = registry.get(key) {
        if let Ok(existing) = existing
            .clone()
            .downcast::<ValidatingZeroCopyListener<Rx>>()
        {
            let existing: Arc<dyn UZeroCopyListener<Rx>> = existing;
            return (existing, false);
        }
    }

    let validating_listener = Arc::new(ValidatingZeroCopyListener { listener });
    registry.insert(key.clone(), validating_listener.clone());
    let validating_listener: Arc<dyn UZeroCopyListener<Rx>> = validating_listener;
    (validating_listener, true)
}

fn zero_copy_listener_for_unregister<Rx>(
    key: &ZeroCopyListenerRegistrationKey,
    fallback: Arc<dyn UZeroCopyListener<Rx>>,
) -> Arc<dyn UZeroCopyListener<Rx>>
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    ZERO_COPY_LISTENER_REGISTRY
        .lock()
        .expect("zero-copy listener registry lock poisoned")
        .get(key)
        .and_then(|listener| {
            listener
                .clone()
                .downcast::<ValidatingZeroCopyListener<Rx>>()
                .ok()
        })
        .map_or(fallback, |listener| {
            listener as Arc<dyn UZeroCopyListener<Rx>>
        })
}

fn zero_copy_transport_pointer<T: ?Sized>(transport: &T) -> usize {
    let ptr = transport as *const T;
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

fn zero_copy_listener_pointer<Rx>(listener: &Arc<dyn UZeroCopyListener<Rx>>) -> usize
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    let ptr = Arc::as_ptr(listener);
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

fn zero_copy_listener_registration_key<T: ?Sized>(
    transport: &T,
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
    listener: usize,
) -> ZeroCopyListenerRegistrationKey {
    ZeroCopyListenerRegistrationKey {
        transport: zero_copy_transport_pointer(transport),
        source_filter: source_filter.clone(),
        sink_filter: sink_filter.cloned(),
        listener,
    }
}

fn verify_zero_copy_filter_criteria(
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> Result<(), UStatus> {
    verify_filter_criteria(source_filter, sink_filter).map_err(|status| *status)
}
