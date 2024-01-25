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

const UPROTOCOL_BASE_URI: &str = "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/uprotocol-core-api-1.5.5/uprotocol";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    get_and_build_protos(
        &[
            // cloudevent proto definitions
            "https://raw.githubusercontent.com/cloudevents/spec/main/cloudevents/formats/cloudevents.proto", 
        ],
        "cloudevents",
    )?;

    get_and_build_protos(
        &[
            // uProtocol-project proto definitions
            format!("{}/uuid.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/uri.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/uattributes.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/upayload.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/umessage.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/ustatus.proto", UPROTOCOL_BASE_URI).as_str(),
            // not used in the SDK yet, but for completeness sake
            format!("{}/file.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/uprotocol_options.proto", UPROTOCOL_BASE_URI).as_str(),
            format!("{}/core/udiscovery/v3/udiscovery.proto", UPROTOCOL_BASE_URI).as_str(),
            format!(
                "{}/core/usubscription/v3/usubscription.proto",
                UPROTOCOL_BASE_URI
            )
            .as_str(),
            format!("{}/core/utwin/v1/utwin.proto", UPROTOCOL_BASE_URI).as_str(),
        ],
        "uprotocol",
    )?;

    Ok(())
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
        // Extract filename from the URL
        let filename = url.rsplit('/').next().unwrap_or_default();
        let dest_path = proto_folder.join(filename);

        // Download the .proto file
        download_and_write_file(url, &dest_path)?;
        proto_files.push(dest_path);
    }

    protobuf_codegen::Codegen::new()
        .protoc()
        // use vendored protoc instead of relying on user provided protobuf installation
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .include(proto_folder)
        .inputs(proto_files)
        .cargo_out_dir(output_folder)
        .run_from_script();

    Ok(())
}

// Retrieves a file from `url` (from GitHub, for instance) and places it in the build directory (`OUT_DIR`) with the name
// provided by `destination` parameter.
fn download_and_write_file(
    url: &str,
    dest_path: &PathBuf,
) -> core::result::Result<(), Box<dyn std::error::Error>> {
    // Send a GET request to the URL

    match ureq::get(url).call() {
        Err(error) => Err(Box::from(error)),
        Ok(response) => {
            if let Some(parent_path) = dest_path.parent() {
                std::fs::create_dir_all(parent_path)?;
            }
            let mut out_file = fs::File::create(dest_path)?;

            // Write the response body directly to the file
            std::io::copy(&mut response.into_reader(), &mut out_file)
                .map(|_| ())
                .map_err(Box::from)
        }
    }
}
