/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::{StableContainerPayload, StablePayload};

#[repr(C)]
#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "example.trybuild.Header")]
struct Header {
    id: u32,
    flags: u16,
    reserved: [u8; 2],
}

fn main() {
    assert_eq!(Header::TYPE_NAME, "example.trybuild.Header");
    let _ = StableContainerPayload::<Header>::encoding();
}
