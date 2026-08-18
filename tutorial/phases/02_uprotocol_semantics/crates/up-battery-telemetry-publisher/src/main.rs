// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use std::sync::Arc;

// Phase-1 used ttl: Some(5000) via UMessageBuilder. Here we pass the same TTL
// through CallOptions, which SimplePublisher converts into UAttributes.ttl.
// See: https://docs.rs/up-rust/latest/up_rust/communication/struct.CallOptions.html

use rand::Rng;
use up_bms_proto::constants::*;
use up_bms_proto::BatteryTelemetry;
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};
use up_rust::{StaticUriProvider, UTransport};
use up_unix_domain_socket_transport::UnixDomainSocketTransport;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    println!("--- Battery telemetry publisher starting ---");

    let uri_provider = Arc::new(StaticUriProvider::new(
        AUTHORITY_NAME,
        PUBLISHER_UE_ID,
        PUBLISHER_UE_VERSION,
    ));
    let socket_path = up_frame_codec::socket_path()?;
    let transport: Arc<dyn UTransport> =
        UnixDomainSocketTransport::connect(&socket_path);
    log::trace!(
        "using UnixDomainSocketTransport::connect → {} (L1 UTransport)",
        socket_path.display()
    );
    let publisher = SimplePublisher::new(transport, uri_provider);

    let mut rng = rand::rng();

    for i in 1..=EXPECTED_MESSAGE_COUNT {
        let telemetry = BatteryTelemetry {
            soc_percent: rng.random_range(75.0..78.9),
            temp_celsius: rng.random_range(20..=25),
            ..Default::default()
        };

        println!(
            "Message {}: SoC = {:.1}%, Temp = {}°C",
            i, telemetry.soc_percent, telemetry.temp_celsius
        );

        // L2: UPayload wraps protobuf bytes + format for SimplePublisher.
        let payload = UPayload::try_from_protobuf(telemetry)?;
        log::trace!(
            "SimplePublisher::publish → UTransport::send (resource_id=0x{BATTERY_TELEMETRY_RESOURCE_ID:04x})"
        );
        publisher
            .publish(
                BATTERY_TELEMETRY_RESOURCE_ID,
                // `CallOptions::for_publish(ttl, priority, sink)`.
                // - ttl (ms): Some(5000) matches Phase 1's explicit TTL of 5 seconds.
                CallOptions::for_publish(Some(5000), None, None),
                Some(payload),
            )
            .await
            .map_err(|err| anyhow::anyhow!("publish failed: {err}"))?;

        println!();
    }

    Ok(())
}
