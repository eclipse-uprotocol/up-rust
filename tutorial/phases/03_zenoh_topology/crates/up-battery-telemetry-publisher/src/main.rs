// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use std::sync::Arc;

use rand::Rng;
use up_bms_proto::constants::*;
use up_bms_proto::BatteryTelemetry;
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};
use up_rust::{StaticUriProvider, UTransport};
use up_transport_zenoh::{zenoh_config, UPTransportZenoh};

#[tokio::main]
#[allow(unreachable_code, unused_variables)]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    println!("--- Battery telemetry publisher starting ---");

    let uri_provider = Arc::new(StaticUriProvider::new(
        AUTHORITY_NAME,
        PUBLISHER_UE_ID,
        PUBLISHER_UE_VERSION,
    ));

    // Phase 3 — Zenoh-backed UTransport via up-transport-zenoh.
    // Replaces Phase 2's UnixDomainSocketTransport::connect.
    //
    // Config::default() opens a Zenoh *peer* with UDP multicast scouting.
    // Peers can discover each other without a zenohd router (peer-to-peer).
    // An optional router is fine too; for a remote endpoint you can set:
    //   config.connect.endpoints = vec!["tcp/<host>:7447".parse()?];
    let transport: Arc<dyn UTransport> =
        Arc::new(
            UPTransportZenoh::builder(AUTHORITY_NAME)
                .map_err(|e| anyhow::anyhow!("builder failed: {e}"))?
                .with_config(zenoh_config::Config::default())
                .build()
                .await
                .map_err(|e| anyhow::anyhow!("Zenoh transport build failed: {e}"))?,
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

        let payload = UPayload::try_from_protobuf(telemetry)?;
        log::trace!(
            "SimplePublisher::publish → UTransport::send (resource_id=0x{BATTERY_TELEMETRY_RESOURCE_ID:04x})"
        );
        publisher
            .publish(
                BATTERY_TELEMETRY_RESOURCE_ID,
                CallOptions::for_publish(Some(5000), None, None),
                Some(payload),
            )
            .await
            .map_err(|err| anyhow::anyhow!("publish failed: {err}"))?;

        println!();
    }

    Ok(())
}
