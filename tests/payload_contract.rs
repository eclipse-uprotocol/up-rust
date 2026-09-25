#![cfg(feature = "payload-contract-fixtures")]

//! Shared payload-contract fixture conformance tests.

/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::mem;
use std::sync::Arc;
use test_case::test_case;

use up_rust::bench_fixtures::payload_contract::{self as fixtures, *};
use up_rust::{
    NativeProfile, NativeProfileAgreement, NativeProfileMode, NativeProfileTable, PayloadEncoding,
    UFrameMetadata, UUri,
};

#[test]
fn payload_contract_manifest_is_canonical_and_hashable() {
    let manifest = fixtures::fixture_manifest();
    assert_eq!(manifest.suite_family, "payload-contract-representative");
    assert_eq!(manifest.fixture_version, PAYLOAD_CONTRACT_FIXTURE_VERSION);
    assert_eq!(manifest.cases.len(), fixtures::all_cases().count());

    let json = fixtures::fixture_manifest_json_canonical();
    assert!(json.contains("\"suite_family\":\"payload-contract-representative\""));
    assert_eq!(fixtures::fixture_manifest_sha256_hex().len(), 64);
}

#[test]
fn payload_contract_representative_constants_and_sizes_match_contract() {
    assert_eq!(SOMEIP_IPV4_UDP_SINGLE_MTU_BUDGET, 1_456);
    assert_eq!(SOMEIP_MTU_SIGNAL_SAMPLE_COUNT, 59);
    assert_eq!(mem::size_of::<SomeIpSignalBatchMtuV1>(), 1_472);
    assert_eq!(RADAR_ARS548_MAX_DETECTIONS, 800);
    assert_eq!(
        mem::size_of::<Ars548DetectionV1>(),
        RADAR_ARS548_DETECTION_RECORD_BYTES
    );
    assert_eq!(RADAR_ARS548_DETECTION_ARRAY_BYTES, 35_200);
    assert_eq!(RADAR_ARS548_DETECTION_MESSAGE_BYTES, 35_336);
    assert_eq!(RADAR_ARS548_MAX_TRACKED_OBJECTS, 50);

    #[cfg(feature = "payload-contract-large-fixtures")]
    {
        assert_eq!(
            mem::size_of::<LidarPointXyzircaedtV1>(),
            LIDAR_XYZIRCAEDT_POINT_BYTES
        );
        assert_eq!(LIDAR_HESAI_AT128_WIDTH, 1_200);
        assert_eq!(LIDAR_HESAI_AT128_HEIGHT, 128);
        assert_eq!(LIDAR_HESAI_AT128_POINT_COUNT, 153_600);
        assert_eq!(LIDAR_HESAI_AT128_POINTS_BYTES, 4_915_200);
        assert_eq!(LIDAR_HESAI_AT128_ROW_STEP_BYTES, 38_400);
        assert_eq!(mem::size_of::<LidarPointCloudHesaiAt128V1>(), 4_915_328);
        assert_eq!(CAMERA_BAYER_RGGB12P_BYTES, 12_441_600);
        assert_eq!(CAMERA_BAYER_RGGB12P_STRIDE_BYTES, 5_760);
        assert_eq!(BITS_PER_SAMPLE_12, 12);
        assert_eq!(CAMERA_CARLA_BGRA32_BYTES, 33_177_600);
        assert_eq!(mem::size_of::<CameraBayerRggb12pFrame8mpV1>(), 12_441_784);
    }
}

#[test_case(PayloadContractCaseKind::CanClassicMax; "CAN classic")]
#[test_case(PayloadContractCaseKind::CanFdMax; "CAN FD")]
#[test_case(PayloadContractCaseKind::SomeIpSingleMtu; "SOMEIP MTU")]
#[test_case(PayloadContractCaseKind::Streamer4k; "stream 4k")]
#[test_case(PayloadContractCaseKind::RadarArs548DetectionList; "radar")]
#[test_case(PayloadContractCaseKind::Streamer64k; "stream 64k")]
fn payload_contract_protobuf_fixtures_round_trip_and_validate(kind: PayloadContractCaseKind) {
    assert_protobuf_fixture(kind);
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[test_case(PayloadContractCaseKind::LidarHesaiAt128PointCloud; "lidar")]
#[test_case(PayloadContractCaseKind::Camera8mpBayerRggb12p; "camera")]
fn large_protobuf_fixtures_round_trip_and_validate(kind: PayloadContractCaseKind) {
    assert_protobuf_fixture(kind);
}

fn assert_protobuf_fixture(kind: PayloadContractCaseKind) {
    let case = fixtures::case_by_kind(kind);
    let bytes = fixtures::protobuf_encoded_bytes_for(case, 11).expect("protobuf encodes");
    assert_eq!(bytes.len(), fixtures::protobuf_encoded_len(case, 11));
    fixtures::validate_protobuf_bytes(case, 11, &bytes).unwrap();
}

#[test_case(PayloadContractCaseKind::CanClassicMax; "CAN classic")]
#[test_case(PayloadContractCaseKind::CanFdMax; "CAN FD")]
#[test_case(PayloadContractCaseKind::SomeIpSingleMtu; "SOMEIP MTU")]
#[test_case(PayloadContractCaseKind::Streamer4k; "stream 4k")]
#[test_case(PayloadContractCaseKind::RadarArs548DetectionList; "radar")]
#[test_case(PayloadContractCaseKind::Streamer64k; "stream 64k")]
fn payload_contract_stable_owned_fixtures_validate_representative_and_exact_bytes(
    kind: PayloadContractCaseKind,
) {
    assert_stable_fixture(kind);
}

#[cfg(feature = "payload-contract-large-fixtures")]
#[test_case(PayloadContractCaseKind::LidarHesaiAt128PointCloud; "lidar")]
#[test_case(PayloadContractCaseKind::Camera8mpBayerRggb12p; "camera")]
fn large_stable_owned_fixtures_validate(kind: PayloadContractCaseKind) {
    assert_stable_fixture(kind);
}

fn assert_stable_fixture(kind: PayloadContractCaseKind) {
    let case = fixtures::case_by_kind(kind);
    // Each test has an isolated one-type deployment, not a public allocation.
    let table = NativeProfileTable::new([(
        PayloadEncoding::from_id(0xF001).unwrap(),
        fixtures::stable_payload_representation(case).unwrap(),
    )])
    .unwrap();
    let profile = NativeProfile::new(
        format!("fixture-test.{}", case.name()),
        1,
        NativeProfileMode::Table(table),
    )
    .unwrap();
    let agreement = NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap();
    let fixture = fixtures::stable_owned_fixture_for(case, 17, &agreement).unwrap();
    let metadata =
        UFrameMetadata::publish(UUri::try_from_parts("fixture", 0x4210, 1, 0x9000).unwrap())
            .with_native_payload_identity(fixture.identity)
            .build()
            .unwrap();
    assert_eq!(
        fixture.stable_type_name,
        fixtures::stable_payload_type_name(case)
    );
    assert_eq!(
        fixture.stable_transport_len,
        fixtures::stable_payload_len(case)
    );
    assert_eq!(fixture.stable_align, fixtures::stable_payload_align(case));
    fixtures::validate_stable_owned_bytes(case, 17, &metadata, &agreement, &fixture.bytes)
        .unwrap_or_else(|_| panic!("{}", case.name()));
    fixtures::validate_stable_owned_bytes_exact(case, 17, &metadata, &agreement, &fixture.bytes)
        .unwrap_or_else(|_| panic!("{}", case.name()));
}
