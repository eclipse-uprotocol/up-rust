/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[repr(C)]
#[derive(up_rust::StablePayload)]
struct MissingTypeName {
    value: u32,
}

fn main() {}
