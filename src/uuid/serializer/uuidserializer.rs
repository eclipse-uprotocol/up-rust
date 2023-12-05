/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use crate::uprotocol::Uuid;
use crate::uuid::serializer::SerializationError;

/// A UUID serializer interface used to serialize/deserialize UUIDs.
///
/// This interface handles the serialization and deserialization of UUIDs into/from different formats.
/// It can serialize a UUID into a `String` representation (Long form) or a byte array (micro form).
///
/// # Type Parameters
///
/// * `T`: The data structure that the UUID will be serialized into.
///   This could be a `String` for the Long form or a `Vec<u8>` (byte array) for the micro form.
pub trait UuidSerializer<T> {
    /// Deserialize from the given format to a Uuid.
    ///
    /// # Arguments
    ///
    /// * `uuid`: The serialized Uuid in format `T`.
    ///
    /// # Returns
    ///
    /// The deserialized Uuid object as `Result<Uuid, SerializationError>`.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if the deserialization process fails. This can happen if the provided serialized data
    /// is not in the correct format, is corrupted, or if any other issues occur during the deserialization process.
    fn deserialize(uuid: T) -> Result<Uuid, SerializationError>;

    /// Serialize from a Uuid to the given format.
    ///
    /// # Arguments
    ///
    /// * `uuid`: The Uuid object to be serialized.
    ///
    /// # Returns
    ///
    /// The serialized Uuid in format `T` as `Result<T, SerializationError>`.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if the serialization process fails. This may occur if the Uuid object contains data
    /// that cannot be serialized into the desired format or if an error occurs during the serialization process.
    fn serialize(uuid: &Uuid) -> Result<T, SerializationError>;
}
