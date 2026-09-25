/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Bounded payload-contract diagnostics.
//!
//! This executable records one-shot allocation and explicit-copy shape. Its
//! elapsed values are smoke diagnostics, not host-performance measurements.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use bytes::Bytes;
use up_rust::bench_fixtures::payload_contract::{
    all_cases, case_by_kind, init_streamer_4k, stable_owned_fixture_for, validate_protobuf_bytes,
    PayloadContractCaseKind, StreamChunk4kV1,
};
use up_rust::communication::zero_copy::Publisher;
use up_rust::communication::CallOptions;
use up_rust::frame::metadata::try_project_umessage_to_frame_metadata_with_native_profile;
use up_rust::{
    LoanedPayload, NativePrefixFrameMetadataCodec, NativeProfile, NativeProfileAgreement,
    NativeProfileMode, NativeProfileTable, PayloadEncoding, PayloadLoanProvenance,
    PreparedTxLoanSpec, StableContainerPayload, StableContainerWireFormat, StablePayload,
    StablePayloadInit, StaticUriProvider, UEncodedLoanedRxFrame, UEncodedRxFrame, UFrameMetadata,
    UMessageBuilder, UStatus, UTxBuffer, UTxLoanSpec, UUninitTxBuffer, UUri, UVecTxBuffer,
    UVecUninitTxBuffer, UWire, UWireMetadataCodec, UWireRx, UWireTransport, UZeroCopyTransportCore,
    UZeroCopyUninitTransportCore,
};

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegates the unchanged layout to the process allocator.
        let pointer = unsafe { System.alloc(layout) };
        record_allocation(pointer, layout.size());
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegates the unchanged layout to the process allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        record_allocation(pointer, layout.size());
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from this allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The pointer and old layout came from this allocator.
        let pointer = unsafe { System.realloc(pointer, layout, new_size) };
        record_allocation(pointer, new_size);
        pointer
    }
}

fn record_allocation(pointer: *mut u8, size: usize) {
    if !pointer.is_null() && COUNTING.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
    }
}

fn measure<T>(
    name: &str,
    explicit_copies: usize,
    validations: usize,
    operation: impl FnOnce() -> T,
) -> T {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    let started = Instant::now();
    COUNTING.store(true, Ordering::Relaxed);
    let result = black_box(operation());
    COUNTING.store(false, Ordering::Relaxed);
    let elapsed = started.elapsed();
    println!(
        "subject={name}\tallocations={}\tallocated_bytes={}\texplicit_copies={explicit_copies}\tvalidations={validations}\telapsed_ns={}\thost_performance_claim=false",
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
        elapsed.as_nanos(),
    );
    result
}

fn benchmark_profile() -> NativeProfileAgreement {
    // Explicit one-type agreement for this same-process benchmark only.
    let table = NativeProfileTable::new([(
        PayloadEncoding::from_id(0xF301).unwrap(),
        StreamChunk4kV1::native_representation(),
    )])
    .unwrap();
    let profile = NativeProfile::new(
        "payload-contract-benchmark",
        1,
        NativeProfileMode::Table(table),
    )
    .unwrap();
    NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap()
}

fn stable_encoding(profile: &NativeProfileAgreement) -> PayloadEncoding {
    StableContainerPayload::<StreamChunk4kV1>::encoding(profile)
        .expect("agreed benchmark representation")
}

fn payload_metadata(profile: &NativeProfileAgreement) -> UFrameMetadata {
    UFrameMetadata::publish(
        UUri::try_from_parts("payload-contract-bench", 0x4210, 1, 0x9000)
            .expect("benchmark URI is valid"),
    )
    .with_native_payload_identity(
        StableContainerPayload::<StreamChunk4kV1>::identity(profile)
            .expect("agreed benchmark representation"),
    )
    .build()
    .expect("benchmark metadata is valid")
}

#[derive(Debug)]
struct RawFrame {
    encoded_metadata: Vec<u8>,
    payload: Vec<u8>,
}

impl UEncodedRxFrame for RawFrame {
    type PayloadReader<'a> = Cursor<&'a [u8]>;
    type PayloadSlices<'a> = core::iter::Once<&'a [u8]>;

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
        core::iter::once(&self.payload)
    }

    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        Some(&self.payload)
    }
}

