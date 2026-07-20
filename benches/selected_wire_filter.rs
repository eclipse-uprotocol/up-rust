/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Criterion bench: selected-wire filter matching.
// criterion harness macros generate pub items that cannot carry docs.
#![allow(missing_docs)]

mod support;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use support::{encoded_frame_for, export_allocation_sample, BenchCore};
use tokio::runtime::Runtime;
use up_rust::{UProtocolNativeWire, UWithNativePrefixWire, UZeroCopyTransport};

fn bench_filter(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");
    let source = support::source_uri();
    let wildcard = support::wildcard_source_uri();

    export_allocation_sample("selected_wire_filter/exact_receive", || {
        let core = BenchCore::default();
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        rt.block_on(transport.receive_zero_copy(&source, None))
            .expect("receive exact")
    });
    export_allocation_sample("selected_wire_filter/wildcard_receive", || {
        let core = BenchCore::default();
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        rt.block_on(transport.receive_zero_copy(&wildcard, None))
            .expect("receive wildcard")
    });

    c.bench_function("selected_wire_filter/exact_receive", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
            let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
            let frame = rt
                .block_on(transport.receive_zero_copy(black_box(&source), None))
                .expect("receive exact");
            black_box(frame)
        });
    });

    c.bench_function("selected_wire_filter/wildcard_receive", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
            let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
            let frame = rt
                .block_on(transport.receive_zero_copy(black_box(&wildcard), None))
                .expect("receive wildcard");
            black_box(frame)
        });
    });
}

criterion_group!(benches, bench_filter);
criterion_main!(benches);
