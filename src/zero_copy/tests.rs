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

#[cfg(all(test, feature = "owned-frame-transport"))]
use crate::UOwnedFrame;
#[cfg(test)]
use std::io::Read;

/// Owned buffer useful for tests, examples, and adapters that emulate a transmit loan.
#[derive(Clone, Debug, PartialEq)]
pub struct UVecTxBuffer {
    metadata: UFrameMetadata,
    storage: Vec<u8>,
    payload_offset: usize,
    payload_len: usize,
}

/// Owned uninitialized buffer useful for tests and examples.
#[derive(Clone, Debug)]
pub struct UVecUninitTxBuffer {
    metadata: UFrameMetadata,
    storage: Vec<MaybeUninit<u8>>,
    payload_offset: usize,
    payload_len: usize,
}

/// In-memory receive lease for tests and examples that need receive-lease shape.
#[derive(Clone, Debug, PartialEq)]
pub struct UVecRxLease {
    metadata: UFrameMetadata,
    payload: Option<Vec<u8>>,
}

impl UVecTxBuffer {
    /// Creates an owned transmit buffer with `payload_len` zero-initialized bytes.
    #[must_use]
    pub fn new(metadata: UFrameMetadata, payload_len: usize) -> Self {
        Self {
            metadata,
            storage: vec![0_u8; payload_len],
            payload_offset: 0,
            payload_len,
        }
    }

    /// Creates an owned transmit buffer whose visible payload starts at `alignment`.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested alignment is invalid or allocation size overflows.
    pub fn with_alignment(
        metadata: UFrameMetadata,
        payload_len: usize,
        alignment: usize,
    ) -> Result<Self, UStatus> {
        validate_payload_layout(payload_len, alignment)?;
        if payload_len == 0 {
            return Ok(Self::new(metadata, payload_len));
        }
        let extra = alignment.saturating_sub(1);
        let storage_len = payload_len.checked_add(extra).ok_or_else(|| {
            invalid_argument("payload length plus alignment padding overflows usize")
        })?;
        let storage = vec![0_u8; storage_len];
        let payload_offset = aligned_offset(storage.as_ptr() as usize, alignment);
        Ok(Self {
            metadata,
            storage,
            payload_offset,
            payload_len,
        })
    }

    /// Converts the buffer into an in-memory receive lease, consuming the emulated loan.
    #[must_use]
    pub fn into_rx_lease(self) -> UVecRxLease {
        let payload = if self.metadata.payload_encoding().is_some() {
            Some(
                self.storage
                    .get(self.payload_range())
                    .expect("UVecTxBuffer payload range must be in bounds")
                    .to_vec(),
            )
        } else {
            None
        };
        UVecRxLease::new_unchecked(self.metadata, payload)
    }

    fn payload_range(&self) -> std::ops::Range<usize> {
        let end = self
            .payload_offset
            .checked_add(self.payload_len)
            .expect("UVecTxBuffer payload range overflow");
        self.payload_offset..end
    }
}

impl UTxBuffer for UVecTxBuffer {
    fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    fn payload(&self) -> &[u8] {
        self.storage
            .get(self.payload_range())
            .expect("UVecTxBuffer payload range must be in bounds")
    }

    fn payload_mut(&mut self) -> &mut [u8] {
        let range = self.payload_range();
        self.storage
            .get_mut(range)
            .expect("UVecTxBuffer payload range must be in bounds")
    }
}

impl UVecUninitTxBuffer {
    /// Creates an owned uninitialized transmit buffer.
    #[must_use]
    pub fn new(metadata: UFrameMetadata, payload_len: usize) -> Self {
        Self {
            metadata,
            storage: vec![MaybeUninit::uninit(); payload_len],
            payload_offset: 0,
            payload_len,
        }
    }

