// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use std::io::{Write, stdout};

use up_frame_codec::deserialize_for_unix_socket;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Bind under `{cwd}/tmp/` (create the directory if needed; clean stale socket).
    let socket_path = up_frame_codec::ensure_socket_dir()?;
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    println!(
        "Battery telemetry subscriber listening on: {}",
        socket_path.display()
    );

    loop {
        let (mut stream, _) = listener.accept().await?;

        // Spawn a dedicated task for each incoming socket connection.
        tokio::spawn(async move {
            // Step A: Read length prefix header (4 bytes).
            let mut len_bytes = [0u8; 4];
            if stream.read_exact(&mut len_bytes).await.is_err() {
                return;
            }
            let body_len = u32::from_be_bytes(len_bytes) as usize;

            // Step B: Read matching body bytes based on length header.
            let mut body_bytes = vec![0u8; body_len];
            if stream.read_exact(&mut body_bytes).await.is_err() {
                return;
            }

            // Step C: Reassemble the framed payload and decode via the codec.
            let mut framed = len_bytes.to_vec();
            framed.extend_from_slice(&body_bytes);

            match deserialize_for_unix_socket(&framed) {
                Ok(u_message) => {
                    if let Some(payload_data) = u_message.payload.as_ref() {
                        let extracted_bytes: Vec<u8> = payload_data.clone().into();
                        let (soc, temp) = unpack_bms_can_frame(&extracted_bytes);

                        let output = format!(
                            "[Battery telemetry subscriber] Processing incoming CAN telemetry...\n\
                             -> State of Charge: {:.1}%\n\
                             -> Cell Temp: {} °C",
                            soc, temp,
                        );
                        println!("{}", output);
                        let _ = stdout().flush();
                    }
                }
                Err(e) => eprintln!("Decode error: {:?}", e),
            }
        });
    }
}

fn unpack_bms_can_frame(can_data: &[u8]) -> (f32, i8) {
    if can_data.len() < 2 { return (0.0, 0); }

    // Unpack according to DBC rules
    let raw_soc = can_data[0];
    let battery_level_pct = raw_soc as f32 * 0.5;

    let temperature_c = can_data[1] as i8;

    (battery_level_pct, temperature_c)
}
