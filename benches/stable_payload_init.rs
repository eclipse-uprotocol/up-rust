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

//! Criterion bench: stable-payload typed initialization into uninit storage.
// criterion harness macros generate pub items that cannot carry docs.
#![allow(missing_docs)]

use std::mem::{self, offset_of, size_of, MaybeUninit};
use std::ptr;

use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup,
    Criterion,
};
use up_rust::StablePayloadInit;

const OWNED_STORAGE_BATCH: BatchSize = BatchSize::NumIterations(64);

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "benchmark.r7.FlatPod")]
struct FlatPod {
    sequence: u64,
    code: u32,
    flags: [u8; 4],
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "benchmark.r7.Bytes4k")]
struct Bytes4k {
    bytes: [u8; 4096],
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "benchmark.r7.Bytes64k")]
struct Bytes64k {
    bytes: [u8; 65_536],
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "benchmark.r7.TypedArray")]
struct TypedArray {
    values: [u32; 1024],
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "benchmark.r7.NestedLeaf")]
struct NestedLeaf {
    x: u32,
    y: u32,
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "benchmark.r7.NestedPayload")]
struct NestedPayload {
    head: NestedLeaf,
    leaves: [NestedLeaf; 8],
    bytes: [u8; 32],
}

fn uninit_bytes<T>(storage: &mut MaybeUninit<T>) -> &mut [MaybeUninit<u8>] {
    // SAFETY: The returned byte slice covers exactly this uniquely borrowed
    // uninitialized storage and inherits its alignment.
    unsafe {
        std::slice::from_raw_parts_mut(
            std::ptr::from_mut(storage).cast::<MaybeUninit<u8>>(),
            mem::size_of::<T>(),
        )
    }
}

#[inline(always)]
fn validated_pointer<T>(storage: &mut MaybeUninit<T>) -> *mut MaybeUninit<T> {
    let bytes = uninit_bytes(storage);
    assert_eq!(bytes.len(), size_of::<T>());
    assert_eq!(bytes.as_ptr().align_offset(mem::align_of::<T>()), 0);
    bytes.as_mut_ptr().cast::<MaybeUninit<T>>()
}

fn bench_owned_storage<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    mut initialize: impl FnMut(&mut MaybeUninit<T>),
) {
    group.bench_function(name, |b| {
        b.iter_batched(
            Box::<T>::new_uninit,
            |mut storage| {
                initialize(storage.as_mut());
                storage
            },
            OWNED_STORAGE_BATCH,
        );
    });
}

/// Writes one field using the pre-builder raw-pointer mechanism.
///
/// # Safety
///
/// `offset` must identify an aligned, in-bounds `U` field in `T` which has not
/// already been initialized.
#[inline(always)]
unsafe fn pointer_write<T, U>(storage: *mut MaybeUninit<T>, offset: usize, value: U) {
    debug_assert!(offset
        .checked_add(size_of::<U>())
        .is_some_and(|end| end <= size_of::<T>()));
    // SAFETY: The caller guarantees that this is an aligned, in-bounds field.
    let destination = unsafe { storage.cast::<u8>().add(offset).cast::<U>() };
    debug_assert_eq!(destination.align_offset(mem::align_of::<U>()), 0);
    // SAFETY: The caller also guarantees exclusive, single initialization.
    unsafe { destination.write(value) };
}

/// Copies a field using the pre-builder raw-pointer mechanism.
///
/// # Safety
///
/// The source and destination must not overlap, and `offset..offset +
/// size_of_val(source)` must be an aligned, in-bounds, uninitialized region of
/// `T` suitable for `U` values.
#[inline(always)]
unsafe fn pointer_copy<T, U>(storage: *mut MaybeUninit<T>, offset: usize, source: &[U]) {
    debug_assert!(offset
        .checked_add(mem::size_of_val(source))
        .is_some_and(|end| end <= size_of::<T>()));
    // SAFETY: The caller guarantees alignment, bounds, and non-overlap.
    let destination = unsafe { storage.cast::<u8>().add(offset).cast::<U>() };
    debug_assert_eq!(destination.align_offset(mem::align_of::<U>()), 0);
    // SAFETY: The caller guarantees that `source.len()` values fit and do not overlap.
    unsafe { ptr::copy_nonoverlapping(source.as_ptr(), destination, source.len()) };
}

