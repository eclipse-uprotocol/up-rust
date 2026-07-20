/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::mem::{self, MaybeUninit};

use up_rust::StablePayloadInit;

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "example.trybuild.InitDuplicateSetter")]
struct InitDuplicateSetter {
    tag: u8,
    count: u16,
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
    let mut storage = MaybeUninit::<InitDuplicateSetter>::uninit();
    let init = InitDuplicateSetter::init_from_uninit_bytes(uninit_bytes(&mut storage)).unwrap();
    let _ = init.tag(1).tag(2).count(3).finish().unwrap();
}
