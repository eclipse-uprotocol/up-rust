/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::{
    ByteBackedStablePayload, LoanPayload, LoanUninitPayload, PayloadCodec, StableContainerPayload,
    StableContainerWireFormat, UWire, UWirePayload,
};

#[repr(C)]
#[derive(Default, up_rust::StablePayload, up_rust::ByteBackedStablePayload)]
#[stable_payload(type_name = "example.trybuild.SelectedWireStableBytes")]
struct SelectedWireStableBytes {
    id: u32,
    flags: u16,
    reserved: [u8; 2],
}

fn assert_wire<W: UWire>() {}

fn assert_wire_loan<W, T>()
where
    W: UWirePayload<T>,
    <W as UWirePayload<T>>::Codec: LoanPayload<T> + LoanUninitPayload<T>,
{
}

fn main() {
    assert_wire::<StableContainerWireFormat>();
    assert_wire_loan::<StableContainerWireFormat, SelectedWireStableBytes>();

    let wire_encoding = <StableContainerWireFormat as UWirePayload<
        SelectedWireStableBytes,
    >>::Codec::payload_encoding();
    let stable_encoding = StableContainerPayload::<SelectedWireStableBytes>::payload_encoding();
    assert!(wire_encoding.is_compatible_with(&stable_encoding));
    assert!(SelectedWireStableBytes::SUPPORTS_BYTE_BACKED_UNINIT);
}
