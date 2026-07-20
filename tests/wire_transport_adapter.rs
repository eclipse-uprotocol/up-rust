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

//! Selected-wire adapter behavior over a scripted encoded core.

use std::{
    collections::VecDeque,
    io::Cursor,
    str::FromStr,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
#[cfg(feature = "owned-frame-transport")]
use bytes::Bytes;
use up_rust::{LoanedPayload, PayloadLoanProvenance, StableContainerWireFormat};
use up_rust::{
    NativePrefixFrameMetadataCodec, PayloadCodec, PayloadEncoding, PreparedTxLoanSpec,
    ProtobufWire, StableContainerPayload, UCode, UEncodedLoanedRxFrame, UEncodedRxFrame,
    UEncodedZeroCopyListener, UFrameMetadata, UFrameView, UMessageBuilder, UProtocolNativeWire,
    UStatus, UTxBuffer, UTxLoanSpec, UUri, UVecTxBuffer, UWire, UWireError, UWireMetadataCodec,
    UWireRx, UWithNativePrefixWire, UZeroCopyListener, UZeroCopyTransport, UZeroCopyTransportCore,
    WireIdentity,
};

#[cfg(feature = "owned-frame-transport")]
use up_rust::{
    EncodedOwnedFrame, PreparedOwnedFrame, UEncodedOwnedListener, UOwnedFrame, UOwnedTransport,
    UOwnedTransportCore,
};

#[derive(Clone, Copy)]
struct SecondTestWire;

impl UWire for SecondTestWire {
    const WIRE_ID: WireIdentity = WireIdentity::new("test.userializer.second-wire", 0x8001);
    const PAYLOAD_FAMILY_ID: WireIdentity =
        WireIdentity::new("test.userializer.second-payload", 0x8002);
    const METADATA_LAYOUT_ID: WireIdentity = UProtocolNativeWire::METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = UProtocolNativeWire::FORMAT_VERSION;
}

#[derive(Clone, Copy)]
struct WrongWireSamePayload;

impl UWire for WrongWireSamePayload {
    const WIRE_ID: WireIdentity = WireIdentity::new("test.userializer.wrong-wire", 0x8003);
    const PAYLOAD_FAMILY_ID: WireIdentity = UProtocolNativeWire::PAYLOAD_FAMILY_ID;
    const METADATA_LAYOUT_ID: WireIdentity = UProtocolNativeWire::METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = UProtocolNativeWire::FORMAT_VERSION;
}

#[derive(Clone, Copy)]
struct SameWireWrongPayload;

impl UWire for SameWireWrongPayload {
    const WIRE_ID: WireIdentity = UProtocolNativeWire::WIRE_ID;
    const PAYLOAD_FAMILY_ID: WireIdentity =
        WireIdentity::new("test.userializer.wrong-payload", 0x8004);
    const METADATA_LAYOUT_ID: WireIdentity = UProtocolNativeWire::METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = UProtocolNativeWire::FORMAT_VERSION;
}

#[derive(Clone, Debug, PartialEq)]
struct InMemoryEncodedRxFrame {
    encoded_metadata: Vec<u8>,
    payload: Vec<u8>,
}

impl InMemoryEncodedRxFrame {
    fn new(encoded_metadata: Vec<u8>, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            encoded_metadata,
            payload: payload.into(),
        }
    }
}

impl UEncodedRxFrame for InMemoryEncodedRxFrame {
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
        Cursor::new(self.payload.as_slice())
    }

    fn payload_slices(&self) -> Self::PayloadSlices<'_> {
        std::iter::once(self.payload.as_slice())
    }

    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        Some(&self.payload)
    }
}

impl UEncodedLoanedRxFrame for InMemoryEncodedRxFrame {
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError> {
        // SAFETY: the in-memory test core stores receive payload bytes directly
        // inside the raw frame and returns a borrow into that frame without
        // allocating, copying, or coalescing for this method.
        Ok(unsafe {
            LoanedPayload::new_unchecked(&self.payload, PayloadLoanProvenance::OpaqueTransportLoan)
        })
    }
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
)]
#[stable_payload(type_name = "example.test.WireAdapterStableBytes")]
struct WireAdapterStableBytes {
    bytes: [u8; 4],
}

#[derive(Clone, Default)]
struct InMemoryWireCore {
    state: Arc<Mutex<InMemoryWireCoreState>>,
}

