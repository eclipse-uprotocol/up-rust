/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compile-fail coverage for validation typestate transport boundaries.

#[test]
fn unvalidated_values_cannot_cross_transport_boundaries() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/validation_typestate/*.rs");
}
