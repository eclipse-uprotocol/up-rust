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

#![cfg_attr(
    not(any(
        feature = "transport-implementer-api",
        feature = "selected-wire-user-api"
    )),
    allow(dead_code)
)]

//! Selected-wire transport adapter core.
//!
//! Product transports implement the small core traits in this module for their
//! physical mechanics. A core accepts metadata bytes already prepared by the
//! selected metadata codec, returns encoded metadata bytes on receive, and
//! validates any physical mirror fields against decoded metadata before public
//! exposure. Product transport modules should not import, match on, or branch
//! by concrete wire families such as `UProtocolNativeWire`; the selected wire is
//! owned by [`UWireTransport<TCore, W, C>`].
//!
//! ## Using the adapter
//!
//! 1. Implement [`UOwnedTransportCore`] for physical storage and encoded
//!    metadata carriage. Core methods do not choose a wire.
//! 2. Wrap the core with [`UWithNativePrefixWire::into_native_prefix_wire_transport`]
//!    for an external or deployment-specific wire, or use
//!    [`UWithNativePrefixWire::into_protobuf_transport`].
//! 3. Use the resulting adapter through the public semantic owned-transport
//!    traits. The adapter encodes metadata before core TX, checks selected
//!    identities on RX, validates the frame, and only then exposes it.
//!
//! This follows the N+M composition model in
//! `up-spec/up-l1/transport_families.adoc`: a core must not branch on concrete
//! wire types, and a wire must not contain transport-specific carriage code.

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use std::{io::Read, marker::PhantomData};

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
use async_trait::async_trait;
#[cfg(feature = "owned-frame-transport")]
use bytes::Bytes;
#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
use tracing::warn;

#[cfg(feature = "zero-copy-transport")]
use crate::payload::loan::BorrowPayload;
use crate::payload::{
    codec::{EncodePayload, PayloadCodec, PayloadDecodeLimit, ReadDecodePayload},
    UWireError,
};
use crate::wire::NativePrefixFrameMetadataCodec;
use crate::wire::ProtobufWire;
#[cfg(feature = "zero-copy-transport")]
use crate::wire::StableContainerWireFormat;
use crate::wire::{UWire, UWireMetadataCodecFor, UWirePayload};
use crate::{validate_frame_view_for_transport, UFrameMetadata, UFrameView, UStatus};
#[cfg(feature = "zero-copy-transport")]
use crate::{
    LoanedPayload, PayloadAlignment, ULoanedContiguousZeroCopyRxFrame, UTxBuffer, UTxLoanSpec,
    UUninitTxBuffer, UZeroCopyListener, UZeroCopyRxLease, UZeroCopyTransport,
    UZeroCopyTransportImpl, UZeroCopyUninitTransportImpl,
};
#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
use crate::{UCode, UUri};
#[cfg(feature = "owned-frame-transport")]
use crate::{UOwnedFrame, UOwnedListener, UOwnedTransportImpl};

/// Generic selected-wire transport adapter.
pub struct UWireTransport<TCore, W, C>
where
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    core: TCore,
    wire: W,
    metadata_codec: C,
    native_profile: Option<crate::NativeProfileAgreement>,
    #[cfg(feature = "zero-copy-transport")]
    zero_copy_listeners: Mutex<HashMap<WireListenerKey, Arc<dyn Any + Send + Sync>>>,
    #[cfg(feature = "owned-frame-transport")]
    owned_listeners: Mutex<HashMap<WireListenerKey, Arc<dyn Any + Send + Sync>>>,
}

impl<TCore, W, C> core::fmt::Debug for UWireTransport<TCore, W, C>
where
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UWireTransport").finish_non_exhaustive()
    }
}

impl<TCore, W, C> UWireTransport<TCore, W, C>
where
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    /// Creates an adapter around a transport core, selected wire marker, and metadata codec.
    #[must_use]
    pub fn new(core: TCore, wire: W, metadata_codec: C) -> Self {
        Self {
            core,
            wire,
            metadata_codec,
            native_profile: None,
            #[cfg(feature = "zero-copy-transport")]
            zero_copy_listeners: Mutex::new(HashMap::new()),
            #[cfg(feature = "owned-frame-transport")]
            owned_listeners: Mutex::new(HashMap::new()),
        }
    }

    /// Creates an adapter with an explicit selected wire and metadata codec.
    #[must_use]
    pub fn with_wire_and_metadata_codec(core: TCore, wire: W, metadata_codec: C) -> Self {
        Self::new(core, wire, metadata_codec)
    }

    /// Constructs a new adapter with an already confirmed native route agreement.
    /// The generation is immutable and is retained by listeners and receive views.
    pub fn with_native_profile(
        core: TCore,
        wire: W,
        metadata_codec: C,
        profile: crate::NativeProfileAgreement,
    ) -> Self {
        let mut adapter = Self::new(core, wire, metadata_codec);
        adapter.native_profile = Some(profile);
        adapter
    }

    /// Returns the wrapped physical transport core.
    #[must_use]
    pub fn core(&self) -> &TCore {
        &self.core
    }

    /// Returns the wrapped physical transport core mutably.
    #[must_use]
    pub fn core_mut(&mut self) -> &mut TCore {
        &mut self.core
    }

    /// Returns the selected metadata codec.
    #[must_use]
    pub fn metadata_codec(&self) -> &C {
        &self.metadata_codec
    }

    /// Consumes the adapter and returns its core, wire, metadata codec and agreement.
    #[must_use]
    pub fn into_parts(self) -> (TCore, W, C, Option<crate::NativeProfileAgreement>) {
        (
            self.core,
            self.wire,
            self.metadata_codec,
            self.native_profile,
        )
    }
}

/// Selected-wire transport using the canonical UFrame metadata field block.
///
/// Ordinary selected-wire construction is canonical-by-default per R2W. The
/// legacy protobuf-`UAttributes` metadata profile remains available only via
/// the explicitly legacy-named aliases/constructors below.
pub type UNativePrefixWireTransport<TCore, W> =
    UWireTransport<TCore, W, NativePrefixFrameMetadataCodec>;

/// Protocol Buffers selected-wire transport with canonical metadata.
pub type ProtobufWireTransport<TCore> = UNativePrefixWireTransport<TCore, ProtobufWire>;

/// Stable-container selected-wire transport with canonical metadata.
#[cfg(feature = "zero-copy-transport")]
pub type StableContainerWireTransport<TCore> =
    UNativePrefixWireTransport<TCore, StableContainerWireFormat>;

/// Lifetime-bound stable-payload initializer for selected-wire TX helpers.
#[cfg(feature = "zero-copy-transport")]
pub struct USelectedWireStablePayloadInit<'a, T>
where
    T: crate::StablePayloadInit,
{
    initializer: <T as crate::StablePayloadInit>::Initializer<'a>,
    _lifetime: PhantomData<&'a mut T>,
}

#[cfg(feature = "zero-copy-transport")]
impl<T> core::fmt::Debug for USelectedWireStablePayloadInit<'_, T>
where
    T: crate::StablePayloadInit,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("USelectedWireStablePayloadInit")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<'a, T> USelectedWireStablePayloadInit<'a, T>
where
    T: crate::StablePayloadInit,
{
    /// Returns the generated field-wise initializer.
    #[must_use]
    pub fn into_initializer(self) -> <T as crate::StablePayloadInit>::Initializer<'a> {
        self.initializer
    }
}

/// Convenience constructors for canonical native-prefix selected-wire transports.
pub trait UWithNativePrefixWire: Sized {
    /// Wraps this core with an external or deployment-specific selected wire using canonical metadata.
    #[must_use]
    fn into_native_prefix_wire_transport<W>(self, wire: W) -> UNativePrefixWireTransport<Self, W>
    where
        W: UWire;

    /// Wraps this core with the Protocol Buffers selected-wire profile.
    #[must_use]
    fn into_protobuf_transport(self) -> ProtobufWireTransport<Self>;

    /// Wraps this core with the stable-container selected wire.
    #[must_use]
    #[cfg(feature = "zero-copy-transport")]
    fn into_stable_container_transport(
        self,
        profile: crate::NativeProfileAgreement,
    ) -> StableContainerWireTransport<Self>;
}

impl<TCore> UWithNativePrefixWire for TCore {
    fn into_native_prefix_wire_transport<W>(self, wire: W) -> UNativePrefixWireTransport<Self, W>
    where
        W: UWire,
    {
        UWireTransport::new(self, wire, NativePrefixFrameMetadataCodec)
    }

    fn into_protobuf_transport(self) -> ProtobufWireTransport<Self> {
        self.into_native_prefix_wire_transport(ProtobufWire)
    }

    #[cfg(feature = "zero-copy-transport")]
    fn into_stable_container_transport(
        self,
        profile: crate::NativeProfileAgreement,
    ) -> StableContainerWireTransport<Self> {
        UWireTransport::with_native_profile(
            self,
            StableContainerWireFormat,
            NativePrefixFrameMetadataCodec,
            profile,
        )
    }
}

/// Exposes the concrete selected wire of an adapter value.
pub trait UHasWire {
    /// Concrete selected wire type.
    type Wire: UWire;

    /// Returns the selected wire marker/value.
    #[must_use]
    fn wire(&self) -> &Self::Wire;

    /// Returns the explicitly configured native route agreement, if any.
    fn native_profile(&self) -> Option<&crate::NativeProfileAgreement> {
        None
    }
}

impl<TCore, W, C> UHasWire for UWireTransport<TCore, W, C>
where
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    type Wire = W;

    fn wire(&self) -> &Self::Wire {
        &self.wire
    }

    fn native_profile(&self) -> Option<&crate::NativeProfileAgreement> {
        self.native_profile.as_ref()
    }
}

#[cfg(feature = "zero-copy-transport")]
/// Marker trait for zero-copy transports with a statically selected wire.
pub trait USelectedWireZeroCopyTransport: UZeroCopyTransport + UHasWire {
    /// Metadata codec used with the selected wire.
    type MetadataCodec: UWireMetadataCodecFor<Self::Wire>;
}

#[cfg(feature = "zero-copy-transport")]
impl<TCore, W, C> USelectedWireZeroCopyTransport for UWireTransport<TCore, W, C>
where
    TCore: UZeroCopyTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    type MetadataCodec = C;
}

#[cfg(feature = "zero-copy-transport")]
/// Prepared zero-copy transmit request passed from the adapter to a core.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedTxLoanSpec {
    metadata: UFrameMetadata,
    encoded_metadata: Vec<u8>,
    payload_len: usize,
    payload_alignment: PayloadAlignment,
}

#[cfg(feature = "zero-copy-transport")]
impl PreparedTxLoanSpec {
    /// Encodes validated metadata for a selected wire.
    ///
    /// # Errors
    ///
    /// Returns an error if selected-wire metadata encoding fails.
    pub fn from_validated<W, C>(spec: UTxLoanSpec, codec: &C) -> Result<Self, UStatus>
    where
        W: UWire,
        C: UWireMetadataCodecFor<W>,
    {
        let encoded_metadata =
            codec.encode_frame_metadata(W::metadata_context(), spec.metadata())?;
        Ok(Self {
            metadata: spec.metadata().clone(),
            encoded_metadata,
            payload_len: spec.payload_len(),
            payload_alignment: spec.payload_alignment_proof(),
        })
    }

