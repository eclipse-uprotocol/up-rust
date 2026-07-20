/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compile-fail pins: wildcard URIs cannot pose as exact filters.

#[test]
fn exact_filter_capability_compile_failures() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/exact_filter_capability/*.rs");
}