    /// Creates an owned uninitialized transmit buffer whose visible payload starts at `alignment`.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested alignment is invalid or allocation size overflows.
    pub fn with_alignment(
        metadata: UFrameMetadata,
        payload_len: usize,
        alignment: usize,
    ) -> Result<Self, UStatus> {
        validate_payload_layout(payload_len, alignment)?;
        if payload_len == 0 {
            return Ok(Self::new(metadata, payload_len));
        }
        let extra = alignment.saturating_sub(1);
        let storage_len = payload_len.checked_add(extra).ok_or_else(|| {
            invalid_argument("payload length plus alignment padding overflows usize")
        })?;
        let storage = vec![MaybeUninit::uninit(); storage_len];
        let payload_offset = aligned_offset(storage.as_ptr() as usize, alignment);
        Ok(Self {
            metadata,
            storage,
            payload_offset,
            payload_len,
        })
    }

    fn payload_range(&self) -> std::ops::Range<usize> {
        let end = self
            .payload_offset
            .checked_add(self.payload_len)
            .expect("UVecUninitTxBuffer payload range overflow");
        self.payload_offset..end
    }
}

impl UUninitTxBuffer for UVecUninitTxBuffer {
    type Initialized = UVecTxBuffer;

    fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    fn payload_len(&self) -> usize {
        self.payload_len
    }

    fn payload_uninit_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        let range = self.payload_range();
        self.storage
            .get_mut(range)
            .expect("UVecUninitTxBuffer payload range must be in bounds")
    }

    /// # Safety
    ///
    /// Caller upholds the documented loan/witness contract for this handle:
    /// exclusive, layout-valid storage (constructors) or complete
    /// initialization of every transported byte (witness discharges).
    unsafe fn assume_payload_init(self) -> Self::Initialized {
        let Self {
            metadata,
            mut storage,
            payload_offset,
            payload_len,
        } = self;
        for slot in storage
            .get_mut(..payload_offset)
            .expect("UVecUninitTxBuffer prefix range must be in bounds")
        {
            slot.write(0);
        }
        let payload_end = payload_offset
            .checked_add(payload_len)
            .expect("UVecUninitTxBuffer payload range overflow");
        for slot in storage
            .get_mut(payload_end..)
            .expect("UVecUninitTxBuffer suffix range must be in bounds")
        {
            slot.write(0);
        }
        let len = storage.len();
        let capacity = storage.capacity();
        let ptr = storage.as_mut_ptr().cast::<u8>();
        std::mem::forget(storage);
        // SAFETY: `MaybeUninit<u8>` has the same layout as `u8`, prefix/suffix
        // bytes were initialized above, and the caller guarantees visible
        // payload bytes are initialized before conversion.
        let storage = unsafe { Vec::from_raw_parts(ptr, len, capacity) };
        UVecTxBuffer {
            metadata,
            storage,
            payload_offset,
            payload_len,
        }
    }
}

impl UVecRxLease {
    /// Creates an in-memory receive lease after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata or payload presence is invalid.
    pub fn new(metadata: UFrameMetadata, payload: Option<Vec<u8>>) -> Result<Self, UStatus> {
        let frame = Self::new_unchecked(metadata, payload);
        validate_frame_view_for_transport(&frame)?;
        Ok(frame)
    }

    /// Creates an in-memory receive lease without validation.
    #[must_use]
    pub fn new_unchecked(metadata: UFrameMetadata, payload: Option<Vec<u8>>) -> Self {
        Self { metadata, payload }
    }

    /// Consumes the lease and returns metadata plus optional payload bytes.
    #[must_use]
    pub fn into_parts(self) -> (UFrameMetadata, Option<Vec<u8>>) {
        (self.metadata, self.payload)
    }

    fn payload_bytes(&self) -> &[u8] {
        self.payload.as_deref().unwrap_or_default()
    }
}

impl UFrameView for UVecRxLease {
    type PayloadReader<'a>
        = Cursor<&'a [u8]>
    where
        Self: 'a;
    type PayloadSlices<'a>
        = std::option::IntoIter<&'a [u8]>
    where
        Self: 'a;

    fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    fn payload_len(&self) -> usize {
        self.payload_bytes().len()
    }

    fn has_payload(&self) -> bool {
        self.payload.is_some()
    }

    fn payload_reader(&self) -> Self::PayloadReader<'_> {
        Cursor::new(self.payload_bytes())
    }

    fn payload_slices(&self) -> Self::PayloadSlices<'_> {
        self.payload.as_deref().into_iter()
    }

    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }
}

