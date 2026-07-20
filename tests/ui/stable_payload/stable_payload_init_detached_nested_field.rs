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
#[stable_payload(type_name = "example.trybuild.DetachedFieldLeaf")]
struct Leaf {
    value: u32,
}

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "example.trybuild.DetachedFieldParent")]
struct Parent {
    leaf: Leaf,
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
    let mut detached = MaybeUninit::<Leaf>::uninit();
    let detached = Leaf::init_from_uninit_bytes(uninit_bytes(&mut detached))
        .unwrap()
        .value(1)
        .finish()
        .unwrap();
    let mut parent = MaybeUninit::<Parent>::uninit();
    let parent = Parent::init_from_uninit_bytes(uninit_bytes(&mut parent)).unwrap();

    let _ = parent.leaf(|_context| Ok(detached));
}
