/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Criterion bench: selected-wire adapter receive path.
// criterion harness macros generate pub items that cannot carry docs.
#![allow(missing_docs)]

mod support;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use std::sync::Arc;
use support::{encoded_frame_for, export_allocation_sample, BenchCore, CountingZeroCopyListener};
use tokio::runtime::Runtime;
use up_rust::{UProtocolNativeWire, UWithNativePrefixWire, UZeroCopyTransport};

fn bench_adapter_receive(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");
    let source = support::source_uri();
    let other_source = support::other_source_uri();

    export_allocation_sample("selected_wire_adapter_receive/accepted", || {
        let core = BenchCore::default();
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        rt.block_on(transport.receive_zero_copy(&source, None))
            .expect("accepted receive")
    });
    export_allocation_sample(
        "selected_wire_adapter_receive/rejected_then_accepted",
        || {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(
                other_source.clone(),
            ));
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
            let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
            rt.block_on(transport.receive_zero_copy(&source, None))
                .expect("receive after reject")
        },
    );
    export_allocation_sample("selected_wire_adapter_receive/listener_fanout_drop", || {
        let core = BenchCore::default();
        let transport = core
            .clone()
            .into_native_prefix_wire_transport(UProtocolNativeWire);
        let listener = Arc::new(CountingZeroCopyListener::default());
        rt.block_on(transport.register_zero_copy_listener(&source, None, listener.clone()))
            .expect("register listener");
        rt.block_on(core.inject(encoded_frame_for::<UProtocolNativeWire>(
            other_source.clone(),
        )));
        rt.block_on(core.inject(encoded_frame_for::<UProtocolNativeWire>(source.clone())));
        (listener.count(), core.listener_count())
    });
    export_allocation_sample("selected_wire_adapter_receive/setup_one_frame", || {
        let core = BenchCore::default();
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
        core.listener_count()
    });
    export_allocation_sample("selected_wire_adapter_receive/setup_two_frames", || {
        let core = BenchCore::default();
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(
            other_source.clone(),
        ));
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
        core.listener_count()
    });
    export_allocation_sample("selected_wire_adapter_receive/rejected_only_drop", || {
        let core = BenchCore::default();
        core.push_rx(encoded_frame_for::<UProtocolNativeWire>(
            other_source.clone(),
        ));
        let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
        match rt.block_on(transport.receive_zero_copy(&source, None)) {
            Ok(_) => panic!("nonmatching frame should be dropped before not found"),
            Err(error) => error,
        }
    });
    export_allocation_sample(
        "selected_wire_adapter_receive/listener_register_only",
        || {
            let core = BenchCore::default();
            let transport = core
                .clone()
                .into_native_prefix_wire_transport(UProtocolNativeWire);
            let listener = Arc::new(CountingZeroCopyListener::default());
            rt.block_on(transport.register_zero_copy_listener(&source, None, listener))
                .expect("register listener");
            core.listener_count()
        },
    );
    export_allocation_sample(
        "selected_wire_adapter_receive/listener_inject_matching_registered",
        || {
            let core = BenchCore::default();
            let transport = core
                .clone()
                .into_native_prefix_wire_transport(UProtocolNativeWire);
            let listener = Arc::new(CountingZeroCopyListener::default());
            rt.block_on(transport.register_zero_copy_listener(&source, None, listener.clone()))
                .expect("register listener");
            rt.block_on(core.inject(encoded_frame_for::<UProtocolNativeWire>(source.clone())));
            listener.count()
        },
    );
    export_allocation_sample(
        "selected_wire_adapter_receive/listener_inject_nonmatching_registered",
        || {
            let core = BenchCore::default();
            let transport = core
                .clone()
                .into_native_prefix_wire_transport(UProtocolNativeWire);
            let listener = Arc::new(CountingZeroCopyListener::default());
            rt.block_on(transport.register_zero_copy_listener(&source, None, listener.clone()))
                .expect("register listener");
            rt.block_on(core.inject(encoded_frame_for::<UProtocolNativeWire>(
                other_source.clone(),
            )));
            listener.count()
        },
    );

    c.bench_function("selected_wire_adapter_receive/setup_one_frame", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(black_box(
                source.clone(),
            )));
            black_box(core.listener_count())
        });
    });

    c.bench_function("selected_wire_adapter_receive/setup_two_frames", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(black_box(
                other_source.clone(),
            )));
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(black_box(
                source.clone(),
            )));
            black_box(core.listener_count())
        });
    });

    c.bench_function("selected_wire_adapter_receive/accepted", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
            let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
            let frame = rt
                .block_on(transport.receive_zero_copy(black_box(&source), None))
                .expect("accepted receive");
            black_box(frame)
        });
    });

    c.bench_function(
        "selected_wire_adapter_receive/rejected_then_accepted",
        |b| {
            b.iter(|| {
                let core = BenchCore::default();
                core.push_rx(encoded_frame_for::<UProtocolNativeWire>(
                    other_source.clone(),
                ));
                core.push_rx(encoded_frame_for::<UProtocolNativeWire>(source.clone()));
                let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
                let frame = rt
                    .block_on(transport.receive_zero_copy(black_box(&source), None))
                    .expect("receive after reject");
                black_box(frame)
            });
        },
    );

    c.bench_function("selected_wire_adapter_receive/rejected_only_drop", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            core.push_rx(encoded_frame_for::<UProtocolNativeWire>(
                other_source.clone(),
            ));
            let transport = core.into_native_prefix_wire_transport(UProtocolNativeWire);
            let error = match rt.block_on(transport.receive_zero_copy(black_box(&source), None)) {
                Ok(_) => panic!("nonmatching frame should be dropped before not found"),
                Err(error) => error,
            };
            black_box(error)
        });
    });

    c.bench_function(
        "selected_wire_adapter_receive/listener_register_only",
        |b| {
            b.iter(|| {
                let core = BenchCore::default();
                let transport = core
                    .clone()
                    .into_native_prefix_wire_transport(UProtocolNativeWire);
                let listener = Arc::new(CountingZeroCopyListener::default());
                rt.block_on(transport.register_zero_copy_listener(&source, None, listener))
                    .expect("register listener");
                black_box(core.listener_count())
            });
        },
    );

    c.bench_function(
        "selected_wire_adapter_receive/listener_inject_matching_registered",
        |b| {
            b.iter_batched(
                || {
                    let core = BenchCore::default();
                    let transport = core
                        .clone()
                        .into_native_prefix_wire_transport(UProtocolNativeWire);
                    let listener = Arc::new(CountingZeroCopyListener::default());
                    rt.block_on(transport.register_zero_copy_listener(
                        &source,
                        None,
                        listener.clone(),
                    ))
                    .expect("register listener");
                    (
                        core,
                        listener,
                        encoded_frame_for::<UProtocolNativeWire>(source.clone()),
                    )
                },
                |(core, listener, frame)| {
                    rt.block_on(core.inject(frame));
                    black_box(listener.count())
                },
                BatchSize::SmallInput,
            );
        },
    );

    c.bench_function(
        "selected_wire_adapter_receive/listener_inject_nonmatching_registered",
        |b| {
            b.iter_batched(
                || {
                    let core = BenchCore::default();
                    let transport = core
                        .clone()
                        .into_native_prefix_wire_transport(UProtocolNativeWire);
                    let listener = Arc::new(CountingZeroCopyListener::default());
                    rt.block_on(transport.register_zero_copy_listener(
                        &source,
                        None,
                        listener.clone(),
                    ))
                    .expect("register listener");
                    (
                        core,
                        listener,
                        encoded_frame_for::<UProtocolNativeWire>(other_source.clone()),
                    )
                },
                |(core, listener, frame)| {
                    rt.block_on(core.inject(frame));
                    black_box(listener.count())
                },
                BatchSize::SmallInput,
            );
        },
    );

    c.bench_function("selected_wire_adapter_receive/listener_fanout_drop", |b| {
        b.iter(|| {
            let core = BenchCore::default();
            let transport = core
                .clone()
                .into_native_prefix_wire_transport(UProtocolNativeWire);
            let listener = Arc::new(CountingZeroCopyListener::default());
            rt.block_on(transport.register_zero_copy_listener(&source, None, listener.clone()))
                .expect("register listener");
            rt.block_on(core.inject(encoded_frame_for::<UProtocolNativeWire>(
                other_source.clone(),
            )));
            rt.block_on(core.inject(encoded_frame_for::<UProtocolNativeWire>(source.clone())));
            black_box((listener.count(), core.listener_count()))
        });
    });
}

criterion_group!(benches, bench_adapter_receive);
criterion_main!(benches);
