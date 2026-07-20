/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[repr(C)]
#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "example.trybuild.UnionPayload")]
union UnionPayload {
    value: u32,
}

fn main() {}
