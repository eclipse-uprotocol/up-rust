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

//! Compile-fail pins: the selected-wire user door does not expose implementer SPI.

#[cfg(all(
    feature = "selected-wire-user-api",
    not(feature = "transport-implementer-api"),
    not(feature = "wire-implementer-api")
))]
#[test]
fn selected_wire_user_api_does_not_expose_implementer_spi() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/role_firewall/*.rs");
}
