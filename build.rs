/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
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

use protobuf_codegen::Customize;

const UPROTOCOL_BASE_URI: &str = "up-spec/up-core-api/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec![
        // uProtocol-project proto definitions
        format!("{}uprotocol/uoptions.proto", UPROTOCOL_BASE_URI),
        // [impl->req~uuid-proto~1]
        format!("{}uprotocol/v1/uuid.proto", UPROTOCOL_BASE_URI),
        // [impl->req~uri-data-model-proto~1]
        format!("{}uprotocol/v1/uri.proto", UPROTOCOL_BASE_URI),
        format!("{}uprotocol/v1/uattributes.proto", UPROTOCOL_BASE_URI),
        format!("{}uprotocol/v1/ucode.proto", UPROTOCOL_BASE_URI),
        format!("{}uprotocol/v1/umessage.proto", UPROTOCOL_BASE_URI),
        format!("{}uprotocol/v1/ustatus.proto", UPROTOCOL_BASE_URI),
        // not used in the SDK yet, but for completeness sake
        format!("{}uprotocol/v1/file.proto", UPROTOCOL_BASE_URI),
        // optional up-core-api features
        #[cfg(feature = "udiscovery")]
        format!(
            "{}uprotocol/core/udiscovery/v3/udiscovery.proto",
            UPROTOCOL_BASE_URI
        ),
        #[cfg(feature = "usubscription")]
        format!(
            "{}uprotocol/core/usubscription/v3/usubscription.proto",
            UPROTOCOL_BASE_URI
        ),
        #[cfg(feature = "utwin")]
        format!("{}uprotocol/core/utwin/v2/utwin.proto", UPROTOCOL_BASE_URI),
    ];

    protobuf_codegen::Codegen::new()
        .protoc()
        // use vendored protoc instead of relying on user provided protobuf installation
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .customize(Customize::default().tokio_bytes(true))
        .include(UPROTOCOL_BASE_URI)
        .inputs(files.as_slice())
        .cargo_out_dir("uprotocol")
        .run_from_script();

    Ok(())
}
