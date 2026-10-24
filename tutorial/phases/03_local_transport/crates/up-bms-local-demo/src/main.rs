// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use async_trait::async_trait;
use rand::Rng;
use up_bms_proto::BatteryTelemetry;
use up_bms_proto::constants::*;
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};
use up_rust::local_transport::LocalTransport;
use up_rust::{LocalUriProvider, StaticUriProvider, UListener, UMessage, UTransport};

struct BatteryTelemetryListener {
    telemetry_seen: AtomicU32,
}

#[async_trait]
impl UListener for BatteryTelemetryListener {
    async fn on_receive(&self, msg: UMessage) {
        match msg.extract_protobuf::<BatteryTelemetry>() {
            Ok(telemetry) => {
                self.telemetry_seen.fetch_add(1, Ordering::SeqCst);
                println!(
                    "[Battery telemetry listener] Processing incoming telemetry...\n\
                     -> State of Charge: {:.1}%\n\
                     -> Cell Temp: {} °C",
                    telemetry.soc_percent, telemetry.temp_celsius,
                );
            }
            Err(err) => eprintln!("Failed to decode BatteryTelemetry payload: {err}"),
        }
    }
}

struct ThermalLoggingListener {
    readings_seen: AtomicU32,
}

#[async_trait]
impl UListener for ThermalLoggingListener {
    async fn on_receive(&self, msg: UMessage) {
        match msg.extract_protobuf::<BatteryTelemetry>() {
            Ok(telemetry) => {
                self.readings_seen.fetch_add(1, Ordering::SeqCst);
                let temp = telemetry.temp_celsius;
                if temp > 25 {
                    println!(
                        "[Thermal logging listener] WARNING — cell temperature {temp}°C exceeds 25°C threshold"
                    );
                } else {
                    println!("[Thermal logging listener] Cell temperature {temp}°C — OK");
                }
            }
            Err(err) => eprintln!("Failed to decode BatteryTelemetry payload: {err}"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Phase 3: two UListeners and SimplePublisher on one in-process LocalTransport.
    let uri_provider = Arc::new(StaticUriProvider::new(
        AUTHORITY_NAME,
        PUBLISHER_UE_ID,
        PUBLISHER_UE_VERSION,
    )?);
    let source_filter = uri_provider.get_resource_uri(BATTERY_TELEMETRY_RESOURCE_ID);

    // One LocalTransport instance: in-process dispatch on the sending thread.
    let transport = Arc::new(LocalTransport::default());
    // AtomicU32: UListener::on_receive takes &self (and the trait is Sync).
    let battery_listener = Arc::new(BatteryTelemetryListener {
        telemetry_seen: AtomicU32::new(0),
    });
    let thermal_listener = Arc::new(ThermalLoggingListener {
        readings_seen: AtomicU32::new(0),
    });

    // register_listener takes Arc by value; clone so we still own a handle for unregister.
    transport
        .register_listener(&source_filter, None, battery_listener.clone())
        .await?;
    transport
        .register_listener(&source_filter, None, thermal_listener.clone())
        .await?;

    let publisher = SimplePublisher::new(transport.clone(), uri_provider);
    let mut rng = rand::rng();

    println!("--- Battery telemetry (in-process LocalTransport) ---");
    for i in 1..=EXPECTED_MESSAGE_COUNT {
        let telemetry = BatteryTelemetry {
            soc_percent: rng.random_range(75.0..78.9),
            temp_celsius: rng.random_range(20..=28),
            ..Default::default()
        };
        println!(
            "Message {}: SoC = {:.1}%, Temp = {}°C",
            i, telemetry.soc_percent, telemetry.temp_celsius
        );
        let payload = UPayload::try_from_protobuf(telemetry)?;
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

    println!(
        "Battery listener received {}; thermal listener received {}.",
        battery_listener.telemetry_seen.load(Ordering::SeqCst),
        thermal_listener.readings_seen.load(Ordering::SeqCst)
    );

    transport
        .unregister_listener(&source_filter, None, battery_listener)
        .await?;
    transport
        .unregister_listener(&source_filter, None, thermal_listener)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn both_listeners_receive_the_same_publish() {
        let uri_provider = Arc::new(
            StaticUriProvider::new(AUTHORITY_NAME, PUBLISHER_UE_ID, PUBLISHER_UE_VERSION).unwrap(),
        );
        let source_filter = uri_provider.get_resource_uri(BATTERY_TELEMETRY_RESOURCE_ID);
        let transport = Arc::new(LocalTransport::default());
        let battery_listener = Arc::new(BatteryTelemetryListener {
            telemetry_seen: AtomicU32::new(0),
        });
        let thermal_listener = Arc::new(ThermalLoggingListener {
            readings_seen: AtomicU32::new(0),
        });
        transport
            .register_listener(&source_filter, None, battery_listener.clone())
            .await
            .unwrap();
        transport
            .register_listener(&source_filter, None, thermal_listener.clone())
            .await
            .unwrap();
        let publisher = SimplePublisher::new(transport, uri_provider);
        let telemetry = BatteryTelemetry {
            soc_percent: 76.0,
            temp_celsius: 27,
            ..Default::default()
        };
        publisher
            .publish(
                BATTERY_TELEMETRY_RESOURCE_ID,
                CallOptions::for_publish(Some(5000), None, None),
                Some(UPayload::try_from_protobuf(telemetry).unwrap()),
            )
            .await
            .unwrap();
        assert_eq!(battery_listener.telemetry_seen.load(Ordering::SeqCst), 1);
        assert_eq!(thermal_listener.readings_seen.load(Ordering::SeqCst), 1);
    }
}
