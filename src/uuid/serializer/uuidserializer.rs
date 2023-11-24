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

pub type UuidSerializationError = ();

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
    /// * `uuid`: The serialized Uuid.
    ///
    /// # Returns
    ///
    /// The deserialized Uuid object.
    fn deserialize(uuid: T) -> Uuid;

    /// Serialize from a Uuid to the given format.
    ///
    /// # Arguments
    ///
    /// * `uuid`: The Uuid object to be serialized.
    ///
    /// # Returns
    ///
    /// The serialized Uuid.
    fn serialize(uuid: &Uuid) -> T;
}
