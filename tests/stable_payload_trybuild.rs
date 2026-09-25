/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compile-time contracts for stable payload initialization and decode limits.

#[test]
fn stable_payload_compile_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/stable_payload_pass/*.rs");
    cases.compile_fail("tests/ui/stable_payload/*.rs");
}
