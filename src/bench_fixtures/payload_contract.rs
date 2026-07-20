/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

// Fixture code intentionally uses fixed-layout protocol structs, deterministic
// sample indexing, and validators that mirror payload fields one-to-one.
#![allow(clippy::indexing_slicing, clippy::too_many_arguments)]

use std::{fmt::Debug, mem};

use ::protobuf::{Message, MessageField};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    payload::{
        stable::{
            InitializedStablePayload, StableContainerPayload, StablePayload, StablePayloadInit,
            StablePayloadInitContext,
        },
        UWireError,
    },
    zero_copy::{
        LoanedPayloadUninitMut, PayloadLoanProvenance, UTxBuffer, UUninitTxBuffer,
        UVecUninitTxBuffer,
    },
    PayloadEncoding, UFrameMetadata, UUri,
};

pub mod proto;
pub mod stable;

pub use stable::*;

const SUITE_FAMILY: &str = "payload-contract-representative";
const CAN_CLASSIC_ID: u32 = 0x18ff_50e5;
const CAN_FD_ID: u32 = 0x18ff_51e5;
const CAN_INTERFACE_ID: u32 = 2;
const SOMEIP_SERVICE_ID: u16 = 0x1234;
const SOMEIP_METHOD_OR_EVENT_ID: u16 = 0x8001;
const SOMEIP_CLIENT_ID: u16 = 0x0042;
const SOMEIP_SESSION_ID: u16 = 0x1001;
const SOMEIP_PROTOCOL_VERSION: u8 = 1;
const SOMEIP_INTERFACE_VERSION: u8 = 3;
const SOMEIP_MESSAGE_TYPE: u8 = 2;
const SOMEIP_RETURN_CODE: u8 = 0;
const STREAM_ID: u64 = 0x5354_5245_414d_0001;
const STREAM_CODEC_RAW: u32 = 1;
const STREAM_FLAG_KEY_FRAME: u32 = 1;
const RADAR_SENSOR_ID: u32 = 548;
#[cfg(feature = "payload-contract-large-fixtures")]
const LIDAR_ID: u32 = 128;
#[cfg(feature = "payload-contract-large-fixtures")]
const CAMERA_ID: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum PayloadContractCaseKind {
    CanClassicMax,
    CanFdMax,
    SomeIpSingleMtu,
    Streamer4k,
    RadarArs548DetectionList,
    Streamer64k,
    #[cfg(feature = "payload-contract-large-fixtures")]
    LidarHesaiAt128PointCloud,
    #[cfg(feature = "payload-contract-large-fixtures")]
    Camera8mpBayerRggb12p,
    #[cfg(feature = "payload-contract-simulator-fixtures")]
    Camera8mpCarlaBgra32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PayloadContractCase {
    case_id: u32,
    name: &'static str,
    kind: PayloadContractCaseKind,
    semantic_reference_len: usize,
    external_wire_reference_len: Option<usize>,
}

impl PayloadContractCase {
    pub const fn new(
        case_id: u32,
        name: &'static str,
        kind: PayloadContractCaseKind,
        semantic_reference_len: usize,
        external_wire_reference_len: Option<usize>,
    ) -> Self {
        Self {
            case_id,
            name,
            kind,
            semantic_reference_len,
            external_wire_reference_len,
        }
    }

    pub const fn case_id(&self) -> u32 {
        self.case_id
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn kind(&self) -> PayloadContractCaseKind {
        self.kind
    }

    pub const fn semantic_reference_len(&self) -> usize {
        self.semantic_reference_len
    }

    pub const fn external_wire_reference_len(&self) -> Option<usize> {
        self.external_wire_reference_len
    }
}

const CORE_CASES: &[PayloadContractCase] = &[
    PayloadContractCase::new(
        1,
        "can_classic_max",
        PayloadContractCaseKind::CanClassicMax,
        8,
        None,
    ),
    PayloadContractCase::new(2, "can_fd_max", PayloadContractCaseKind::CanFdMax, 64, None),
    PayloadContractCase::new(
        3,
        "someip_single_mtu",
        PayloadContractCaseKind::SomeIpSingleMtu,
        SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET,
        Some(SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET),
    ),
    PayloadContractCase::new(
        4,
        "streamer_4k",
        PayloadContractCaseKind::Streamer4k,
        4 * 1_024,
        None,
    ),
    PayloadContractCase::new(
        5,
        "radar_ars548_detection_list",
        PayloadContractCaseKind::RadarArs548DetectionList,
        RADAR_ARS548_DETECTION_MESSAGE_BYTES,
        Some(RADAR_ARS548_DETECTION_MESSAGE_BYTES),
    ),
    PayloadContractCase::new(
        6,
        "streamer_64k",
        PayloadContractCaseKind::Streamer64k,
        64 * 1_024,
        None,
    ),
];

#[cfg(feature = "payload-contract-large-fixtures")]
const LARGE_SENSOR_CASES: &[PayloadContractCase] = &[
    PayloadContractCase::new(
        7,
        "lidar_hesai_at128_1200x128_xyzircaedt",
        PayloadContractCaseKind::LidarHesaiAt128PointCloud,
        LIDAR_HESAI_AT128_POINTS_BYTES,
        None,
    ),
    PayloadContractCase::new(
        8,
        "camera_8mp_3840x2160_bayer_rggb12p",
        PayloadContractCaseKind::Camera8mpBayerRggb12p,
        CAMERA_BAYER_RGGB12P_BYTES,
        None,
    ),
];

#[cfg(not(feature = "payload-contract-large-fixtures"))]
const LARGE_SENSOR_CASES: &[PayloadContractCase] = &[];

const SIMULATOR_NATIVE_CASES: &[PayloadContractCase] = &[];

pub fn core_cases() -> &'static [PayloadContractCase] {
    CORE_CASES
}

pub fn large_sensor_cases() -> &'static [PayloadContractCase] {
    LARGE_SENSOR_CASES
}

pub fn simulator_native_cases() -> &'static [PayloadContractCase] {
    SIMULATOR_NATIVE_CASES
}

pub fn all_cases() -> impl Iterator<Item = &'static PayloadContractCase> {
    core_cases()
        .iter()
        .chain(large_sensor_cases())
        .chain(simulator_native_cases())
}

pub fn case_by_kind(kind: PayloadContractCaseKind) -> &'static PayloadContractCase {
    all_cases()
        .find(|case| case.kind == kind)
        .expect("payload contract case kind must be enabled")
}

pub fn stable_payload_len(case: &PayloadContractCase) -> usize {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => mem::size_of::<CanClassicFrameV1>(),
        PayloadContractCaseKind::CanFdMax => mem::size_of::<CanFdFrameV1>(),
        PayloadContractCaseKind::SomeIpSingleMtu => mem::size_of::<SomeIpSignalBatchMtuV1>(),
        PayloadContractCaseKind::Streamer4k => mem::size_of::<StreamChunk4kV1>(),
        PayloadContractCaseKind::RadarArs548DetectionList => {
            mem::size_of::<RadarDetectionListArs548V1>()
        }
        PayloadContractCaseKind::Streamer64k => mem::size_of::<StreamChunk64kV1>(),
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            mem::size_of::<LidarPointCloudHesaiAt128V1>()
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            mem::size_of::<CameraBayerRggb12pFrame8mpV1>()
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => 0,
    }
}

pub fn stable_payload_type_name(case: &PayloadContractCase) -> &'static str {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => CanClassicFrameV1::stable_type_name(),
        PayloadContractCaseKind::CanFdMax => CanFdFrameV1::stable_type_name(),
        PayloadContractCaseKind::SomeIpSingleMtu => SomeIpSignalBatchMtuV1::stable_type_name(),
        PayloadContractCaseKind::Streamer4k => StreamChunk4kV1::stable_type_name(),
        PayloadContractCaseKind::RadarArs548DetectionList => {
            RadarDetectionListArs548V1::stable_type_name()
        }
        PayloadContractCaseKind::Streamer64k => StreamChunk64kV1::stable_type_name(),
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            LidarPointCloudHesaiAt128V1::stable_type_name()
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            CameraBayerRggb12pFrame8mpV1::stable_type_name()
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => {
            "org.eclipse.uprotocol.bench.v1.CameraCarlaBgra32Frame8mpV1"
        }
    }
}

pub fn stable_payload_align(case: &PayloadContractCase) -> usize {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => mem::align_of::<CanClassicFrameV1>(),
        PayloadContractCaseKind::CanFdMax => mem::align_of::<CanFdFrameV1>(),
        PayloadContractCaseKind::SomeIpSingleMtu => mem::align_of::<SomeIpSignalBatchMtuV1>(),
        PayloadContractCaseKind::Streamer4k => mem::align_of::<StreamChunk4kV1>(),
        PayloadContractCaseKind::RadarArs548DetectionList => {
            mem::align_of::<RadarDetectionListArs548V1>()
        }
        PayloadContractCaseKind::Streamer64k => mem::align_of::<StreamChunk64kV1>(),
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            mem::align_of::<LidarPointCloudHesaiAt128V1>()
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            mem::align_of::<CameraBayerRggb12pFrame8mpV1>()
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => 1,
    }
}

#[derive(Serialize)]
pub struct PayloadContractCaseManifest {
    pub case_id: u32,
    pub name: &'static str,
    pub kind: PayloadContractCaseKind,
    pub semantic_reference_len: usize,
    pub external_wire_reference_len: Option<usize>,
    pub protobuf_message: &'static str,
    pub protobuf_transport_len: usize,
    pub stable_type_name: &'static str,
    pub stable_transport_len: usize,
    pub stable_align: usize,
}

#[derive(Serialize)]
pub struct PayloadContractFixtureManifest {
    pub suite_family: &'static str,
    pub fixture_version: u32,
    pub cases: Vec<PayloadContractCaseManifest>,
}

pub fn fixture_manifest() -> PayloadContractFixtureManifest {
    let mut cases = all_cases()
        .map(|case| PayloadContractCaseManifest {
            case_id: case.case_id,
            name: case.name,
            kind: case.kind,
            semantic_reference_len: case.semantic_reference_len,
            external_wire_reference_len: case.external_wire_reference_len,
            protobuf_message: protobuf_message_name(case),
            protobuf_transport_len: protobuf_encoded_len(case, 1),
            stable_type_name: stable_payload_type_name(case),
            stable_transport_len: stable_payload_len(case),
            stable_align: stable_payload_align(case),
        })
        .collect::<Vec<_>>();
    cases.sort_by_key(|case| case.case_id);
    PayloadContractFixtureManifest {
        suite_family: SUITE_FAMILY,
        fixture_version: PAYLOAD_CONTRACT_FIXTURE_VERSION,
        cases,
    }
}

pub fn fixture_manifest_json_canonical() -> String {
    serde_json::to_string(&fixture_manifest()).expect("fixture manifest serialization succeeds")
}

pub fn fixture_manifest_sha256_hex() -> String {
    let hash = Sha256::digest(fixture_manifest_json_canonical().as_bytes());
    let mut text = String::with_capacity(hash.len() * 2);
    for byte in hash {
        use std::fmt::Write as _;
        write!(&mut text, "{byte:02x}").expect("write to String cannot fail");
    }
    text
}

pub enum ProtobufFixture {
    CanClassicMax(proto::CanClassicFrame),
    CanFdMax(proto::CanFdFrame),
    SomeIpSingleMtu(proto::SomeIpSignalBatch),
    Streamer4k(proto::StreamChunk),
    RadarArs548DetectionList(proto::Ars548DetectionList),
    Streamer64k(proto::StreamChunk),
    #[cfg(feature = "payload-contract-large-fixtures")]
    LidarHesaiAt128PointCloud(proto::LidarPointCloudFrame),
    #[cfg(feature = "payload-contract-large-fixtures")]
    Camera8mpBayerRggb12p(proto::CameraBayerFrame),
    #[cfg(feature = "payload-contract-simulator-fixtures")]
    Camera8mpCarlaBgra32(proto::CameraCarlaBgraFrame),
}