#[derive(Default)]
struct InMemoryWireCoreState {
    prepared_tx: Vec<PreparedTxLoanSpec>,
    received: VecDeque<InMemoryEncodedRxFrame>,
    zero_copy_listeners: Vec<Arc<dyn UEncodedZeroCopyListener<InMemoryEncodedRxFrame>>>,
    zero_copy_registered: Vec<usize>,
    zero_copy_unregistered: Vec<usize>,
    zero_copy_listener_filters: Vec<(UUri, Option<UUri>)>,
    zero_copy_unregister_filters: Vec<(UUri, Option<UUri>)>,
    zero_copy_register_calls: usize,
    #[cfg(feature = "owned-frame-transport")]
    prepared_owned: Vec<PreparedOwnedFrame>,
    #[cfg(feature = "owned-frame-transport")]
    owned_received: VecDeque<EncodedOwnedFrame>,
    #[cfg(feature = "owned-frame-transport")]
    owned_listeners: Vec<Arc<dyn UEncodedOwnedListener>>,
    #[cfg(feature = "owned-frame-transport")]
    owned_receive_filters: Vec<(UUri, Option<UUri>)>,
    #[cfg(feature = "owned-frame-transport")]
    owned_listener_filters: Vec<(UUri, Option<UUri>)>,
    #[cfg(feature = "owned-frame-transport")]
    owned_unregister_filters: Vec<(UUri, Option<UUri>)>,
}

impl InMemoryWireCore {
    fn last_prepared_tx(&self) -> PreparedTxLoanSpec {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .prepared_tx
            .last()
            .expect("prepared TX")
            .clone()
    }

    fn push_zero_copy_rx(&self, frame: InMemoryEncodedRxFrame) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .received
            .push_back(frame);
    }

    async fn inject_zero_copy(&self, frame: InMemoryEncodedRxFrame) {
        let listeners = self
            .state
            .lock()
            .expect("core state lock poisoned")
            .zero_copy_listeners
            .clone();
        for listener in listeners {
            listener.on_receive_encoded_zero_copy(frame.clone()).await;
        }
    }

    fn zero_copy_register_calls(&self) -> usize {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .zero_copy_register_calls
    }

    fn registered_and_unregistered_same_listener(&self) -> bool {
        let state = self.state.lock().expect("core state lock poisoned");
        state.zero_copy_registered.last() == state.zero_copy_unregistered.last()
    }

    fn zero_copy_listener_count(&self) -> usize {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .zero_copy_listeners
            .len()
    }

    fn last_zero_copy_listener_filter(&self) -> (UUri, Option<UUri>) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .zero_copy_listener_filters
            .last()
            .expect("zero-copy listener filter")
            .clone()
    }

    fn last_zero_copy_unregister_filter(&self) -> (UUri, Option<UUri>) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .zero_copy_unregister_filters
            .last()
            .expect("zero-copy unregister filter")
            .clone()
    }

    #[cfg(feature = "owned-frame-transport")]
    fn last_prepared_owned(&self) -> PreparedOwnedFrame {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .prepared_owned
            .last()
            .expect("prepared owned frame")
            .clone()
    }

    #[cfg(feature = "owned-frame-transport")]
    fn push_owned_rx(&self, frame: EncodedOwnedFrame) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .owned_received
            .push_back(frame);
    }

    #[cfg(feature = "owned-frame-transport")]
    async fn inject_owned(&self, frame: EncodedOwnedFrame) {
        let listeners = self
            .state
            .lock()
            .expect("core state lock poisoned")
            .owned_listeners
            .clone();
        for listener in listeners {
            listener.on_receive_encoded_owned(frame.clone()).await;
        }
    }

    #[cfg(feature = "owned-frame-transport")]
    fn last_owned_receive_filter(&self) -> (UUri, Option<UUri>) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .owned_receive_filters
            .last()
            .expect("owned receive filter")
            .clone()
    }

    #[cfg(feature = "owned-frame-transport")]
    fn last_owned_listener_filter(&self) -> (UUri, Option<UUri>) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .owned_listener_filters
            .last()
            .expect("owned listener filter")
            .clone()
    }

    #[cfg(feature = "owned-frame-transport")]
    fn last_owned_unregister_filter(&self) -> (UUri, Option<UUri>) {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .owned_unregister_filters
            .last()
            .expect("owned unregister filter")
            .clone()
    }
}