    /// Creates a prepared loan spec from metadata bytes that are already encoded
    /// for the selected wire associated with `metadata`.
    ///
    /// This is an advanced adapter/core boundary helper for owned-loopback
    /// bridges. Callers must pass `encoded_metadata` produced by the same
    /// selected wire that will decode `metadata` on receive; this constructor
    /// validates the decoded metadata and payload layout but cannot prove that
    /// the opaque metadata bytes were produced by a particular wire.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata, payload presence, or payload alignment is
    /// invalid for a zero-copy transmit loan.
    pub fn from_encoded_parts(
        metadata: UFrameMetadata,
        encoded_metadata: impl Into<Vec<u8>>,
        payload_len: usize,
        payload_alignment: usize,
    ) -> Result<Self, UStatus> {
        let spec = if metadata.payload_encoding().is_some() {
            UTxLoanSpec::payload(metadata, payload_len, payload_alignment)?
        } else {
            if payload_len != 0 {
                return Err(UStatus::fail_with_code(
                    UCode::InvalidArgument,
                    "prepared TX spec without payload encoding cannot carry payload bytes",
                ));
            }
            if payload_alignment != 1 {
                return Err(UStatus::fail_with_code(
                    UCode::InvalidArgument,
                    "prepared TX spec without payload uses alignment 1",
                ));
            }
            UTxLoanSpec::no_payload(metadata)?
        };
        Ok(Self {
            metadata: spec.metadata().clone(),
            encoded_metadata: encoded_metadata.into(),
            payload_len: spec.payload_len(),
            payload_alignment: spec.payload_alignment_proof(),
        })
    }

    /// Returns the decoded frame metadata used to prepare this request.
    #[must_use]
    pub fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    /// Returns selected-wire encoded metadata bytes.
    #[must_use]
    pub fn encoded_metadata(&self) -> &[u8] {
        &self.encoded_metadata
    }

    /// Returns the visible application payload length requested from the core.
    #[must_use]
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }

    /// Returns the visible application payload alignment requested from the core.
    #[must_use]
    pub fn payload_alignment(&self) -> usize {
        self.payload_alignment.as_usize()
    }

    /// Returns the validated visible application payload alignment proof.
    #[must_use]
    pub fn payload_alignment_proof(&self) -> PayloadAlignment {
        self.payload_alignment
    }

    /// Returns whether the request carries a payload, including a present empty payload.
    #[must_use]
    pub fn has_payload(&self) -> bool {
        self.metadata.payload_encoding().is_some()
    }

    /// Consumes this request and returns its parts.
    #[must_use]
    pub fn into_parts(self) -> (UFrameMetadata, Vec<u8>, usize, usize) {
        (
            self.metadata,
            self.encoded_metadata,
            self.payload_len,
            self.payload_alignment.as_usize(),
        )
    }
}

/// Raw encoded receive object returned by a transport core.
///
/// This is an implementation-boundary trait. Raw encoded receive objects should
/// not implement public frame or lease traits directly; public receive paths
/// expose [`UWireRx<Rx, W, C>`] after selected-wire metadata decode and validation.
///
/// The object must retain all resources backing both metadata and payload views
/// for its lifetime, including after listener unregistration or core destruction.
/// Retaining a sample-handle allocation without its native proxy/mapping owner
/// does not satisfy this contract.
pub trait UEncodedRxFrame {
    /// Ordered payload reader type.
    type PayloadReader<'a>: Read + 'a
    where
        Self: 'a;
    /// Ordered payload slices iterator type.
    type PayloadSlices<'a>: Iterator<Item = &'a [u8]> + 'a
    where
        Self: 'a;

    /// Returns selected-wire encoded metadata bytes.
    fn encoded_metadata(&self) -> &[u8];

    /// Returns the visible application payload length.
    fn payload_len(&self) -> usize;

    /// Returns an ordered reader over the application payload bytes.
    fn payload_reader(&self) -> Self::PayloadReader<'_>;

    /// Returns ordered borrowed payload slices.
    fn payload_slices(&self) -> Self::PayloadSlices<'_>;

    /// Returns a contiguous borrowed payload view when available without copying.
    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        None
    }
}

#[cfg(feature = "zero-copy-transport")]
/// Raw encoded receive object that can prove its contiguous payload is loan-backed.
pub trait UEncodedLoanedRxFrame: UEncodedRxFrame {
    /// Returns one contiguous loan-backed application payload view.
    ///
    /// Implementations must not allocate, copy, or coalesce payload bytes to
    /// satisfy this method.
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError>;
}

/// Public zero-copy receive lease after selected-wire metadata validation.
pub struct UWireRx<Rx, W, C>
where
    W: UWire,
{
    metadata: UFrameMetadata,
    native_profile: Option<crate::NativeProfileAgreement>,
    raw: Rx,
    _wire: PhantomData<W>,
    _metadata_codec: PhantomData<C>,
}

impl<Rx, W, C> core::fmt::Debug for UWireRx<Rx, W, C>
where
    W: UWire,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UWireRx").finish_non_exhaustive()
    }
}

impl<Rx, W, C> UWireRx<Rx, W, C>
where
    Rx: UEncodedRxFrame,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    /// Decodes metadata from a raw encoded receive object and validates the public frame view.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata bytes are malformed, selected-wire checks
    /// fail, or the resulting public frame view violates transport invariants.
    pub fn try_from_encoded(raw: Rx, codec: &C) -> Result<Self, UStatus> {
        Self::from_encoded_in_context(raw, codec, None)
    }

    /// Decodes a view while retaining the confirmed agreement for its route.
    /// Payload bytes remain opaque until an explicit typed operation is requested.
    ///
    /// # Errors
    ///
    /// Returns the same framing/metadata errors as [`Self::try_from_encoded`].
    pub fn try_from_encoded_with_native_profile(
        raw: Rx,
        codec: &C,
        profile: crate::NativeProfileAgreement,
    ) -> Result<Self, UStatus> {
        Self::from_encoded_in_context(raw, codec, Some(profile))
    }

    fn from_encoded_in_context(
        raw: Rx,
        codec: &C,
        native_profile: Option<crate::NativeProfileAgreement>,
    ) -> Result<Self, UStatus> {
        let metadata =
            codec.decode_frame_metadata(W::metadata_context(), raw.encoded_metadata())?;
        let frame = Self {
            metadata,
            native_profile,
            raw,
            _wire: PhantomData,
            _metadata_codec: PhantomData,
        };
        validate_frame_view_for_transport(&frame)?;
        Ok(frame)
    }

    /// Returns the raw encoded receive object behind this public wrapper.
    #[must_use]
    pub fn raw(&self) -> &Rx {
        &self.raw
    }

    /// Consumes the public wrapper and returns the raw encoded receive object.
    #[must_use]
    pub fn into_raw(self) -> Rx {
        self.raw
    }

    /// Decodes this frame's payload using the selected wire `W`.
    ///
    /// Prefer this selected-wire helper on receive values produced by
    /// explicit selected-wire adapter construction. Use
    /// [`UFrameView::decode_payload_from_reader_as`] only for
    /// explicit low-level codec selection.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame has missing or incompatible payload encoding,
    /// has no payload, or if the selected wire cannot decode the payload bytes.
    pub fn decode_payload<T>(&self, limit: PayloadDecodeLimit) -> Result<T, UWireError>
    where
        W: UWirePayload<T>,
        <W as UWirePayload<T>>::Codec: ReadDecodePayload<T>,
    {
        <<W as UWirePayload<T>>::Codec as crate::payload::codec::PayloadCodec>::verify_metadata(
            &self.metadata,
            self.native_profile.as_ref(),
        )?;
        if !self.has_payload() {
            return Err(UWireError::MissingPayload);
        }
        <W as UWirePayload<T>>::Codec::decode_payload_from_reader(
            self.payload_reader(),
            self.payload_len(),
            limit,
        )
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<Rx, W, C> UWireRx<Rx, W, C>
where
    Rx: UEncodedLoanedRxFrame,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    /// Borrows this frame's payload through the selected wire mapping.
    ///
    /// # Errors
    ///
    /// Returns an error for missing or incompatible encoding, absent/non-loaned
    /// payload storage, invalid size/alignment or invalid field bits.
    pub fn borrow_payload<T>(&self) -> Result<&T, UWireError>
    where
        W: UWirePayload<T>,
        <W as UWirePayload<T>>::Codec: BorrowPayload<T>,
    {
        <<W as UWirePayload<T>>::Codec as crate::PayloadCodec>::verify_metadata(
            &self.metadata,
            self.native_profile.as_ref(),
        )?;
        if !self.has_payload() {
            return Err(UWireError::MissingPayload);
        }
        let payload = self.raw.loaned_contiguous_payload()?;
        <<W as UWirePayload<T>>::Codec as BorrowPayload<T>>::borrow_payload(payload.bytes())
    }

    /// Borrows this frame's payload through the selected wire's expert lane.
    ///
    /// Encoding, payload presence and loan-backed contiguity remain checked.
    /// `unchecked` permits but does not guarantee bit-validation elision.
    ///
    /// # Safety
    ///
    /// The caller must prove closed producer provenance, typed construction of
    /// the exact requested Rust representation and recursive bit validity for
    /// `T`. Wire/profile identity, size, alignment and loan-backed contiguity
    /// remain checked; only the recursive bit validator may be elided.
    pub unsafe fn borrow_payload_unchecked<T>(&self) -> Result<&T, UWireError>
    where
        W: UWirePayload<T>,
        <W as UWirePayload<T>>::Codec: BorrowPayload<T>,
    {
        <<W as UWirePayload<T>>::Codec as crate::PayloadCodec>::verify_metadata(
            &self.metadata,
            self.native_profile.as_ref(),
        )?;
        if !self.has_payload() {
            return Err(UWireError::MissingPayload);
        }
        let payload = self.raw.loaned_contiguous_payload()?;
        unsafe {
            <<W as UWirePayload<T>>::Codec as BorrowPayload<T>>::borrow_payload_unchecked(
                payload.bytes(),
            )
        }
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<TCore, W, C> UWireTransport<TCore, W, C>
where
    TCore: UZeroCopyTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    /// Encodes a typed value directly into an initialized selected-wire TX loan.
    ///
    /// # Errors
    ///
    /// Returns an error for incompatible metadata, layout, loan, encoding or send
    /// failures.
    pub async fn send_initialized_payload<T>(
        &self,
        metadata: UFrameMetadata,
        value: &T,
    ) -> Result<(), UStatus>
    where
        W: UWirePayload<T>,
        <W as UWirePayload<T>>::Codec: crate::EncodePayload<T>,
    {
        <<W as UWirePayload<T>>::Codec as crate::PayloadCodec>::verify_metadata(
            &metadata,
            self.native_profile.as_ref(),
        )
        .map_err(UStatus::from)?;
        let layout = <W as UWirePayload<T>>::Codec::payload_layout(value).map_err(UStatus::from)?;
        let spec = UTxLoanSpec::payload(metadata, layout.len(), layout.align())?;
        let mut buffer = self.loan_validated_tx(spec).await?;
        <W as UWirePayload<T>>::Codec::encode_payload(value, buffer.payload_mut())
            .map_err(UStatus::from)?;
        self.send_validated_zero_copy(buffer).await
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<TCore, W, C> UWireTransport<TCore, W, C>
where
    TCore: UZeroCopyUninitTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    /// Initializes a generic selected-wire uninitialized TX loan and sends it.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid metadata/layout, initialization, loan or send
    /// failures.
    pub async fn send_uninit_payload<F>(
        &self,
        metadata: UFrameMetadata,
        payload_len: usize,
        payload_alignment: usize,
        initialize: F,
    ) -> Result<(), UStatus>
    where
        F: FnOnce(TCore::UninitTx) -> Result<TCore::Tx, UStatus> + Send,
    {
        let spec = UTxLoanSpec::payload(metadata, payload_len, payload_alignment)?;
        let buffer = self.loan_validated_uninit_tx(spec).await?;
        let buffer = initialize(buffer)?;
        self.send_validated_zero_copy(buffer).await
    }

    /// Initializes a stable payload through its generated typestate and sends it.
    ///
    /// # Errors
    ///
    /// Returns an error for incompatible encoding/layout, incomplete or invalid
    /// initialization, loan or send failures.
    pub async fn send_stable_payload<T, F>(
        &self,
        metadata: UFrameMetadata,
        initialize: F,
    ) -> Result<(), UStatus>
    where
        T: crate::StablePayload + crate::StablePayloadInit,
        W: UWirePayload<T, Codec = crate::StableContainerPayload<T>>,
        F: for<'a> FnOnce(
                USelectedWireStablePayloadInit<'a, T>,
            ) -> crate::InitializedStablePayload<'a, T>
            + Send,
    {
        crate::StableContainerPayload::<T>::verify_metadata(
            &metadata,
            self.native_profile.as_ref(),
        )
        .map_err(UStatus::from)?;
        let spec = UTxLoanSpec::payload(
            metadata,
            core::mem::size_of::<T>(),
            core::mem::align_of::<T>(),
        )?;
        let mut buffer = self.loan_validated_uninit_tx(spec).await?;
        let payload_address = buffer.payload_uninit_mut().as_mut_ptr().cast::<u8>();
        let init = T::init(buffer.payload_uninit_mut()).map_err(UStatus::from)?;
        let initialized = initialize(USelectedWireStablePayloadInit {
            initializer: init,
            _lifetime: PhantomData,
        });
        if core::mem::size_of::<T>() != 0 && initialized.as_bytes().as_ptr() != payload_address {
            return Err(UStatus::from(UWireError::invalid_payload(
                "stable initializer proof does not belong to the selected TX loan",
            )));
        }
        if !T::validate_field_bytes(initialized.as_bytes()) {
            return Err(UStatus::from(UWireError::invalid_payload(
                "stable initializer produced invalid field bits",
            )));
        }
        // SAFETY: Generated typestate and the validator prove complete stable
        // payload initialization before the buffer becomes sendable.
        let buffer = unsafe { buffer.assume_payload_initialized() };
        self.send_validated_zero_copy(buffer).await
    }
}

impl<Rx, W, C> UFrameView for UWireRx<Rx, W, C>
where
    Rx: UEncodedRxFrame,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    type PayloadReader<'a>
        = Rx::PayloadReader<'a>
    where
        Self: 'a;
    type PayloadSlices<'a>
        = Rx::PayloadSlices<'a>
    where
        Self: 'a;

    fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    fn native_profile(&self) -> Option<&crate::NativeProfileAgreement> {
        self.native_profile.as_ref()
    }

    fn payload_len(&self) -> usize {
        self.raw.payload_len()
    }

    fn has_payload(&self) -> bool {
        self.payload_len() > 0 || self.metadata.payload_encoding().is_some()
    }

    fn payload_reader(&self) -> Self::PayloadReader<'_> {
        self.raw.payload_reader()
    }

    fn payload_slices(&self) -> Self::PayloadSlices<'_> {
        self.raw.payload_slices()
    }

    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        self.raw.try_contiguous_payload()
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<Rx, W, C> UZeroCopyRxLease for UWireRx<Rx, W, C>
where
    Rx: UEncodedRxFrame,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
}

