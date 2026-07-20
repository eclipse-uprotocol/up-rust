/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::{ExactUUri, UUri};

fn exact_source_only(source: &ExactUUri) -> &UUri {
    source.as_uuri()
}

fn main() {
    let source = UUri::try_from_parts("vehicle-a", 0x4210, 0x01, 0x9000).unwrap();

    let _ = exact_source_only(&source);
}