pub fn protobuf_fixture_for(case: &PayloadContractCase, sequence: u32) -> ProtobufFixture {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => {
            ProtobufFixture::CanClassicMax(build_proto_can_classic(case, sequence))
        }
        PayloadContractCaseKind::CanFdMax => {
            ProtobufFixture::CanFdMax(build_proto_can_fd(case, sequence))
        }
        PayloadContractCaseKind::SomeIpSingleMtu => {
            ProtobufFixture::SomeIpSingleMtu(build_proto_someip(case, sequence))
        }
        PayloadContractCaseKind::Streamer4k => {
            ProtobufFixture::Streamer4k(build_proto_stream(case, sequence, 4 * 1_024))
        }
        PayloadContractCaseKind::RadarArs548DetectionList => {
            ProtobufFixture::RadarArs548DetectionList(build_proto_radar(case, sequence))
        }
        PayloadContractCaseKind::Streamer64k => {
            ProtobufFixture::Streamer64k(build_proto_stream(case, sequence, 64 * 1_024))
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            ProtobufFixture::LidarHesaiAt128PointCloud(build_proto_lidar(case, sequence))
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            ProtobufFixture::Camera8mpBayerRggb12p(build_proto_camera_bayer(case, sequence))
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => {
            ProtobufFixture::Camera8mpCarlaBgra32(proto::CameraCarlaBgraFrame::new())
        }
    }
}

pub fn protobuf_encoded_len(case: &PayloadContractCase, sequence: u32) -> usize {
    match protobuf_fixture_for(case, sequence) {
        ProtobufFixture::CanClassicMax(value) => value.compute_size() as usize,
        ProtobufFixture::CanFdMax(value) => value.compute_size() as usize,
        ProtobufFixture::SomeIpSingleMtu(value) => value.compute_size() as usize,
        ProtobufFixture::Streamer4k(value) | ProtobufFixture::Streamer64k(value) => {
            value.compute_size() as usize
        }
        ProtobufFixture::RadarArs548DetectionList(value) => value.compute_size() as usize,
        #[cfg(feature = "payload-contract-large-fixtures")]
        ProtobufFixture::LidarHesaiAt128PointCloud(value) => value.compute_size() as usize,
        #[cfg(feature = "payload-contract-large-fixtures")]
        ProtobufFixture::Camera8mpBayerRggb12p(value) => value.compute_size() as usize,
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        ProtobufFixture::Camera8mpCarlaBgra32(value) => value.compute_size() as usize,
    }
}

pub fn protobuf_encoded_bytes_for(
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<Vec<u8>, UWireError> {
    let fixture = protobuf_fixture_for(case, sequence);
    let bytes = match fixture {
        ProtobufFixture::CanClassicMax(value) => value.write_to_bytes(),
        ProtobufFixture::CanFdMax(value) => value.write_to_bytes(),
        ProtobufFixture::SomeIpSingleMtu(value) => value.write_to_bytes(),
        ProtobufFixture::Streamer4k(value) | ProtobufFixture::Streamer64k(value) => {
            value.write_to_bytes()
        }
        ProtobufFixture::RadarArs548DetectionList(value) => value.write_to_bytes(),
        #[cfg(feature = "payload-contract-large-fixtures")]
        ProtobufFixture::LidarHesaiAt128PointCloud(value) => value.write_to_bytes(),
        #[cfg(feature = "payload-contract-large-fixtures")]
        ProtobufFixture::Camera8mpBayerRggb12p(value) => value.write_to_bytes(),
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        ProtobufFixture::Camera8mpCarlaBgra32(value) => value.write_to_bytes(),
    };
    bytes.map_err(|error| UWireError::serialization_error(error.to_string()))
}

pub fn validate_protobuf_bytes(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
) -> Result<(), UWireError> {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => {
            let value = proto::CanClassicFrame::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_can_classic(case, sequence, &value)
        }
        PayloadContractCaseKind::CanFdMax => {
            let value = proto::CanFdFrame::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_can_fd(case, sequence, &value)
        }
        PayloadContractCaseKind::SomeIpSingleMtu => {
            let value = proto::SomeIpSignalBatch::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_someip(case, sequence, &value)
        }
        PayloadContractCaseKind::Streamer4k | PayloadContractCaseKind::Streamer64k => {
            let value = proto::StreamChunk::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_stream(case, sequence, &value)
        }
        PayloadContractCaseKind::RadarArs548DetectionList => {
            let value = proto::Ars548DetectionList::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_radar(case, sequence, &value)
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            let value = proto::LidarPointCloudFrame::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_lidar(case, sequence, &value)
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            let value = proto::CameraBayerFrame::parse_from_bytes(bytes)
                .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
            validate_proto_camera_bayer(case, sequence, &value)
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => Ok(()),
    }
}

pub trait StablePayloadContractView: StablePayload {
    fn case_kind() -> PayloadContractCaseKind;
    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError>;
}

pub fn validate_stable_payload<T: StablePayloadContractView>(
    case: &PayloadContractCase,
    sequence: u32,
    payload: &T,
) -> Result<(), UWireError> {
    ensure_eq("stable case kind", T::case_kind(), case.kind)?;
    payload.validate_contract(case, sequence)
}

pub struct StableOwnedFixture {
    pub encoding: PayloadEncoding,
    pub bytes: Vec<u8>,
    pub stable_type_name: &'static str,
    pub stable_transport_len: usize,
    pub stable_align: usize,
}

pub fn stable_owned_fixture_for(
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<StableOwnedFixture, UWireError> {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => {
            stable_owned_fixture_for_type::<CanClassicFrameV1>(|init, _| {
                init_can_classic_max(init, sequence)
            })
        }
        PayloadContractCaseKind::CanFdMax => {
            stable_owned_fixture_for_type::<CanFdFrameV1>(|init, _| init_can_fd_max(init, sequence))
        }
        PayloadContractCaseKind::SomeIpSingleMtu => {
            stable_owned_fixture_for_type::<SomeIpSignalBatchMtuV1>(|init, _| {
                init_someip_single_mtu(init, sequence)
            })
        }
        PayloadContractCaseKind::Streamer4k => {
            stable_owned_fixture_for_type::<StreamChunk4kV1>(|init, _| {
                init_streamer_4k(init, sequence)
            })
        }
        PayloadContractCaseKind::RadarArs548DetectionList => {
            stable_owned_fixture_for_type::<RadarDetectionListArs548V1>(|init, _| {
                init_radar_ars548_detection_list(init, sequence)
            })
        }
        PayloadContractCaseKind::Streamer64k => {
            stable_owned_fixture_for_type::<StreamChunk64kV1>(|init, _| {
                init_streamer_64k(init, sequence)
            })
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            stable_owned_fixture_for_type::<LidarPointCloudHesaiAt128V1>(|init, _| {
                init_lidar_hesai_at128_point_cloud(init, sequence)
            })
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            stable_owned_fixture_for_type::<CameraBayerRggb12pFrame8mpV1>(|init, _| {
                init_camera_8mp_bayer_rggb12p(init, sequence)
            })
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => Err(UWireError::invalid_payload(
            "CARLA BGRA32 stable fixture is not implemented in representative v1",
        )),
    }
}

pub fn validate_stable_owned_bytes(
    case: &PayloadContractCase,
    sequence: u32,
    encoding: Option<&PayloadEncoding>,
    bytes: &[u8],
) -> Result<(), UWireError> {
    validate_stable_encoding_for_case(case, encoding)?;
    ensure_eq("stable owned length", bytes.len(), stable_payload_len(case))?;
    validate_stable_owned_samples(case, sequence, bytes)
}

pub fn validate_stable_owned_bytes_exact(
    case: &PayloadContractCase,
    sequence: u32,
    encoding: Option<&PayloadEncoding>,
    bytes: &[u8],
) -> Result<(), UWireError> {
    validate_stable_owned_bytes(case, sequence, encoding, bytes)?;
    let expected = stable_owned_fixture_for(case, sequence)?;
    if bytes != expected.bytes.as_slice() {
        return Err(UWireError::invalid_payload(format!(
            "stable owned bytes for {} do not match canonical fixture",
            case.name
        )));
    }
    Ok(())
}

pub fn init_can_classic_max<'a>(
    init: CanClassicFrameV1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, CanClassicFrameV1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::CanClassicMax);
    init.header(|header| init_header(header, case, sequence))?
        .interface_id(CAN_INTERFACE_ID)
        .can_id(CAN_CLASSIC_ID)
        .flags(CAN_FLAG_EFF)
        .reserved0(0)
        .bus_timestamp_ns(timestamp_ns(sequence))
        .len(8)
        .len8_dlc(8)
        .data_fill_with(|index| pattern_byte(case.case_id, sequence, index))
        .reserved1([0; 2])
        .checksum(fixture_checksum(case, sequence))
        .finish()
}

pub fn init_can_fd_max<'a>(
    init: CanFdFrameV1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, CanFdFrameV1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::CanFdMax);
    init.header(|header| init_header(header, case, sequence))?
        .interface_id(CAN_INTERFACE_ID)
        .can_id(CAN_FD_ID)
        .flags(CAN_FLAG_EFF | CAN_FD_FLAG_BRS | CAN_FD_FLAG_ESI)
        .reserved0(0)
        .bus_timestamp_ns(timestamp_ns(sequence))
        .len(64)
        .data_fill_with(|index| pattern_byte(case.case_id, sequence, index))
        .reserved1([0; 3])
        .checksum(fixture_checksum(case, sequence))
        .finish()
}

pub fn init_someip_single_mtu<'a>(
    init: SomeIpSignalBatchMtuV1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, SomeIpSignalBatchMtuV1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::SomeIpSingleMtu);
    init.header(|header| init_header(header, case, sequence))?
        .source_timestamp_ns(timestamp_ns(sequence))
        .length(SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET as u32)
        .sample_count(SOMEIP_MTU_SIGNAL_SAMPLE_COUNT as u32)
        .service_id(SOMEIP_SERVICE_ID)
        .method_or_event_id(SOMEIP_METHOD_OR_EVENT_ID)
        .client_id(SOMEIP_CLIENT_ID)
        .session_id(SOMEIP_SESSION_ID)
        .protocol_version(SOMEIP_PROTOCOL_VERSION)
        .interface_version(SOMEIP_INTERFACE_VERSION)
        .message_type(SOMEIP_MESSAGE_TYPE)
        .return_code(SOMEIP_RETURN_CODE)
        .reserved0(0)
        .samples(|index, sample| init_signal_sample(sample, sequence, index))?
        .checksum(fixture_checksum(case, sequence))
        .reserved1(0)
        .finish()
}

pub fn init_streamer_4k<'a>(
    init: StreamChunk4kV1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, StreamChunk4kV1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::Streamer4k);
    init.meta(|meta| init_stream_meta(meta, case, sequence, 4 * 1_024))?
        .chunk_fill_with(|index| pattern_byte(case.case_id, sequence, index))
        .finish()
}

pub fn init_radar_ars548_detection_list<'a>(
    init: RadarDetectionListArs548V1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, RadarDetectionListArs548V1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::RadarArs548DetectionList);
    init.header(|header| init_header(header, case, sequence))?
        .measurement_timestamp_ns(timestamp_ns(sequence))
        .sensor_id(RADAR_SENSOR_ID)
        .measurement_counter(sequence)
        .cycle_counter(sequence.wrapping_mul(3))
        .list_numofdetections(RADAR_ARS548_MAX_DETECTIONS as u32)
        .detections(|index, detection| init_radar_detection(detection, sequence, index))?
        .checksum(fixture_checksum(case, sequence))
        .reserved0(0)
        .finish()
}

