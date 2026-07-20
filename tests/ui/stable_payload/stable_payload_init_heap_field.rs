/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[repr(C)]
#[derive(up_rust::StablePayloadInit)]
#[stable_payload(type_name = "example.trybuild.InitHeapField")]
struct InitHeapField {
    value: String,
}

fn main() {}
