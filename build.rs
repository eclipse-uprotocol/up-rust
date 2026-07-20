#![allow(missing_docs)] // build script, not public API
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

#[cfg(feature = "up-core-api")]
const UPROTOCOL_BASE_URI: &str = "up-spec/up-core-api/";
#[cfg(feature = "payload-contract-fixtures")]
const PAYLOAD_CONTRACT_PROTO_BASE_URI: &str = "test-fixtures/proto/";

#[cfg(feature = "up-core-api")]
fn proto_api() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec![
        // uProtocol-project proto definitions
        format!("{}uprotocol/uoptions.proto", UPROTOCOL_BASE_URI),
        // [impl->req~uuid-proto~1]
        // [impl->req~uuid-type~1]
        format!("{}uprotocol/v1/uuid.proto", UPROTOCOL_BASE_URI),
        // [impl->req~uri-data-model-proto~1]
        format!("{}uprotocol/v1/uri.proto", UPROTOCOL_BASE_URI),
        // [impl->req~uattributes-data-model-impl~1]
        // [impl->req~uattributes-data-model-proto~1]
        format!("{}uprotocol/v1/uattributes.proto", UPROTOCOL_BASE_URI),
        format!("{}uprotocol/v1/ucode.proto", UPROTOCOL_BASE_URI),
        // [impl->req~umessage-data-model-impl~1]
        // [impl->req~umessage-data-model-proto~1]
        format!("{}uprotocol/v1/umessage.proto", UPROTOCOL_BASE_URI),
        // [impl->req~ustatus-data-model-impl~1]
        // [impl->req~ustatus-data-model-proto~1]
        format!("{}uprotocol/v1/ustatus.proto", UPROTOCOL_BASE_URI),
        // not used in the SDK yet, but for completeness sake
        format!("{}uprotocol/v1/file.proto", UPROTOCOL_BASE_URI),
        // optional uProtocol Core API types
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
    ];

    protobuf_codegen::Codegen::new()
        .protoc()
        // use vendored protoc instead of relying on user provided protobuf installation
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .customize(protobuf_codegen::Customize::default().tokio_bytes(true))
        .include(UPROTOCOL_BASE_URI)
        .inputs(files.as_slice())
        .cargo_out_dir("uprotocol")
        .run_from_script();
    Ok(())
}

#[cfg(feature = "cloudevents")]
fn cloudevents() -> Result<(), Box<dyn std::error::Error>> {
    protobuf_codegen::Codegen::new()
        .protoc()
        // use vendored protoc instead of relying on user provided protobuf installation
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .include("proto")
        .inputs(["proto/io/cloudevents/v1/cloudevents.proto"])
        .cargo_out_dir("cloudevents")
        .run_from_script();

    Ok(())
}

#[cfg(feature = "payload-contract-fixtures")]
fn generate_payload_contract_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = [
        format!("{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/common.proto"),
        format!("{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/can.proto"),
        format!("{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/someip_signal_batch.proto"),
        format!("{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/stream_chunk.proto"),
        format!(
            "{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/radar_ars548_detection_list.proto"
        ),
        format!("{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/lidar_point_cloud.proto"),
        format!("{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/camera_bayer_frame.proto"),
        format!(
            "{PAYLOAD_CONTRACT_PROTO_BASE_URI}uprotocol/bench/v1/camera_carla_bgra_frame.proto"
        ),
    ];

    for input in &inputs {
        println!("cargo:rerun-if-changed={input}");
    }

    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path()?)
        .customize(protobuf_codegen::Customize::default())
        .include(PAYLOAD_CONTRACT_PROTO_BASE_URI)
        .inputs(inputs)
        .cargo_out_dir("payload_contract_fixtures")
        .run_from_script();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "up-core-api")]
    proto_api()?;

    #[cfg(feature = "cloudevents")]
    cloudevents()?;

    #[cfg(feature = "payload-contract-fixtures")]
    generate_payload_contract_fixtures()?;

    Ok(())
}