impl UZeroCopyRxLease for UVecRxLease {}

impl ULoanedContiguousZeroCopyRxFrame for UVecRxLease {
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError> {
        let payload = self.payload.as_deref().ok_or(UWireError::MissingPayload)?;
        // SAFETY: `UVecRxLease` is the local vector-backed test receive lease
        // selected in `USR-04B` preflight as the positive fake loan proof.
        Ok(unsafe {
            LoanedPayload::new_unchecked(payload, PayloadLoanProvenance::OpaqueTransportLoan)
        })
    }
}

#[cfg(any(test, feature = "test-util"))]
#[derive(Default)]
struct InMemoryState {
    sent: Vec<UVecRxLease>,
    queue: VecDeque<UVecRxLease>,
    listeners: Vec<Arc<dyn UZeroCopyListener<UVecRxLease>>>,
}

/// In-memory zero-copy transport for tests and examples.
#[cfg(any(test, feature = "test-util"))]
#[derive(Clone, Default)]
pub struct InMemoryZeroCopyTransport {
    state: Arc<Mutex<InMemoryState>>,
}

#[cfg(any(test, feature = "test-util"))]
impl core::fmt::Debug for InMemoryZeroCopyTransport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InMemoryZeroCopyTransport")
            .finish_non_exhaustive()
    }
}

#[cfg(any(test, feature = "test-util"))]
impl InMemoryZeroCopyTransport {
    /// Returns frames sent through [`UZeroCopyTransport::send_zero_copy`].
    #[must_use]
    pub fn sent_frames(&self) -> Vec<UVecRxLease> {
        self.state
            .lock()
            .expect("zero-copy state lock poisoned")
            .sent
            .clone()
    }

    /// Injects a frame into the receive queue and registered zero-copy listeners.
    pub async fn inject(&self, frame: UVecRxLease) {
        self.enqueue_and_dispatch(frame).await;
    }

    async fn enqueue_and_dispatch(&self, frame: UVecRxLease) {
        let listeners = {
            let mut state = self.state.lock().expect("zero-copy state lock poisoned");
            state.queue.push_back(frame.clone());
            state.listeners.clone()
        };
        for listener in listeners {
            listener.on_receive_zero_copy(frame.clone()).await;
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
#[async_trait]
impl UZeroCopyTransportImpl for InMemoryZeroCopyTransport {
    type Tx = UVecTxBuffer;
    type Rx = UVecRxLease;

    async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
        UVecTxBuffer::with_alignment(
            spec.metadata().clone(),
            spec.payload_len(),
            spec.payload_alignment(),
        )
    }

    async fn send_validated_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
        let frame = buffer.into_rx_lease();
        {
            self.state
                .lock()
                .expect("zero-copy state lock poisoned")
                .sent
                .push(frame.clone());
        }
        self.enqueue_and_dispatch(frame).await;
        Ok(())
    }

    async fn receive_validated_zero_copy(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        self.state
            .lock()
            .expect("zero-copy state lock poisoned")
            .queue
            .pop_front()
            .ok_or_else(|| UStatus::fail_with_code(UCode::NotFound, "no frame available"))
    }

    async fn register_validated_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        self.state
            .lock()
            .expect("zero-copy state lock poisoned")
            .listeners
            .push(listener);
        Ok(())
    }

    async fn unregister_validated_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        let mut state = self.state.lock().expect("zero-copy state lock poisoned");
        let Some(index) = state
            .listeners
            .iter()
            .position(|existing| Arc::ptr_eq(existing, &listener))
        else {
            return Err(UStatus::fail_with_code(
                UCode::NotFound,
                "no such zero-copy listener registered for filters",
            ));
        };
        state.listeners.remove(index);
        Ok(())
    }
}

#[cfg(any(test, feature = "test-util"))]
#[async_trait]
impl UZeroCopyUninitTransportImpl for InMemoryZeroCopyTransport {
    type UninitTx = UVecUninitTxBuffer;

