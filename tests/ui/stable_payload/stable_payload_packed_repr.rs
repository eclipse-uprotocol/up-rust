/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[repr(C, packed)]
#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "example.trybuild.PackedPayload")]
struct PackedPayload {
    small: u8,
    large: u32,
}

fn main() {}