pub fn init_streamer_64k<'a>(
    init: StreamChunk64kV1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, StreamChunk64kV1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::Streamer64k);
    init.meta(|meta| init_stream_meta(meta, case, sequence, 64 * 1_024))?
        .chunk_fill_with(|index| pattern_byte(case.case_id, sequence, index))
        .finish()
}

#[cfg(feature = "payload-contract-large-fixtures")]
pub fn init_lidar_hesai_at128_point_cloud<'a>(
    init: LidarPointCloudHesaiAt128V1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, LidarPointCloudHesaiAt128V1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::LidarHesaiAt128PointCloud);
    let cache = LidarPointTrigCache::new();
    init.header(|header| init_header(header, case, sequence))?
        .frame_counter(sequence as u64)
        .sensor_timestamp_ns(timestamp_ns(sequence))
        .scan_duration_ns(LIDAR_HESAI_AT128_SCAN_DURATION_NS)
        .lidar_id(LIDAR_ID)
        .width(LIDAR_HESAI_AT128_WIDTH)
        .height(LIDAR_HESAI_AT128_HEIGHT)
        .point_count(LIDAR_HESAI_AT128_POINT_COUNT as u32)
        .point_step(LIDAR_XYZIRCAEDT_POINT_BYTES as u32)
        .row_step(LIDAR_HESAI_AT128_ROW_STEP_BYTES)
        .points_per_second(LIDAR_HESAI_AT128_POINTS_PER_SECOND)
        .horizontal_fov_mdeg(LIDAR_HESAI_AT128_HORIZONTAL_FOV_MDEG)
        .vertical_fov_mdeg(LIDAR_HESAI_AT128_VERTICAL_FOV_MDEG)
        .horizontal_resolution_mdeg(LIDAR_HESAI_AT128_HORIZONTAL_RESOLUTION_MDEG)
        .vertical_resolution_mdeg(LIDAR_HESAI_AT128_VERTICAL_RESOLUTION_MDEG)
        .range_10pct_reflectivity_m(LIDAR_HESAI_AT128_RANGE_10PCT_REFLECTIVITY_M)
        .point_format(LIDAR_POINT_FORMAT_XYZIRCAEDT)
        .is_bigendian(0)
        .is_dense(1)
        .reserved0([0; 2])
        .checksum(fixture_checksum(case, sequence))
        .extrinsics(|pose| init_pose(pose, sequence))?
        .points(|index, point| init_lidar_point_cached(point, sequence, index, &cache))?
        .reserved1(0)
        .finish()
}

#[cfg(all(
    feature = "perf-diagnostics",
    feature = "payload-contract-large-fixtures"
))]
#[doc(hidden)]
pub fn init_lidar_hesai_at128_point_cloud_per_point_trig<'a>(
    init: LidarPointCloudHesaiAt128V1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, LidarPointCloudHesaiAt128V1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::LidarHesaiAt128PointCloud);
    init.header(|header| init_header(header, case, sequence))?
        .frame_counter(sequence as u64)
        .sensor_timestamp_ns(timestamp_ns(sequence))
        .scan_duration_ns(LIDAR_HESAI_AT128_SCAN_DURATION_NS)
        .lidar_id(LIDAR_ID)
        .width(LIDAR_HESAI_AT128_WIDTH)
        .height(LIDAR_HESAI_AT128_HEIGHT)
        .point_count(LIDAR_HESAI_AT128_POINT_COUNT as u32)
        .point_step(LIDAR_XYZIRCAEDT_POINT_BYTES as u32)
        .row_step(LIDAR_HESAI_AT128_ROW_STEP_BYTES)
        .points_per_second(LIDAR_HESAI_AT128_POINTS_PER_SECOND)
        .horizontal_fov_mdeg(LIDAR_HESAI_AT128_HORIZONTAL_FOV_MDEG)
        .vertical_fov_mdeg(LIDAR_HESAI_AT128_VERTICAL_FOV_MDEG)
        .horizontal_resolution_mdeg(LIDAR_HESAI_AT128_HORIZONTAL_RESOLUTION_MDEG)
        .vertical_resolution_mdeg(LIDAR_HESAI_AT128_VERTICAL_RESOLUTION_MDEG)
        .range_10pct_reflectivity_m(LIDAR_HESAI_AT128_RANGE_10PCT_REFLECTIVITY_M)
        .point_format(LIDAR_POINT_FORMAT_XYZIRCAEDT)
        .is_bigendian(0)
        .is_dense(1)
        .reserved0([0; 2])
        .checksum(fixture_checksum(case, sequence))
        .extrinsics(|pose| init_pose(pose, sequence))?
        .points(|index, point| init_lidar_point(point, sequence, index))?
        .reserved1(0)
        .finish()
}

#[cfg(feature = "payload-contract-large-fixtures")]
pub fn init_camera_8mp_bayer_rggb12p<'a>(
    init: CameraBayerRggb12pFrame8mpV1Init<'a>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, CameraBayerRggb12pFrame8mpV1>, UWireError> {
    let case = case_by_kind(PayloadContractCaseKind::Camera8mpBayerRggb12p);
    init.header(|header| init_header(header, case, sequence))?
        .frame_counter(sequence as u64)
        .sensor_timestamp_ns(timestamp_ns(sequence))
        .exposure_start_ns(timestamp_ns(sequence).saturating_sub(1_000_000))
        .camera_id(CAMERA_ID)
        .width(CAMERA_8MP_WIDTH)
        .height(CAMERA_8MP_HEIGHT)
        .stride_bytes(CAMERA_BAYER_RGGB12P_STRIDE_BYTES)
        .bayer_pattern(BAYER_PATTERN_RGGB)
        .bits_per_sample(BITS_PER_SAMPLE_12)
        .packed_layout(PACKED_LAYOUT_12P_LSB)
        .exposure_time_us(8_000)
        .analog_gain(1.5)
        .digital_gain(1.0)
        .intrinsics(init_camera_intrinsics)?
        .extrinsics(|pose| init_pose(pose, sequence))?
        .roi(|context| {
            let roi = context.into_init();
            roi.x(0)
                .y(0)
                .width(CAMERA_8MP_WIDTH)
                .height(CAMERA_8MP_HEIGHT)
                .finish()
        })?
        .checksum(fixture_checksum(case, sequence))
        .pixels_fill_with(|index| pattern_byte(case.case_id, sequence, index))
        .finish()
}

impl StablePayloadContractView for CanClassicFrameV1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::CanClassicMax
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_header(&self.header, case, sequence)?;
        ensure_eq("CAN interface_id", self.interface_id, CAN_INTERFACE_ID)?;
        ensure_eq("CAN id", self.can_id, CAN_CLASSIC_ID)?;
        ensure_eq("CAN flags", self.flags, CAN_FLAG_EFF)?;
        ensure_eq(
            "CAN timestamp",
            self.bus_timestamp_ns,
            timestamp_ns(sequence),
        )?;
        ensure_eq("CAN len", self.len, 8)?;
        ensure_eq("CAN len8_dlc", self.len8_dlc, 8)?;
        validate_all_pattern_bytes(case, sequence, &self.data)?;
        ensure_eq(
            "CAN checksum",
            self.checksum,
            fixture_checksum(case, sequence),
        )
    }
}

impl StablePayloadContractView for CanFdFrameV1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::CanFdMax
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_header(&self.header, case, sequence)?;
        ensure_eq("CAN FD interface_id", self.interface_id, CAN_INTERFACE_ID)?;
        ensure_eq("CAN FD id", self.can_id, CAN_FD_ID)?;
        ensure_eq(
            "CAN FD flags",
            self.flags,
            CAN_FLAG_EFF | CAN_FD_FLAG_BRS | CAN_FD_FLAG_ESI,
        )?;
        ensure_eq(
            "CAN FD timestamp",
            self.bus_timestamp_ns,
            timestamp_ns(sequence),
        )?;
        ensure_eq("CAN FD len", self.len, 64)?;
        validate_all_pattern_bytes(case, sequence, &self.data)?;
        ensure_eq(
            "CAN FD checksum",
            self.checksum,
            fixture_checksum(case, sequence),
        )
    }
}

impl StablePayloadContractView for SomeIpSignalBatchMtuV1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::SomeIpSingleMtu
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_header(&self.header, case, sequence)?;
        validate_someip_fields(
            self.source_timestamp_ns,
            self.length,
            self.sample_count,
            self.service_id,
            self.method_or_event_id,
            self.client_id,
            self.session_id,
            self.protocol_version,
            self.interface_version,
            self.message_type,
            self.return_code,
            self.checksum,
            case,
            sequence,
        )?;
        for index in sample_indices(SOMEIP_MTU_SIGNAL_SAMPLE_COUNT) {
            validate_signal_sample(&self.samples[index], sequence, index)?;
        }
        Ok(())
    }
}

impl StablePayloadContractView for StreamChunk4kV1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::Streamer4k
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_stream_meta(&self.meta, case, sequence, 4 * 1_024)?;
        validate_sample_pattern_bytes(case, sequence, &self.chunk)
    }
}

impl StablePayloadContractView for StreamChunk64kV1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::Streamer64k
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_stream_meta(&self.meta, case, sequence, 64 * 1_024)?;
        validate_sample_pattern_bytes(case, sequence, &self.chunk)
    }
}

impl StablePayloadContractView for RadarDetectionListArs548V1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::RadarArs548DetectionList
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_header(&self.header, case, sequence)?;
        ensure_eq(
            "radar timestamp",
            self.measurement_timestamp_ns,
            timestamp_ns(sequence),
        )?;
        ensure_eq("radar sensor id", self.sensor_id, RADAR_SENSOR_ID)?;
        ensure_eq(
            "radar measurement counter",
            self.measurement_counter,
            sequence,
        )?;
        ensure_eq(
            "radar cycle counter",
            self.cycle_counter,
            sequence.wrapping_mul(3),
        )?;
        ensure_eq(
            "radar detection count",
            self.list_numofdetections,
            RADAR_ARS548_MAX_DETECTIONS as u32,
        )?;
        for index in sample_indices(RADAR_ARS548_MAX_DETECTIONS) {
            validate_radar_detection(&self.detections[index], sequence, index)?;
        }
        ensure_eq(
            "radar checksum",
            self.checksum,
            fixture_checksum(case, sequence),
        )
    }
}

#[cfg(feature = "payload-contract-large-fixtures")]
impl StablePayloadContractView for LidarPointCloudHesaiAt128V1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::LidarHesaiAt128PointCloud
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_header(&self.header, case, sequence)?;
        validate_lidar_fields(
            self.frame_counter,
            self.sensor_timestamp_ns,
            self.scan_duration_ns,
            self.lidar_id,
            self.width,
            self.height,
            self.point_count,
            self.point_step,
            self.row_step,
            self.points_per_second,
            self.horizontal_fov_mdeg,
            self.vertical_fov_mdeg,
            self.horizontal_resolution_mdeg,
            self.vertical_resolution_mdeg,
            self.range_10pct_reflectivity_m,
            self.point_format,
            self.is_bigendian,
            self.is_dense,
            self.checksum,
            case,
            sequence,
        )?;
        validate_pose(&self.extrinsics, sequence)?;
        for index in sample_indices(LIDAR_HESAI_AT128_POINT_COUNT) {
            validate_lidar_point(&self.points[index], sequence, index)?;
        }
        Ok(())
    }
}

#[cfg(feature = "payload-contract-large-fixtures")]
impl StablePayloadContractView for CameraBayerRggb12pFrame8mpV1 {
    fn case_kind() -> PayloadContractCaseKind {
        PayloadContractCaseKind::Camera8mpBayerRggb12p
    }

