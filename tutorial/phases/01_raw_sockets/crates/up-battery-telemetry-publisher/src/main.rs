// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use rand::Rng;
use up_rust::{UMessageBuilder, UPayloadFormat, UUri};

use up_frame_codec::serialize_for_unix_socket;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("--- Battery telemetry publisher starting ---");

    // 1. Build a source UUri for the battery telemetry publisher entity.
    //    Pattern: authority + numeric entity + version + numeric resource.
    let source_uri = UUri::try_from_parts("my_own_car", 0x1010, 1, 0x8001)?;

    // 2. Build and send 5 messages with random battery/temperature values.
    let mut rng = rand::rng();

    for i in 1..=5 {
        let battery_pct: f32 = rng.random_range(75.0..78.9);
        let temp_c: i8 = rng.random_range(20..=25);

        println!("Message {}: SoC = {:.1}%, Temp = {}°C", i, battery_pct, temp_c);

        let message = UMessageBuilder::publish(source_uri.clone())
            .with_ttl(5000)
            .build_with_payload(
                pack_bms_can_frame(battery_pct, temp_c).to_vec(),
                UPayloadFormat::UPAYLOAD_FORMAT_RAW,
            )?;

        let framed = serialize_for_unix_socket(&message)?;

        let socket_path = up_frame_codec::socket_path()?;
        let mut stream = UnixStream::connect(&socket_path).await?;
        stream.write_all(&framed).await?;
        stream.flush().await?;

        println!("   Sent {} bytes.\n", framed.len());
    }

    Ok(())
}

fn pack_bms_can_frame(battery_level_pct: f32, temperature_c: i8) -> [u8; 8] {
    let mut can_data = [0u8; 8];

    // Convert using a DBC scale of 0.5 (e.g. 75.0 / 0.5 = 150)
    let raw_soc = (battery_level_pct / 0.5) as u8;

    can_data[0] = raw_soc;
    can_data[1] = temperature_c as u8;

    can_data
}
