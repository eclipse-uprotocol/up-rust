/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compile-time boundary for the selected-wire unchecked receive lane.

#[test]
fn selected_wire_unsafe_boundary() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/selected_wire_payload_pass/*.rs");
    cases.compile_fail("tests/ui/selected_wire_payload/*.rs");
}
