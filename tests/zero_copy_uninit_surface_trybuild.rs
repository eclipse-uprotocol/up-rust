/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Feature-surface pins: the uninit API appears exactly when its features are enabled.

#[test]
fn zero_copy_uninit_feature_surface() {
    // The uninitialized-loan surface (implementer and user side) ships
    // with the zero-copy family as one unit; this harness requires the
    // family feature, so both pins are unconditional.
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/zero_copy_uninit/implementer_present.rs");
    tests.pass("tests/ui/zero_copy_uninit/user_present.rs");
}
