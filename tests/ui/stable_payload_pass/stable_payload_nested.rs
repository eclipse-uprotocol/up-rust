/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::StablePayload;

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::ByteBackedStablePayload)]
#[stable_payload(type_name = "example.trybuild.NestedHeader")]
struct NestedHeader {
    sequence: u32,
}

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::ByteBackedStablePayload)]
#[stable_payload(type_name = "example.trybuild.NestedFrame")]
struct NestedFrame {
    header: NestedHeader,
    payload: [u8; 8],
}

fn main() {
    assert_eq!(NestedFrame::TYPE_NAME, "example.trybuild.NestedFrame");
    up_rust::assert_stable_payload_byte_backed_uninit::<NestedFrame>();
}