/// Fills typed array elements using direct pointer writes.
///
/// # Safety
///
/// `offset` must identify an aligned, in-bounds, uninitialized `[U; len]`
/// region of `T`.
#[inline(always)]
unsafe fn pointer_fill<T, U: Copy>(
    storage: *mut MaybeUninit<T>,
    offset: usize,
    len: usize,
    value: U,
) {
    debug_assert!(size_of::<U>()
        .checked_mul(len)
        .and_then(|byte_len| offset.checked_add(byte_len))
        .is_some_and(|end| end <= size_of::<T>()));
    // SAFETY: The caller guarantees alignment and bounds for all `len` elements.
    let destination = unsafe { storage.cast::<u8>().add(offset).cast::<U>() };
    debug_assert_eq!(destination.align_offset(mem::align_of::<U>()), 0);
    for index in 0..len {
        // SAFETY: The caller guarantees the complete array region is writable.
        unsafe { destination.add(index).write(value) };
    }
}

/// Zeroes an implicit padding gap using the pre-builder pointer mechanism.
///
/// # Safety
///
/// `start..end` must be an in-bounds padding region of `T` which has not
/// already been initialized.
#[inline(always)]
unsafe fn pointer_zero_padding<T>(storage: *mut MaybeUninit<T>, start: usize, end: usize) {
    debug_assert!(start <= end && end <= size_of::<T>());
    // SAFETY: The caller guarantees the byte range is in-bounds padding.
    let destination = unsafe { storage.cast::<u8>().add(start) };
    // SAFETY: Every bit pattern is valid for padding bytes.
    unsafe { destination.write_bytes(0, end - start) };
}

/// Initializes one nested leaf through direct field pointers.
///
/// # Safety
///
/// `leaf_offset` must identify an in-bounds, uninitialized `NestedLeaf` within
/// `T`.
#[inline(always)]
unsafe fn pointer_write_leaf<T>(storage: *mut MaybeUninit<T>, leaf_offset: usize, x: u32, y: u32) {
    let x_offset = leaf_offset + offset_of!(NestedLeaf, x);
    let y_offset = leaf_offset + offset_of!(NestedLeaf, y);
    // SAFETY: The caller provides a valid leaf region; `offset_of!` and
    // `size_of` identify its fields and any padding exactly.
    unsafe {
        pointer_zero_padding(storage, leaf_offset, x_offset);
        pointer_write(storage, x_offset, x);
        pointer_zero_padding(storage, x_offset + size_of::<u32>(), y_offset);
        pointer_write(storage, y_offset, y);
        pointer_zero_padding(
            storage,
            y_offset + size_of::<u32>(),
            leaf_offset + size_of::<NestedLeaf>(),
        );
    }
}