    fn validate_contract(
        &self,
        case: &PayloadContractCase,
        sequence: u32,
    ) -> Result<(), UWireError> {
        validate_header(&self.header, case, sequence)?;
        validate_camera_fields(
            self.frame_counter,
            self.sensor_timestamp_ns,
            self.exposure_start_ns,
            self.camera_id,
            self.width,
            self.height,
            self.stride_bytes,
            self.bayer_pattern,
            self.bits_per_sample,
            self.packed_layout,
            self.exposure_time_us,
            self.analog_gain,
            self.digital_gain,
            self.checksum,
            case,
            sequence,
        )?;
        validate_camera_intrinsics(&self.intrinsics)?;
        validate_pose(&self.extrinsics, sequence)?;
        ensure_eq("camera roi x", self.roi.x, 0)?;
        ensure_eq("camera roi y", self.roi.y, 0)?;
        ensure_eq("camera roi width", self.roi.width, CAMERA_8MP_WIDTH)?;
        ensure_eq("camera roi height", self.roi.height, CAMERA_8MP_HEIGHT)?;
        validate_sample_pattern_bytes(case, sequence, &self.pixels)
    }
}

fn protobuf_message_name(case: &PayloadContractCase) -> &'static str {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => "uprotocol.bench.v1.CanClassicFrame",
        PayloadContractCaseKind::CanFdMax => "uprotocol.bench.v1.CanFdFrame",
        PayloadContractCaseKind::SomeIpSingleMtu => "uprotocol.bench.v1.SomeIpSignalBatch",
        PayloadContractCaseKind::Streamer4k | PayloadContractCaseKind::Streamer64k => {
            "uprotocol.bench.v1.StreamChunk"
        }
        PayloadContractCaseKind::RadarArs548DetectionList => {
            "uprotocol.bench.v1.Ars548DetectionList"
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            "uprotocol.bench.v1.LidarPointCloudFrame"
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => "uprotocol.bench.v1.CameraBayerFrame",
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => "uprotocol.bench.v1.CameraCarlaBgraFrame",
    }
}

fn stable_owned_fixture_for_type<T>(
    init: impl for<'a> FnOnce(
        <T as StablePayloadInit>::Init<'a>,
        std::marker::PhantomData<&'a mut T>,
    ) -> Result<InitializedStablePayload<'a, T>, UWireError>,
) -> Result<StableOwnedFixture, UWireError>
where
    T: StablePayload + StablePayloadInit,
{
    let encoding = StableContainerPayload::<T>::encoding();
    let metadata = fixture_metadata(Some(encoding.clone()));
    let mut buffer =
        UVecUninitTxBuffer::with_alignment(metadata, mem::size_of::<T>(), mem::align_of::<T>())
            .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
    {
        let payload = buffer.payload_uninit_mut();
        // SAFETY: `UVecUninitTxBuffer` exposes the exact visible payload range for this emulated loan.
        let loaned = unsafe {
            LoanedPayloadUninitMut::new_unchecked(
                payload,
                PayloadLoanProvenance::OpaqueTransportLoan,
            )
        };
        let initializer = T::init_from_uninit_payload(loaned)?;
        let _initialized = init(initializer, std::marker::PhantomData)?;
    }
    // SAFETY: the generated initializer returned the completion proof consumed above.
    let buffer = unsafe { buffer.assume_payload_init() };
    Ok(StableOwnedFixture {
        encoding,
        bytes: buffer.payload().to_vec(),
        stable_type_name: T::stable_type_name(),
        stable_transport_len: mem::size_of::<T>(),
        stable_align: mem::align_of::<T>(),
    })
}

fn fixture_metadata(payload_encoding: Option<PayloadEncoding>) -> UFrameMetadata {
    let mut builder = UFrameMetadata::publish(
        UUri::try_from_parts("payload-contract-fixture", 0x4210, 1, 0x9000)
            .expect("fixture URI is valid"),
    );
    if let Some(payload_encoding) = payload_encoding {
        builder = builder.with_payload_encoding(payload_encoding);
    }
    builder.build().expect("fixture metadata is valid")
}

fn validate_stable_encoding_for_case(
    case: &PayloadContractCase,
    encoding: Option<&PayloadEncoding>,
) -> Result<(), UWireError> {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => {
            validate_stable_encoding::<CanClassicFrameV1>(encoding)
        }
        PayloadContractCaseKind::CanFdMax => validate_stable_encoding::<CanFdFrameV1>(encoding),
        PayloadContractCaseKind::SomeIpSingleMtu => {
            validate_stable_encoding::<SomeIpSignalBatchMtuV1>(encoding)
        }
        PayloadContractCaseKind::Streamer4k => {
            validate_stable_encoding::<StreamChunk4kV1>(encoding)
        }
        PayloadContractCaseKind::RadarArs548DetectionList => {
            validate_stable_encoding::<RadarDetectionListArs548V1>(encoding)
        }
        PayloadContractCaseKind::Streamer64k => {
            validate_stable_encoding::<StreamChunk64kV1>(encoding)
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            validate_stable_encoding::<LidarPointCloudHesaiAt128V1>(encoding)
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            validate_stable_encoding::<CameraBayerRggb12pFrame8mpV1>(encoding)
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => Ok(()),
    }
}

fn validate_stable_encoding<T: StablePayload>(
    encoding: Option<&PayloadEncoding>,
) -> Result<(), UWireError> {
    <StableContainerPayload<T> as crate::payload::codec::PayloadCodec>::verify_encoding(encoding)
}

fn build_proto_can_classic(case: &PayloadContractCase, sequence: u32) -> proto::CanClassicFrame {
    let mut message = proto::CanClassicFrame::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.interface_id = CAN_INTERFACE_ID;
    message.can_id = CAN_CLASSIC_ID;
    message.is_extended_id = true;
    message.len = 8;
    message.len8_dlc = 8;
    message.bus_timestamp_ns = timestamp_ns(sequence);
    message.data = pattern_vec(case.case_id, sequence, 8);
    message.checksum = fixture_checksum(case, sequence);
    message
}

fn build_proto_can_fd(case: &PayloadContractCase, sequence: u32) -> proto::CanFdFrame {
    let mut message = proto::CanFdFrame::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.interface_id = CAN_INTERFACE_ID;
    message.can_id = CAN_FD_ID;
    message.is_extended_id = true;
    message.bitrate_switch = true;
    message.error_state_indicator = true;
    message.len = 64;
    message.bus_timestamp_ns = timestamp_ns(sequence);
    message.data = pattern_vec(case.case_id, sequence, 64);
    message.checksum = fixture_checksum(case, sequence);
    message
}

fn build_proto_someip(case: &PayloadContractCase, sequence: u32) -> proto::SomeIpSignalBatch {
    let mut message = proto::SomeIpSignalBatch::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.service_id = u32::from(SOMEIP_SERVICE_ID);
    message.method_or_event_id = u32::from(SOMEIP_METHOD_OR_EVENT_ID);
    message.length = SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET as u32;
    message.client_id = u32::from(SOMEIP_CLIENT_ID);
    message.session_id = u32::from(SOMEIP_SESSION_ID);
    message.protocol_version = u32::from(SOMEIP_PROTOCOL_VERSION);
    message.interface_version = u32::from(SOMEIP_INTERFACE_VERSION);
    message.message_type = u32::from(SOMEIP_MESSAGE_TYPE);
    message.return_code = u32::from(SOMEIP_RETURN_CODE);
    message.source_timestamp_ns = timestamp_ns(sequence);
    message.samples = (0..SOMEIP_MTU_SIGNAL_SAMPLE_COUNT)
        .map(|index| {
            let sample = signal_sample_value(sequence, index);
            let mut message = proto::SignalSample::new();
            message.signal_id = sample.signal_id;
            message.status = sample.status;
            message.timestamp_ns = sample.timestamp_ns;
            message.value = sample.value;
            message
        })
        .collect();
    message.checksum = fixture_checksum(case, sequence);
    message
}

fn build_proto_stream(
    case: &PayloadContractCase,
    sequence: u32,
    chunk_len: usize,
) -> proto::StreamChunk {
    let mut message = proto::StreamChunk::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.stream_id = STREAM_ID;
    message.codec = STREAM_CODEC_RAW;
    message.flags = STREAM_FLAG_KEY_FRAME;
    message.byte_offset = u64::from(sequence).saturating_mul(chunk_len as u64);
    message.chunk_index = sequence;
    message.chunk_count = sequence.saturating_add(7);
    message.source_timestamp_ns = timestamp_ns(sequence);
    message.chunk = pattern_vec(case.case_id, sequence, chunk_len);
    message.checksum = fixture_checksum(case, sequence);
    message
}