    async fn loan_validated_uninit_tx(&self, spec: UTxLoanSpec) -> Result<Self::UninitTx, UStatus> {
        UVecUninitTxBuffer::with_alignment(
            spec.metadata().clone(),
            spec.payload_len(),
            spec.payload_alignment(),
        )
    }
}
#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::payload::stable::StablePayloadVariant;
    use crate::test_support::StableTestBytes as StableBytes;
    use crate::{PayloadEncoding, UMessageBuilder};
    use std::sync::Mutex as StdMutex;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct OtherStableBytes {
        bytes: [u8; 4],
    }

    // SAFETY: upholds the POD layout contract: repr(C), declared padding
    // only, every byte of a live value initialized.
    unsafe impl StablePayload for OtherStableBytes {
        const TYPE_NAME: &'static str = "uprotocol.test.OtherStableBytes";
    }

    #[repr(C, align(4))]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct AlignedStableBytes {
        bytes: [u8; 4],
    }

    // SAFETY: upholds the POD layout contract: repr(C), declared padding
    // only, every byte of a live value initialized.
    unsafe impl StablePayload for AlignedStableBytes {
        const TYPE_NAME: &'static str = "uprotocol.test.AlignedStableBytes";
    }

    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("failed to create topic")
    }

    fn wildcard_topic_filter() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0xffff).expect("failed to create filter")
    }

    fn metadata_without_encoding() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(message.attributes(), None)
            .expect("metadata")
    }

    fn metadata_with_encoding() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(PayloadEncoding::RAW),
        )
        .expect("metadata")
    }

    fn stable_metadata<T: StablePayload>() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(StableContainerPayload::<T>::encoding()),
        )
        .expect("metadata")
    }

    fn stable_encoding_with<T: StablePayload>(
        type_name: &str,
        variant: StablePayloadVariant,
        size: usize,
        alignment: usize,
    ) -> PayloadEncoding {
        PayloadEncoding::custom(
            StableContainerPayload::<T>::ENCODING_ID,
            format!(
                "application/vnd.uprotocol.stable-container;type=\"{type_name}\";variant={variant};size={size};align={alignment}"
            ),
        )
        .expect("stable encoding")
    }

    #[test]
    fn tx_loan_no_payload_rejects_encoding() {
        let error = UTxLoanSpec::no_payload(metadata_with_encoding()).unwrap_err();
        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    #[test]
    fn tx_loan_payload_rejects_missing_encoding() {
        let error = UTxLoanSpec::payload(metadata_without_encoding(), 8, 1).unwrap_err();
        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    #[test]
    fn present_empty_payload_preserves_encoding() {
        let spec = UTxLoanSpec::present_empty_payload(metadata_with_encoding()).unwrap();
        assert!(spec.has_payload());
        assert_eq!(spec.payload_len(), 0);
        assert_eq!(spec.payload_alignment(), 1);
        assert_eq!(
            spec.metadata().payload_encoding(),
            metadata_with_encoding().payload_encoding()
        );
    }

    #[test]
    fn payload_alignment_must_be_power_of_two() {
        let error = UTxPayloadSpec::present(16, 3).unwrap_err();
        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    #[test]
    fn payload_alignment_proof_rejects_zero() {
        let error = PayloadAlignment::new(0).unwrap_err();
        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    #[test]
    fn payload_alignment_proof_rejects_non_power_of_two() {
        let error = PayloadAlignment::new(6).unwrap_err();
        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    #[test]
    fn payload_alignment_proof_round_trips_raw_value() {
        let alignment = PayloadAlignment::new(8).unwrap();
        let spec = UTxLoanSpec::new(
            metadata_with_encoding(),
            UTxPayloadSpec::present_with_alignment(16, alignment),
        )
        .unwrap();
        assert_eq!(alignment.as_usize(), 8);
        assert_eq!(spec.payload_alignment(), 8);
        assert_eq!(spec.payload_alignment_proof(), alignment);
    }

    struct CountingTransport {
        loan_calls: StdMutex<usize>,
        send_calls: StdMutex<usize>,
        receive: StdMutex<Option<UVecRxLease>>,
    }

    #[async_trait]
    impl UZeroCopyTransportImpl for CountingTransport {
        type Tx = UVecTxBuffer;
        type Rx = UVecRxLease;

        async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus> {
            *self.loan_calls.lock().expect("loan lock") += 1;
            UVecTxBuffer::with_alignment(
                spec.metadata().clone(),
                spec.payload_len(),
                spec.payload_alignment(),
            )
        }

        async fn send_validated_zero_copy(&self, _buffer: Self::Tx) -> Result<(), UStatus> {
            *self.send_calls.lock().expect("send lock") += 1;
            Ok(())
        }

        async fn receive_validated_zero_copy(
            &self,
            _source_filter: &UUri,
            _sink_filter: Option<&UUri>,
        ) -> Result<Self::Rx, UStatus> {
            self.receive
                .lock()
                .expect("receive lock")
                .take()
                .ok_or_else(|| UStatus::fail_with_code(UCode::NotFound, "none"))
        }
    }

    #[tokio::test]
    async fn invalid_spec_rejected_at_the_typestate_transition() {
        // Passing an unvalidated spec to `loan_tx` is a compile error; the
        // runtime rejection lives at the typestate transition:
        let invalid = UTxLoanSpec::new_unchecked(metadata_with_encoding(), UTxPayloadSpec::Absent);
        let error = invalid.validate().unwrap_err();
        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    #[tokio::test]
    async fn invalid_tx_buffer_rejected_before_send_implementation() {
        let transport = CountingTransport {
            loan_calls: StdMutex::new(0),
            send_calls: StdMutex::new(0),
            receive: StdMutex::new(None),
        };
        let buffer = UVecTxBuffer::new(metadata_without_encoding(), 4);

        let error = transport.send_zero_copy(buffer).await.unwrap_err();

        assert_eq!(error.code(), UCode::InvalidArgument);
        assert_eq!(*transport.send_calls.lock().expect("send lock"), 0);
    }

    #[tokio::test]
    async fn invalid_receive_lease_rejected_before_return() {
        let transport = CountingTransport {
            loan_calls: StdMutex::new(0),
            send_calls: StdMutex::new(0),
            receive: StdMutex::new(Some(UVecRxLease::new_unchecked(
                metadata_with_encoding(),
                None,
            ))),
        };

        let error = transport
            .receive_zero_copy(&wildcard_topic_filter(), None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), UCode::InvalidArgument);
    }

    struct SegmentedFrame {
        metadata: UFrameMetadata,
        first: Vec<u8>,
        second: Vec<u8>,
    }

    impl UFrameView for SegmentedFrame {
        type PayloadReader<'a>
            = Cursor<Vec<u8>>
        where
            Self: 'a;
        type PayloadSlices<'a>
            = std::vec::IntoIter<&'a [u8]>
        where
            Self: 'a;

        fn metadata(&self) -> &UFrameMetadata {
            &self.metadata
        }

        fn payload_len(&self) -> usize {
            self.first.len() + self.second.len()
        }

        fn payload_reader(&self) -> Self::PayloadReader<'_> {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&self.first);
            bytes.extend_from_slice(&self.second);
            Cursor::new(bytes)
        }

        fn payload_slices(&self) -> Self::PayloadSlices<'_> {
            vec![self.first.as_slice(), self.second.as_slice()].into_iter()
        }
    }

    #[test]
    fn frame_view_reader_and_slices_preserve_order() {
        let frame = SegmentedFrame {
            metadata: metadata_with_encoding(),
            first: b"abc".to_vec(),
            second: b"def".to_vec(),
        };
        let mut from_reader = Vec::new();
        frame
            .payload_reader()
            .read_to_end(&mut from_reader)
            .expect("reader");
        let from_slices: Vec<u8> = frame.payload_slices().flatten().copied().collect();

        assert_eq!(from_reader, b"abcdef");
        assert_eq!(from_slices, b"abcdef");
        validate_frame_view_for_transport(&frame).unwrap();
    }

    #[test]
    fn stable_borrow_accepts_loan_backed_contiguous_payload() {
        let value = StableBytes { bytes: *b"loan" };
        let frame = UVecRxLease::new(stable_metadata::<StableBytes>(), Some(value.bytes.to_vec()))
            .expect("stable frame");

        let borrowed = frame.borrow_stable_payload::<StableBytes>().unwrap();

        assert_eq!(borrowed, &value);
        assert_eq!(
            frame.payload_loan_provenance().unwrap(),
            PayloadLoanProvenance::OpaqueTransportLoan
        );
    }

    #[test]
    fn stable_borrow_rejects_wrong_type_metadata() {
        let value = StableBytes { bytes: *b"loan" };
        let frame = UVecRxLease::new(
            stable_metadata::<OtherStableBytes>(),
            Some(value.bytes.to_vec()),
        )
        .expect("stable frame");

        let error = frame.borrow_stable_payload::<StableBytes>().unwrap_err();

        assert!(matches!(error, UWireError::InvalidPayload(_)));
    }

    #[test]
    fn stable_borrow_rejects_wrong_size_metadata() {
        let value = StableBytes { bytes: *b"loan" };
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(stable_encoding_with::<StableBytes>(
                StableBytes::TYPE_NAME,
                StablePayloadVariant::FixedSize,
                std::mem::size_of::<StableBytes>() + 1,
                std::mem::align_of::<StableBytes>(),
            )),
        )
        .expect("metadata");
        let frame = UVecRxLease::new(metadata, Some(value.bytes.to_vec())).expect("stable frame");

        let error = frame.borrow_stable_payload::<StableBytes>().unwrap_err();

        assert!(matches!(error, UWireError::InvalidPayload(_)));
    }

    #[test]
    fn stable_borrow_rejects_insufficient_advertised_alignment() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(stable_encoding_with::<AlignedStableBytes>(
                AlignedStableBytes::TYPE_NAME,
                StablePayloadVariant::FixedSize,
                std::mem::size_of::<AlignedStableBytes>(),
                1,
            )),
        )
        .expect("metadata");
        let frame = UVecRxLease::new(metadata, Some(vec![0_u8; 4])).expect("stable frame");

        let error = frame
            .borrow_stable_payload::<AlignedStableBytes>()
            .unwrap_err();

        assert!(matches!(error, UWireError::InvalidPayload(_)));
    }

    #[test]
    fn stable_borrow_rejects_payload_length_mismatch() {
        let frame = UVecRxLease::new(stable_metadata::<StableBytes>(), Some(vec![1, 2, 3]))
            .expect("stable frame");

        let error = frame.borrow_stable_payload::<StableBytes>().unwrap_err();

        assert!(
            matches!(error, UWireError::InvalidPayload(message) if message.contains("payload length"))
        );
    }

    #[test]
    fn stable_borrow_rejects_absent_payload() {
        let frame = UVecRxLease::new_unchecked(stable_metadata::<StableBytes>(), None);

        let error = frame.borrow_stable_payload::<StableBytes>().unwrap_err();

        assert_eq!(error, UWireError::MissingPayload);
    }

    #[test]
    fn segmented_frame_is_not_loan_backed_proof() {
        let frame = SegmentedFrame {
            metadata: stable_metadata::<StableBytes>(),
            first: vec![1, 2],
            second: vec![3, 4],
        };

        assert_eq!(frame.try_contiguous_payload(), None);
        validate_frame_view_for_transport(&frame).unwrap();
    }

    #[tokio::test]
    async fn in_memory_zero_copy_transport_round_trips_payload() {
        let transport = InMemoryZeroCopyTransport::default();
        let spec = UTxLoanSpec::payload(metadata_with_encoding(), 4, 1).unwrap();
        let mut buffer = transport.loan_tx(spec).await.unwrap();
        buffer.payload_mut().copy_from_slice(b"test");

        transport.send_zero_copy(buffer).await.unwrap();
        assert_eq!(transport.sent_frames().len(), 1);
        let received = transport
            .receive_zero_copy(&wildcard_topic_filter(), None)
            .await
            .unwrap();

        assert_eq!(received.try_contiguous_payload(), Some(b"test".as_slice()));

        transport
            .inject(UVecRxLease::new(metadata_with_encoding(), Some(b"next".to_vec())).unwrap())
            .await;
        let injected = transport
            .receive_zero_copy(&wildcard_topic_filter(), None)
            .await
            .unwrap();
        assert_eq!(injected.try_contiguous_payload(), Some(b"next".as_slice()));
    }

    #[tokio::test]
    async fn stable_initialized_tx_helper_sends_stable_payload() {
        let transport = InMemoryZeroCopyTransport::default();

        transport
            .send_loaned_payload_as::<StableContainerPayload<StableBytes>, StableBytes>(
                stable_metadata::<StableBytes>(),
                |payload| payload.bytes.copy_from_slice(b"init"),
            )
            .await
            .expect("send initialized stable payload");

        let sent = transport.sent_frames();
        assert_eq!(sent.len(), 1);
        let sent = sent.first().expect("one sent frame");
        assert_eq!(
            sent.metadata().payload_encoding(),
            Some(&StableContainerPayload::<StableBytes>::encoding())
        );
        assert_eq!(
            sent.borrow_stable_payload::<StableBytes>().unwrap(),
            &StableBytes { bytes: *b"init" }
        );
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn stable_uninit_tx_helper_sends_byte_backed_payload() {
        let transport = InMemoryZeroCopyTransport::default();

        transport
            .send_uninit_loaned_payload_as::<StableContainerPayload<StableBytes>, StableBytes>(
                stable_metadata::<StableBytes>(),
                |slot| Ok(slot.write(StableBytes { bytes: *b"noze" })),
            )
            .await
            .expect("send uninit stable payload");

        let sent = transport.sent_frames();
        assert_eq!(sent.len(), 1);
        let sent = sent.first().expect("one sent frame");
        assert_eq!(
            sent.borrow_stable_payload::<StableBytes>().unwrap(),
            &StableBytes { bytes: *b"noze" }
        );
    }

    #[cfg(feature = "zero-copy-transport")]
    #[tokio::test]
    async fn stable_uninit_tx_helper_uses_stable_payload_init_builder() {
        let transport = InMemoryZeroCopyTransport::default();

        transport
            .send_uninit_stable_payload_as::<StableBytes>(
                stable_metadata::<StableBytes>(),
                |context| context.into_init().bytes_from_array(b"zcpy").finish(),
            )
            .await
            .expect("send stable init payload");

        let sent = transport.sent_frames();
        assert_eq!(sent.len(), 1);
        let sent = sent.first().expect("one sent frame");
        assert_eq!(
            sent.borrow_stable_payload::<StableBytes>().unwrap(),
            &StableBytes { bytes: *b"zcpy" }
        );
    }

    #[cfg(feature = "perf-diagnostics")]
    #[tokio::test]
    async fn phased_stable_uninit_helper_preserves_send_semantics() {
        let transport = InMemoryZeroCopyTransport::default();

        let phases = transport
            .send_uninit_stable_payload_as_phased::<StableBytes>(
                stable_metadata::<StableBytes>(),
                |context| context.into_init().bytes_from_array(b"perf").finish(),
            )
            .await
            .expect("send phased stable init payload");

        assert!(phases.total >= phases.accounted());
        assert_eq!(phases.residual, phases.total - phases.accounted());
        let sent = transport.sent_frames();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent.first()
                .expect("one sent frame")
                .borrow_stable_payload::<StableBytes>()
                .unwrap(),
            &StableBytes { bytes: *b"perf" }
        );
        let _clock_overhead = UninitStableSendPhases::calibrate_clock_overhead(32);
    }

    #[cfg(feature = "owned-frame-transport")]
    #[test]
    fn owned_frame_implements_frame_view_when_feature_enabled() {
        let frame = UOwnedFrame::with_payload(
            metadata_with_encoding(),
            bytes::Bytes::from_static(b"owned"),
        )
        .expect("owned frame");

        assert_eq!(UFrameView::payload_len(&frame), 5);
        assert_eq!(frame.try_contiguous_payload(), Some(b"owned".as_slice()));
    }
}
