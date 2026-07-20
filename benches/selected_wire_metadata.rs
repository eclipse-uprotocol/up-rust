/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Criterion bench: native-prefix metadata encode/decode.
// criterion harness macros generate pub items that cannot carry docs.
#![allow(missing_docs)]

mod support;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use support::{
    allocation_sample, decode_metadata, encoded_metadata, export_allocation_sample, metadata_for,
    reset_allocations, AllocationSample, SameWireWrongPayload, WrongWireSamePayload,
};
use up_rust::{UProtocolNativeWire, UWire, UWireMetadataCodec};

fn bench_metadata(c: &mut Criterion) {
    let metadata = metadata_for(support::source_uri());
    let encoded = encoded_metadata::<UProtocolNativeWire>(&metadata);

    export_allocation_sample("selected_wire_metadata/encode", || {
        encoded_metadata::<UProtocolNativeWire>(&metadata)
    });
    export_allocation_sample("selected_wire_metadata/decode", || {
        decode_metadata::<UProtocolNativeWire>(&encoded)
    });
    export_allocation_sample("selected_wire_metadata/wrong_wire_reject", || {
        up_rust::NativePrefixFrameMetadataCodec
            .decode_frame_metadata(WrongWireSamePayload::metadata_context(), &encoded)
            .expect_err("wrong wire must reject")
    });
    export_allocation_sample("selected_wire_metadata/payload_family_reject", || {
        up_rust::NativePrefixFrameMetadataCodec
            .decode_frame_metadata(SameWireWrongPayload::metadata_context(), &encoded)
            .expect_err("wrong payload family must reject")
    });

    c.bench_function("selected_wire_metadata/encode", |b| {
        b.iter(|| {
            reset_allocations();
            let bytes = encoded_metadata::<UProtocolNativeWire>(black_box(&metadata));
            let sample = allocation_sample();
            black_box((bytes, sample))
        });
    });

    c.bench_function("selected_wire_metadata/decode", |b| {
        b.iter(|| {
            reset_allocations();
            let decoded = decode_metadata::<UProtocolNativeWire>(black_box(&encoded));
            let sample = allocation_sample();
            black_box((decoded, sample))
        });
    });

    c.bench_function("selected_wire_metadata/wrong_wire_reject", |b| {
        b.iter(|| {
            reset_allocations();
            let error = up_rust::NativePrefixFrameMetadataCodec
                .decode_frame_metadata(
                    WrongWireSamePayload::metadata_context(),
                    black_box(&encoded),
                )
                .expect_err("wrong wire must reject");
            let sample = allocation_sample();
            black_box((error, sample))
        });
    });

    c.bench_function("selected_wire_metadata/payload_family_reject", |b| {
        b.iter(|| {
            reset_allocations();
            let error = up_rust::NativePrefixFrameMetadataCodec
                .decode_frame_metadata(
                    SameWireWrongPayload::metadata_context(),
                    black_box(&encoded),
                )
                .expect_err("wrong payload family must reject");
            let sample: AllocationSample = allocation_sample();
            black_box((error, sample))
        });
    });
}

criterion_group!(benches, bench_metadata);
criterion_main!(benches);
