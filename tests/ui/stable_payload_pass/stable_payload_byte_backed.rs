/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::ByteBackedStablePayload;

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::ByteBackedStablePayload)]
#[stable_payload(type_name = "example.trybuild.ByteBackedHeader")]
struct ByteBackedHeader {
    id: u32,
    flags: u16,
    reserved: [u8; 2],
}

fn main() {
    assert!(ByteBackedHeader::SUPPORTS_BYTE_BACKED_UNINIT);
    assert!(up_rust::stable_payload_supports_byte_backed_uninit::<
        ByteBackedHeader,
    >());
    up_rust::assert_stable_payload_byte_backed_uninit::<ByteBackedHeader>();
}
