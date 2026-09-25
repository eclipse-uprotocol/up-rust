/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

// rust-protobuf owns the included identifiers and implementations. Keep this
// boundary limited to lints generated source cannot control.
#[allow(
    missing_docs,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_attributes,
    unused_results
)]
mod generated {
    include!(concat!(
        env!("OUT_DIR"),
        "/payload_contract_fixtures/mod.rs"
    ));
}

pub use generated::{
    camera_bayer_frame::CameraBayerFrame,
    camera_carla_bgra_frame::CameraCarlaBgraFrame,
    can::{CanClassicFrame, CanFdFrame},
    common::{BenchHeader, CameraIntrinsics, Pose3d, Quaternionf, Roi, Vector3f},
    lidar_point_cloud::LidarPointCloudFrame,
    radar_ars548_detection_list::{Ars548Detection, Ars548DetectionList},
    someip_signal_batch::{SignalSample, SomeIpSignalBatch},
    stream_chunk::StreamChunk,
};
