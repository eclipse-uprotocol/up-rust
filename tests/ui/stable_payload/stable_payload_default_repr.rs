/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "example.trybuild.DefaultRepr")]
struct DefaultRepr {
    value: u32,
}

fn main() {}
