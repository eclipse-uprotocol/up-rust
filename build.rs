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

use prost_build::Config;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> std::io::Result<()> {
    // use vendored protoc instead of relying on user provided protobuf installation
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());

    if let Err(err) = get_and_build_protos(
        &[
            // cloudevent proto definitions
            "https://raw.githubusercontent.com/cloudevents/spec/main/cloudevents/formats/cloudevents.proto", 

            // uProtocol-project proto definitions
            "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/main/src/main/proto/uuid.proto",
            "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/main/src/main/proto/uri.proto",
            "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/main/src/main/proto/uattributes.proto",
            "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/main/src/main/proto/upayload.proto",
            "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/main/src/main/proto/umessage.proto",
            "https://raw.githubusercontent.com/eclipse-uprotocol/uprotocol-core-api/main/src/main/proto/ustatus.proto",
        ]
    ) {
        let error_message = format!("Failed to fetch and build protobuf file: {:?}", err);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, error_message));
    }

    Ok(())
}

// Fetch protobuf definitions from `url`, and build them with prost_build
fn get_and_build_protos(urls: &[&str]) -> core::result::Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut proto_files = Vec::new();

    for url in urls.iter() {
        // Extract filename from the URL
        let filename = url.rsplit('/').next().unwrap_or_default();
        let dest_path = Path::new(&out_dir).join(filename);

        // Download the .proto file
        if let Err(err) = download_and_write_file(url, filename) {
            panic!("Failed to download and write file: {:?}", err);
        }
        proto_files.push(dest_path);
    }

    // Compile all .proto files together
    let mut config = Config::new();

    // Some proto files contain comments that will be interpreted as rustdoc comments (and fail to compile)
    config.disable_comments(["."]);

    config.compile_protos(&proto_files, &[&out_dir])?;

    Ok(())
}

// Retreives a file from `url` (from GitHub, for instance) and places it in the build directory (`OUT_DIR`) with the name
// provided by `destination` parameter.
fn download_and_write_file(
    url: &str,
    destination: &str,
) -> core::result::Result<(), Box<dyn std::error::Error>> {
    // Send a GET request to the URL
    let resp = ureq::get(url).call();

    match resp {
        Err(error) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            error.to_string(),
        ))),
        Ok(response) => {
            let out_dir = env::var_os("OUT_DIR").unwrap();
            let dest_path = Path::new(&out_dir).join(destination);
            let mut out_file = fs::File::create(dest_path)?;

            // Write the response body directly to the file
            let _ = std::io::copy(&mut response.into_reader(), &mut out_file);

            Ok(())
        }
    }
}
