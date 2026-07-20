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

//! Miri-run round-trips for stable-container payloads over the in-memory transport.

use std::mem::{self, MaybeUninit};

use up_rust::test_support::StableTestBytes as StableBytes;
use up_rust::{
    LoanPayload, LoanUninitPayload, LoanedPayloadUninitMut, PayloadLoanProvenance,
    StableContainerPayload, StablePayloadInit, UFrameMetadata, ULoanedContiguousZeroCopyRxFrame,
    UMessageBuilder, UUri, UVecRxLease, UUID,
};

#[repr(C)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    up_rust::StablePayload,
    up_rust::ByteBackedStablePayload,
    up_rust::StablePayloadInit,
)]
#[stable_payload(type_name = "example.miri.InitLeaf")]
struct InitLeaf {
    x: u16,
    y: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "example.miri.InitMessage")]
struct InitMessage {
    tag: u8,
    count: u32,
    leaf: InitLeaf,
    bytes: [u8; 4],
    words: [u16; 2],
    leaves: [InitLeaf; 2],
}

fn topic() -> UUri {
    UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("valid topic")
}

fn metadata<T: up_rust::StablePayload>() -> UFrameMetadata {
    let fixed_id = UUID::from_u64_pair(0x0000_0000_0001_7000, 0x8010_1010_1010_1a1a)
        .expect("fixed UUID should be valid");
    let message = UMessageBuilder::publish(topic())
        .with_message_id(fixed_id)
        .build()
        .expect("message");
    message
        .attributes()
        .to_frame_metadata(StableContainerPayload::<T>::encoding())
        .expect("stable metadata")
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

#[test]
fn stable_payload_derive_loan_borrow_is_miri_friendly() {
    up_rust::assert_stable_payload_byte_backed_uninit::<StableBytes>();

    let value = StableBytes { bytes: *b"miri" };
    let frame = UVecRxLease::new(metadata::<StableBytes>(), Some(value.bytes.to_vec()))
        .expect("loan-backed stable frame");

    let borrowed = frame
        .borrow_stable_payload::<StableBytes>()
        .expect("borrow stable payload");

    assert_eq!(borrowed, &value);
}

#[test]
fn stable_payload_init_builder_initializes_fields_and_padding() -> Result<(), up_rust::UWireError> {
    let mut storage = MaybeUninit::<InitMessage>::uninit();
    let init = InitMessage::init_from_uninit_bytes(uninit_bytes(&mut storage))?;
    let init = init
        .tag(7)
        .count(42)
        .leaf_value(InitLeaf { x: 1, y: 2 })
        .bytes_from_array(b"miri")
        .words_from_slice(&[10, 11])?
        .leaves(|index, context| context.into_init().x(index as u16).y(9).finish())?;
    let _finished = init.finish()?;

    // SAFETY: `finish()` proves every semantic field and generated padding gap
    // has been initialized.
    let raw = unsafe {
        std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), mem::size_of::<InitMessage>())
    };
    assert_eq!(raw.get(1..4).expect("padding bytes"), &[0, 0, 0]);

    // SAFETY: The builder completion proof above initialized one `InitMessage`.
    let value = unsafe { storage.assume_init() };
    assert_eq!(value.tag, 7);
    assert_eq!(value.count, 42);
    assert_eq!(value.leaf, InitLeaf { x: 1, y: 2 });
    assert_eq!(value.bytes, *b"miri");
    assert_eq!(value.words, [10, 11]);
    assert_eq!(
        value.leaves,
        [InitLeaf { x: 0, y: 9 }, InitLeaf { x: 1, y: 9 }]
    );

    Ok(())
}

#[test]
fn stable_payload_init_rejects_wrong_length() {
    let mut bytes = vec![MaybeUninit::<u8>::uninit(); mem::size_of::<InitMessage>() - 1];
    let err = match InitMessage::init_from_uninit_bytes(&mut bytes) {
        Ok(_) => panic!("wrong-length init unexpectedly succeeded"),
        Err(err) => err,
    };

    assert!(
        err.to_string().contains("payload length must be"),
        "unexpected error: {err}"
    );
}

#[test]
fn stable_initialized_tx_loan_payload_is_miri_friendly() -> Result<(), up_rust::UWireError> {
    let mut bytes = vec![0_u8; mem::size_of::<StableBytes>()];

    let payload = StableContainerPayload::<StableBytes>::loan_payload(&mut bytes)?;
    payload.bytes.copy_from_slice(b"send");

    let frame = UVecRxLease::new(metadata::<StableBytes>(), Some(bytes)).expect("stable frame");
    let borrowed = frame
        .borrow_stable_payload::<StableBytes>()
        .expect("borrow sent stable payload");

    assert_eq!(borrowed.bytes, *b"send");
    Ok(())
}

#[test]
fn stable_uninit_tx_loan_payload_is_miri_friendly() -> Result<(), up_rust::UWireError> {
    let mut storage = MaybeUninit::<StableBytes>::uninit();
    let payload = uninit_bytes(&mut storage);
    // SAFETY: `payload` covers exactly the uninitialized storage for one
    // `StableBytes` value and is uniquely borrowed for this test.
    let payload = unsafe {
        LoanedPayloadUninitMut::new_unchecked(payload, PayloadLoanProvenance::OpaqueTransportLoan)
    };
    let slot = StableContainerPayload::<StableBytes>::loan_uninit_payload(payload)?;
    let _initialized = slot.write(StableBytes { bytes: *b"zero" });

    // SAFETY: the loaned uninit slot wrote one initialized `StableBytes` value.
    let value = unsafe { storage.assume_init() };
    assert_eq!(value.bytes, *b"zero");
    Ok(())
}
