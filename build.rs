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

fn main() -> std::io::Result<()> {
    // use vendored protoc instead of relying on user provided protobuf installation
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());

    if let Err(err) = get_and_build_protos(
        &["https://raw.githubusercontent.com/cloudevents/spec/main/cloudevents/formats/cloudevents.proto", 
        "https://raw.githubusercontent.com/googleapis/googleapis/master/google/rpc/status.proto"]
    ) {
        let error_message = format!("Failed to fetch and build protobuf file: {:?}", err);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, error_message));
    }

    Ok(())
}

// Fetch protobuf definitions from `url`, and build them with prost_build
fn get_and_build_protos(urls: &[&str]) -> core::result::Result<(), Box<dyn std::error::Error>> {
    for url in urls.iter() {
        // Extract filename from the URL
        let filename = url.rsplit('/').next().unwrap_or_default();
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join(filename);

        // fetch cloudevents protobuf
        if let Err(err) = download_and_write_file(url, filename) {
            panic!("Failed to download and write file: {:?}", err);
        }

        // build cloudevents protobuf
        prost_build::compile_protos(&[dest_path], &[out_dir.to_str().unwrap()])?;
    }
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

            println!("file dest: {:?}", dest_path.to_str());

            // let dest_path = Path::new(destination);
            let mut out_file = fs::File::create(&dest_path)?;

            // Write the response body directly to the file
            let _ = std::io::copy(&mut response.into_reader(), &mut out_file);

            Ok(())
        }
    }
}
