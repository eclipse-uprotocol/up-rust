/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::mem::{self, MaybeUninit};

use up_rust::{
    InitializedStablePayload, StablePayloadInit, StablePayloadInitContext, UWireError,
};

#[repr(C)]
#[derive(
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "example.trybuild.DetachedTopLevel")]
struct Payload {
    value: u32,
}

fn uninit_bytes<T>(storage: &mut MaybeUninit<T>) -> &mut [MaybeUninit<u8>] {
    unsafe {
        std::slice::from_raw_parts_mut(
            std::ptr::from_mut(storage).cast::<MaybeUninit<u8>>(),
            mem::size_of::<T>(),
        )
    }
}

fn accepts_initializer(
    _init: impl for<'slot> FnOnce(
        StablePayloadInitContext<'slot, Payload>,
    ) -> Result<InitializedStablePayload<'slot, Payload>, UWireError>,
) {
}

fn main() {
    let mut detached = MaybeUninit::<Payload>::uninit();
    let detached = Payload::init_from_uninit_bytes(uninit_bytes(&mut detached))
        .unwrap()
        .value(1)
        .finish()
        .unwrap();

    accepts_initializer(|_context| Ok(detached));
}
