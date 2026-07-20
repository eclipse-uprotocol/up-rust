/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::mem::{self, MaybeUninit};

use up_rust::StablePayloadInit;

#[repr(C)]
#[derive(
    Clone,
    Copy,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "example.init.Leaf")]
struct Leaf {
    x: u16,
    y: u16,
}

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "example.init.Parent")]
struct Parent {
    tag: u8,
    count: u32,
    leaf: Leaf,
    bytes: [u8; 4],
    words: [u16; 2],
    leaves: [Leaf; 2],
}

fn uninit_bytes<T>(storage: &mut MaybeUninit<T>) -> &mut [MaybeUninit<u8>] {
    // SAFETY: The returned byte slice covers exactly the uninitialized storage
    // for `T` and inherits its alignment.
    unsafe {
        std::slice::from_raw_parts_mut(
            std::ptr::from_mut(storage).cast::<MaybeUninit<u8>>(),
            mem::size_of::<T>(),
        )
    }
}

fn main() -> Result<(), up_rust::UWireError> {
    let mut storage = MaybeUninit::<Parent>::uninit();
    let init = Parent::init_from_uninit_bytes(uninit_bytes(&mut storage))?;
    let init = init
        .tag(7)
        .count(42)
        .leaf_value(Leaf { x: 1, y: 2 })
        .bytes_fill_with(|index| index as u8)
        .words_from_slice(&[10, 11])?
        .leaves(|index, context| context.into_init().x(index as u16).y(9).finish())?;
    let _finished = init.finish()?;
    Ok(())
}
