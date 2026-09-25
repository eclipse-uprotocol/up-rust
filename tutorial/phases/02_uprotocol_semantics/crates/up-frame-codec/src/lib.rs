// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

use std::path::PathBuf;

use protobuf::Message;
use up_rust::UMessage;

/// Socket file name under `{current_working_dir}/tmp/`.
pub const SOCKET_FILE_NAME: &str = "uprotocol_twin.sock";

/// Returns `{current_working_dir}/tmp/uprotocol_twin.sock`.
///
/// Both publisher and subscriber must be started with the same working directory
/// so they share this path. Does not create directories.
pub fn socket_path() -> Result<PathBuf, anyhow::Error> {
    let mut path = std::env::current_dir()?;
    path.push("tmp");
    path.push(SOCKET_FILE_NAME);
    Ok(path)
}

/// Ensures `{current_working_dir}/tmp` exists, then returns [`socket_path`].
pub fn ensure_socket_dir() -> Result<PathBuf, anyhow::Error> {
    let path = socket_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

/// Encodes a UMessage into a Unix-socket compatible framed byte buffer.
///
/// The frame consists of a 4-byte Big-Endian length prefix followed by the
/// protobuf-encoded UMessage payload.
pub fn serialize_for_unix_socket(msg: &UMessage) -> Result<Vec<u8>, anyhow::Error> {
    let envelope_bytes = msg.write_to_bytes()?;

    let msg_len = envelope_bytes.len() as u32;
    let mut framed_buffer = msg_len.to_be_bytes().to_vec();
    framed_buffer.append(&mut envelope_bytes.to_vec()); // Length is prefixed

    Ok(framed_buffer)
}

/// Decodes a framed byte buffer produced by [`serialize_for_unix_socket`].
///
/// Expects the same layout: 4-byte Big-Endian length prefix, then protobuf body.
pub fn deserialize_for_unix_socket(framed: &[u8]) -> Result<UMessage, anyhow::Error> {
    if framed.len() < 4 {
        anyhow::bail!("framed buffer too short for length prefix");
    }

    let body_len = u32::from_be_bytes(framed[0..4].try_into()?) as usize;
    let end = 4 + body_len;
    if framed.len() < end {
        anyhow::bail!("framed buffer too short for declared body length");
    }

    Ok(UMessage::parse_from_bytes(&framed[4..end])?)
}