#[async_trait]
impl UZeroCopyTransportCore for InMemoryWireCore {
    type Tx = UVecTxBuffer;
    type Rx = InMemoryEncodedRxFrame;

    async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .prepared_tx
            .push(spec.clone());
        UVecTxBuffer::with_alignment(
            spec.metadata().clone(),
            spec.payload_len(),
            spec.payload_alignment(),
        )
    }

    async fn send_prepared_zero_copy(&self, _buffer: Self::Tx) -> Result<(), UStatus> {
        Ok(())
    }

    async fn receive_encoded_zero_copy(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .received
            .pop_front()
            .ok_or_else(|| UStatus::fail_with_code(UCode::NotFound, "no frame available"))
    }

    async fn register_encoded_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        let mut state = self.state.lock().expect("core state lock poisoned");
        state.zero_copy_register_calls += 1;
        state
            .zero_copy_listener_filters
            .push((source_filter.clone(), sink_filter.cloned()));
        state
            .zero_copy_registered
            .push(encoded_listener_id(&listener));
        state.zero_copy_listeners.push(listener);
        Ok(())
    }

    async fn unregister_encoded_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        let mut state = self.state.lock().expect("core state lock poisoned");
        state
            .zero_copy_unregister_filters
            .push((source_filter.clone(), sink_filter.cloned()));
        state
            .zero_copy_unregistered
            .push(encoded_listener_id(&listener));
        let Some(index) = state
            .zero_copy_listeners
            .iter()
            .position(|registered| Arc::ptr_eq(registered, &listener))
        else {
            return Err(UStatus::fail_with_code(
                UCode::NotFound,
                "zero-copy listener not registered",
            ));
        };
        state.zero_copy_listeners.remove(index);
        Ok(())
    }
}

#[cfg(feature = "owned-frame-transport")]
#[async_trait]
impl UOwnedTransportCore for InMemoryWireCore {
    async fn send_prepared_owned(&self, frame: PreparedOwnedFrame) -> Result<(), UStatus> {
        self.state
            .lock()
            .expect("core state lock poisoned")
            .prepared_owned
            .push(frame);
        Ok(())
    }

    async fn receive_encoded_owned(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<EncodedOwnedFrame, UStatus> {
        let mut state = self.state.lock().expect("core state lock poisoned");
        state
            .owned_receive_filters
            .push((source_filter.clone(), sink_filter.cloned()));
        state
            .owned_received
            .pop_front()
            .ok_or_else(|| UStatus::fail_with_code(UCode::NotFound, "no frame available"))
    }

    async fn register_encoded_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UEncodedOwnedListener>,
    ) -> Result<(), UStatus> {
        let mut state = self.state.lock().expect("core state lock poisoned");
        state
            .owned_listener_filters
            .push((source_filter.clone(), sink_filter.cloned()));
        state.owned_listeners.push(listener);
        Ok(())
    }

    async fn unregister_encoded_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UEncodedOwnedListener>,
    ) -> Result<(), UStatus> {
        let mut state = self.state.lock().expect("core state lock poisoned");
        state
            .owned_unregister_filters
            .push((source_filter.clone(), sink_filter.cloned()));
        let Some(index) = state
            .owned_listeners
            .iter()
            .position(|registered| Arc::ptr_eq(registered, &listener))
        else {
            return Err(UStatus::fail_with_code(
                UCode::NotFound,
                "owned listener not registered",
            ));
        };
        state.owned_listeners.remove(index);
        Ok(())
    }
}

#[cfg(feature = "owned-frame-transport")]
#[derive(Default)]
struct CountingOwnedListener {
    payloads: Mutex<Vec<Vec<u8>>>,
}

#[cfg(feature = "owned-frame-transport")]
impl CountingOwnedListener {
    fn payloads(&self) -> Vec<Vec<u8>> {
        self.payloads
            .lock()
            .expect("owned payloads lock poisoned")
            .clone()
    }
}

#[cfg(feature = "owned-frame-transport")]
#[async_trait]
impl up_rust::UOwnedListener for CountingOwnedListener {
    async fn on_receive_owned(&self, frame: UOwnedFrame) {
        self.payloads
            .lock()
            .expect("owned payloads lock poisoned")
            .push(frame.payload_bytes().to_vec());
    }
}