#[cfg(feature = "zero-copy-transport")]
impl<Rx, C> ULoanedContiguousZeroCopyRxFrame for UWireRx<Rx, StableContainerWireFormat, C>
where
    Rx: UEncodedLoanedRxFrame,
    C: UWireMetadataCodecFor<StableContainerWireFormat>,
{
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError> {
        self.raw.loaned_contiguous_payload()
    }
}

#[cfg(feature = "zero-copy-transport")]
/// Listener used by cores to deliver raw encoded zero-copy receive objects.
#[async_trait]
pub trait UEncodedZeroCopyListener<Rx>: Send + Sync
where
    Rx: UEncodedRxFrame + Send + 'static,
{
    /// Handles one raw encoded receive object.
    async fn on_receive_encoded_zero_copy(&self, frame: Rx);
}

/// *Role: implemented by transports that stay a dumb byte pipe; [`UWireTransport`](crate::UWireTransport) composes wires and codecs above it (recommended default) — see the [trait map](crate::guide::trait_map).*
///
#[cfg(feature = "zero-copy-transport")]
/// Encoded physical zero-copy mechanics implemented by product transports.
///
/// Implementing this core buys [`UWireTransport`] composition with every
/// compatible `UWire` and metadata codec. The prepared loan spec contains
/// metadata bytes already encoded by the selected profile; this trait must
/// carry them without interpreting, relabeling, or re-encoding them.
///
/// TX loan and commit are required. Pull receive and listener hooks default to
/// unsupported. `UZeroCopyUninitTransportCore` adds one uninitialized-loan
/// operation, while the adapter supplies validation, identity checks, stable
/// initialization, semantic public traits, and listener wrappers.
#[async_trait]
pub trait UZeroCopyTransportCore: Send + Sync {
    /// Transport-specific transmit loan type.
    type Tx: UTxBuffer + Send;

    /// Transport-specific raw encoded receive type.
    type Rx: UEncodedRxFrame + Send + 'static;

    /// Reserves transmit storage for a request with already encoded metadata bytes.
    async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus>;

    /// Commits a transmit loan prepared by this core.
    async fn send_prepared_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus>;

    /// Receives one matching raw encoded frame from cores that support pull receive.
    async fn receive_encoded_zero_copy(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        Err(unimplemented())
    }

    /// Registers a raw encoded listener after public filter validation.
    async fn register_encoded_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        Err(unimplemented())
    }

    /// Unregisters a raw encoded listener after public filter validation.
    ///
    /// On success no new callback may enter for this registration. A binding
    /// may let an already-entered callback finish under its documented policy.
    /// Previously delivered receive objects retain their storage independently.
    async fn unregister_encoded_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        Err(unimplemented())
    }
}

#[cfg(feature = "zero-copy-transport")]
/// Optional encoded-core capability for uninitialized transmit loans.
///
/// This one additional operation enables the adapter's checked two-phase stable
/// initialization paths. The core returns storage matching the prepared layout;
/// the generic layer owns initialization witnesses and commit eligibility.
#[async_trait]
pub trait UZeroCopyUninitTransportCore: UZeroCopyTransportCore {
    /// Transport-specific uninitialized transmit loan type.
    type UninitTx: UUninitTxBuffer<Initialized = Self::Tx> + Send;

    /// Reserves uninitialized storage for a request with already encoded metadata bytes.
    async fn loan_prepared_uninit_tx(
        &self,
        spec: PreparedTxLoanSpec,
    ) -> Result<Self::UninitTx, UStatus>;
}

#[cfg(feature = "zero-copy-transport")]
#[async_trait]
impl<TCore, W, C> UZeroCopyTransportImpl for UWireTransport<TCore, W, C>
where
    TCore: UZeroCopyTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    type Tx = TCore::Tx;
    type Rx = UWireRx<TCore::Rx, W, C>;

    async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
        self.core
            .loan_prepared_tx(PreparedTxLoanSpec::from_validated::<W, C>(
                spec,
                &self.metadata_codec,
            )?)
            .await
    }

    async fn send_validated_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
        self.core.send_prepared_zero_copy(buffer).await
    }

    async fn receive_validated_zero_copy(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        let core_source_filter = selected_wire_core_source_filter_for(source_filter);
        loop {
            let frame = self
                .core
                .receive_encoded_zero_copy(&core_source_filter, sink_filter)
                .await?;
            let frame = UWireRx::from_encoded_in_context(
                frame,
                &self.metadata_codec,
                self.native_profile.clone(),
            )?;
            if wire_frame_matches(&frame, source_filter, sink_filter) {
                return Ok(frame);
            }
        }
    }

    async fn register_validated_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        let key = listener_key(
            source_filter,
            sink_filter,
            zero_copy_listener_pointer::<TCore::Rx, W, C>(&listener),
        );
        let (listener, inserted) =
            self.registered_zero_copy_listener(&key, source_filter, sink_filter, listener);
        let core_source_filter = selected_wire_core_source_filter_for(source_filter);
        let result = self
            .core
            .register_encoded_zero_copy_listener(&core_source_filter, sink_filter, listener)
            .await;
        if result.is_err() && inserted {
            self.zero_copy_listeners
                .lock()
                .expect("wire zero-copy listener registry lock poisoned")
                .remove(&key);
        }
        result
    }

    async fn unregister_validated_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        let key = listener_key(
            source_filter,
            sink_filter,
            zero_copy_listener_pointer::<TCore::Rx, W, C>(&listener),
        );
        let listener = self.zero_copy_listener_for_unregister(&key, listener);
        let core_source_filter = selected_wire_core_source_filter_for(source_filter);
        let result = self
            .core
            .unregister_encoded_zero_copy_listener(&core_source_filter, sink_filter, listener)
            .await;
        if result.is_ok() {
            self.zero_copy_listeners
                .lock()
                .expect("wire zero-copy listener registry lock poisoned")
                .remove(&key);
        }
        result
    }
}

#[cfg(feature = "zero-copy-transport")]
#[async_trait]
impl<TCore, W, C> UZeroCopyUninitTransportImpl for UWireTransport<TCore, W, C>
where
    TCore: UZeroCopyUninitTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    type UninitTx = TCore::UninitTx;

    async fn loan_validated_uninit_tx(&self, spec: UTxLoanSpec) -> Result<Self::UninitTx, UStatus> {
        self.core
            .loan_prepared_uninit_tx(PreparedTxLoanSpec::from_validated::<W, C>(
                spec,
                &self.metadata_codec,
            )?)
            .await
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<TCore, W, C> UWireTransport<TCore, W, C>
where
    TCore: UZeroCopyTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    fn registered_zero_copy_listener(
        &self,
        key: &WireListenerKey,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<UWireRx<TCore::Rx, W, C>>>,
    ) -> (Arc<dyn UEncodedZeroCopyListener<TCore::Rx>>, bool) {
        let mut registry = self
            .zero_copy_listeners
            .lock()
            .expect("wire zero-copy listener registry lock poisoned");
        if let Some(existing) = registry.get(key) {
            if let Ok(existing) = existing
                .clone()
                .downcast::<WireZeroCopyListener<TCore::Rx, W, C>>()
            {
                return (existing, false);
            }
        }

        let wrapped = Arc::new(WireZeroCopyListener::<TCore::Rx, W, C> {
            source_filter: source_filter.clone(),
            sink_filter: sink_filter.cloned(),
            listener,
            metadata_codec: self.metadata_codec.clone(),
            native_profile: self.native_profile.clone(),
            _wire: PhantomData,
        });
        registry.insert(key.clone(), wrapped.clone());
        (wrapped, true)
    }

    fn zero_copy_listener_for_unregister(
        &self,
        key: &WireListenerKey,
        fallback: Arc<dyn UZeroCopyListener<UWireRx<TCore::Rx, W, C>>>,
    ) -> Arc<dyn UEncodedZeroCopyListener<TCore::Rx>> {
        self.zero_copy_listeners
            .lock()
            .expect("wire zero-copy listener registry lock poisoned")
            .get(key)
            .and_then(|listener| {
                listener
                    .clone()
                    .downcast::<WireZeroCopyListener<TCore::Rx, W, C>>()
                    .ok()
            })
            .unwrap_or_else(|| {
                Arc::new(WireZeroCopyListener::<TCore::Rx, W, C> {
                    source_filter: key.source_filter.clone(),
                    sink_filter: key.sink_filter.clone(),
                    listener: fallback,
                    metadata_codec: self.metadata_codec.clone(),
                    native_profile: self.native_profile.clone(),
                    _wire: PhantomData,
                })
            })
    }
}