fn build_proto_radar(case: &PayloadContractCase, sequence: u32) -> proto::Ars548DetectionList {
    let mut message = proto::Ars548DetectionList::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.sensor_id = RADAR_SENSOR_ID;
    message.measurement_counter = sequence;
    message.cycle_counter = sequence.wrapping_mul(3);
    message.measurement_timestamp_ns = timestamp_ns(sequence);
    message.list_numofdetections = RADAR_ARS548_MAX_DETECTIONS as u32;
    message.detections = (0..RADAR_ARS548_MAX_DETECTIONS)
        .map(|index| {
            let detection = radar_detection_value(sequence, index);
            let mut message = proto::Ars548Detection::new();
            message.azimuth_angle = detection.azimuth_angle;
            message.azimuth_angle_std = detection.azimuth_angle_std;
            message.invalid_flags = u32::from(detection.invalid_flags);
            message.elevation_angle = detection.elevation_angle;
            message.elevation_angle_std = detection.elevation_angle_std;
            message.range = detection.range;
            message.range_std = detection.range_std;
            message.range_rate = detection.range_rate;
            message.range_rate_std = detection.range_rate_std;
            message.rcs = i32::from(detection.rcs);
            message.measurement_id = u32::from(detection.measurement_id);
            message.positive_predictive_value = u32::from(detection.positive_predictive_value);
            message.classification = u32::from(detection.classification);
            message.multi_target_probability = u32::from(detection.multi_target_probability);
            message.object_id = u32::from(detection.object_id);
            message.ambiguity_flag = u32::from(detection.ambiguity_flag);
            message.sort_index = u32::from(detection.sort_index);
            message
        })
        .collect();
    message.checksum = fixture_checksum(case, sequence);
    message
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn build_proto_lidar(case: &PayloadContractCase, sequence: u32) -> proto::LidarPointCloudFrame {
    let mut message = proto::LidarPointCloudFrame::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.lidar_id = LIDAR_ID;
    message.frame_counter = u64::from(sequence);
    message.sensor_timestamp_ns = timestamp_ns(sequence);
    message.width = LIDAR_HESAI_AT128_WIDTH;
    message.height = LIDAR_HESAI_AT128_HEIGHT;
    message.point_count = LIDAR_HESAI_AT128_POINT_COUNT as u32;
    message.point_step = LIDAR_XYZIRCAEDT_POINT_BYTES as u32;
    message.row_step = LIDAR_HESAI_AT128_ROW_STEP_BYTES;
    message.points_per_second = LIDAR_HESAI_AT128_POINTS_PER_SECOND;
    message.scan_duration_ns = LIDAR_HESAI_AT128_SCAN_DURATION_NS;
    message.horizontal_fov_mdeg = LIDAR_HESAI_AT128_HORIZONTAL_FOV_MDEG;
    message.vertical_fov_mdeg = LIDAR_HESAI_AT128_VERTICAL_FOV_MDEG;
    message.horizontal_resolution_mdeg = LIDAR_HESAI_AT128_HORIZONTAL_RESOLUTION_MDEG;
    message.vertical_resolution_mdeg = LIDAR_HESAI_AT128_VERTICAL_RESOLUTION_MDEG;
    message.range_10pct_reflectivity_m = LIDAR_HESAI_AT128_RANGE_10PCT_REFLECTIVITY_M;
    message.point_format = LIDAR_POINT_FORMAT_XYZIRCAEDT;
    message.is_dense = true;
    message.extrinsics = MessageField::some(proto_pose(sequence));
    let mut points = vec![0_u8; LIDAR_HESAI_AT128_POINTS_BYTES];
    for index in 0..LIDAR_HESAI_AT128_POINT_COUNT {
        write_lidar_point_le(&mut points, sequence, index);
    }
    message.points_xyzircaedt_le = points;
    message.checksum = fixture_checksum(case, sequence);
    message
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn build_proto_camera_bayer(case: &PayloadContractCase, sequence: u32) -> proto::CameraBayerFrame {
    let mut message = proto::CameraBayerFrame::new();
    message.header = MessageField::some(proto_header(case, sequence));
    message.camera_id = CAMERA_ID;
    message.frame_counter = u64::from(sequence);
    message.sensor_timestamp_ns = timestamp_ns(sequence);
    message.width = CAMERA_8MP_WIDTH;
    message.height = CAMERA_8MP_HEIGHT;
    message.stride_bytes = CAMERA_BAYER_RGGB12P_STRIDE_BYTES;
    message.bayer_pattern = BAYER_PATTERN_RGGB;
    message.bits_per_sample = BITS_PER_SAMPLE_12;
    message.packed_layout = PACKED_LAYOUT_12P_LSB;
    message.exposure_start_ns = timestamp_ns(sequence).saturating_sub(1_000_000);
    message.exposure_time_us = 8_000;
    message.analog_gain = 1.5;
    message.digital_gain = 1.0;
    message.intrinsics = MessageField::some(proto_camera_intrinsics());
    message.extrinsics = MessageField::some(proto_pose(sequence));
    let mut roi = proto::Roi::new();
    roi.width = CAMERA_8MP_WIDTH;
    roi.height = CAMERA_8MP_HEIGHT;
    message.roi = MessageField::some(roi);
    message.pixels = pattern_vec(case.case_id, sequence, CAMERA_BAYER_RGGB12P_BYTES);
    message.checksum = fixture_checksum(case, sequence);
    message
}

fn proto_header(case: &PayloadContractCase, sequence: u32) -> proto::BenchHeader {
    let mut header = proto::BenchHeader::new();
    header.case_id = case.case_id;
    header.sequence = sequence;
    header.semantic_reference_len = case.semantic_reference_len as u32;
    header.schema_version = PAYLOAD_CONTRACT_FIXTURE_VERSION;
    header
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn proto_pose(sequence: u32) -> proto::Pose3d {
    let pose = pose_value(sequence);
    let mut translation = proto::Vector3f::new();
    translation.x = pose.translation_m.x;
    translation.y = pose.translation_m.y;
    translation.z = pose.translation_m.z;
    let mut rotation = proto::Quaternionf::new();
    rotation.x = pose.rotation.x;
    rotation.y = pose.rotation.y;
    rotation.z = pose.rotation.z;
    rotation.w = pose.rotation.w;
    let mut message = proto::Pose3d::new();
    message.translation_m = MessageField::some(translation);
    message.rotation = MessageField::some(rotation);
    message
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn proto_camera_intrinsics() -> proto::CameraIntrinsics {
    let mut message = proto::CameraIntrinsics::new();
    message.fx = 1_950.0;
    message.fy = 1_950.0;
    message.cx = 1_920.0;
    message.cy = 1_080.0;
    message.distortion_model = 1;
    message.distortion = vec![0.1, -0.03, 0.001, 0.0005, 0.0, 0.0, 0.0, 0.0];
    message
}

fn init_header<'a>(
    context: StablePayloadInitContext<'a, BenchHeaderV1>,
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, BenchHeaderV1>, UWireError> {
    let init = context.into_init();
    init.case_id(case.case_id)
        .sequence(sequence)
        .semantic_reference_len(case.semantic_reference_len as u32)
        .schema_version(PAYLOAD_CONTRACT_FIXTURE_VERSION)
        .finish()
}

fn init_signal_sample<'a>(
    context: StablePayloadInitContext<'a, SignalSampleV1>,
    sequence: u32,
    index: usize,
) -> Result<InitializedStablePayload<'a, SignalSampleV1>, UWireError> {
    let init = context.into_init();
    let sample = signal_sample_value(sequence, index);
    init.signal_id(sample.signal_id)
        .status(sample.status)
        .timestamp_ns(sample.timestamp_ns)
        .value(sample.value)
        .finish()
}

fn init_stream_meta<'a>(
    context: StablePayloadInitContext<'a, StreamChunkHeaderV1>,
    case: &PayloadContractCase,
    sequence: u32,
    chunk_len: usize,
) -> Result<InitializedStablePayload<'a, StreamChunkHeaderV1>, UWireError> {
    let init = context.into_init();
    init.header(|header| init_header(header, case, sequence))?
        .stream_id(STREAM_ID)
        .codec(STREAM_CODEC_RAW)
        .flags(STREAM_FLAG_KEY_FRAME)
        .byte_offset(u64::from(sequence).saturating_mul(chunk_len as u64))
        .chunk_index(sequence)
        .chunk_count(sequence.saturating_add(7))
        .source_timestamp_ns(timestamp_ns(sequence))
        .checksum(fixture_checksum(case, sequence))
        .reserved0(0)
        .finish()
}

fn init_radar_detection<'a>(
    context: StablePayloadInitContext<'a, Ars548DetectionV1>,
    sequence: u32,
    index: usize,
) -> Result<InitializedStablePayload<'a, Ars548DetectionV1>, UWireError> {
    let init = context.into_init();
    let detection = radar_detection_value(sequence, index);
    init.azimuth_angle(detection.azimuth_angle)
        .azimuth_angle_std(detection.azimuth_angle_std)
        .elevation_angle(detection.elevation_angle)
        .elevation_angle_std(detection.elevation_angle_std)
        .range(detection.range)
        .range_std(detection.range_std)
        .range_rate(detection.range_rate)
        .range_rate_std(detection.range_rate_std)
        .measurement_id(detection.measurement_id)
        .object_id(detection.object_id)
        .sort_index(detection.sort_index)
        .invalid_flags(detection.invalid_flags)
        .rcs(detection.rcs)
        .positive_predictive_value(detection.positive_predictive_value)
        .classification(detection.classification)
        .multi_target_probability(detection.multi_target_probability)
        .ambiguity_flag(detection.ambiguity_flag)
        .finish()
}

#[cfg(all(
    feature = "perf-diagnostics",
    feature = "payload-contract-large-fixtures"
))]
fn init_lidar_point<'a>(
    context: StablePayloadInitContext<'a, LidarPointXyzircaedtV1>,
    sequence: u32,
    index: usize,
) -> Result<InitializedStablePayload<'a, LidarPointXyzircaedtV1>, UWireError> {
    let init = context.into_init();
    let point = lidar_point_value(sequence, index);
    init.x(point.x)
        .y(point.y)
        .z(point.z)
        .intensity(point.intensity)
        .return_type(point.return_type)
        .channel(point.channel)
        .azimuth(point.azimuth)
        .elevation(point.elevation)
        .distance(point.distance)
        .time_offset_ns(point.time_offset_ns)
        .finish()
}

#[cfg(feature = "payload-contract-large-fixtures")]
struct LidarPointTrigCache {
    azimuth: [(f32, f32, f32); LIDAR_HESAI_AT128_WIDTH as usize],
    elevation: [(f32, f32); LIDAR_HESAI_AT128_HEIGHT as usize],
}

#[cfg(feature = "payload-contract-large-fixtures")]
impl LidarPointTrigCache {
    fn new() -> Self {
        Self {
            azimuth: std::array::from_fn(|column| {
                let azimuth = -60.0 + column as f32 * 0.1;
                (
                    azimuth,
                    azimuth.to_radians().cos(),
                    azimuth.to_radians().sin(),
                )
            }),
            elevation: std::array::from_fn(|row| {
                let elevation = -12.7 + row as f32 * 0.2;
                (elevation, elevation.to_radians().sin())
            }),
        }
    }
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn init_lidar_point_cached<'a>(
    context: StablePayloadInitContext<'a, LidarPointXyzircaedtV1>,
    sequence: u32,
    index: usize,
    cache: &LidarPointTrigCache,
) -> Result<InitializedStablePayload<'a, LidarPointXyzircaedtV1>, UWireError> {
    let column = index % LIDAR_HESAI_AT128_WIDTH as usize;
    let row = index / LIDAR_HESAI_AT128_WIDTH as usize;
    let (azimuth, azimuth_cos, azimuth_sin) = cache.azimuth[column];
    let (elevation, elevation_sin) = cache.elevation[row];
    let distance = 5.0 + (index % 2_000) as f32 * 0.05;
    context
        .into_init()
        .x(distance * azimuth_cos)
        .y(distance * azimuth_sin)
        .z(distance * elevation_sin)
        .intensity(pattern_byte(7, sequence, index))
        .return_type(1)
        .channel(row as u16)
        .azimuth(azimuth)
        .elevation(elevation)
        .distance(distance)
        .time_offset_ns(
            ((index as u64 * u64::from(LIDAR_HESAI_AT128_SCAN_DURATION_NS))
                / LIDAR_HESAI_AT128_POINT_COUNT as u64) as u32,
        )
        .finish()
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn init_pose<'a>(
    context: StablePayloadInitContext<'a, Pose3dV1>,
    sequence: u32,
) -> Result<InitializedStablePayload<'a, Pose3dV1>, UWireError> {
    let init = context.into_init();
    let pose = pose_value(sequence);
    init.translation_m(|context| {
        let translation = context.into_init();
        translation
            .x(pose.translation_m.x)
            .y(pose.translation_m.y)
            .z(pose.translation_m.z)
            .finish()
    })?
    .rotation(|context| {
        let rotation = context.into_init();
        rotation
            .x(pose.rotation.x)
            .y(pose.rotation.y)
            .z(pose.rotation.z)
            .w(pose.rotation.w)
            .finish()
    })?
    .finish()
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn init_camera_intrinsics<'a>(
    context: StablePayloadInitContext<'a, CameraIntrinsicsV1>,
) -> Result<InitializedStablePayload<'a, CameraIntrinsicsV1>, UWireError> {
    let init = context.into_init();
    init.fx(1_950.0)
        .fy(1_950.0)
        .cx(1_920.0)
        .cy(1_080.0)
        .skew(0.0)
        .distortion_model(1)
        .distortion_from_slice(&[0.1, -0.03, 0.001, 0.0005, 0.0, 0.0, 0.0, 0.0])?
        .finish()
}