fn encoded_listener_id(
    listener: &Arc<dyn UEncodedZeroCopyListener<InMemoryEncodedRxFrame>>,
) -> usize {
    let ptr = Arc::as_ptr(listener);
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

#[derive(Default)]
struct CountingZeroCopyListener {
    payloads: Mutex<Vec<Vec<u8>>>,
}

impl CountingZeroCopyListener {
    fn payloads(&self) -> Vec<Vec<u8>> {
        self.payloads
            .lock()
            .expect("payloads lock poisoned")
            .clone()
    }
}

#[async_trait]
impl<W> UZeroCopyListener<UWireRx<InMemoryEncodedRxFrame, W, NativePrefixFrameMetadataCodec>>
    for CountingZeroCopyListener
where
    W: UWire + Send + Sync + 'static,
{
    async fn on_receive_zero_copy(
        &self,
        frame: UWireRx<InMemoryEncodedRxFrame, W, NativePrefixFrameMetadataCodec>,
    ) {
        self.payloads
            .lock()
            .expect("payloads lock poisoned")
            .push(frame.try_contiguous_payload().unwrap_or_default().to_vec());
    }
}

fn topic() -> UUri {
    UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic URI")
}

fn notification_sink() -> UUri {
    UUri::try_from_parts("cloud", 0x1234, 0x01, 0x0000).expect("notification sink URI")
}

fn other_notification_sink() -> UUri {
    UUri::try_from_parts("other-cloud", 0x1234, 0x01, 0x0000).expect("other notification sink URI")
}

fn valid_source_filter() -> UUri {
    UUri::from_str("//vehicle/4210/1/9000").expect("valid source filter")
}

fn invalid_source_filter() -> UUri {
    UUri::from_str("//vehicle/4210/1/10").expect("invalid source filter fixture")
}

fn broad_physical_source_filter() -> UUri {
    UUri::try_from_parts("*", u32::MAX, u8::MAX, u16::MAX)
        .expect("valid broad physical source filter")
}

fn metadata_with_payload() -> UFrameMetadata {
    metadata_with_payload_encoding(PayloadEncoding::RAW)
}

fn metadata_with_payload_encoding(payload_encoding: PayloadEncoding) -> UFrameMetadata {
    let message = UMessageBuilder::publish(topic()).build().expect("message");
    message
        .attributes()
        .to_frame_metadata(payload_encoding)
        .expect("metadata")
}

fn notification_metadata_with_payload(destination: UUri) -> UFrameMetadata {
    let message = UMessageBuilder::notification(topic(), destination)
        .build()
        .expect("notification message");
    message
        .attributes()
        .to_frame_metadata(PayloadEncoding::RAW)
        .expect("notification metadata")
}

#[cfg(feature = "owned-frame-transport")]
fn nonmatching_metadata_with_payload() -> UFrameMetadata {
    let source = UUri::try_from_parts("other", 0x4210, 0x01, 0x9000).expect("other URI");
    let message = UMessageBuilder::publish(source).build().expect("message");
    message
        .attributes()
        .to_frame_metadata(PayloadEncoding::RAW)
        .expect("metadata")
}

fn metadata_with_protobuf_payload() -> UFrameMetadata {
    metadata_with_payload_encoding(ProtobufWire::payload_encoding())
}

fn tx_spec(metadata: UFrameMetadata, len: usize) -> UTxLoanSpec {
    UTxLoanSpec::payload(metadata, len, 1).expect("TX loan spec")
}

fn encoded_frame<W>(metadata: &UFrameMetadata, payload: &'static [u8]) -> InMemoryEncodedRxFrame
where
    W: UWire,
{
    InMemoryEncodedRxFrame::new(
        NativePrefixFrameMetadataCodec
            .encode_frame_metadata(W::metadata_context(), metadata)
            .expect("encoded metadata"),
        payload,
    )
}

#[cfg(feature = "owned-frame-transport")]
fn encoded_owned_frame<W>(metadata: &UFrameMetadata, payload: &'static [u8]) -> EncodedOwnedFrame
where
    W: UWire,
{
    EncodedOwnedFrame::new(
        NativePrefixFrameMetadataCodec
            .encode_frame_metadata(W::metadata_context(), metadata)
            .expect("encoded metadata"),
        Some(Bytes::from_static(payload)),
    )
}

#[tokio::test]
async fn zero_copy_loan_passes_encoded_metadata_to_core_for_selected_wires() {
    async fn assert_wire<W>(wire: W, metadata: UFrameMetadata)
    where
        W: UWire + Send + Sync + 'static,
    {
        let core = InMemoryWireCore::default();
        let transport = core.clone().into_native_prefix_wire_transport(wire);

        let mut tx = transport
            .loan_tx(tx_spec(metadata.clone(), 3))
            .await
            .expect("loan TX");
        tx.payload_mut().copy_from_slice(b"abc");
        transport.send_zero_copy(tx).await.expect("send zero-copy");

        let prepared = core.last_prepared_tx();
        assert_eq!(prepared.metadata(), &metadata);
        assert_eq!(prepared.payload_len(), 3);
        assert_eq!(prepared.payload_alignment(), 1);
        assert_eq!(
            NativePrefixFrameMetadataCodec
                .decode_frame_metadata(W::metadata_context(), prepared.encoded_metadata())
                .expect("decode metadata"),
            metadata
        );
    }

    assert_wire(UProtocolNativeWire, metadata_with_payload()).await;
    assert_wire(SecondTestWire, metadata_with_payload()).await;
    assert_wire(ProtobufWire, metadata_with_protobuf_payload()).await;
}

#[tokio::test]
async fn zero_copy_pull_receive_borrows_payload_through_selected_wire_mapping() {
    let metadata = metadata_with_payload_encoding(
        StableContainerPayload::<WireAdapterStableBytes>::encoding(),
    );
    let core = InMemoryWireCore::default();
    let transport = core.clone().into_stable_container_transport();
    core.push_zero_copy_rx(encoded_frame::<StableContainerWireFormat>(
        &metadata,
        b"\x07\x00\x2a\x00",
    ));

    let frame = transport
        .receive_zero_copy(&valid_source_filter(), None)
        .await
        .expect("receive stable-container frame");
    let payload: &WireAdapterStableBytes = frame.borrow_payload().expect("borrow payload");

    assert_eq!(payload.bytes, [7, 0, 42, 0]);
}

#[tokio::test]
async fn zero_copy_pull_receive_rejects_wrong_wire_before_public_exposure() {
    let metadata = metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    core.push_zero_copy_rx(encoded_frame::<WrongWireSamePayload>(&metadata, b"bad"));

    let status = match transport
        .receive_zero_copy(&valid_source_filter(), None)
        .await
    {
        Ok(_) => panic!("wrong selected wire must be rejected"),
        Err(status) => status,
    };

    assert_eq!(status.code(), UCode::InvalidArgument);
    assert!(status
        .message()
        .is_some_and(|message| message.contains("wrong selected wire")));
}

#[tokio::test]
async fn zero_copy_pull_receive_rejects_payload_family_mismatch_before_public_exposure() {
    let metadata = metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    core.push_zero_copy_rx(encoded_frame::<SameWireWrongPayload>(&metadata, b"bad"));

    let status = match transport
        .receive_zero_copy(&valid_source_filter(), None)
        .await
    {
        Ok(_) => panic!("wrong payload family must be rejected"),
        Err(status) => status,
    };

    assert_eq!(status.code(), UCode::InvalidArgument);
    assert!(status
        .message()
        .is_some_and(|message| message.contains("payload family mismatch")));
}

#[tokio::test]
async fn zero_copy_pull_receive_rejects_malformed_metadata_before_public_exposure() {
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    core.push_zero_copy_rx(InMemoryEncodedRxFrame::new(b"not-upwm".to_vec(), b"bad"));

    let status = match transport
        .receive_zero_copy(&valid_source_filter(), None)
        .await
    {
        Ok(_) => panic!("malformed metadata must be rejected"),
        Err(status) => status,
    };

    assert_eq!(status.code(), UCode::InvalidArgument);
}

#[tokio::test]
async fn zero_copy_listener_drops_invalid_metadata_and_unregisters_same_wrapped_listener() {
    let metadata = metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    let listener = Arc::new(CountingZeroCopyListener::default());
    let source = valid_source_filter();

    transport
        .register_zero_copy_listener(&source, None, listener.clone())
        .await
        .expect("register listener");

    core.inject_zero_copy(encoded_frame::<UProtocolNativeWire>(&metadata, b"ok"))
        .await;
    core.inject_zero_copy(encoded_frame::<WrongWireSamePayload>(&metadata, b"bad"))
        .await;
    core.inject_zero_copy(InMemoryEncodedRxFrame::new(b"not-upwm".to_vec(), b"bad"))
        .await;

    assert_eq!(listener.payloads(), vec![b"ok".to_vec()]);

    transport
        .unregister_zero_copy_listener(&source, None, listener.clone())
        .await
        .expect("unregister listener");

    assert!(core.registered_and_unregistered_same_listener());
    assert_eq!(core.zero_copy_listener_count(), 0);

    core.inject_zero_copy(encoded_frame::<UProtocolNativeWire>(&metadata, b"after"))
        .await;
    assert_eq!(listener.payloads(), vec![b"ok".to_vec()]);
}

#[tokio::test]
async fn zero_copy_listener_passes_sink_filter_to_encoded_core() {
    let metadata = notification_metadata_with_payload(notification_sink());
    let other_metadata = notification_metadata_with_payload(other_notification_sink());
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    let listener = Arc::new(CountingZeroCopyListener::default());
    let source = valid_source_filter();
    let sink = notification_sink();

    transport
        .register_zero_copy_listener(&source, Some(&sink), listener.clone())
        .await
        .expect("register sink-bearing listener");

    let (source_filter, sink_filter) = core.last_zero_copy_listener_filter();
    assert_eq!(source_filter, broad_physical_source_filter());
    assert_eq!(sink_filter, Some(sink.clone()));

    core.inject_zero_copy(encoded_frame::<UProtocolNativeWire>(
        &other_metadata,
        b"skip",
    ))
    .await;
    core.inject_zero_copy(encoded_frame::<UProtocolNativeWire>(&metadata, b"match"))
        .await;

    assert_eq!(listener.payloads(), vec![b"match".to_vec()]);

    transport
        .unregister_zero_copy_listener(&source, Some(&sink), listener)
        .await
        .expect("unregister sink-bearing listener");

    let (source_filter, sink_filter) = core.last_zero_copy_unregister_filter();
    assert_eq!(source_filter, broad_physical_source_filter());
    assert_eq!(sink_filter, Some(sink));
}

#[tokio::test]
async fn zero_copy_filter_validation_runs_before_core_registration() {
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    let listener = Arc::new(CountingZeroCopyListener::default());

    let status = transport
        .register_zero_copy_listener(&invalid_source_filter(), None, listener)
        .await
        .expect_err("invalid filter must be rejected");

    assert_eq!(status.code(), UCode::InvalidArgument);
    assert_eq!(core.zero_copy_register_calls(), 0);
}

#[cfg(feature = "owned-frame-transport")]
#[tokio::test]
async fn owned_send_passes_encoded_metadata_to_core() {
    let metadata = metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    let frame = UOwnedFrame::with_payload(metadata.clone(), Bytes::from_static(b"owned"))
        .expect("owned frame");

    transport.send_owned(frame).await.expect("send owned");

    let prepared = core.last_prepared_owned();
    assert_eq!(prepared.metadata(), &metadata);
    assert_eq!(prepared.payload().map(Bytes::as_ref), Some(&b"owned"[..]));
    assert_eq!(
        NativePrefixFrameMetadataCodec
            .decode_frame_metadata(
                UProtocolNativeWire::metadata_context(),
                prepared.encoded_metadata(),
            )
            .expect("decode metadata"),
        metadata
    );
}

#[cfg(feature = "owned-frame-transport")]
#[tokio::test]
async fn owned_receive_rejects_wrong_wire_before_public_exposure() {
    let metadata = metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    core.push_owned_rx(EncodedOwnedFrame::new(
        NativePrefixFrameMetadataCodec
            .encode_frame_metadata(WrongWireSamePayload::metadata_context(), &metadata)
            .expect("encoded metadata"),
        Some(Bytes::from_static(b"bad")),
    ));

    let status = transport
        .receive_owned(&valid_source_filter(), None)
        .await
        .expect_err("wrong selected wire must be rejected");

    assert_eq!(status.code(), UCode::InvalidArgument);
    assert!(status
        .message()
        .is_some_and(|message| message.contains("wrong selected wire")));
}

#[cfg(feature = "owned-frame-transport")]
#[tokio::test]
async fn owned_pull_receive_filters_after_selected_wire_decode() {
    let metadata = metadata_with_payload();
    let other_metadata = nonmatching_metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    core.push_owned_rx(encoded_owned_frame::<UProtocolNativeWire>(
        &other_metadata,
        b"skip",
    ));
    core.push_owned_rx(encoded_owned_frame::<UProtocolNativeWire>(
        &metadata, b"match",
    ));

    let frame = transport
        .receive_owned(&valid_source_filter(), None)
        .await
        .expect("receive matching owned frame");

    assert_eq!(frame.payload_bytes(), b"match");
    let (source_filter, sink_filter) = core.last_owned_receive_filter();
    assert_eq!(source_filter, valid_source_filter());
    assert!(sink_filter.is_none());
}

#[cfg(feature = "owned-frame-transport")]
#[tokio::test]
async fn owned_listener_filters_after_selected_wire_decode() {
    let metadata = metadata_with_payload();
    let other_metadata = nonmatching_metadata_with_payload();
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    let listener = Arc::new(CountingOwnedListener::default());
    let source = valid_source_filter();

    transport
        .register_owned_listener(&source, None, listener.clone())
        .await
        .expect("register owned listener");

    core.inject_owned(encoded_owned_frame::<UProtocolNativeWire>(
        &other_metadata,
        b"skip",
    ))
    .await;
    core.inject_owned(encoded_owned_frame::<WrongWireSamePayload>(
        &metadata, b"bad",
    ))
    .await;
    core.inject_owned(encoded_owned_frame::<UProtocolNativeWire>(
        &metadata, b"match",
    ))
    .await;

    assert_eq!(listener.payloads(), vec![b"match".to_vec()]);
    let (source_filter, sink_filter) = core.last_owned_listener_filter();
    assert_eq!(source_filter, broad_physical_source_filter());
    assert!(sink_filter.is_none());
}

#[cfg(feature = "owned-frame-transport")]
#[tokio::test]
async fn owned_listener_passes_sink_filter_to_encoded_core() {
    let metadata = notification_metadata_with_payload(notification_sink());
    let other_metadata = notification_metadata_with_payload(other_notification_sink());
    let core = InMemoryWireCore::default();
    let transport = core
        .clone()
        .into_native_prefix_wire_transport(UProtocolNativeWire);
    let listener = Arc::new(CountingOwnedListener::default());
    let source = valid_source_filter();
    let sink = notification_sink();

    transport
        .register_owned_listener(&source, Some(&sink), listener.clone())
        .await
        .expect("register sink-bearing owned listener");

    let (source_filter, sink_filter) = core.last_owned_listener_filter();
    assert_eq!(source_filter, broad_physical_source_filter());
    assert_eq!(sink_filter, Some(sink.clone()));

    core.inject_owned(encoded_owned_frame::<UProtocolNativeWire>(
        &other_metadata,
        b"skip",
    ))
    .await;
    core.inject_owned(encoded_owned_frame::<UProtocolNativeWire>(
        &metadata, b"match",
    ))
    .await;

    assert_eq!(listener.payloads(), vec![b"match".to_vec()]);

    transport
        .unregister_owned_listener(&source, Some(&sink), listener)
        .await
        .expect("unregister sink-bearing owned listener");

    let (source_filter, sink_filter) = core.last_owned_unregister_filter();
    assert_eq!(source_filter, broad_physical_source_filter());
    assert_eq!(sink_filter, Some(sink));
}

#[cfg(feature = "owned-frame-transport")]
#[test]
fn prepared_tx_spec_from_encoded_parts_preserves_encoded_metadata() {
    let metadata = metadata_with_payload();
    let encoded_metadata = b"already-selected-wire".to_vec();

    let spec =
        PreparedTxLoanSpec::from_encoded_parts(metadata.clone(), encoded_metadata.clone(), 4, 2)
            .expect("prepared TX spec");

    assert_eq!(spec.metadata(), &metadata);
    assert_eq!(spec.encoded_metadata(), encoded_metadata);
    assert_eq!(spec.payload_len(), 4);
    assert_eq!(spec.payload_alignment(), 2);
}

#[cfg(feature = "owned-frame-transport")]
#[test]
fn prepared_tx_spec_from_encoded_parts_rejects_invalid_payload_layout() {
    let error =
        PreparedTxLoanSpec::from_encoded_parts(metadata_with_payload(), b"encoded".to_vec(), 4, 3)
            .expect_err("invalid alignment must fail");

    assert_eq!(error.code(), UCode::InvalidArgument);
}