#[cfg(feature = "zero-copy-transport")]
struct WireZeroCopyListener<Rx, W, C>
where
    Rx: UEncodedRxFrame + Send + 'static,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: Arc<dyn UZeroCopyListener<UWireRx<Rx, W, C>>>,
    metadata_codec: C,
    native_profile: Option<crate::NativeProfileAgreement>,
    _wire: PhantomData<W>,
}

#[cfg(feature = "zero-copy-transport")]
#[async_trait]
impl<Rx, W, C> UEncodedZeroCopyListener<Rx> for WireZeroCopyListener<Rx, W, C>
where
    Rx: UEncodedRxFrame + Send + 'static,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Send + Sync + 'static,
{
    async fn on_receive_encoded_zero_copy(&self, frame: Rx) {
        match UWireRx::<Rx, W, C>::from_encoded_in_context(
            frame,
            &self.metadata_codec,
            self.native_profile.clone(),
        ) {
            Ok(frame)
                if wire_frame_matches(&frame, &self.source_filter, self.sink_filter.as_ref()) =>
            {
                self.listener.on_receive_zero_copy(frame).await;
            }
            Ok(_) => {}
            Err(error) => warn!(%error, "dropping invalid selected-wire zero-copy frame"),
        }
    }
}

#[cfg(feature = "zero-copy-transport")]
fn wire_frame_matches<Rx, W, C>(
    frame: &UWireRx<Rx, W, C>,
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> bool
where
    Rx: UEncodedRxFrame,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    selected_wire_metadata_matches(frame.metadata(), source_filter, sink_filter)
}

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
fn selected_wire_metadata_matches(
    metadata: &UFrameMetadata,
    source: &UUri,
    sink: Option<&UUri>,
) -> bool {
    source.matches(metadata.source())
        && match (sink, metadata.sink()) {
            (None, None) => true,
            (Some(filter), Some(actual)) => filter.matches(actual),
            _ => false,
        }
}

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
fn selected_wire_core_source_filter_for(source_filter: &UUri) -> UUri {
    // Preserve useful authority/entity scope even in a partial wildcard. The
    // binding chooses any necessary physical broadening; decoded metadata is
    // still filtered independently before public delivery.
    source_filter.clone()
}

#[cfg(feature = "owned-frame-transport")]
/// Prepared owned frame passed from the adapter to an owned core.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedOwnedFrame {
    metadata: UFrameMetadata,
    encoded_metadata: Vec<u8>,
    payload: Option<Bytes>,
}

#[cfg(feature = "owned-frame-transport")]
impl PreparedOwnedFrame {
    /// Encodes validated owned frame metadata for a selected wire.
    ///
    /// # Errors
    ///
    /// Returns an error if selected-wire metadata encoding fails.
    pub fn from_validated<W, C>(frame: UOwnedFrame, codec: &C) -> Result<Self, UStatus>
    where
        W: UWire,
        C: UWireMetadataCodecFor<W>,
    {
        let (metadata, payload) = frame.into_parts();
        let encoded_metadata = codec.encode_frame_metadata(W::metadata_context(), &metadata)?;
        Ok(Self {
            metadata,
            encoded_metadata,
            payload,
        })
    }

    /// Returns decoded frame metadata.
    #[must_use]
    pub fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    /// Returns selected-wire encoded metadata bytes.
    #[must_use]
    pub fn encoded_metadata(&self) -> &[u8] {
        &self.encoded_metadata
    }

    /// Returns owned payload bytes, if present.
    #[must_use]
    pub fn payload(&self) -> Option<&Bytes> {
        self.payload.as_ref()
    }

    /// Consumes this frame and returns its parts.
    #[must_use]
    pub fn into_parts(self) -> (UFrameMetadata, Vec<u8>, Option<Bytes>) {
        (self.metadata, self.encoded_metadata, self.payload)
    }
}

#[cfg(feature = "owned-frame-transport")]
/// Encoded owned frame returned by an owned core.
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedOwnedFrame {
    encoded_metadata: Vec<u8>,
    payload: Option<Bytes>,
}

#[cfg(feature = "owned-frame-transport")]
impl EncodedOwnedFrame {
    /// Creates an encoded owned frame from selected-wire metadata bytes and payload.
    #[must_use]
    pub fn new(encoded_metadata: impl Into<Vec<u8>>, payload: Option<Bytes>) -> Self {
        Self {
            encoded_metadata: encoded_metadata.into(),
            payload,
        }
    }

    /// Returns selected-wire encoded metadata bytes.
    #[must_use]
    pub fn encoded_metadata(&self) -> &[u8] {
        &self.encoded_metadata
    }

    /// Returns owned payload bytes, if present.
    #[must_use]
    pub fn payload(&self) -> Option<&Bytes> {
        self.payload.as_ref()
    }

    /// Decodes this raw frame into a public owned frame.
    ///
    /// # Errors
    ///
    /// Returns an error if selected-wire metadata decode or owned-frame validation fails.
    pub fn decode<W, C>(self, codec: &C) -> Result<UOwnedFrame, UStatus>
    where
        W: UWire,
        C: UWireMetadataCodecFor<W>,
    {
        let metadata =
            codec.decode_frame_metadata(W::metadata_context(), &self.encoded_metadata)?;
        UOwnedFrame::new(metadata, self.payload).map_err(invalid_metadata)
    }

    /// Consumes this frame and returns its parts.
    #[must_use]
    pub fn into_parts(self) -> (Vec<u8>, Option<Bytes>) {
        (self.encoded_metadata, self.payload)
    }
}

#[cfg(feature = "owned-frame-transport")]
/// Listener used by cores to deliver raw encoded owned frames.
#[async_trait]
pub trait UEncodedOwnedListener: Send + Sync {
    /// Handles one raw encoded owned frame.
    async fn on_receive_encoded_owned(&self, frame: EncodedOwnedFrame);
}

/// *Role: implemented by transports carrying already-encoded owned frames; the wire adapter composes above it — see the [trait map](crate::guide::trait_map).*
///
#[cfg(feature = "owned-frame-transport")]
/// Encoded physical owned-frame mechanics implemented by product transports.
///
/// Implementing this core buys selected-wire metadata encoding/decoding,
/// identity rejection, semantic frame validation, and the public owned-frame
/// transport API from [`UWireTransport`]. The required send receives metadata
/// already encoded for the selected profile. Pull receive and listener hooks
/// default to unsupported.
#[async_trait]
pub trait UOwnedTransportCore: Send + Sync {
    /// Sends an owned frame with already encoded metadata bytes.
    async fn send_prepared_owned(&self, frame: PreparedOwnedFrame) -> Result<(), UStatus>;

    /// Receives one matching raw encoded owned frame from cores that support pull receive.
    async fn receive_encoded_owned(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<EncodedOwnedFrame, UStatus> {
        Err(unimplemented())
    }

    /// Registers a raw encoded owned listener after public filter validation.
    async fn register_encoded_owned_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UEncodedOwnedListener>,
    ) -> Result<(), UStatus> {
        Err(unimplemented())
    }

    /// Unregisters a raw encoded owned listener after public filter validation.
    async fn unregister_encoded_owned_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UEncodedOwnedListener>,
    ) -> Result<(), UStatus> {
        Err(unimplemented())
    }
}

#[cfg(feature = "owned-frame-transport")]
#[async_trait]
impl<TCore, W, C> UOwnedTransportImpl for UWireTransport<TCore, W, C>
where
    TCore: UOwnedTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    async fn send_validated_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus> {
        self.core
            .send_prepared_owned(PreparedOwnedFrame::from_validated::<W, C>(
                frame,
                &self.metadata_codec,
            )?)
            .await
    }

    async fn receive_validated_owned(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<UOwnedFrame, UStatus> {
        let core_source_filter = selected_wire_core_source_filter_for(source_filter);
        loop {
            let frame = self
                .core
                .receive_encoded_owned(&core_source_filter, sink_filter)
                .await?
                .decode::<W, C>(&self.metadata_codec)?;
            if owned_frame_matches(&frame, source_filter, sink_filter) {
                return Ok(frame);
            }
        }
    }

    async fn register_validated_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        let key = listener_key(
            source_filter,
            sink_filter,
            owned_listener_pointer(&listener),
        );
        let (listener, inserted) =
            self.registered_owned_listener(&key, source_filter, sink_filter, listener);
        let core_source_filter = selected_wire_core_source_filter_for(source_filter);
        let result = self
            .core
            .register_encoded_owned_listener(&core_source_filter, sink_filter, listener)
            .await;
        if result.is_err() && inserted {
            self.owned_listeners
                .lock()
                .expect("wire owned listener registry lock poisoned")
                .remove(&key);
        }
        result
    }

    async fn unregister_validated_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        let key = listener_key(
            source_filter,
            sink_filter,
            owned_listener_pointer(&listener),
        );
        let listener = self.owned_listener_for_unregister(&key, listener);
        let core_source_filter = selected_wire_core_source_filter_for(source_filter);
        let result = self
            .core
            .unregister_encoded_owned_listener(&core_source_filter, sink_filter, listener)
            .await;
        if result.is_ok() {
            self.owned_listeners
                .lock()
                .expect("wire owned listener registry lock poisoned")
                .remove(&key);
        }
        result
    }
}

#[cfg(feature = "owned-frame-transport")]
impl<TCore, W, C> UWireTransport<TCore, W, C>
where
    TCore: UOwnedTransportCore,
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Clone + Send + Sync + 'static,
{
    fn registered_owned_listener(
        &self,
        key: &WireListenerKey,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> (Arc<dyn UEncodedOwnedListener>, bool) {
        let mut registry = self
            .owned_listeners
            .lock()
            .expect("wire owned listener registry lock poisoned");
        if let Some(existing) = registry.get(key) {
            if let Ok(existing) = existing.clone().downcast::<WireOwnedListener<W, C>>() {
                return (existing, false);
            }
        }

        let wrapped = Arc::new(WireOwnedListener::<W, C> {
            source_filter: source_filter.clone(),
            sink_filter: sink_filter.cloned(),
            listener,
            metadata_codec: self.metadata_codec.clone(),
            _wire: PhantomData,
        });
        registry.insert(key.clone(), wrapped.clone());
        (wrapped, true)
    }

    fn owned_listener_for_unregister(
        &self,
        key: &WireListenerKey,
        fallback: Arc<dyn UOwnedListener>,
    ) -> Arc<dyn UEncodedOwnedListener> {
        self.owned_listeners
            .lock()
            .expect("wire owned listener registry lock poisoned")
            .get(key)
            .and_then(|listener| listener.clone().downcast::<WireOwnedListener<W, C>>().ok())
            .unwrap_or_else(|| {
                Arc::new(WireOwnedListener::<W, C> {
                    source_filter: key.source_filter.clone(),
                    sink_filter: key.sink_filter.clone(),
                    listener: fallback,
                    metadata_codec: self.metadata_codec.clone(),
                    _wire: PhantomData,
                })
            })
    }
}

#[cfg(feature = "owned-frame-transport")]
struct WireOwnedListener<W, C>
where
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: Arc<dyn UOwnedListener>,
    metadata_codec: C,
    _wire: PhantomData<W>,
}

