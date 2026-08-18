// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use async_trait::async_trait;
use tokio::sync::Notify;
use up_bms_proto::constants::*;
use up_bms_proto::BatteryTelemetry;
use up_rust::{LocalUriProvider, StaticUriProvider, UListener, UMessage, UTransport};
use up_transport_zenoh::{zenoh_config, UPTransportZenoh};

struct BatteryTelemetryListener {
    received: Arc<AtomicU32>,
    shutdown: Arc<Notify>,
}

#[async_trait]
impl UListener for BatteryTelemetryListener {
    async fn on_receive(&self, msg: UMessage) {
        match msg.extract_protobuf::<BatteryTelemetry>() {
            Ok(telemetry) => {
                let count = self.received.fetch_add(1, Ordering::SeqCst) + 1;
                log::trace!(
                    "UListener::on_receive via transport dispatch (message {count}/{EXPECTED_MESSAGE_COUNT})"
                );
                println!(
                    "[Battery telemetry subscriber] Processing incoming telemetry...\n\
                     -> State of Charge: {:.1}%\n\
                     -> Cell Temp: {} °C",
                    telemetry.soc_percent, telemetry.temp_celsius,
                );

                if count >= EXPECTED_MESSAGE_COUNT {
                    self.shutdown.notify_one();
                }
            }
            Err(err) => eprintln!("Failed to decode BatteryTelemetry payload: {err}"),
        }
    }
}

#[tokio::main]
#[allow(unreachable_code, unused_variables)]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let uri_provider = StaticUriProvider::new(
        AUTHORITY_NAME,
        PUBLISHER_UE_ID,
        PUBLISHER_UE_VERSION,
    );
    let source_filter = uri_provider.get_resource_uri(BATTERY_TELEMETRY_RESOURCE_ID);

    let received = Arc::new(AtomicU32::new(0));
    let shutdown = Arc::new(Notify::new());
    let listener = Arc::new(BatteryTelemetryListener {
        received,
        shutdown: shutdown.clone(),
    });

    // Phase 3 — Zenoh-backed UTransport via up-transport-zenoh.
    // Replaces Phase 2's UnixDomainSocketTransport::bind.
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

    transport
        .register_listener(&source_filter, None, listener)
        .await?;
    log::trace!(
        "registered UListener with source filter resource_id=0x{BATTERY_TELEMETRY_RESOURCE_ID:04x}"
    );

    println!(
        "Battery telemetry subscriber listening (expecting {} messages)",
        EXPECTED_MESSAGE_COUNT
    );

    shutdown.notified().await;
    println!("Received {EXPECTED_MESSAGE_COUNT} messages — exiting.");

    Ok(())
}
