/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

struct DropField;

impl Drop for DropField {
    fn drop(&mut self) {}
}

// SAFETY: This intentionally bogus impl allows the derive to reach the
// top-level `needs_drop` rejection used by the compile-fail test.
unsafe impl up_rust::payload::stable::StablePayloadField for DropField {}

#[repr(C)]
#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "example.trybuild.DropContainer")]
struct DropContainer {
    value: DropField,
}

fn main() {}
