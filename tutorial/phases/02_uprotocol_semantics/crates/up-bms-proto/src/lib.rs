// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

//! Shared BMS telemetry protobuf types and Phase 2 demo constants.
//!
//! The `.proto` schema lives at `proto/bms_telemetry.proto`; Rust types are generated
//! at build time by `build.rs` (see Phase 2 tutorial).

pub mod constants {
    /// Authority for all Phase 2 demo URIs (aligned with Phase 1).
    pub const AUTHORITY_NAME: &str = "my_own_car";
    pub const PUBLISHER_UE_ID: u32 = 0x1010;
    pub const PUBLISHER_UE_VERSION: u8 = 0x01;
    pub const BATTERY_TELEMETRY_RESOURCE_ID: u16 = 0x8001;
    pub const EXPECTED_MESSAGE_COUNT: u32 = 5;
}

mod generated {
    include!(concat!(env!("OUT_DIR"), "/gen/mod.rs"));
}

pub use generated::bms_telemetry::BatteryTelemetry;
