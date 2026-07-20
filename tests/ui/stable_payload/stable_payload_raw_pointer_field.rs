/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[repr(C)]
#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "example.trybuild.RawPointerField")]
struct RawPointerField {
    value: *const u32,
}

fn main() {}