impl UEncodedLoanedRxFrame for RawFrame {
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, up_rust::UWireError> {
        // SAFETY: `payload` is owned by this receive object and remains alive for
        // the returned borrow; no allocation, copy, or coalescing occurs here.
        Ok(unsafe {
            LoanedPayload::new_unchecked(&self.payload, PayloadLoanProvenance::OpaqueTransportLoan)
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct NullCore;

#[async_trait]
impl UZeroCopyTransportCore for NullCore {
    type Tx = UVecTxBuffer;
    type Rx = RawFrame;

    async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
        let (metadata, _, payload_len, payload_alignment) = spec.into_parts();
        UVecTxBuffer::with_alignment(metadata, payload_len, payload_alignment)
    }

    async fn send_prepared_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus> {
        black_box(buffer.payload());
        Ok(())
    }
}

#[async_trait]
impl UZeroCopyUninitTransportCore for NullCore {
    type UninitTx = UVecUninitTxBuffer;

    async fn loan_prepared_uninit_tx(
        &self,
        spec: PreparedTxLoanSpec,
    ) -> Result<Self::UninitTx, UStatus> {
        let (metadata, _, payload_len, payload_alignment) = spec.into_parts();
        UVecUninitTxBuffer::with_alignment(metadata, payload_len, payload_alignment)
    }
}

fn main() {
    let profile = benchmark_profile();
    println!("native_profile_mode=table\tnative_profile_domain={}\tnative_profile_version={}\tnative_profile_digest={}",
        profile.profile().domain(), profile.profile().version(), hex::encode(profile.profile().content_digest()));
    let smoke = std::env::var_os("UP_PAYLOAD_BENCH_SMOKE").is_some();
    let iterations = if smoke { 1 } else { 10 };
    println!(
        "mode={}\titerations={iterations}\telapsed_values_are_diagnostic_only=true",
        if smoke { "smoke" } else { "bounded" }
    );

    for case in all_cases() {
        println!(
            "payload={}\tsemantic_bytes={}\tstable_bytes={}\tstable_alignment={}",
            case.name(),
            case.semantic_reference_len(),
            up_rust::bench_fixtures::payload_contract::stable_payload_len(case),
            up_rust::bench_fixtures::payload_contract::stable_payload_align(case),
        );
    }

    let case = case_by_kind(PayloadContractCaseKind::Streamer4k);
    let canonical = stable_owned_fixture_for(case, 1, &profile).expect("canonical stable fixture");

    for sequence in 0..iterations {
        measure("generated_uninitialized_init", 0, 1, || {
            let mut buffer = UVecUninitTxBuffer::with_alignment(
                payload_metadata(&profile),
                canonical.bytes.len(),
                canonical.stable_align,
            )
            .expect("uninitialized loan");
            {
                let initialized = init_streamer_4k(
                    StreamChunk4kV1::init(buffer.payload_uninit_mut())
                        .expect("stable initializer construction"),
                    sequence,
                )
                .expect("generated stable initialization");
                black_box(initialized.as_bytes());
            }
            // SAFETY: The generated typestate initializer completed every field.
            unsafe { buffer.assume_payload_initialized() }
        });

        measure("initialized_loan_copy", 1, 0, || {
            let mut buffer = UVecTxBuffer::with_alignment(
                payload_metadata(&profile),
                canonical.bytes.len(),
                canonical.stable_align,
            )
            .expect("initialized loan");
            buffer.payload_mut().copy_from_slice(&canonical.bytes);
            buffer
        });

        measure("direct_memcpy_control", 1, 0, || {
            let mut bytes = vec![0_u8; canonical.bytes.len()];
            bytes.copy_from_slice(&canonical.bytes);
            bytes
        });

        measure("direct_metadata_builder", 0, 1, || {
            payload_metadata(&profile)
        });
        measure("umessage_projection", 0, 2, || {
            let mut builder = UMessageBuilder::publish(
                UUri::try_from_parts("payload-contract-bench", 0x4210, 1, 0x9000)
                    .expect("benchmark URI is valid"),
            );
            let message = builder
                .build_with_payload(Bytes::from_static(b"x"), stable_encoding(&profile))
                .expect("benchmark message");
            try_project_umessage_to_frame_metadata_with_native_profile(&message, &profile)
                .expect("metadata projection")
        });

        let codec = NativePrefixFrameMetadataCodec;
        let metadata = payload_metadata(&profile);
        let encoded_metadata = measure("metadata_encode", 0, 1, || {
            codec
                .encode_frame_metadata(StableContainerWireFormat::metadata_context(), &metadata)
                .expect("metadata encode")
        });
        measure("metadata_decode", 0, 1, || {
            codec
                .decode_frame_metadata(
                    StableContainerWireFormat::metadata_context(),
                    &encoded_metadata,
                )
                .expect("metadata decode")
        });

        measure("prepared_loan", 0, 1, || {
            PreparedTxLoanSpec::from_validated::<
                StableContainerWireFormat,
                NativePrefixFrameMetadataCodec,
            >(
                UTxLoanSpec::payload(
                    payload_metadata(&profile),
                    canonical.bytes.len(),
                    canonical.stable_align,
                )
                .expect("TX loan spec"),
                &codec,
            )
            .expect("prepared TX loan")
        });

        let receive = UWireRx::<
            RawFrame,
            StableContainerWireFormat,
            NativePrefixFrameMetadataCodec,
        >::try_from_encoded_with_native_profile(
            RawFrame {
                encoded_metadata: encoded_metadata.clone(),
                payload: canonical.bytes.clone(),
            },
            &codec,
            profile.clone(),
        )
        .expect("selected-wire receive");
        measure("selected_wire_borrow", 0, 1, || {
            receive
                .borrow_payload::<StreamChunk4kV1>()
                .expect("stable borrow")
        });

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("benchmark runtime");
        let prepared = PreparedTxLoanSpec::from_validated::<
            StableContainerWireFormat,
            NativePrefixFrameMetadataCodec,
        >(
            UTxLoanSpec::payload(
                payload_metadata(&profile),
                canonical.bytes.len(),
                canonical.stable_align,
            )
            .expect("TX loan spec"),
            &codec,
        )
        .expect("prepared TX loan");
        measure("core_only_loan_commit", 0, 0, || {
            runtime
                .block_on(async {
                    let buffer = NullCore.loan_prepared_tx(prepared).await?;
                    NullCore.send_prepared_zero_copy(buffer).await
                })
                .expect("core-only loan and commit")
        });

        let transport = UWireTransport::with_native_profile(
            NullCore,
            StableContainerWireFormat,
            NativePrefixFrameMetadataCodec,
            profile.clone(),
        );
        measure("adapter_core_stable_send", 0, 2, || {
            runtime
                .block_on(transport.send_stable_payload::<StreamChunk4kV1, _>(
                    payload_metadata(&profile),
                    |init| {
                        init_streamer_4k(init.into_initializer(), sequence)
                            .expect("generated stable initialization")
                    },
                ))
                .expect("adapter/core send")
        });

        let transport = Arc::new(UWireTransport::with_native_profile(
            NullCore,
            StableContainerWireFormat,
            NativePrefixFrameMetadataCodec,
            profile.clone(),
        ));
        let provider = Arc::new(
            StaticUriProvider::new("payload-contract-bench", 0x4210, 1)
                .expect("benchmark URI provider"),
        );
        let publisher = Publisher::new(transport, provider);
        measure("l2_stable_publish", 0, 2, || {
            runtime
                .block_on(publisher.publish_stable::<StreamChunk4kV1, _>(
                    0x9000,
                    CallOptions::for_publish(None, None, None),
                    |init| {
                        init_streamer_4k(init.into_initializer(), sequence)
                            .expect("generated stable initialization")
                    },
                ))
                .expect("L2 stable publish")
        });
    }

    for case in all_cases() {
        let bytes = up_rust::bench_fixtures::payload_contract::protobuf_encoded_bytes_for(case, 1)
            .expect("protobuf fixture encoding");
        validate_protobuf_bytes(case, 1, &bytes).expect("protobuf fixture validation");
    }
}
