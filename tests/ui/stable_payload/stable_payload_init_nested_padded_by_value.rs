/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::mem::{self, MaybeUninit};

use up_rust::StablePayloadInit;

#[repr(C)]
#[derive(
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "example.trybuild.InitPaddedNested")]
struct InitPaddedNested {
    tag: u8,
    count: u32,
}

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "example.trybuild.InitPaddedOuter")]
struct InitPaddedOuter {
    nested: InitPaddedNested,
}

fn uninit_bytes<T>(storage: &mut MaybeUninit<T>) -> &mut [MaybeUninit<u8>] {
    unsafe {
        std::slice::from_raw_parts_mut(
            std::ptr::from_mut(storage).cast::<MaybeUninit<u8>>(),
            mem::size_of::<T>(),
        )
    }
}

fn main() {
    let mut storage = MaybeUninit::<InitPaddedOuter>::uninit();
    let init = InitPaddedOuter::init_from_uninit_bytes(uninit_bytes(&mut storage)).unwrap();
    let _ = init
        .nested_value(InitPaddedNested { tag: 1, count: 2 })
        .finish()
        .unwrap();
}