fn validate_proto_can_classic(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::CanClassicFrame,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    ensure_eq(
        "protobuf CAN interface_id",
        message.interface_id,
        CAN_INTERFACE_ID,
    )?;
    ensure_eq("protobuf CAN id", message.can_id, CAN_CLASSIC_ID)?;
    ensure("protobuf CAN EFF", message.is_extended_id)?;
    ensure_eq("protobuf CAN RTR", message.is_remote_request, false)?;
    ensure_eq("protobuf CAN ERR", message.is_error_frame, false)?;
    ensure_eq("protobuf CAN len", message.len, 8)?;
    ensure_eq("protobuf CAN len8_dlc", message.len8_dlc, 8)?;
    ensure_eq(
        "protobuf CAN timestamp",
        message.bus_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    validate_all_pattern_bytes(case, sequence, &message.data)?;
    ensure_eq(
        "protobuf CAN checksum",
        message.checksum,
        fixture_checksum(case, sequence),
    )
}

fn validate_proto_can_fd(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::CanFdFrame,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    ensure_eq(
        "protobuf CAN FD interface_id",
        message.interface_id,
        CAN_INTERFACE_ID,
    )?;
    ensure_eq("protobuf CAN FD id", message.can_id, CAN_FD_ID)?;
    ensure("protobuf CAN FD EFF", message.is_extended_id)?;
    ensure("protobuf CAN FD BRS", message.bitrate_switch)?;
    ensure("protobuf CAN FD ESI", message.error_state_indicator)?;
    ensure_eq("protobuf CAN FD len", message.len, 64)?;
    ensure_eq(
        "protobuf CAN FD timestamp",
        message.bus_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    validate_all_pattern_bytes(case, sequence, &message.data)?;
    ensure_eq(
        "protobuf CAN FD checksum",
        message.checksum,
        fixture_checksum(case, sequence),
    )
}

fn validate_proto_someip(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::SomeIpSignalBatch,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    validate_someip_fields(
        message.source_timestamp_ns,
        message.length,
        message.samples.len() as u32,
        message.service_id as u16,
        message.method_or_event_id as u16,
        message.client_id as u16,
        message.session_id as u16,
        message.protocol_version as u8,
        message.interface_version as u8,
        message.message_type as u8,
        message.return_code as u8,
        message.checksum,
        case,
        sequence,
    )?;
    for index in sample_indices(SOMEIP_MTU_SIGNAL_SAMPLE_COUNT) {
        let sample = message
            .samples
            .get(index)
            .ok_or_else(|| UWireError::invalid_payload("missing SOME/IP sample"))?;
        let expected = signal_sample_value(sequence, index);
        ensure_eq(
            "protobuf SOME/IP signal_id",
            sample.signal_id,
            expected.signal_id,
        )?;
        ensure_eq("protobuf SOME/IP status", sample.status, expected.status)?;
        ensure_eq(
            "protobuf SOME/IP timestamp",
            sample.timestamp_ns,
            expected.timestamp_ns,
        )?;
        ensure_eq("protobuf SOME/IP value", sample.value, expected.value)?;
    }
    Ok(())
}

fn validate_proto_stream(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::StreamChunk,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    validate_stream_fields(
        message.stream_id,
        message.codec,
        message.flags,
        message.byte_offset,
        message.chunk_index,
        message.chunk_count,
        message.source_timestamp_ns,
        message.checksum,
        case,
        sequence,
        message.chunk.len(),
    )?;
    validate_sample_pattern_bytes(case, sequence, &message.chunk)
}

fn validate_proto_radar(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::Ars548DetectionList,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    ensure_eq(
        "protobuf radar sensor id",
        message.sensor_id,
        RADAR_SENSOR_ID,
    )?;
    ensure_eq(
        "protobuf radar measurement counter",
        message.measurement_counter,
        sequence,
    )?;
    ensure_eq(
        "protobuf radar cycle counter",
        message.cycle_counter,
        sequence.wrapping_mul(3),
    )?;
    ensure_eq(
        "protobuf radar timestamp",
        message.measurement_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    ensure_eq(
        "protobuf radar detection count",
        message.list_numofdetections,
        RADAR_ARS548_MAX_DETECTIONS as u32,
    )?;
    ensure_eq(
        "protobuf radar repeated count",
        message.detections.len(),
        RADAR_ARS548_MAX_DETECTIONS,
    )?;
    for index in sample_indices(RADAR_ARS548_MAX_DETECTIONS) {
        let detection = message
            .detections
            .get(index)
            .ok_or_else(|| UWireError::invalid_payload("missing radar detection"))?;
        let expected = radar_detection_value(sequence, index);
        ensure_eq("protobuf radar range", detection.range, expected.range)?;
        ensure_eq(
            "protobuf radar range rate",
            detection.range_rate,
            expected.range_rate,
        )?;
        ensure_eq(
            "protobuf radar measurement id",
            detection.measurement_id,
            u32::from(expected.measurement_id),
        )?;
        ensure_eq(
            "protobuf radar object id",
            detection.object_id,
            u32::from(expected.object_id),
        )?;
    }
    ensure_eq(
        "protobuf radar checksum",
        message.checksum,
        fixture_checksum(case, sequence),
    )
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn validate_proto_lidar(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::LidarPointCloudFrame,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    validate_lidar_fields(
        message.frame_counter,
        message.sensor_timestamp_ns,
        message.scan_duration_ns,
        message.lidar_id,
        message.width,
        message.height,
        message.point_count,
        message.point_step,
        message.row_step,
        message.points_per_second,
        message.horizontal_fov_mdeg,
        message.vertical_fov_mdeg,
        message.horizontal_resolution_mdeg,
        message.vertical_resolution_mdeg,
        message.range_10pct_reflectivity_m,
        message.point_format,
        u8::from(message.is_bigendian),
        u8::from(message.is_dense),
        message.checksum,
        case,
        sequence,
    )?;
    ensure_eq(
        "protobuf lidar points bytes",
        message.points_xyzircaedt_le.len(),
        LIDAR_HESAI_AT128_POINTS_BYTES,
    )?;
    for index in sample_indices(LIDAR_HESAI_AT128_POINT_COUNT) {
        validate_lidar_point_le(&message.points_xyzircaedt_le, sequence, index)?;
    }
    Ok(())
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn validate_proto_camera_bayer(
    case: &PayloadContractCase,
    sequence: u32,
    message: &proto::CameraBayerFrame,
) -> Result<(), UWireError> {
    validate_proto_header(message.header.as_ref(), case, sequence)?;
    validate_camera_fields(
        message.frame_counter,
        message.sensor_timestamp_ns,
        message.exposure_start_ns,
        message.camera_id,
        message.width,
        message.height,
        message.stride_bytes,
        message.bayer_pattern,
        message.bits_per_sample,
        message.packed_layout,
        message.exposure_time_us,
        message.analog_gain,
        message.digital_gain,
        message.checksum,
        case,
        sequence,
    )?;
    ensure_eq(
        "protobuf camera pixels",
        message.pixels.len(),
        CAMERA_BAYER_RGGB12P_BYTES,
    )?;
    validate_sample_pattern_bytes(case, sequence, &message.pixels)
}

fn validate_proto_header(
    header: Option<&proto::BenchHeader>,
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<(), UWireError> {
    let header = header.ok_or_else(|| UWireError::invalid_payload("missing protobuf header"))?;
    ensure_eq("protobuf case_id", header.case_id, case.case_id)?;
    ensure_eq("protobuf sequence", header.sequence, sequence)?;
    ensure_eq(
        "protobuf semantic_reference_len",
        header.semantic_reference_len,
        case.semantic_reference_len as u32,
    )?;
    ensure_eq(
        "protobuf schema_version",
        header.schema_version,
        PAYLOAD_CONTRACT_FIXTURE_VERSION,
    )
}

fn validate_header(
    header: &BenchHeaderV1,
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<(), UWireError> {
    ensure_eq("stable case_id", header.case_id, case.case_id)?;
    ensure_eq("stable sequence", header.sequence, sequence)?;
    ensure_eq(
        "stable semantic_reference_len",
        header.semantic_reference_len,
        case.semantic_reference_len as u32,
    )?;
    ensure_eq(
        "stable schema_version",
        header.schema_version,
        PAYLOAD_CONTRACT_FIXTURE_VERSION,
    )
}

fn validate_stable_owned_samples(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
) -> Result<(), UWireError> {
    match case.kind {
        PayloadContractCaseKind::CanClassicMax => {
            validate_stable_owned_header(
                case,
                sequence,
                bytes,
                mem::offset_of!(CanClassicFrameV1, header),
            )?;
            ensure_eq(
                "owned CAN len",
                read_u8(bytes, mem::offset_of!(CanClassicFrameV1, len))?,
                8,
            )?;
            validate_owned_pattern_field(
                case,
                sequence,
                bytes,
                mem::offset_of!(CanClassicFrameV1, data),
                8,
                true,
            )?;
            ensure_eq(
                "owned CAN checksum",
                read_u32(bytes, mem::offset_of!(CanClassicFrameV1, checksum))?,
                fixture_checksum(case, sequence),
            )
        }
        PayloadContractCaseKind::CanFdMax => {
            validate_stable_owned_header(
                case,
                sequence,
                bytes,
                mem::offset_of!(CanFdFrameV1, header),
            )?;
            ensure_eq(
                "owned CAN FD len",
                read_u8(bytes, mem::offset_of!(CanFdFrameV1, len))?,
                64,
            )?;
            validate_owned_pattern_field(
                case,
                sequence,
                bytes,
                mem::offset_of!(CanFdFrameV1, data),
                64,
                true,
            )?;
            ensure_eq(
                "owned CAN FD checksum",
                read_u32(bytes, mem::offset_of!(CanFdFrameV1, checksum))?,
                fixture_checksum(case, sequence),
            )
        }
        PayloadContractCaseKind::SomeIpSingleMtu => {
            validate_stable_owned_header(
                case,
                sequence,
                bytes,
                mem::offset_of!(SomeIpSignalBatchMtuV1, header),
            )?;
            ensure_eq(
                "owned SOME/IP sample count",
                read_u32(bytes, mem::offset_of!(SomeIpSignalBatchMtuV1, sample_count))?,
                SOMEIP_MTU_SIGNAL_SAMPLE_COUNT as u32,
            )?;
            let samples_offset = mem::offset_of!(SomeIpSignalBatchMtuV1, samples);
            for index in sample_indices(SOMEIP_MTU_SIGNAL_SAMPLE_COUNT) {
                let offset = samples_offset + index * mem::size_of::<SignalSampleV1>();
                ensure_eq(
                    "owned SOME/IP signal id",
                    read_u32(bytes, offset + mem::offset_of!(SignalSampleV1, signal_id))?,
                    signal_sample_value(sequence, index).signal_id,
                )?;
            }
            Ok(())
        }
        PayloadContractCaseKind::Streamer4k => validate_stable_owned_stream(
            case,
            sequence,
            bytes,
            mem::offset_of!(StreamChunk4kV1, meta),
            mem::offset_of!(StreamChunk4kV1, chunk),
            4 * 1_024,
        ),
        PayloadContractCaseKind::RadarArs548DetectionList => {
            validate_stable_owned_header(
                case,
                sequence,
                bytes,
                mem::offset_of!(RadarDetectionListArs548V1, header),
            )?;
            ensure_eq(
                "owned radar count",
                read_u32(
                    bytes,
                    mem::offset_of!(RadarDetectionListArs548V1, list_numofdetections),
                )?,
                RADAR_ARS548_MAX_DETECTIONS as u32,
            )?;
            let detections_offset = mem::offset_of!(RadarDetectionListArs548V1, detections);
            for index in sample_indices(RADAR_ARS548_MAX_DETECTIONS) {
                let offset = detections_offset + index * mem::size_of::<Ars548DetectionV1>();
                ensure_eq(
                    "owned radar range",
                    read_f32(bytes, offset + mem::offset_of!(Ars548DetectionV1, range))?,
                    radar_detection_value(sequence, index).range,
                )?;
            }
            Ok(())
        }
        PayloadContractCaseKind::Streamer64k => validate_stable_owned_stream(
            case,
            sequence,
            bytes,
            mem::offset_of!(StreamChunk64kV1, meta),
            mem::offset_of!(StreamChunk64kV1, chunk),
            64 * 1_024,
        ),
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::LidarHesaiAt128PointCloud => {
            validate_stable_owned_header(
                case,
                sequence,
                bytes,
                mem::offset_of!(LidarPointCloudHesaiAt128V1, header),
            )?;
            ensure_eq(
                "owned lidar point count",
                read_u32(
                    bytes,
                    mem::offset_of!(LidarPointCloudHesaiAt128V1, point_count),
                )?,
                LIDAR_HESAI_AT128_POINT_COUNT as u32,
            )?;
            let points_offset = mem::offset_of!(LidarPointCloudHesaiAt128V1, points);
            for index in sample_indices(LIDAR_HESAI_AT128_POINT_COUNT) {
                let offset = points_offset + index * mem::size_of::<LidarPointXyzircaedtV1>();
                ensure_eq(
                    "owned lidar x",
                    read_f32(bytes, offset + mem::offset_of!(LidarPointXyzircaedtV1, x))?,
                    lidar_point_value(sequence, index).x,
                )?;
            }
            Ok(())
        }
        #[cfg(feature = "payload-contract-large-fixtures")]
        PayloadContractCaseKind::Camera8mpBayerRggb12p => {
            validate_stable_owned_header(
                case,
                sequence,
                bytes,
                mem::offset_of!(CameraBayerRggb12pFrame8mpV1, header),
            )?;
            ensure_eq(
                "owned camera width",
                read_u32(bytes, mem::offset_of!(CameraBayerRggb12pFrame8mpV1, width))?,
                CAMERA_8MP_WIDTH,
            )?;
            validate_owned_pattern_field(
                case,
                sequence,
                bytes,
                mem::offset_of!(CameraBayerRggb12pFrame8mpV1, pixels),
                CAMERA_BAYER_RGGB12P_BYTES,
                false,
            )
        }
        #[cfg(feature = "payload-contract-simulator-fixtures")]
        PayloadContractCaseKind::Camera8mpCarlaBgra32 => Ok(()),
    }
}

fn validate_stable_owned_header(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
    offset: usize,
) -> Result<(), UWireError> {
    ensure_eq(
        "owned stable case_id",
        read_u32(bytes, offset)?,
        case.case_id,
    )?;
    ensure_eq(
        "owned stable sequence",
        read_u32(bytes, offset + 4)?,
        sequence,
    )?;
    ensure_eq(
        "owned stable semantic length",
        read_u32(bytes, offset + 8)?,
        case.semantic_reference_len as u32,
    )?;
    ensure_eq(
        "owned stable schema version",
        read_u32(bytes, offset + 12)?,
        PAYLOAD_CONTRACT_FIXTURE_VERSION,
    )
}

fn validate_stable_owned_stream(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
    meta_offset: usize,
    chunk_offset: usize,
    chunk_len: usize,
) -> Result<(), UWireError> {
    ensure_eq(
        "owned stream id",
        read_u64(
            bytes,
            meta_offset + mem::offset_of!(StreamChunkHeaderV1, stream_id),
        )?,
        STREAM_ID,
    )?;
    ensure_eq(
        "owned stream checksum",
        read_u32(
            bytes,
            meta_offset + mem::offset_of!(StreamChunkHeaderV1, checksum),
        )?,
        fixture_checksum(case, sequence),
    )?;
    validate_owned_pattern_field(case, sequence, bytes, chunk_offset, chunk_len, false)
}

fn validate_owned_pattern_field(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
    offset: usize,
    len: usize,
    exhaustive: bool,
) -> Result<(), UWireError> {
    if exhaustive {
        for index in 0..len {
            ensure_eq(
                "owned pattern byte",
                read_u8(bytes, offset + index)?,
                pattern_byte(case.case_id, sequence, index),
            )?;
        }
    } else {
        for index in sample_indices(len) {
            ensure_eq(
                "owned pattern byte",
                read_u8(bytes, offset + index)?,
                pattern_byte(case.case_id, sequence, index),
            )?;
        }
    }
    Ok(())
}

fn validate_someip_fields(
    source_timestamp_ns: u64,
    length: u32,
    sample_count: u32,
    service_id: u16,
    method_or_event_id: u16,
    client_id: u16,
    session_id: u16,
    protocol_version: u8,
    interface_version: u8,
    message_type: u8,
    return_code: u8,
    checksum: u32,
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<(), UWireError> {
    ensure_eq(
        "SOME/IP timestamp",
        source_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    ensure_eq(
        "SOME/IP length",
        length,
        SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET as u32,
    )?;
    ensure_eq(
        "SOME/IP sample count",
        sample_count,
        SOMEIP_MTU_SIGNAL_SAMPLE_COUNT as u32,
    )?;
    ensure_eq("SOME/IP service_id", service_id, SOMEIP_SERVICE_ID)?;
    ensure_eq(
        "SOME/IP method_or_event_id",
        method_or_event_id,
        SOMEIP_METHOD_OR_EVENT_ID,
    )?;
    ensure_eq("SOME/IP client_id", client_id, SOMEIP_CLIENT_ID)?;
    ensure_eq("SOME/IP session_id", session_id, SOMEIP_SESSION_ID)?;
    ensure_eq(
        "SOME/IP protocol_version",
        protocol_version,
        SOMEIP_PROTOCOL_VERSION,
    )?;
    ensure_eq(
        "SOME/IP interface_version",
        interface_version,
        SOMEIP_INTERFACE_VERSION,
    )?;
    ensure_eq("SOME/IP message_type", message_type, SOMEIP_MESSAGE_TYPE)?;
    ensure_eq("SOME/IP return_code", return_code, SOMEIP_RETURN_CODE)?;
    ensure_eq(
        "SOME/IP checksum",
        checksum,
        fixture_checksum(case, sequence),
    )
}

fn validate_signal_sample(
    actual: &SignalSampleV1,
    sequence: u32,
    index: usize,
) -> Result<(), UWireError> {
    let expected = signal_sample_value(sequence, index);
    ensure_eq("SOME/IP signal_id", actual.signal_id, expected.signal_id)?;
    ensure_eq("SOME/IP status", actual.status, expected.status)?;
    ensure_eq(
        "SOME/IP timestamp",
        actual.timestamp_ns,
        expected.timestamp_ns,
    )?;
    ensure_eq("SOME/IP value", actual.value, expected.value)
}

fn validate_stream_meta(
    actual: &StreamChunkHeaderV1,
    case: &PayloadContractCase,
    sequence: u32,
    chunk_len: usize,
) -> Result<(), UWireError> {
    validate_header(&actual.header, case, sequence)?;
    validate_stream_fields(
        actual.stream_id,
        actual.codec,
        actual.flags,
        actual.byte_offset,
        actual.chunk_index,
        actual.chunk_count,
        actual.source_timestamp_ns,
        actual.checksum,
        case,
        sequence,
        chunk_len,
    )
}

fn validate_stream_fields(
    stream_id: u64,
    codec: u32,
    flags: u32,
    byte_offset: u64,
    chunk_index: u32,
    chunk_count: u32,
    source_timestamp_ns: u64,
    checksum: u32,
    case: &PayloadContractCase,
    sequence: u32,
    chunk_len: usize,
) -> Result<(), UWireError> {
    ensure_eq("stream id", stream_id, STREAM_ID)?;
    ensure_eq("stream codec", codec, STREAM_CODEC_RAW)?;
    ensure_eq("stream flags", flags, STREAM_FLAG_KEY_FRAME)?;
    ensure_eq(
        "stream byte offset",
        byte_offset,
        u64::from(sequence).saturating_mul(chunk_len as u64),
    )?;
    ensure_eq("stream chunk index", chunk_index, sequence)?;
    ensure_eq(
        "stream chunk count",
        chunk_count,
        sequence.saturating_add(7),
    )?;
    ensure_eq(
        "stream timestamp",
        source_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    ensure_eq(
        "stream checksum",
        checksum,
        fixture_checksum(case, sequence),
    )
}

fn validate_radar_detection(
    actual: &Ars548DetectionV1,
    sequence: u32,
    index: usize,
) -> Result<(), UWireError> {
    let expected = radar_detection_value(sequence, index);
    ensure_eq(
        "radar azimuth",
        actual.azimuth_angle,
        expected.azimuth_angle,
    )?;
    ensure_eq(
        "radar elevation",
        actual.elevation_angle,
        expected.elevation_angle,
    )?;
    ensure_eq("radar range", actual.range, expected.range)?;
    ensure_eq("radar range rate", actual.range_rate, expected.range_rate)?;
    ensure_eq(
        "radar measurement id",
        actual.measurement_id,
        expected.measurement_id,
    )?;
    ensure_eq("radar object id", actual.object_id, expected.object_id)?;
    ensure_eq("radar sort index", actual.sort_index, expected.sort_index)
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[allow(clippy::too_many_arguments)]
fn validate_lidar_fields(
    frame_counter: u64,
    sensor_timestamp_ns: u64,
    scan_duration_ns: u32,
    lidar_id: u32,
    width: u32,
    height: u32,
    point_count: u32,
    point_step: u32,
    row_step: u32,
    points_per_second: u32,
    horizontal_fov_mdeg: u32,
    vertical_fov_mdeg: u32,
    horizontal_resolution_mdeg: u32,
    vertical_resolution_mdeg: u32,
    range_10pct_reflectivity_m: u32,
    point_format: u32,
    is_bigendian: u8,
    is_dense: u8,
    checksum: u32,
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<(), UWireError> {
    ensure_eq("lidar frame_counter", frame_counter, u64::from(sequence))?;
    ensure_eq(
        "lidar timestamp",
        sensor_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    ensure_eq(
        "lidar scan_duration_ns",
        scan_duration_ns,
        LIDAR_HESAI_AT128_SCAN_DURATION_NS,
    )?;
    ensure_eq("lidar id", lidar_id, LIDAR_ID)?;
    ensure_eq("lidar width", width, LIDAR_HESAI_AT128_WIDTH)?;
    ensure_eq("lidar height", height, LIDAR_HESAI_AT128_HEIGHT)?;
    ensure_eq(
        "lidar point_count",
        point_count,
        LIDAR_HESAI_AT128_POINT_COUNT as u32,
    )?;
    ensure_eq(
        "lidar point_step",
        point_step,
        LIDAR_XYZIRCAEDT_POINT_BYTES as u32,
    )?;
    ensure_eq("lidar row_step", row_step, LIDAR_HESAI_AT128_ROW_STEP_BYTES)?;
    ensure_eq(
        "lidar points_per_second",
        points_per_second,
        LIDAR_HESAI_AT128_POINTS_PER_SECOND,
    )?;
    ensure_eq(
        "lidar horizontal_fov",
        horizontal_fov_mdeg,
        LIDAR_HESAI_AT128_HORIZONTAL_FOV_MDEG,
    )?;
    ensure_eq(
        "lidar vertical_fov",
        vertical_fov_mdeg,
        LIDAR_HESAI_AT128_VERTICAL_FOV_MDEG,
    )?;
    ensure_eq(
        "lidar horizontal_resolution",
        horizontal_resolution_mdeg,
        LIDAR_HESAI_AT128_HORIZONTAL_RESOLUTION_MDEG,
    )?;
    ensure_eq(
        "lidar vertical_resolution",
        vertical_resolution_mdeg,
        LIDAR_HESAI_AT128_VERTICAL_RESOLUTION_MDEG,
    )?;
    ensure_eq(
        "lidar range reference",
        range_10pct_reflectivity_m,
        LIDAR_HESAI_AT128_RANGE_10PCT_REFLECTIVITY_M,
    )?;
    ensure_eq(
        "lidar point_format",
        point_format,
        LIDAR_POINT_FORMAT_XYZIRCAEDT,
    )?;
    ensure_eq("lidar is_bigendian", is_bigendian, 0)?;
    ensure_eq("lidar is_dense", is_dense, 1)?;
    ensure_eq("lidar checksum", checksum, fixture_checksum(case, sequence))
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn validate_lidar_point(
    actual: &LidarPointXyzircaedtV1,
    sequence: u32,
    index: usize,
) -> Result<(), UWireError> {
    let expected = lidar_point_value(sequence, index);
    ensure_eq("lidar x", actual.x, expected.x)?;
    ensure_eq("lidar y", actual.y, expected.y)?;
    ensure_eq("lidar z", actual.z, expected.z)?;
    ensure_eq("lidar intensity", actual.intensity, expected.intensity)?;
    ensure_eq("lidar channel", actual.channel, expected.channel)?;
    ensure_eq("lidar distance", actual.distance, expected.distance)
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[allow(clippy::too_many_arguments)]
fn validate_camera_fields(
    frame_counter: u64,
    sensor_timestamp_ns: u64,
    exposure_start_ns: u64,
    camera_id: u32,
    width: u32,
    height: u32,
    stride_bytes: u32,
    bayer_pattern: u32,
    bits_per_sample: u32,
    packed_layout: u32,
    exposure_time_us: u32,
    analog_gain: f32,
    digital_gain: f32,
    checksum: u32,
    case: &PayloadContractCase,
    sequence: u32,
) -> Result<(), UWireError> {
    ensure_eq("camera frame_counter", frame_counter, u64::from(sequence))?;
    ensure_eq(
        "camera timestamp",
        sensor_timestamp_ns,
        timestamp_ns(sequence),
    )?;
    ensure_eq(
        "camera exposure_start",
        exposure_start_ns,
        timestamp_ns(sequence).saturating_sub(1_000_000),
    )?;
    ensure_eq("camera id", camera_id, CAMERA_ID)?;
    ensure_eq("camera width", width, CAMERA_8MP_WIDTH)?;
    ensure_eq("camera height", height, CAMERA_8MP_HEIGHT)?;
    ensure_eq(
        "camera stride",
        stride_bytes,
        CAMERA_BAYER_RGGB12P_STRIDE_BYTES,
    )?;
    ensure_eq("camera bayer pattern", bayer_pattern, BAYER_PATTERN_RGGB)?;
    ensure_eq("camera bits", bits_per_sample, BITS_PER_SAMPLE_12)?;
    ensure_eq("camera packed layout", packed_layout, PACKED_LAYOUT_12P_LSB)?;
    ensure_eq("camera exposure time", exposure_time_us, 8_000)?;
    ensure_eq("camera analog gain", analog_gain, 1.5)?;
    ensure_eq("camera digital gain", digital_gain, 1.0)?;
    ensure_eq(
        "camera checksum",
        checksum,
        fixture_checksum(case, sequence),
    )
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn validate_camera_intrinsics(actual: &CameraIntrinsicsV1) -> Result<(), UWireError> {
    ensure_eq("camera fx", actual.fx, 1_950.0)?;
    ensure_eq("camera fy", actual.fy, 1_950.0)?;
    ensure_eq("camera cx", actual.cx, 1_920.0)?;
    ensure_eq("camera cy", actual.cy, 1_080.0)?;
    ensure_eq("camera distortion model", actual.distortion_model, 1)
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn validate_pose(actual: &Pose3dV1, sequence: u32) -> Result<(), UWireError> {
    let expected = pose_value(sequence);
    ensure_eq("pose x", actual.translation_m.x, expected.translation_m.x)?;
    ensure_eq("pose y", actual.translation_m.y, expected.translation_m.y)?;
    ensure_eq("pose z", actual.translation_m.z, expected.translation_m.z)?;
    ensure_eq("pose w", actual.rotation.w, expected.rotation.w)
}

fn validate_all_pattern_bytes(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
) -> Result<(), UWireError> {
    for (index, byte) in bytes.iter().copied().enumerate() {
        ensure_eq(
            "pattern byte",
            byte,
            pattern_byte(case.case_id, sequence, index),
        )?;
    }
    Ok(())
}

fn validate_sample_pattern_bytes(
    case: &PayloadContractCase,
    sequence: u32,
    bytes: &[u8],
) -> Result<(), UWireError> {
    ensure_eq("pattern length", bytes.len(), case.semantic_reference_len)?;
    for index in sample_indices(bytes.len()) {
        let actual = bytes
            .get(index)
            .copied()
            .ok_or_else(|| UWireError::invalid_payload("missing sample byte"))?;
        ensure_eq(
            "pattern sample byte",
            actual,
            pattern_byte(case.case_id, sequence, index),
        )?;
    }
    Ok(())
}

fn signal_sample_value(sequence: u32, index: usize) -> SignalSampleV1 {
    SignalSampleV1 {
        signal_id: 0x5000 + index as u32,
        status: (index as u32) & 0x3,
        timestamp_ns: timestamp_ns(sequence).saturating_add(index as u64),
        value: f64::from(sequence) + index as f64 * 0.25,
    }
}

fn radar_detection_value(sequence: u32, index: usize) -> Ars548DetectionV1 {
    let base = sequence as f32 + index as f32;
    Ars548DetectionV1 {
        azimuth_angle: -0.6 + base * 0.0001,
        azimuth_angle_std: 0.01,
        elevation_angle: -0.2 + base * 0.00005,
        elevation_angle_std: 0.02,
        range: 2.0 + base * 0.1,
        range_std: 0.05,
        range_rate: -15.0 + index as f32 * 0.01,
        range_rate_std: 0.03,
        measurement_id: index as u16,
        object_id: (index % 256) as u16,
        sort_index: (RADAR_ARS548_MAX_DETECTIONS - index - 1) as u16,
        invalid_flags: (index & 0x3) as u8,
        rcs: ((index % 64) as i8) - 32,
        positive_predictive_value: (80 + index % 20) as u8,
        classification: (index % 8) as u8,
        multi_target_probability: (index % 100) as u8,
        ambiguity_flag: (index % 2) as u8,
    }
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn lidar_point_value(sequence: u32, index: usize) -> LidarPointXyzircaedtV1 {
    let column = (index as u32) % LIDAR_HESAI_AT128_WIDTH;
    let row = (index as u32) / LIDAR_HESAI_AT128_WIDTH;
    let azimuth = -60.0 + column as f32 * 0.1;
    let elevation = -12.7 + row as f32 * 0.2;
    let distance = 5.0 + (index % 2_000) as f32 * 0.05;
    LidarPointXyzircaedtV1 {
        x: distance * azimuth.to_radians().cos(),
        y: distance * azimuth.to_radians().sin(),
        z: distance * elevation.to_radians().sin(),
        intensity: pattern_byte(7, sequence, index),
        return_type: 1,
        channel: row as u16,
        azimuth,
        elevation,
        distance,
        time_offset_ns: ((index as u64 * u64::from(LIDAR_HESAI_AT128_SCAN_DURATION_NS))
            / LIDAR_HESAI_AT128_POINT_COUNT as u64) as u32,
    }
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn pose_value(sequence: u32) -> Pose3dV1 {
    Pose3dV1 {
        translation_m: Vector3fV1 {
            x: 1.0 + sequence as f32 * 0.01,
            y: 0.5,
            z: 1.2,
        },
        rotation: QuaternionfV1 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
    }
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn write_lidar_point_le(bytes: &mut [u8], sequence: u32, index: usize) {
    let point = lidar_point_value(sequence, index);
    let offset = index * LIDAR_XYZIRCAEDT_POINT_BYTES;
    bytes[offset..offset + 4].copy_from_slice(&point.x.to_le_bytes());
    bytes[offset + 4..offset + 8].copy_from_slice(&point.y.to_le_bytes());
    bytes[offset + 8..offset + 12].copy_from_slice(&point.z.to_le_bytes());
    bytes[offset + 12] = point.intensity;
    bytes[offset + 13] = point.return_type;
    bytes[offset + 14..offset + 16].copy_from_slice(&point.channel.to_le_bytes());
    bytes[offset + 16..offset + 20].copy_from_slice(&point.azimuth.to_le_bytes());
    bytes[offset + 20..offset + 24].copy_from_slice(&point.elevation.to_le_bytes());
    bytes[offset + 24..offset + 28].copy_from_slice(&point.distance.to_le_bytes());
    bytes[offset + 28..offset + 32].copy_from_slice(&point.time_offset_ns.to_le_bytes());
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn validate_lidar_point_le(bytes: &[u8], sequence: u32, index: usize) -> Result<(), UWireError> {
    let offset = index * LIDAR_XYZIRCAEDT_POINT_BYTES;
    let expected = lidar_point_value(sequence, index);
    ensure_eq("protobuf lidar x", read_f32_le(bytes, offset)?, expected.x)?;
    ensure_eq(
        "protobuf lidar intensity",
        read_u8(bytes, offset + 12)?,
        expected.intensity,
    )?;
    ensure_eq(
        "protobuf lidar channel",
        read_u16_le(bytes, offset + 14)?,
        expected.channel,
    )?;
    ensure_eq(
        "protobuf lidar distance",
        read_f32_le(bytes, offset + 24)?,
        expected.distance,
    )
}

fn sample_indices(len: usize) -> [usize; 3] {
    if len == 0 {
        [0, 0, 0]
    } else {
        [0, len / 2, len - 1]
    }
}

fn pattern_vec(case_id: u32, sequence: u32, len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| pattern_byte(case_id, sequence, index))
        .collect()
}

fn pattern_byte(case_id: u32, sequence: u32, index: usize) -> u8 {
    let value = (index as u32)
        .wrapping_mul(31)
        .wrapping_add(sequence.wrapping_mul(17))
        .wrapping_add(case_id.wrapping_mul(13));
    value as u8
}

fn fixture_checksum(case: &PayloadContractCase, sequence: u32) -> u32 {
    0xa5a5_0000
        ^ case.case_id.rotate_left(3)
        ^ sequence.rotate_left(11)
        ^ (case.semantic_reference_len as u32)
}

fn timestamp_ns(sequence: u32) -> u64 {
    1_700_000_000_000_000_000_u64 + u64::from(sequence) * 1_000_000
}

fn ensure(condition: &'static str, value: bool) -> Result<(), UWireError> {
    if value {
        Ok(())
    } else {
        Err(UWireError::invalid_payload(condition))
    }
}

fn ensure_eq<T>(name: &'static str, actual: T, expected: T) -> Result<(), UWireError>
where
    T: PartialEq + Debug,
{
    if actual == expected {
        Ok(())
    } else {
        Err(UWireError::invalid_payload(format!(
            "{name} mismatch: expected {expected:?}, got {actual:?}"
        )))
    }
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, UWireError> {
    bytes
        .get(offset)
        .copied()
        .ok_or_else(|| UWireError::invalid_payload("stable owned byte offset out of bounds"))
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, UWireError> {
    let mut value = [0_u8; 2];
    value.copy_from_slice(read_exact(bytes, offset, 2)?);
    Ok(u16::from_le_bytes(value))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, UWireError> {
    let mut value = [0_u8; 4];
    value.copy_from_slice(read_exact(bytes, offset, 4)?);
    Ok(u32::from_ne_bytes(value))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, UWireError> {
    let mut value = [0_u8; 8];
    value.copy_from_slice(read_exact(bytes, offset, 8)?);
    Ok(u64::from_ne_bytes(value))
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, UWireError> {
    let mut value = [0_u8; 4];
    value.copy_from_slice(read_exact(bytes, offset, 4)?);
    Ok(f32::from_ne_bytes(value))
}

#[cfg(feature = "payload-contract-large-fixtures")]
fn read_f32_le(bytes: &[u8], offset: usize) -> Result<f32, UWireError> {
    let mut value = [0_u8; 4];
    value.copy_from_slice(read_exact(bytes, offset, 4)?);
    Ok(f32::from_le_bytes(value))
}

fn read_exact(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8], UWireError> {
    bytes
        .get(offset..offset + len)
        .ok_or_else(|| UWireError::invalid_payload("stable owned byte range out of bounds"))
}
