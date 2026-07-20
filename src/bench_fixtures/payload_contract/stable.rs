/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

// These benchmark fixtures are intentionally large fixed-layout stable payloads.
#![allow(clippy::type_complexity)]

use crate::payload::stable::StablePayloadInit;

pub const PAYLOAD_CONTRACT_FIXTURE_VERSION: u32 = 1;

pub const CAN_FLAG_EFF: u32 = 1 << 0;
pub const CAN_FLAG_RTR: u32 = 1 << 1;
pub const CAN_FLAG_ERR: u32 = 1 << 2;
pub const CAN_FD_FLAG_BRS: u32 = 1 << 8;
pub const CAN_FD_FLAG_ESI: u32 = 1 << 9;

pub const SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET: usize = 1_456;
pub const SOMEIP_MTU_SIGNAL_SAMPLE_COUNT: usize = 59;

pub const RADAR_ARS548_MAX_DETECTIONS: usize = 800;
pub const RADAR_ARS548_DETECTION_RECORD_BYTES: usize = 44;
pub const RADAR_ARS548_DETECTION_ARRAY_BYTES: usize =
    RADAR_ARS548_MAX_DETECTIONS * RADAR_ARS548_DETECTION_RECORD_BYTES;
pub const RADAR_ARS548_DETECTION_MESSAGE_BYTES: usize = 35_336;
pub const RADAR_ARS548_MAX_TRACKED_OBJECTS: usize = 50;

pub const LIDAR_POINT_FORMAT_XYZIRCAEDT: u32 = 1;
pub const LIDAR_XYZIRCAEDT_POINT_BYTES: usize = 32;
pub const LIDAR_HESAI_AT128_WIDTH: u32 = 1_200;
pub const LIDAR_HESAI_AT128_HEIGHT: u32 = 128;
pub const LIDAR_HESAI_AT128_POINT_COUNT: usize = 153_600;
pub const LIDAR_HESAI_AT128_ROW_STEP_BYTES: u32 = 38_400;
pub const LIDAR_HESAI_AT128_POINTS_BYTES: usize = 4_915_200;
pub const LIDAR_HESAI_AT128_POINTS_PER_SECOND: u32 = 1_536_000;
pub const LIDAR_HESAI_AT128_SCAN_DURATION_NS: u32 = 100_000_000;
pub const LIDAR_HESAI_AT128_HORIZONTAL_FOV_MDEG: u32 = 120_000;
pub const LIDAR_HESAI_AT128_VERTICAL_FOV_MDEG: u32 = 25_400;
pub const LIDAR_HESAI_AT128_HORIZONTAL_RESOLUTION_MDEG: u32 = 100;
pub const LIDAR_HESAI_AT128_VERTICAL_RESOLUTION_MDEG: u32 = 200;
pub const LIDAR_HESAI_AT128_RANGE_10PCT_REFLECTIVITY_M: u32 = 210;

