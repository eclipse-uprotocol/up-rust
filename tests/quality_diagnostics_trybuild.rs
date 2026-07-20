/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compile-fail pins for diagnostic quality of common API misuse.

#[cfg(not(feature = "payload-contract-fixtures"))]
#[test]
fn quality_diagnostics_are_stable() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/quality_diagnostics/*.rs");
}
