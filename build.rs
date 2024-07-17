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

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use protobuf_codegen::Customize;

const UPROTOCOL_BASE_URI: &str =
    "https://raw.githubusercontent.com/eclipse-uprotocol/up-spec/main/up-core-api/";
const UPROTOCOL_REL_PATH: &str = "uprotocol/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    get_and_build_protos(
        &[
            // uProtocol-project proto definitions
            format!("{}{}uoptions.proto", UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH).as_str(),
            // [impl->req~uuid-proto~1]
            format!("{}{}v1/uuid.proto", UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH).as_str(),
            // [impl->req~uri-data-model-proto~1]
            format!("{}{}v1/uri.proto", UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH).as_str(),
            format!(
                "{}{}v1/uattributes.proto",
                UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH
            )
            .as_str(),
            format!("{}{}v1/ucode.proto", UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH).as_str(),
            format!(
                "{}{}v1/umessage.proto",
                UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH
            )
            .as_str(),
            format!(
                "{}{}v1/ustatus.proto",
                UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH
            )
            .as_str(),
            // not used in the SDK yet, but for completeness sake
            format!("{}{}v1/file.proto", UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH).as_str(),
            // optional up-core-api features
            #[cfg(feature = "udiscovery")]
            format!(
                "{}{}core/udiscovery/v3/udiscovery.proto",
                UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH
            )
            .as_str(),
            #[cfg(feature = "usubscription")]
            format!(
                "{}{}core/usubscription/v3/usubscription.proto",
                UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH
            )
            .as_str(),
            #[cfg(feature = "utwin")]
            format!(
                "{}{}core/utwin/v2/utwin.proto",
                UPROTOCOL_BASE_URI, UPROTOCOL_REL_PATH
            )
            .as_str(),
        ],
        "uprotocol",
    )
    .map_err(|e| {
        println!(
            "failed to generate types from uProtocol proto3 definitions: {}",
            e
        );
        e
    })
}

// Fetch protobuf definitions from `url`, and build them with prost_build
fn get_and_build_protos(
    urls: &[&str],
    output_folder: &str,
) -> core::result::Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let proto_folder = Path::new(&out_dir).join("proto");
    let mut proto_files = Vec::new();

    for url in urls {
        // Extract relative filename from the URL
        let filename = url.strip_prefix(UPROTOCOL_BASE_URI).unwrap_or_default();
        let dest_path = proto_folder.join(filename);

        // Download the .proto file
        download_and_write_file(url, &dest_path)?;
        proto_files.push(dest_path);
    }

    protobuf_codegen::Codegen::new()
        .protoc()
        // use vendored protoc instead of relying on user provided protobuf installation
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .customize(Customize::default().tokio_bytes(true))
        .include(proto_folder)
        .inputs(proto_files)
        .cargo_out_dir(output_folder)
        .run_from_script();

    Ok(())
}

// Retrieves a file from `url` (from GitHub, for instance) and places it in the build directory (`OUT_DIR`) with the name
// provided by `dest_path` parameter.
fn download_and_write_file(
    url: &str,
    dest_path: &PathBuf,
) -> core::result::Result<(), Box<dyn std::error::Error>> {
    // Send a GET request to the URL
    reqwest::blocking::get(url)
        .map_err(Box::from)
        .and_then(|mut response| {
            if let Some(parent_path) = dest_path.parent() {
                std::fs::create_dir_all(parent_path)?;
            }
            let mut out_file = fs::File::create(dest_path)?;
            response
                .copy_to(&mut out_file)
                .map(|_| ())
                .map_err(|e| e.to_string().into())
        })
}
