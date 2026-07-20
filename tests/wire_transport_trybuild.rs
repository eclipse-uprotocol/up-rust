/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compile-fail pins for selected-wire transport misuse.

#[cfg(not(feature = "owned-frame-transport"))]
#[test]
fn wire_transport_compile_failures() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/wire_transport_pass/*.rs");
    tests.compile_fail("tests/ui/wire_transport/*.rs");
}