#[cfg(feature = "owned-frame-transport")]
#[async_trait]
impl<W, C> UEncodedOwnedListener for WireOwnedListener<W, C>
where
    W: UWire + Send + Sync + 'static,
    C: UWireMetadataCodecFor<W> + Send + Sync + 'static,
{
    async fn on_receive_encoded_owned(&self, frame: EncodedOwnedFrame) {
        match frame.decode::<W, C>(&self.metadata_codec) {
            Ok(frame)
                if owned_frame_matches(&frame, &self.source_filter, self.sink_filter.as_ref()) =>
            {
                self.listener.on_receive_owned(frame).await;
            }
            Ok(_) => {}
            Err(error) => warn!(%error, "dropping invalid selected-wire owned frame"),
        }
    }
}

#[cfg(feature = "owned-frame-transport")]
fn owned_frame_matches(
    frame: &UOwnedFrame,
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> bool {
    selected_wire_metadata_matches(frame.metadata(), source_filter, sink_filter)
}

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct WireListenerKey {
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: usize,
}

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
fn listener_key(
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
    listener: usize,
) -> WireListenerKey {
    WireListenerKey {
        source_filter: source_filter.clone(),
        sink_filter: sink_filter.cloned(),
        listener,
    }
}

#[cfg(feature = "zero-copy-transport")]
fn zero_copy_listener_pointer<Rx, W, C>(
    listener: &Arc<dyn UZeroCopyListener<UWireRx<Rx, W, C>>>,
) -> usize
where
    Rx: UEncodedRxFrame,
    W: UWire,
    C: UWireMetadataCodecFor<W>,
{
    let ptr = Arc::as_ptr(listener);
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

#[cfg(feature = "owned-frame-transport")]
fn owned_listener_pointer(listener: &Arc<dyn UOwnedListener>) -> usize {
    let ptr = Arc::as_ptr(listener);
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

#[cfg(any(feature = "zero-copy-transport", feature = "owned-frame-transport"))]
fn unimplemented() -> UStatus {
    UStatus::fail_with_code(UCode::Unimplemented, "not implemented")
}

#[cfg(feature = "owned-frame-transport")]
fn invalid_metadata(error: crate::UFrameMetadataError) -> UStatus {
    UStatus::fail_with_code(UCode::InvalidArgument, error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, io::Cursor, sync::Mutex as StdMutex};

    #[cfg(feature = "zero-copy-transport")]
    use std::sync::Arc;

    #[cfg(feature = "protobuf-support")]
    use protobuf::well_known_types::wrappers::StringValue;

    use super::*;
    use crate::wire::{UProtocolNativeWire, UWireMetadataCodec};
    #[cfg(feature = "zero-copy-transport")]
    use crate::{
        BorrowPayload, PayloadLoanProvenance, StableContainerPayload, StablePayloadField,
        UTxPayloadSpec, UVecRxLease, UVecTxBuffer, UVecUninitTxBuffer,
    };
    #[cfg(feature = "protobuf-support")]
    use crate::{EncodePayload, ProtobufPayload};
    use crate::{PayloadEncoding, UMessageBuilder};
    use test_case::test_case;

    #[derive(Clone)]
    struct RawRx {
        encoded_metadata: Vec<u8>,
        payload: Vec<u8>,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload, crate::StablePayloadInit)]
    #[stable_payload(type_name = "uprotocol.test.WireStableBytes")]
    struct WireStableBytes {
        bytes: [u8; 4],
    }

    #[cfg(feature = "zero-copy-transport")]
    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.WireBool")]
    struct WireBool {
        value: bool,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.WireNestedInner")]
    struct WireNestedInner {
        value: bool,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.WireNested")]
    struct WireNested {
        inner: WireNestedInner,
        padding: [u8; 3],
        letter: char,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.WireAligned")]
    struct WireAligned {
        value: u32,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[derive(Debug)]
    struct CheckedDefaultCodec;

    #[cfg(feature = "zero-copy-transport")]
    impl crate::PayloadCodecIdentity for CheckedDefaultCodec {
        fn name() -> &'static str {
            "checked-default-test"
        }

        fn encoding() -> PayloadEncoding {
            PayloadEncoding::RAW
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    unsafe impl BorrowPayload<WireBool> for CheckedDefaultCodec {
        fn borrow_payload(src: &[u8]) -> Result<&WireBool, UWireError> {
            if src.len() != core::mem::size_of::<WireBool>() || !WireBool::validate_field_bytes(src)
            {
                return Err(UWireError::invalid_payload("invalid checked-default bool"));
            }
            // SAFETY: WireBool has alignment one and its bool byte was checked.
            Ok(unsafe { &*src.as_ptr().cast::<WireBool>() })
        }
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

    #[cfg(feature = "zero-copy-transport")]
    impl UEncodedLoanedRxFrame for RawRx {
        fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError> {
            // SAFETY: This test raw receive type models a transport-owned
            // contiguous receive loan for the selected-wire wrapper proof.
            Ok(unsafe {
                LoanedPayload::new_unchecked(
                    self.payload.as_slice(),
                    PayloadLoanProvenance::OpaqueTransportLoan,
                )
            })
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    #[repr(align(8))]
    struct AlignedPayloadStorage([u8; 5]);

    #[cfg(feature = "zero-copy-transport")]
    struct MisalignedRawRx {
        encoded_metadata: Vec<u8>,
        storage: AlignedPayloadStorage,
    }

    #[cfg(feature = "zero-copy-transport")]
    impl MisalignedRawRx {
        fn payload(&self) -> &[u8] {
            &self.storage.0[1..]
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    impl UEncodedRxFrame for MisalignedRawRx {
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
            self.payload().len()
        }

        fn payload_reader(&self) -> Self::PayloadReader<'_> {
            Cursor::new(self.payload())
        }

        fn payload_slices(&self) -> Self::PayloadSlices<'_> {
            std::iter::once(self.payload())
        }

        fn try_contiguous_payload(&self) -> Option<&[u8]> {
            Some(self.payload())
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    impl UEncodedLoanedRxFrame for MisalignedRawRx {
        fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError> {
            // SAFETY: The payload is borrowed directly from this raw receive object.
            Ok(unsafe {
                LoanedPayload::new_unchecked(
                    self.payload(),
                    PayloadLoanProvenance::OpaqueTransportLoan,
                )
            })
        }
    }

    #[derive(Default)]
    struct RecordingCore {
        #[cfg(feature = "zero-copy-transport")]
        prepared: StdMutex<Vec<PreparedTxLoanSpec>>,
        #[cfg(feature = "zero-copy-transport")]
        sent: StdMutex<Vec<UVecTxBuffer>>,
        #[cfg(feature = "zero-copy-transport")]
        listeners: StdMutex<Vec<Arc<dyn UEncodedZeroCopyListener<RawRx>>>>,
        received: StdMutex<VecDeque<RawRx>>,
        receive_filters: StdMutex<Vec<(UUri, Option<UUri>)>>,
        listener_filters: StdMutex<Vec<(bool, UUri, Option<UUri>)>>,
        #[cfg(feature = "owned-frame-transport")]
        owned_listeners: StdMutex<Vec<Arc<dyn UEncodedOwnedListener>>>,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[async_trait]
    impl UZeroCopyTransportCore for RecordingCore {
        type Tx = UVecTxBuffer;
        type Rx = RawRx;

        async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
            self.prepared.lock().unwrap().push(spec.clone());
            UVecTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }

        async fn send_prepared_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
            self.sent.lock().unwrap().push(buffer);
            Ok(())
        }

        async fn receive_encoded_zero_copy(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
        ) -> Result<Self::Rx, UStatus> {
            self.receive_filters
                .lock()
                .unwrap()
                .push((source_filter.clone(), sink_filter.cloned()));
            self.received.lock().unwrap().pop_front().ok_or_else(|| {
                UStatus::fail_with_code(UCode::NotFound, "no test encoded frame available")
            })
        }

        async fn register_encoded_zero_copy_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
        ) -> Result<(), UStatus> {
            self.listener_filters.lock().unwrap().push((
                true,
                source_filter.clone(),
                sink_filter.cloned(),
            ));
            self.listeners.lock().unwrap().push(listener);
            Ok(())
        }

        async fn unregister_encoded_zero_copy_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
        ) -> Result<(), UStatus> {
            self.listener_filters.lock().unwrap().push((
                false,
                source_filter.clone(),
                sink_filter.cloned(),
            ));
            self.listeners
                .lock()
                .unwrap()
                .retain(|registered| !Arc::ptr_eq(registered, &listener));
            Ok(())
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    #[async_trait]
    impl UZeroCopyUninitTransportCore for RecordingCore {
        type UninitTx = UVecUninitTxBuffer;

        async fn loan_prepared_uninit_tx(
            &self,
            spec: PreparedTxLoanSpec,
        ) -> Result<Self::UninitTx, UStatus> {
            self.prepared.lock().unwrap().push(spec.clone());
            UVecUninitTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }
    }

    #[cfg(feature = "owned-frame-transport")]
    #[async_trait]
    impl UOwnedTransportCore for RecordingCore {
        async fn register_encoded_owned_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UEncodedOwnedListener>,
        ) -> Result<(), UStatus> {
            self.listener_filters.lock().unwrap().push((
                true,
                source_filter.clone(),
                sink_filter.cloned(),
            ));
            self.owned_listeners.lock().unwrap().push(listener);
            Ok(())
        }

        async fn unregister_encoded_owned_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UEncodedOwnedListener>,
        ) -> Result<(), UStatus> {
            self.listener_filters.lock().unwrap().push((
                false,
                source_filter.clone(),
                sink_filter.cloned(),
            ));
            self.owned_listeners
                .lock()
                .unwrap()
                .retain(|registered| !Arc::ptr_eq(registered, &listener));
            Ok(())
        }

        async fn send_prepared_owned(&self, _frame: PreparedOwnedFrame) -> Result<(), UStatus> {
            Ok(())
        }

        async fn receive_encoded_owned(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
        ) -> Result<EncodedOwnedFrame, UStatus> {
            self.receive_filters
                .lock()
                .unwrap()
                .push((source_filter.clone(), sink_filter.cloned()));
            self.received
                .lock()
                .unwrap()
                .pop_front()
                .map(|raw| EncodedOwnedFrame::new(raw.encoded_metadata, Some(raw.payload.into())))
                .ok_or_else(|| {
                    UStatus::fail_with_code(UCode::NotFound, "no test owned frame available")
                })
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    #[derive(Default)]
    struct RecordingZeroCopyListener<W: UWire = UProtocolNativeWire> {
        frames: StdMutex<Vec<UWireRx<RawRx, W, NativePrefixFrameMetadataCodec>>>,
    }

    #[cfg(feature = "zero-copy-transport")]
    #[async_trait]
    impl<W: UWire + Send + Sync + 'static>
        UZeroCopyListener<UWireRx<RawRx, W, NativePrefixFrameMetadataCodec>>
        for RecordingZeroCopyListener<W>
    {
        async fn on_receive_zero_copy(
            &self,
            frame: UWireRx<RawRx, W, NativePrefixFrameMetadataCodec>,
        ) {
            self.frames.lock().unwrap().push(frame);
        }
    }

    #[cfg(feature = "owned-frame-transport")]
    #[derive(Default)]
    struct RecordingOwnedListener(StdMutex<Vec<UOwnedFrame>>);

    #[cfg(feature = "owned-frame-transport")]
    #[async_trait]
    impl UOwnedListener for RecordingOwnedListener {
        async fn on_receive_owned(&self, frame: UOwnedFrame) {
            self.0.lock().unwrap().push(frame);
        }
    }

    #[cfg(feature = "owned-frame-transport")]
    #[tokio::test]
    async fn sinkless_owned_listener_does_not_duplicate_notifications() {
        use crate::UOwnedTransport;
        let transport = RecordingCore::default().into_protobuf_transport();
        let source = UUri::try_from_parts("source", 0x4210, 1, 0x8001).unwrap();
        let sink = UUri::try_from_parts("sink", 0x4210, 1, 0).unwrap();
        let publishes = Arc::new(RecordingOwnedListener::default());
        let notifications = Arc::new(RecordingOwnedListener::default());
        transport
            .register_owned_listener(&source, None, publishes.clone())
            .await
            .unwrap();
        transport
            .register_owned_listener(&source, Some(&sink), notifications.clone())
            .await
            .unwrap();
        let metadata = UFrameMetadata::notification(source, sink)
            .with_payload_encoding(PayloadEncoding::PROTOBUF)
            .build()
            .unwrap();
        let encoded = NativePrefixFrameMetadataCodec
            .encode_frame_metadata(ProtobufWire::metadata_context(), &metadata)
            .unwrap();
        let listeners = transport.core().owned_listeners.lock().unwrap().clone();
        for listener in listeners {
            listener
                .on_receive_encoded_owned(EncodedOwnedFrame::new(
                    encoded.clone(),
                    Some(Bytes::from_static(b"notify")),
                ))
                .await;
        }
        assert!(
            publishes.0.lock().unwrap().is_empty(),
            "None means no sink, not any sink"
        );
        assert_eq!(notifications.0.lock().unwrap().len(), 1);
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn sinkless_zero_copy_listener_does_not_duplicate_notifications() {
        let transport =
            RecordingCore::default().into_native_prefix_wire_transport(UProtocolNativeWire);
        let source = UUri::try_from_parts("source", 0x4210, 1, 0x8001).unwrap();
        let sink = UUri::try_from_parts("sink", 0x4210, 1, 0).unwrap();
        let publishes = Arc::new(RecordingZeroCopyListener::default());
        let notifications = Arc::new(RecordingZeroCopyListener::default());
        transport
            .register_validated_zero_copy_listener(&source, None, publishes.clone())
            .await
            .unwrap();
        transport
            .register_validated_zero_copy_listener(&source, Some(&sink), notifications.clone())
            .await
            .unwrap();
        let metadata = UFrameMetadata::notification(source, sink)
            .with_payload_encoding(PayloadEncoding::PROTOBUF)
            .build()
            .unwrap();
        let encoded = NativePrefixFrameMetadataCodec
            .encode_frame_metadata(UProtocolNativeWire::metadata_context(), &metadata)
            .unwrap();
        let listeners = transport.core().listeners.lock().unwrap().clone();
        for listener in listeners {
            listener
                .on_receive_encoded_zero_copy(RawRx {
                    encoded_metadata: encoded.clone(),
                    payload: b"notify".to_vec(),
                })
                .await;
        }
        assert!(
            publishes.frames.lock().unwrap().is_empty(),
            "None means no sink, not any sink"
        );
        assert_eq!(notifications.frames.lock().unwrap().len(), 1);
    }

    struct CompileCore;

    #[cfg(feature = "zero-copy-transport")]
    #[async_trait]
    impl UZeroCopyTransportCore for CompileCore {
        type Tx = UVecTxBuffer;
        type Rx = RawRx;

        async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
            UVecTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }

        async fn send_prepared_zero_copy(&self, _buffer: Self::Tx) -> Result<(), UStatus> {
            Ok(())
        }
    }

    #[cfg(feature = "zero-copy-transport")]
    #[async_trait]
    impl UZeroCopyUninitTransportCore for CompileCore {
        type UninitTx = UVecUninitTxBuffer;

        async fn loan_prepared_uninit_tx(
            &self,
            spec: PreparedTxLoanSpec,
        ) -> Result<Self::UninitTx, UStatus> {
            UVecUninitTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }
    }

    #[cfg(feature = "owned-frame-transport")]
    #[async_trait]
    impl UOwnedTransportCore for CompileCore {
        async fn send_prepared_owned(&self, _frame: PreparedOwnedFrame) -> Result<(), UStatus> {
            Ok(())
        }
    }

    fn metadata_with_payload() -> UFrameMetadata {
        metadata_with_payload_encoding(PayloadEncoding::RAW)
    }

    fn metadata_with_payload_encoding(payload_encoding: PayloadEncoding) -> UFrameMetadata {
        metadata_with_topic_and_payload_encoding(0x9000, payload_encoding)
    }

    #[cfg(feature = "zero-copy-transport")]
    fn stable_metadata<T: crate::StablePayload>() -> UFrameMetadata {
        let identity = StableContainerPayload::<T>::identity(&stable_profile()).unwrap();
        metadata_with_payload_encoding(identity.encoding())
            .with_native_payload_identity(identity)
            .unwrap()
    }

    #[cfg(feature = "zero-copy-transport")]
    fn stable_profile() -> crate::NativeProfileAgreement {
        use crate::{
            NativeProfile, NativeProfileAgreement, NativeProfileMode, NativeProfileTable,
            StablePayload,
        };
        let entries = [
            (0xF101, WireStableBytes::native_representation()),
            (0xF102, WireBool::native_representation()),
            (0xF103, WireNested::native_representation()),
            (0xF104, WireAligned::native_representation()),
        ]
        .map(|(id, representation)| (PayloadEncoding::from_id(id).unwrap(), representation));
        let profile = NativeProfile::new(
            "wire-tests",
            1,
            NativeProfileMode::Table(NativeProfileTable::new(entries).unwrap()),
        )
        .unwrap();
        NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap()
    }

    #[cfg(feature = "zero-copy-transport")]
    fn stable_rx<R: UEncodedRxFrame>(
        raw: R,
    ) -> UWireRx<R, StableContainerWireFormat, NativePrefixFrameMetadataCodec> {
        UWireRx::try_from_encoded_with_native_profile(
            raw,
            &NativePrefixFrameMetadataCodec,
            stable_profile(),
        )
        .unwrap()
    }

    fn metadata_with_topic_and_payload_encoding(
        resource_id: u16,
        payload_encoding: PayloadEncoding,
    ) -> UFrameMetadata {
        let topic = UUri::try_from_parts("vehicle", 0x4210, 0x01, resource_id).expect("topic URI");
        let message = UMessageBuilder::publish(topic).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(payload_encoding),
        )
        .expect("metadata")
    }

    fn raw_frame_for_topic(resource_id: u16, payload: &[u8]) -> RawRx {
        let metadata = metadata_with_topic_and_payload_encoding(resource_id, PayloadEncoding::RAW);
        RawRx {
            encoded_metadata: encode_metadata::<UProtocolNativeWire>(&metadata),
            payload: payload.to_vec(),
        }
    }

    fn encode_metadata<W>(metadata: &UFrameMetadata) -> Vec<u8>
    where
        W: UWire,
    {
        NativePrefixFrameMetadataCodec
            .encode_frame_metadata(W::metadata_context(), metadata)
            .unwrap()
    }

    #[test]
    fn wire_rx_decodes_metadata_and_delegates_payload() {
        let metadata = metadata_with_payload();
        let encoded_metadata = encode_metadata::<UProtocolNativeWire>(&metadata);
        let raw = RawRx {
            encoded_metadata,
            payload: b"abc".to_vec(),
        };

        let rx = UWireRx::<RawRx, UProtocolNativeWire, NativePrefixFrameMetadataCodec>::try_from_encoded(
            raw,
            &NativePrefixFrameMetadataCodec,
        )
        .unwrap();

        assert_eq!(rx.metadata(), &metadata);
        assert_eq!(rx.payload_len(), 3);
        assert_eq!(rx.try_contiguous_payload(), Some(&b"abc"[..]));
    }

    #[cfg(feature = "protobuf-support")]
    #[test]
    fn wire_rx_decodes_payload_with_selected_wire() {
        let value = StringValue {
            value: "selected-wire".to_string(),
            special_fields: Default::default(),
        };
        let payload = ProtobufWire::encode_payload_owned(&value).unwrap();
        let metadata = metadata_with_payload_encoding(ProtobufPayload::encoding());
        let encoded_metadata = encode_metadata::<ProtobufWire>(&metadata);
        let raw = RawRx {
            encoded_metadata,
            payload: payload.to_vec(),
        };

        let rx = UWireRx::<RawRx, ProtobufWire, NativePrefixFrameMetadataCodec>::try_from_encoded(
            raw,
            &NativePrefixFrameMetadataCodec,
        )
        .unwrap();
        let decoded: StringValue = rx.decode_payload(PayloadDecodeLimit::new(1024)).unwrap();

        assert_eq!(decoded.value, "selected-wire");
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn selected_stable_wire_safe_borrow_validates_encoding_and_bits() {
        let value = WireBool { value: true };
        let metadata = stable_metadata::<WireBool>();
        let valid = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![1],
        };
        let valid = stable_rx(valid);
        assert_eq!(valid.borrow_payload::<WireBool>().unwrap(), &value);

        let invalid = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![2],
        };
        let invalid = stable_rx(invalid);
        assert!(invalid.borrow_payload::<WireBool>().is_err());

        let wrong_metadata = metadata_with_payload_encoding(PayloadEncoding::RAW);
        let wrong = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&wrong_metadata),
            payload: vec![1],
        };
        let wrong = stable_rx(wrong);
        assert!(wrong.borrow_payload::<WireBool>().is_err());
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn selected_stable_wire_rejects_nested_bool_and_char_bits() {
        let metadata = stable_metadata::<WireNested>();
        let mut nested_bool = vec![0; core::mem::size_of::<WireNested>()];
        let bool_offset = core::mem::offset_of!(WireNested, inner)
            + core::mem::offset_of!(WireNestedInner, value);
        *nested_bool.get_mut(bool_offset).unwrap() = 2;
        let raw = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: nested_bool,
        };
        let rx = stable_rx(raw);
        assert!(rx.borrow_payload::<WireNested>().is_err());

        let mut invalid_char = vec![0; core::mem::size_of::<WireNested>()];
        *invalid_char.get_mut(bool_offset).unwrap() = 1;
        let char_offset = core::mem::offset_of!(WireNested, letter);
        invalid_char
            .get_mut(char_offset..char_offset + core::mem::size_of::<char>())
            .unwrap()
            .copy_from_slice(&0x0011_0000_u32.to_ne_bytes());
        let raw = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: invalid_char,
        };
        let rx = stable_rx(raw);
        assert!(rx.borrow_payload::<WireNested>().is_err());
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn selected_stable_wire_rejects_truncated_and_misaligned_payloads() {
        let metadata = stable_metadata::<WireAligned>();
        let truncated = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![0; core::mem::size_of::<WireAligned>() - 1],
        };
        let truncated = stable_rx(truncated);
        assert!(truncated.borrow_payload::<WireAligned>().is_err());

        let misaligned = MisalignedRawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            storage: AlignedPayloadStorage([0; 5]),
        };
        assert_ne!(
            (misaligned.payload().as_ptr() as usize) % core::mem::align_of::<WireAligned>(),
            0
        );
        let misaligned = stable_rx(misaligned);
        assert!(misaligned.borrow_payload::<WireAligned>().is_err());
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn stable_loan_bridge_rejects_malformed_and_wrong_encoding() {
        let metadata = stable_metadata::<WireBool>();
        let malformed = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![2],
        };
        let malformed = stable_rx(malformed);
        assert!(malformed
            .borrow_stable_payload::<WireBool>(&stable_profile())
            .is_err());

        let wrong_metadata = metadata_with_payload_encoding(PayloadEncoding::RAW);
        let wrong = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&wrong_metadata),
            payload: vec![1],
        };
        let wrong = stable_rx(wrong);
        assert!(wrong
            .borrow_stable_payload::<WireBool>(&stable_profile())
            .is_err());
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn inherited_unchecked_default_remains_checked() {
        assert!(unsafe {
            <CheckedDefaultCodec as BorrowPayload<WireBool>>::borrow_payload_unchecked(&[2])
        }
        .is_err());
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn selected_stable_wire_unchecked_lane_accepts_caller_proven_payload() {
        let metadata = stable_metadata::<WireBool>();
        let raw = RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![1],
        };
        let rx = stable_rx(raw);

        let borrowed = unsafe { rx.borrow_payload_unchecked::<WireBool>() }.unwrap();
        assert!(borrowed.value);
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test_case::test_case(false, None; "checked missing token")]
    #[test_case::test_case(false, Some(1); "checked wrong token")]
    #[test_case::test_case(true, None; "unchecked missing token")]
    #[test_case::test_case(true, Some(1); "unchecked wrong token")]
    fn native_identity_is_checked_in_both_borrow_lanes(unchecked: bool, delta: Option<u32>) {
        let original = stable_metadata::<WireBool>();
        let encoding = *original.payload_encoding().unwrap();
        let mut metadata = original
            .clone()
            .without_payload_encoding()
            .with_payload_encoding(encoding)
            .unwrap();
        if let Some(delta) = delta {
            metadata = metadata
                .with_native_type_token(crate::NativeTypeToken::from_u32(
                    original
                        .native_type_token()
                        .unwrap()
                        .as_u32()
                        .wrapping_add(delta),
                ))
                .unwrap();
        }
        let frame = stable_rx(RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![1],
        });
        let result = if unchecked {
            // SAFETY: This producer supplies a valid WireBool byte and alignment.
            // The intentionally wrong metadata is a checked precondition.
            unsafe { frame.borrow_payload_unchecked::<WireBool>() }
        } else {
            frame.borrow_payload::<WireBool>()
        };
        assert!(matches!(
            result,
            Err(UWireError::UnsupportedNativeTypeToken { .. })
        ));
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test_case::test_case(false; "checked missing profile")]
    #[test_case::test_case(true; "unchecked missing profile")]
    fn native_bytes_without_route_context_remain_opaque(unchecked: bool) {
        let metadata = stable_metadata::<WireBool>();
        let frame = UWireRx::<RawRx, StableContainerWireFormat, NativePrefixFrameMetadataCodec>::try_from_encoded(
            RawRx { encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata), payload: vec![1] },
            &NativePrefixFrameMetadataCodec,
        ).unwrap();
        assert_eq!(frame.try_contiguous_payload(), Some(&[1][..]));
        let result = if unchecked {
            // SAFETY: Known typed producer, exact WireBool representation and bits.
            unsafe { frame.borrow_payload_unchecked::<WireBool>() }
        } else {
            frame.borrow_payload::<WireBool>()
        };
        assert!(matches!(result, Err(UWireError::NativeProfile(_))));
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn received_frame_keeps_its_generation_when_a_route_is_replaced() {
        let metadata = stable_metadata::<WireBool>();
        let frame = stable_rx(RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![1],
        });
        let old = stable_profile();
        let next =
            crate::NativeProfile::new("wire-tests", 2, old.profile().mode().clone()).unwrap();
        let next = old.successor(Arc::new(next.clone()), &next).unwrap();
        assert!(frame.borrow_payload::<WireBool>().unwrap().value);
        assert!(matches!(
            frame.borrow_stable_payload::<WireBool>(&next),
            Err(UWireError::NativeProfile(
                crate::NativeProfileError::ProfileMismatch
            ))
        ));
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn configured_native_pull_receive_retains_profile_context() {
        let metadata = stable_metadata::<WireBool>();
        let source = metadata.source().clone();
        let core = RecordingCore::default();
        core.received.lock().unwrap().push_back(RawRx {
            encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
            payload: vec![1],
        });
        let transport = core.into_stable_container_transport(stable_profile());
        let frame = transport
            .receive_validated_zero_copy(&source, None)
            .await
            .unwrap();
        assert!(frame.borrow_payload::<WireBool>().unwrap().value);
        assert_eq!(
            frame.native_profile().unwrap().profile(),
            transport.native_profile().unwrap().profile()
        );
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn configured_native_listener_retains_profile_context() {
        let metadata = stable_metadata::<WireBool>();
        let transport = RecordingCore::default().into_stable_container_transport(stable_profile());
        let listener = Arc::new(RecordingZeroCopyListener::<StableContainerWireFormat>::default());
        transport
            .register_validated_zero_copy_listener(metadata.source(), None, listener.clone())
            .await
            .unwrap();
        let encoded_listener = transport
            .core()
            .listeners
            .lock()
            .unwrap()
            .first()
            .unwrap()
            .clone();
        encoded_listener
            .on_receive_encoded_zero_copy(RawRx {
                encoded_metadata: encode_metadata::<StableContainerWireFormat>(&metadata),
                payload: vec![1],
            })
            .await;
        let received = listener.frames.lock().unwrap();
        let frame = received
            .first()
            .expect("native listener received one frame");
        assert!(frame.borrow_payload::<WireBool>().unwrap().value);
        assert_eq!(
            frame.native_profile().unwrap().profile(),
            transport.native_profile().unwrap().profile()
        );
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test_case::test_case(false; "private table native TX")]
    #[test_case::test_case(true; "contract defined native TX")]
    #[tokio::test]
    async fn selected_wire_stable_initializer_sends_typed_payload(contract_defined: bool) {
        let profile = if contract_defined {
            let representation = <WireStableBytes as crate::StablePayload>::native_representation();
            let contract = crate::NativeContract::new("wire.test.event", representation).unwrap();
            let profile = crate::NativeProfile::new(
                "wire-tests",
                1,
                crate::NativeProfileMode::ContractDefined(contract),
            )
            .unwrap();
            crate::NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap()
        } else {
            stable_profile()
        };
        let identity = StableContainerPayload::<WireStableBytes>::identity(&profile).unwrap();
        let metadata = stable_metadata::<WireStableBytes>()
            .without_payload_encoding()
            .with_native_payload_identity(identity)
            .unwrap();
        let transport = RecordingCore::default().into_stable_container_transport(profile.clone());
        transport
            .send_stable_payload::<WireStableBytes, _>(metadata.clone(), |init| {
                init.into_initializer()
                    .bytes_from_slice(b"wire")
                    .unwrap()
                    .finish()
            })
            .await
            .unwrap();

        let sent = transport.core().sent.lock().unwrap();
        assert_eq!(sent.first().expect("one stable payload").payload(), b"wire");
        let prepared = transport.core().prepared.lock().unwrap();
        let encoded_metadata = prepared.first().unwrap().encoded_metadata().to_vec();
        let decoded = NativePrefixFrameMetadataCodec
            .decode_frame_metadata(
                StableContainerWireFormat::metadata_context(),
                &encoded_metadata,
            )
            .unwrap();
        assert_eq!(decoded, metadata);
        let received = UWireRx::<RawRx, StableContainerWireFormat, NativePrefixFrameMetadataCodec>::try_from_encoded_with_native_profile(
            RawRx { encoded_metadata, payload: b"wire".to_vec() }, &NativePrefixFrameMetadataCodec, profile
        ).unwrap();
        assert_eq!(
            received.borrow_payload::<WireStableBytes>().unwrap().bytes,
            *b"wire"
        );
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn native_tx_rejects_missing_token_before_loan_or_initialization() {
        let transport = RecordingCore::default().into_stable_container_transport(stable_profile());
        let metadata = stable_metadata::<WireStableBytes>();
        let encoding = *metadata.payload_encoding().unwrap();
        let metadata = metadata
            .without_payload_encoding()
            .with_payload_encoding(encoding)
            .unwrap();
        assert!(transport
            .send_stable_payload::<WireStableBytes, _>(metadata, |_| {
                panic!("invalid identity must not reach initialization")
            })
            .await
            .is_err());
        assert!(transport.core().prepared.lock().unwrap().is_empty());
        assert!(transport.core().sent.lock().unwrap().is_empty());
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn selected_wire_stable_initializer_rejects_foreign_proof() {
        let foreign = Box::into_raw(Box::new([core::mem::MaybeUninit::uninit(); 4]));
        // SAFETY: The allocation remains live until the proof is consumed by
        // the awaited call and is reclaimed below.
        let foreign_storage = unsafe { &mut *foreign };
        let foreign_proof = <WireStableBytes as crate::StablePayloadInit>::init(foreign_storage)
            .unwrap()
            .bytes_from_slice(b"away")
            .unwrap()
            .finish();
        let transport = RecordingCore::default().into_stable_container_transport(stable_profile());
        let result = transport
            .send_stable_payload::<WireStableBytes, _>(
                stable_metadata::<WireStableBytes>(),
                move |_init| foreign_proof,
            )
            .await;
        // SAFETY: The proof borrowing this allocation was consumed and dropped
        // before the awaited call returned, so ownership can be reconstructed.
        unsafe { drop(Box::from_raw(foreign)) };

        assert!(result.is_err());
        assert!(transport.core().sent.lock().unwrap().is_empty());
    }

    #[cfg(all(feature = "zero-copy-transport", feature = "protobuf-support"))]
    #[tokio::test]
    async fn selected_wire_initialized_and_generic_uninit_helpers_send() {
        let protobuf = RecordingCore::default().into_protobuf_transport();
        let value = StringValue {
            value: "selected".to_string(),
            special_fields: Default::default(),
        };
        protobuf
            .send_initialized_payload(
                metadata_with_payload_encoding(ProtobufPayload::encoding()),
                &value,
            )
            .await
            .unwrap();
        assert_eq!(protobuf.core().sent.lock().unwrap().len(), 1);

        let native =
            RecordingCore::default().into_native_prefix_wire_transport(UProtocolNativeWire);
        native
            .send_uninit_payload(metadata_with_payload(), 4, 1, |mut buffer| {
                for (slot, value) in buffer.payload_uninit_mut().iter_mut().zip(*b"wire") {
                    slot.write(value);
                }
                // SAFETY: Every payload slot was initialized in the loop above.
                Ok(unsafe { buffer.assume_payload_initialized() })
            })
            .await
            .unwrap();
        let sent = native.core().sent.lock().unwrap();
        assert_eq!(sent.first().expect("one raw payload").payload(), b"wire");
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn receive_filters_after_selected_wire_decode() {
        let core = RecordingCore::default();
        core.received.lock().unwrap().extend([
            raw_frame_for_topic(0x9001, b"drop"),
            raw_frame_for_topic(0x9000, b"keep"),
        ]);
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        let source_filter = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();

        let frame = transport
            .receive_validated_zero_copy(&source_filter, None)
            .await
            .expect("matching decoded frame");

        assert_eq!(frame.try_contiguous_payload(), Some(&b"keep"[..]));
        assert!(transport.core().received.lock().unwrap().is_empty());
        let filters = transport.core().receive_filters.lock().unwrap();
        assert_eq!(filters.len(), 2);
        let filter = filters.first().expect("first receive filter");
        assert_eq!(filter.0, source_filter);
        assert_eq!(filter.1, None);
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn receive_rejects_invalid_selected_wire_frame_before_later_frames() {
        let core = RecordingCore::default();
        core.received.lock().unwrap().extend([
            RawRx {
                encoded_metadata: b"invalid selected-wire metadata".to_vec(),
                payload: b"drop".to_vec(),
            },
            raw_frame_for_topic(0x9000, b"keep"),
        ]);
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        let source_filter = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();

        let status = match transport
            .receive_validated_zero_copy(&source_filter, None)
            .await
        {
            Ok(_) => panic!("invalid selected-wire metadata must be rejected"),
            Err(status) => status,
        };
        assert_eq!(status.code(), UCode::InvalidArgument);
        assert!(status
            .message()
            .is_some_and(|message| message.contains("wrong native-prefix metadata magic")));
        assert_eq!(transport.core().received.lock().unwrap().len(), 1);

        let frame = transport
            .receive_validated_zero_copy(&source_filter, None)
            .await
            .expect("later matching decoded frame");

        assert_eq!(frame.try_contiguous_payload(), Some(&b"keep"[..]));
        assert!(transport.core().received.lock().unwrap().is_empty());
        let filters = transport.core().receive_filters.lock().unwrap();
        assert_eq!(filters.len(), 2);
        let filter = filters.first().expect("first receive filter");
        assert_eq!(filter.0, source_filter);
        assert_eq!(filter.1, None);
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn receive_preserves_partial_core_source_filter() {
        let core = RecordingCore::default();
        core.received
            .lock()
            .unwrap()
            .push_back(raw_frame_for_topic(0x9000, b"keep"));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        let source_filter = UUri::try_from_parts("vehicle", 0x4210, 0x01, u16::MAX).unwrap();

        let frame = transport
            .receive_validated_zero_copy(&source_filter, None)
            .await
            .expect("matching decoded frame");

        assert_eq!(frame.try_contiguous_payload(), Some(&b"keep"[..]));
        let filters = transport.core().receive_filters.lock().unwrap();
        assert_eq!(filters.len(), 1);
        let filter = filters.first().expect("one receive filter");
        assert_eq!(filter.0, source_filter);
        assert_eq!(filter.1, None);
    }

    #[cfg(feature = "owned-frame-transport")]
    #[tokio::test]
    async fn owned_receive_uses_exact_core_filter_for_exact_source_filter() {
        let core = RecordingCore::default();
        core.received
            .lock()
            .unwrap()
            .push_back(raw_frame_for_topic(0x9000, b"owned"));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        let source_filter = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();

        let frame = transport
            .receive_validated_owned(&source_filter, None)
            .await
            .expect("matching owned frame");

        assert_eq!(frame.payload_bytes(), b"owned");
        let filters = transport.core().receive_filters.lock().unwrap();
        assert_eq!(filters.len(), 1);
        let filter = filters.first().expect("one receive filter");
        assert_eq!(filter.0, source_filter);
        assert_eq!(filter.1, None);
    }

    #[cfg(feature = "owned-frame-transport")]
    #[tokio::test]
    async fn owned_receive_preserves_partial_core_source_filter() {
        let core = RecordingCore::default();
        core.received
            .lock()
            .unwrap()
            .push_back(raw_frame_for_topic(0x9000, b"owned"));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        let source_filter = UUri::try_from_parts("vehicle", 0x4210, 0x01, u16::MAX).unwrap();

        let frame = transport
            .receive_validated_owned(&source_filter, None)
            .await
            .expect("matching owned frame");

        assert_eq!(frame.payload_bytes(), b"owned");
        let filters = transport.core().receive_filters.lock().unwrap();
        assert_eq!(filters.len(), 1);
        let filter = filters.first().expect("one receive filter");
        assert_eq!(filter.0, source_filter);
        assert_eq!(filter.1, None);
    }

    #[test]
    fn selected_wire_core_source_filter_preserves_exact_and_partial_patterns() {
        let exact = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();
        let wildcard = UUri::try_from_parts("vehicle", 0x4210, 0x01, u16::MAX).unwrap();

        assert_eq!(selected_wire_core_source_filter_for(&exact), exact);
        assert_eq!(selected_wire_core_source_filter_for(&wildcard), wildcard);
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test_case(0x9000; "exact listener filter remains exact")]
    #[test_case(u16::MAX; "wildcard listener filter remains broad")]
    #[tokio::test]
    async fn listener_filters_and_owning_leases_survive_adapter_lifecycle(resource: u16) {
        let transport =
            RecordingCore::default().into_native_prefix_wire_transport(UProtocolNativeWire);
        let source = UUri::try_from_parts("vehicle", 0x4210, 1, resource).unwrap();
        let listener = Arc::new(RecordingZeroCopyListener::default());
        transport
            .register_validated_zero_copy_listener(&source, None, listener.clone())
            .await
            .unwrap();
        let encoded_listener = transport
            .core()
            .listeners
            .lock()
            .unwrap()
            .first()
            .expect("registered listener")
            .clone();
        encoded_listener
            .on_receive_encoded_zero_copy(raw_frame_for_topic(0x9000, b"retained"))
            .await;
        let frame = listener.frames.lock().unwrap().pop().unwrap();
        let address = frame.try_contiguous_payload().unwrap().as_ptr();
        transport
            .unregister_validated_zero_copy_listener(&source, None, listener)
            .await
            .unwrap();
        let expected = selected_wire_core_source_filter_for(&source);
        assert_eq!(
            *transport.core().listener_filters.lock().unwrap(),
            vec![(true, expected.clone(), None), (false, expected, None)]
        );
        assert!(transport.core().listeners.lock().unwrap().is_empty());
        drop(encoded_listener);
        drop(transport);
        // This proves adapter ownership, not a native allocator's implementation.
        assert_eq!(frame.try_contiguous_payload().unwrap(), b"retained");
        assert_eq!(frame.try_contiguous_payload().unwrap().as_ptr(), address);
    }

    #[cfg(feature = "owned-frame-transport")]
    #[test_case(0x9000; "exact owned listener filter remains exact")]
    #[test_case(u16::MAX; "wildcard owned listener filter remains broad")]
    #[tokio::test]
    async fn owned_listener_registration_and_removal_use_the_same_core_filter(resource: u16) {
        let transport =
            RecordingCore::default().into_native_prefix_wire_transport(UProtocolNativeWire);
        let source = UUri::try_from_parts("vehicle", 0x4210, 1, resource).unwrap();
        let listener = Arc::new(RecordingOwnedListener::default());
        transport
            .register_validated_owned_listener(&source, None, listener.clone())
            .await
            .unwrap();
        let encoded_listener = transport
            .core()
            .owned_listeners
            .lock()
            .unwrap()
            .first()
            .expect("registered owned listener")
            .clone();
        let raw = raw_frame_for_topic(0x9000, b"owned-after-unregister");
        encoded_listener
            .on_receive_encoded_owned(EncodedOwnedFrame::new(
                raw.encoded_metadata,
                Some(raw.payload.into()),
            ))
            .await;
        transport
            .unregister_validated_owned_listener(&source, None, listener.clone())
            .await
            .unwrap();
        let expected = selected_wire_core_source_filter_for(&source);
        assert_eq!(
            *transport.core().listener_filters.lock().unwrap(),
            vec![(true, expected.clone(), None), (false, expected, None)]
        );
        assert!(transport.core().owned_listeners.lock().unwrap().is_empty());
        drop(encoded_listener);
        drop(transport);
        assert_eq!(
            listener
                .0
                .lock()
                .unwrap()
                .first()
                .expect("delivered owned frame")
                .payload_bytes(),
            b"owned-after-unregister"
        );
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn listener_filters_after_selected_wire_decode() {
        let source_filter = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();
        let listener = Arc::new(RecordingZeroCopyListener::default());
        let wire_listener =
            WireZeroCopyListener::<RawRx, UProtocolNativeWire, NativePrefixFrameMetadataCodec> {
                source_filter,
                sink_filter: None,
                listener: listener.clone(),
                metadata_codec: NativePrefixFrameMetadataCodec,
                native_profile: None,
                _wire: PhantomData,
            };

        wire_listener
            .on_receive_encoded_zero_copy(raw_frame_for_topic(0x9001, b"drop"))
            .await;
        wire_listener
            .on_receive_encoded_zero_copy(raw_frame_for_topic(0x9000, b"keep"))
            .await;

        let frames = listener.frames.lock().unwrap();
        assert_eq!(frames.len(), 1);
        let frame = frames.first().expect("one received frame");
        assert_eq!(frame.try_contiguous_payload(), Some(&b"keep"[..]));
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn wire_transport_fits_zero_copy_blanket_boundaries() {
        fn assert_zero_copy_impl<T: UZeroCopyTransportImpl>() {}
        fn assert_uninit_impl<T: UZeroCopyUninitTransportImpl>() {}

        type Transport =
            UWireTransport<CompileCore, UProtocolNativeWire, NativePrefixFrameMetadataCodec>;
        assert_zero_copy_impl::<Transport>();
        assert_uninit_impl::<Transport>();
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn prepared_tx_spec_carries_metadata_bytes_and_layout() {
        let metadata = metadata_with_payload();
        let spec = crate::UTxLoanSpec::new(
            metadata.clone(),
            UTxPayloadSpec::Present {
                len: 4,
                alignment: crate::PayloadAlignment::new(2).unwrap(),
            },
        )
        .unwrap();

        let prepared = PreparedTxLoanSpec::from_validated::<
            UProtocolNativeWire,
            NativePrefixFrameMetadataCodec,
        >(spec, &NativePrefixFrameMetadataCodec)
        .unwrap();

        assert_eq!(prepared.metadata(), &metadata);
        assert_eq!(prepared.payload_len(), 4);
        assert_eq!(prepared.payload_alignment(), 2);
        assert_eq!(
            prepared.payload_alignment_proof(),
            crate::PayloadAlignment::new(2).unwrap()
        );
        assert!(!prepared.encoded_metadata().is_empty());
    }

    #[cfg(feature = "owned-frame-transport")]
    #[test]
    fn wire_transport_fits_owned_blanket_boundary() {
        fn assert_owned_impl<T: UOwnedTransportImpl>() {}

        type Transport =
            UWireTransport<CompileCore, UProtocolNativeWire, NativePrefixFrameMetadataCodec>;
        assert_owned_impl::<Transport>();
    }

    #[test]
    fn with_wire_constructs_adapter() {
        let transport = CompileCore.into_native_prefix_wire_transport(UProtocolNativeWire);
        let _: &UProtocolNativeWire = transport.wire();
        let _: &CompileCore = transport.core();
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn existing_public_rx_lease_still_compiles_independently() {
        fn assert_public_rx<T: UZeroCopyRxLease>() {}

        assert_public_rx::<UVecRxLease>();
    }
}