fn stable_payload_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("stable_payload_init");
    let bytes_4k = vec![0x5a_u8; 4096];
    let bytes_64k = vec![0xa5_u8; 65_536];
    let typed = vec![0xdead_beef_u32; 1024];
    let flags = [1_u8, 2, 3, 4];

    bench_owned_storage(
        &mut group,
        "pointer/flat_pod",
        |storage: &mut MaybeUninit<FlatPod>| {
            let storage = validated_pointer(storage);
            let sequence_offset = offset_of!(FlatPod, sequence);
            let code_offset = offset_of!(FlatPod, code);
            let flags_offset = offset_of!(FlatPod, flags);
            // SAFETY: `offset_of!`/`size_of` describe disjoint fields and all
            // padding in this exclusively owned `FlatPod` allocation.
            unsafe {
                pointer_zero_padding(storage, 0, sequence_offset);
                pointer_write(storage, sequence_offset, black_box(42_u64));
                pointer_zero_padding(storage, sequence_offset + size_of::<u64>(), code_offset);
                pointer_write(storage, code_offset, black_box(7_u32));
                pointer_zero_padding(storage, code_offset + size_of::<u32>(), flags_offset);
                pointer_copy(storage, flags_offset, black_box(flags.as_slice()));
                pointer_zero_padding(
                    storage,
                    flags_offset + size_of::<[u8; 4]>(),
                    size_of::<FlatPod>(),
                );
            }
        },
    );
    bench_owned_storage(
        &mut group,
        "borrowed/flat_pod",
        |storage: &mut MaybeUninit<FlatPod>| {
            let _initialized = FlatPod::init_from_uninit_bytes(uninit_bytes(storage))
                .expect("valid flat POD storage")
                .sequence(black_box(42_u64))
                .code(black_box(7_u32))
                .flags_from_array(black_box(&flags))
                .finish()
                .expect("complete flat POD");
        },
    );

    bench_owned_storage(
        &mut group,
        "pointer/bytes_4k_copy",
        |storage: &mut MaybeUninit<Bytes4k>| {
            let storage = validated_pointer(storage);
            let bytes_offset = offset_of!(Bytes4k, bytes);
            // SAFETY: The black-boxed source has exactly the field length, and the
            // layout constants cover the owned allocation exactly once.
            unsafe {
                pointer_zero_padding(storage, 0, bytes_offset);
                pointer_copy(storage, bytes_offset, black_box(bytes_4k.as_slice()));
                pointer_zero_padding(
                    storage,
                    bytes_offset + size_of::<[u8; 4096]>(),
                    size_of::<Bytes4k>(),
                );
            }
        },
    );
    bench_owned_storage(
        &mut group,
        "borrowed/bytes_4k_copy",
        |storage: &mut MaybeUninit<Bytes4k>| {
            let _initialized = Bytes4k::init_from_uninit_bytes(uninit_bytes(storage))
                .expect("valid 4 KiB storage")
                .bytes_from_slice(black_box(bytes_4k.as_slice()))
                .expect("exact 4 KiB input")
                .finish()
                .expect("complete 4 KiB payload");
        },
    );

    bench_owned_storage(
        &mut group,
        "pointer/bytes_64k_copy",
        |storage: &mut MaybeUninit<Bytes64k>| {
            let storage = validated_pointer(storage);
            let bytes_offset = offset_of!(Bytes64k, bytes);
            // SAFETY: The black-boxed source has exactly the field length, and the
            // layout constants cover the owned allocation exactly once.
            unsafe {
                pointer_zero_padding(storage, 0, bytes_offset);
                pointer_copy(storage, bytes_offset, black_box(bytes_64k.as_slice()));
                pointer_zero_padding(
                    storage,
                    bytes_offset + size_of::<[u8; 65_536]>(),
                    size_of::<Bytes64k>(),
                );
            }
        },
    );
    bench_owned_storage(
        &mut group,
        "borrowed/bytes_64k_copy",
        |storage: &mut MaybeUninit<Bytes64k>| {
            let _initialized = Bytes64k::init_from_uninit_bytes(uninit_bytes(storage))
                .expect("valid 64 KiB storage")
                .bytes_from_slice(black_box(bytes_64k.as_slice()))
                .expect("exact 64 KiB input")
                .finish()
                .expect("complete 64 KiB payload");
        },
    );

    bench_owned_storage(
        &mut group,
        "pointer/typed_array_copy",
        |storage: &mut MaybeUninit<TypedArray>| {
            let storage = validated_pointer(storage);
            let values_offset = offset_of!(TypedArray, values);
            // SAFETY: The black-boxed source has 1024 elements, and the layout
            // constants cover the owned allocation exactly once.
            unsafe {
                pointer_zero_padding(storage, 0, values_offset);
                pointer_copy(storage, values_offset, black_box(typed.as_slice()));
                pointer_zero_padding(
                    storage,
                    values_offset + size_of::<[u32; 1024]>(),
                    size_of::<TypedArray>(),
                );
            }
        },
    );
    bench_owned_storage(
        &mut group,
        "borrowed/typed_array_copy",
        |storage: &mut MaybeUninit<TypedArray>| {
            let _initialized = TypedArray::init_from_uninit_bytes(uninit_bytes(storage))
                .expect("valid typed-array storage")
                .values_from_slice(black_box(typed.as_slice()))
                .expect("exact typed-array input")
                .finish()
                .expect("complete typed-array payload");
        },
    );

    bench_owned_storage(
        &mut group,
        "pointer/typed_array_fill",
        |storage: &mut MaybeUninit<TypedArray>| {
            let storage = validated_pointer(storage);
            let values_offset = offset_of!(TypedArray, values);
            // SAFETY: The layout constants describe the 1024-element field and all
            // padding in this exclusively owned allocation.
            unsafe {
                pointer_zero_padding(storage, 0, values_offset);
                pointer_fill(storage, values_offset, 1024, black_box(0x1020_3040_u32));
                pointer_zero_padding(
                    storage,
                    values_offset + size_of::<[u32; 1024]>(),
                    size_of::<TypedArray>(),
                );
            }
        },
    );
    bench_owned_storage(
        &mut group,
        "borrowed/typed_array_fill",
        |storage: &mut MaybeUninit<TypedArray>| {
            let _initialized = TypedArray::init_from_uninit_bytes(uninit_bytes(storage))
                .expect("valid typed-array storage")
                .values_fill(black_box(0x1020_3040_u32))
                .finish()
                .expect("complete typed-array payload");
        },
    );

    bench_owned_storage(
        &mut group,
        "pointer/nested_builders",
        |storage: &mut MaybeUninit<NestedPayload>| {
            let storage = validated_pointer(storage);
            let head_offset = offset_of!(NestedPayload, head);
            let leaves_offset = offset_of!(NestedPayload, leaves);
            let bytes_offset = offset_of!(NestedPayload, bytes);
            // SAFETY: The layout constants identify every nested field and padding
            // byte exactly once in this exclusively owned allocation.
            unsafe {
                pointer_zero_padding(storage, 0, head_offset);
                pointer_write_leaf(storage, head_offset, black_box(1_u32), black_box(2_u32));
                pointer_zero_padding(
                    storage,
                    head_offset + size_of::<NestedLeaf>(),
                    leaves_offset,
                );
                for index in 0..8 {
                    pointer_write_leaf(
                        storage,
                        leaves_offset + index * size_of::<NestedLeaf>(),
                        black_box(index as u32),
                        black_box((index + 10) as u32),
                    );
                }
                pointer_zero_padding(
                    storage,
                    leaves_offset + size_of::<[NestedLeaf; 8]>(),
                    bytes_offset,
                );
                pointer_fill(storage, bytes_offset, 32, black_box(0x33_u8));
                pointer_zero_padding(
                    storage,
                    bytes_offset + size_of::<[u8; 32]>(),
                    size_of::<NestedPayload>(),
                );
            }
        },
    );
    bench_owned_storage(
        &mut group,
        "borrowed/nested_builders",
        |storage: &mut MaybeUninit<NestedPayload>| {
            let _initialized = NestedPayload::init_from_uninit_bytes(uninit_bytes(storage))
                .expect("valid nested storage")
                .head(|context| {
                    context
                        .into_init()
                        .x(black_box(1_u32))
                        .y(black_box(2_u32))
                        .finish()
                })
                .expect("complete nested head")
                .leaves(|index, context| {
                    context
                        .into_init()
                        .x(black_box(index as u32))
                        .y(black_box((index + 10) as u32))
                        .finish()
                })
                .expect("complete nested array")
                .bytes_fill(black_box(0x33_u8))
                .finish()
                .expect("complete nested payload");
        },
    );

    bench_owned_storage(
        &mut group,
        "control/direct_maybe_uninit_write",
        |storage: &mut MaybeUninit<FlatPod>| {
            storage.write(FlatPod {
                sequence: black_box(42_u64),
                code: black_box(7_u32),
                flags: black_box(flags),
            });
        },
    );

    #[cfg(all(
        feature = "perf-diagnostics",
        feature = "payload-contract-large-fixtures"
    ))]
    {
        use up_rust::bench_fixtures::payload_contract::{
            self, stable::LidarPointCloudHesaiAt128V1,
        };
        bench_owned_storage(
            &mut group,
            "borrowed/lidar_per_point_trig",
            |storage: &mut MaybeUninit<LidarPointCloudHesaiAt128V1>| {
                let init =
                    LidarPointCloudHesaiAt128V1::init_from_uninit_bytes(uninit_bytes(storage))
                        .expect("valid LiDAR storage");
                let _initialized =
                    payload_contract::init_lidar_hesai_at128_point_cloud_per_point_trig(
                        init,
                        black_box(7),
                    )
                    .expect("complete LiDAR point cloud");
            },
        );
        bench_owned_storage(
            &mut group,
            "borrowed/lidar_cached_trig",
            |storage: &mut MaybeUninit<LidarPointCloudHesaiAt128V1>| {
                let init =
                    LidarPointCloudHesaiAt128V1::init_from_uninit_bytes(uninit_bytes(storage))
                        .expect("valid LiDAR storage");
                let _initialized =
                    payload_contract::init_lidar_hesai_at128_point_cloud(init, black_box(7))
                        .expect("complete LiDAR point cloud");
            },
        );
    }

    group.finish();
}

criterion_group!(benches, stable_payload_init);
criterion_main!(benches);
