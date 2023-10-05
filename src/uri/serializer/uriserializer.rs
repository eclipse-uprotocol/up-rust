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

use crate::uri::datamodel::uuri::UUri;

pub type UriSerializationError = ();

/// UUri serializer that will serialize a UUri object to either a `String` or `Vec<u8>`.
///
/// For more information, please refer to the [uprotocol specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc)
///
/// - `T`: The serialization format
pub trait UriSerializer<T> {
    /// Deserialize from the given format to a UUri.
    ///
    /// # Arguments
    ///
    /// * `uri`: The serialized UUri.
    ///
    /// # Returns
    ///
    /// The deserialized UUri object.
    fn deserialize(uri: T) -> UUri;

    /// Serialize from a UUri to the given format.
    ///
    /// # Arguments
    ///
    /// * `uri`: The UUri object to be serialized.
    ///
    /// # Returns
    ///
    /// The serialized UUri.
    fn serialize(uri: &UUri) -> T;
}
