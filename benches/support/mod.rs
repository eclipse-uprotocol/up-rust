/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#![allow(dead_code)]

use async_trait::async_trait;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::VecDeque;
use std::hint::black_box;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use up_rust::{
    NativePrefixFrameMetadataCodec, PayloadEncoding, PreparedTxLoanSpec, UCode, UEncodedRxFrame,
    UEncodedZeroCopyListener, UFrameMetadata, UProtocolNativeWire, UStatus, UTxBuffer, UUri,
    UVecTxBuffer, UWire, UWireMetadataCodec, UWireRx, UZeroCopyListener, UZeroCopyTransportCore,
    WireIdentity,
};

pub struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationSample {
    pub allocations: usize,
    pub bytes: usize,
}

pub fn reset_allocations() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

pub fn allocation_sample() -> AllocationSample {
    AllocationSample {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
    }
}

pub fn export_allocation_sample<T>(selector: &str, operation: impl FnOnce() -> T) {
    reset_allocations();
    let result = operation();
    let sample = allocation_sample();
    black_box(result);
    eprintln!(
        "P51_ALLOCATION_SAMPLE selector={selector} allocations={} bytes={}",
        sample.allocations, sample.bytes
    );
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchRxFrame {
    encoded_metadata: Vec<u8>,
    payload: Vec<u8>,
}

impl BenchRxFrame {
    pub fn new(encoded_metadata: Vec<u8>, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            encoded_metadata,
            payload: payload.into(),
        }
    }
}

impl UEncodedRxFrame for BenchRxFrame {
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

#[derive(Clone, Default)]
pub struct BenchCore {
    state: Arc<Mutex<BenchCoreState>>,
}

#[derive(Default)]
struct BenchCoreState {
    received: VecDeque<BenchRxFrame>,
    listeners: Vec<Arc<dyn UEncodedZeroCopyListener<BenchRxFrame>>>,
    prepared_tx: Vec<PreparedTxLoanSpec>,
    sent_payloads: Vec<Vec<u8>>,
}

impl BenchCore {
    pub fn push_rx(&self, frame: BenchRxFrame) {
        self.state
            .lock()
            .expect("bench core lock poisoned")
            .received
            .push_back(frame);
    }

    pub async fn inject(&self, frame: BenchRxFrame) {
        let listeners = self
            .state
            .lock()
            .expect("bench core lock poisoned")
            .listeners
            .clone();
        for listener in listeners {
            listener.on_receive_encoded_zero_copy(frame.clone()).await;
        }
    }

    pub fn listener_count(&self) -> usize {
        self.state
            .lock()
            .expect("bench core lock poisoned")
            .listeners
            .len()
    }
}

#[async_trait]
impl UZeroCopyTransportCore for BenchCore {
    type Tx = UVecTxBuffer;
    type Rx = BenchRxFrame;

    async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
        self.state
            .lock()
            .expect("bench core lock poisoned")
            .prepared_tx
            .push(spec.clone());
        UVecTxBuffer::with_alignment(
            spec.metadata().clone(),
            spec.payload_len(),
            spec.payload_alignment(),
        )
    }

    async fn send_prepared_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
        self.state
            .lock()
            .expect("bench core lock poisoned")
            .sent_payloads
            .push(buffer.payload().to_vec());
        Ok(())
    }

    async fn receive_encoded_zero_copy(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus> {
        self.state
            .lock()
            .expect("bench core lock poisoned")
            .received
            .pop_front()
            .ok_or_else(|| UStatus::fail_with_code(UCode::NotFound, "no bench frame queued"))
    }

    async fn register_encoded_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        self.state
            .lock()
            .expect("bench core lock poisoned")
            .listeners
            .push(listener);
        Ok(())
    }

    async fn unregister_encoded_zero_copy_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        listener: Arc<dyn UEncodedZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus> {
        let mut state = self.state.lock().expect("bench core lock poisoned");
        let Some(index) = state
            .listeners
            .iter()
            .position(|registered| Arc::ptr_eq(registered, &listener))
        else {
            return Err(UStatus::fail_with_code(
                UCode::NotFound,
                "bench listener not registered",
            ));
        };
        state.listeners.remove(index);
        Ok(())
    }
}

pub struct WrongWireSamePayload;

impl UWire for WrongWireSamePayload {
    const WIRE_ID: WireIdentity = WireIdentity::new("bench.wrong-wire", 0x9001);
    const PAYLOAD_FAMILY_ID: WireIdentity = UProtocolNativeWire::PAYLOAD_FAMILY_ID;
    const METADATA_LAYOUT_ID: WireIdentity = UProtocolNativeWire::METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = UProtocolNativeWire::FORMAT_VERSION;
}

pub struct SameWireWrongPayload;

impl UWire for SameWireWrongPayload {
    const WIRE_ID: WireIdentity = UProtocolNativeWire::WIRE_ID;
    const PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("bench.wrong-payload", 0x9002);
    const METADATA_LAYOUT_ID: WireIdentity = UProtocolNativeWire::METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = UProtocolNativeWire::FORMAT_VERSION;
}

pub fn source_uri() -> UUri {
    UUri::try_from_parts("authority-a", 0x5BA0, 0x01, 0x8001).expect("source uri")
}

pub fn other_source_uri() -> UUri {
    UUri::try_from_parts("authority-a", 0x5BA1, 0x01, 0x8002).expect("other source uri")
}

pub fn wildcard_source_uri() -> UUri {
    UUri::try_from_parts("authority-a", u32::MAX, u8::MAX, u16::MAX).expect("wildcard source uri")
}

pub fn metadata_for(source: UUri) -> UFrameMetadata {
    UFrameMetadata::publish(source)
        .with_payload_encoding(PayloadEncoding::PROTOBUF)
        .build()
        .expect("bench metadata")
}

pub fn encoded_metadata<W: UWire>(metadata: &UFrameMetadata) -> Vec<u8> {
    NativePrefixFrameMetadataCodec
        .encode_frame_metadata(W::metadata_context(), metadata)
        .expect("encode metadata")
}

pub fn decode_metadata<W: UWire>(encoded: &[u8]) -> UFrameMetadata {
    NativePrefixFrameMetadataCodec
        .decode_frame_metadata(W::metadata_context(), encoded)
        .expect("decode metadata")
}

pub fn encoded_frame_for<W: UWire>(source: UUri) -> BenchRxFrame {
    let metadata = metadata_for(source);
    BenchRxFrame::new(encoded_metadata::<W>(&metadata), b"bench-payload".to_vec())
}

pub fn native_wire_rx(
    frame: BenchRxFrame,
) -> UWireRx<BenchRxFrame, UProtocolNativeWire, NativePrefixFrameMetadataCodec> {
    UWireRx::try_from_encoded(frame, &NativePrefixFrameMetadataCodec).expect("wire rx")
}

#[derive(Default)]
pub struct CountingZeroCopyListener {
    count: AtomicUsize,
}

impl CountingZeroCopyListener {
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl UZeroCopyListener<UWireRx<BenchRxFrame, UProtocolNativeWire, NativePrefixFrameMetadataCodec>>
    for CountingZeroCopyListener
{
    async fn on_receive_zero_copy(
        &self,
        _frame: UWireRx<BenchRxFrame, UProtocolNativeWire, NativePrefixFrameMetadataCodec>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }
}