pub const CAMERA_8MP_WIDTH: u32 = 3_840;
pub const CAMERA_8MP_HEIGHT: u32 = 2_160;
pub const CAMERA_BAYER_RGGB12P_STRIDE_BYTES: u32 = 5_760;
pub const CAMERA_BAYER_RGGB12P_BYTES: usize = 12_441_600;
pub const CAMERA_CARLA_BGRA32_BYTES: usize = 33_177_600;
pub const BAYER_PATTERN_RGGB: u32 = 1;
pub const PIXEL_FORMAT_BAYER_RGGB12P: u32 = 1;
pub const BITS_PER_SAMPLE_12: u32 = 12;
pub const PACKED_LAYOUT_12P_LSB: u32 = 1;

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.BenchHeaderV1")]
pub struct BenchHeaderV1 {
    pub case_id: u32,
    pub sequence: u32,
    pub semantic_reference_len: u32,
    pub schema_version: u32,
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.Vector3fV1")]
pub struct Vector3fV1 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.QuaternionfV1")]
pub struct QuaternionfV1 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.Pose3dV1")]
pub struct Pose3dV1 {
    pub translation_m: Vector3fV1,
    pub rotation: QuaternionfV1,
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.CameraIntrinsicsV1")]
pub struct CameraIntrinsicsV1 {
    pub fx: f32,
    pub fy: f32,
    pub cx: f32,
    pub cy: f32,
    pub skew: f32,
    pub distortion_model: u32,
    pub distortion: [f32; 8],
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.RoiV1")]
pub struct RoiV1 {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.CanClassicFrameV1")]
pub struct CanClassicFrameV1 {
    pub header: BenchHeaderV1,
    pub interface_id: u32,
    pub can_id: u32,
    pub flags: u32,
    pub reserved0: u32,
    pub bus_timestamp_ns: u64,
    pub len: u8,
    pub len8_dlc: u8,
    pub data: [u8; 8],
    pub reserved1: [u8; 2],
    pub checksum: u32,
}

#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.CanFdFrameV1")]
pub struct CanFdFrameV1 {
    pub header: BenchHeaderV1,
    pub interface_id: u32,
    pub can_id: u32,
    pub flags: u32,
    pub reserved0: u32,
    pub bus_timestamp_ns: u64,
    pub len: u8,
    pub data: [u8; 64],
    pub reserved1: [u8; 3],
    pub checksum: u32,
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.SignalSampleV1")]
pub struct SignalSampleV1 {
    pub signal_id: u32,
    pub status: u32,
    pub timestamp_ns: u64,
    pub value: f64,
}

#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.SomeIpSignalBatchMtuV1")]
pub struct SomeIpSignalBatchMtuV1 {
    pub header: BenchHeaderV1,
    pub source_timestamp_ns: u64,
    pub length: u32,
    pub sample_count: u32,
    pub service_id: u16,
    pub method_or_event_id: u16,
    pub client_id: u16,
    pub session_id: u16,
    pub protocol_version: u8,
    pub interface_version: u8,
    pub message_type: u8,
    pub return_code: u8,
    pub reserved0: u32,
    pub samples: [SignalSampleV1; SOMEIP_MTU_SIGNAL_SAMPLE_COUNT],
    pub checksum: u32,
    pub reserved1: u32,
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.StreamChunkHeaderV1")]
pub struct StreamChunkHeaderV1 {
    pub header: BenchHeaderV1,
    pub stream_id: u64,
    pub codec: u32,
    pub flags: u32,
    pub byte_offset: u64,
    pub chunk_index: u32,
    pub chunk_count: u32,
    pub source_timestamp_ns: u64,
    pub checksum: u32,
    pub reserved0: u32,
}

#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.StreamChunk4kV1")]
pub struct StreamChunk4kV1 {
    pub meta: StreamChunkHeaderV1,
    pub chunk: [u8; 4_096],
}

#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.StreamChunk64kV1")]
pub struct StreamChunk64kV1 {
    pub meta: StreamChunkHeaderV1,
    pub chunk: [u8; 65_536],
}

#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.Ars548DetectionV1")]
pub struct Ars548DetectionV1 {
    pub azimuth_angle: f32,
    pub azimuth_angle_std: f32,
    pub elevation_angle: f32,
    pub elevation_angle_std: f32,
    pub range: f32,
    pub range_std: f32,
    pub range_rate: f32,
    pub range_rate_std: f32,
    pub measurement_id: u16,
    pub object_id: u16,
    pub sort_index: u16,
    pub invalid_flags: u8,
    pub rcs: i8,
    pub positive_predictive_value: u8,
    pub classification: u8,
    pub multi_target_probability: u8,
    pub ambiguity_flag: u8,
}

#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.RadarDetectionListArs548V1")]
pub struct RadarDetectionListArs548V1 {
    pub header: BenchHeaderV1,
    pub measurement_timestamp_ns: u64,
    pub sensor_id: u32,
    pub measurement_counter: u32,
    pub cycle_counter: u32,
    pub list_numofdetections: u32,
    pub detections: [Ars548DetectionV1; RADAR_ARS548_MAX_DETECTIONS],
    pub checksum: u32,
    pub reserved0: u32,
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[repr(C)]
#[derive(
    Clone, Copy, crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit,
)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.LidarPointXyzircaedtV1")]
pub struct LidarPointXyzircaedtV1 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub intensity: u8,
    pub return_type: u8,
    pub channel: u16,
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub time_offset_ns: u32,
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.LidarPointCloudHesaiAt128V1")]
pub struct LidarPointCloudHesaiAt128V1 {
    pub header: BenchHeaderV1,
    pub frame_counter: u64,
    pub sensor_timestamp_ns: u64,
    pub scan_duration_ns: u32,
    pub lidar_id: u32,
    pub width: u32,
    pub height: u32,
    pub point_count: u32,
    pub point_step: u32,
    pub row_step: u32,
    pub points_per_second: u32,
    pub horizontal_fov_mdeg: u32,
    pub vertical_fov_mdeg: u32,
    pub horizontal_resolution_mdeg: u32,
    pub vertical_resolution_mdeg: u32,
    pub range_10pct_reflectivity_m: u32,
    pub point_format: u32,
    pub is_bigendian: u8,
    pub is_dense: u8,
    pub reserved0: [u8; 2],
    pub checksum: u32,
    pub extrinsics: Pose3dV1,
    pub points: [LidarPointXyzircaedtV1; LIDAR_HESAI_AT128_POINT_COUNT],
    pub reserved1: u32,
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[repr(C)]
#[derive(crate::StablePayload, crate::ByteBackedStablePayload, crate::StablePayloadInit)]
#[stable_payload(type_name = "org.eclipse.uprotocol.bench.v1.CameraBayerRggb12pFrame8mpV1")]
pub struct CameraBayerRggb12pFrame8mpV1 {
    pub header: BenchHeaderV1,
    pub frame_counter: u64,
    pub sensor_timestamp_ns: u64,
    pub exposure_start_ns: u64,
    pub camera_id: u32,
    pub width: u32,
    pub height: u32,
    pub stride_bytes: u32,
    pub bayer_pattern: u32,
    pub bits_per_sample: u32,
    pub packed_layout: u32,
    pub exposure_time_us: u32,
    pub analog_gain: f32,
    pub digital_gain: f32,
    pub intrinsics: CameraIntrinsicsV1,
    pub extrinsics: Pose3dV1,
    pub roi: RoiV1,
    pub checksum: u32,
    pub pixels: [u8; CAMERA_BAYER_RGGB12P_BYTES],
}

pub type CanClassicFrameV1Init<'a> = <CanClassicFrameV1 as StablePayloadInit>::Init<'a>;
pub type CanFdFrameV1Init<'a> = <CanFdFrameV1 as StablePayloadInit>::Init<'a>;
pub type SomeIpSignalBatchMtuV1Init<'a> = <SomeIpSignalBatchMtuV1 as StablePayloadInit>::Init<'a>;
pub type StreamChunk4kV1Init<'a> = <StreamChunk4kV1 as StablePayloadInit>::Init<'a>;
pub type RadarDetectionListArs548V1Init<'a> =
    <RadarDetectionListArs548V1 as StablePayloadInit>::Init<'a>;
pub type StreamChunk64kV1Init<'a> = <StreamChunk64kV1 as StablePayloadInit>::Init<'a>;

#[cfg(feature = "payload-contract-large-fixtures")]
pub type LidarPointCloudHesaiAt128V1Init<'a> =
    <LidarPointCloudHesaiAt128V1 as StablePayloadInit>::Init<'a>;
#[cfg(feature = "payload-contract-large-fixtures")]
pub type CameraBayerRggb12pFrame8mpV1Init<'a> =
    <CameraBayerRggb12pFrame8mpV1 as StablePayloadInit>::Init<'a>;
