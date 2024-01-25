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

use crate::uprotocol::UUri;
use crate::uri::serializer::SerializationError;
use crate::uri::validator::UriValidator;

/// `UUri`s are used in transport layers and hence need to be serialized.
///
/// Each transport supports different serialization formats. For more information,
/// please refer to the [uProtocol URI specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/basics/uri.adoc).
///
/// # Type Parameters
/// * `T`: The data structure that the `UUri` will be serialized into.
///   For example, `String` or `Vec<u8>` (to represent byte arrays).
pub trait UriSerializer<T> {
    /// Deserialize from the format to a `UUri`.
    ///
    /// # Arguments
    /// * `uri` - The serialized `UUri` in format `T`.
    ///
    /// # Returns
    /// Returns a `Result<UUri, SerializationError>` representing the deserialized `UUri` object from the serialized format.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if the deserialization process fails. This can occur if the serialized input
    /// is not in a valid format, is corrupt, or if other issues arise during the deserialization process.
    fn deserialize(uri: T) -> Result<UUri, SerializationError>;

    /// Serializes a `UUri` into a specific serialization format.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` object to be serialized into the format `T`.
    ///
    /// # Returns
    /// Returns a `Result<T, SerializationError>` representing the serialized `UUri` in the specified format.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if the serialization process fails. This may be due to reasons such as incompatible data
    /// in the `UUri` that cannot be represented in the desired format, or errors that occur during the serialization process.
    fn serialize(uri: &UUri) -> Result<T, SerializationError>;

    /// Builds a fully resolved `UUri` from the serialized long format and the serialized micro format.
    ///
    /// # Arguments
    /// * `long_uri` - `UUri` serialized as a string.
    /// * `micro_uri` - `UUri` serialized as a byte slice.
    ///
    /// # Returns
    /// Returns an `Option<UUri>` object serialized from one of the forms. Returns `None` if the URI
    /// cannot be resolved.
    fn build_resolved(long_uri: &str, micro_uri: &[u8]) -> Option<UUri> {
        if long_uri.is_empty() && micro_uri.is_empty() {
            return Some(UUri {
                ..Default::default()
            });
        }

        let long_uri = UUri::from(long_uri);
        let micro_uri = UUri::from(micro_uri.to_vec());

        let mut auth = micro_uri.authority.unwrap_or_default();
        let mut ue = micro_uri.entity.unwrap_or_default();
        let mut ure = long_uri.resource.unwrap_or_default();

        if let Some(authority) = long_uri.authority.as_ref() {
            if let Some(name) = authority.get_name() {
                auth.name = Some(name.to_owned());
            }
        }
        if let Some(entity) = long_uri.entity.as_ref() {
            ue.name = entity.name.clone();
        }
        if let Some(resource) = micro_uri.resource.as_ref() {
            ure.id = resource.id;
        }

        let uri = UUri {
            authority: Some(auth).into(),
            entity: Some(ue).into(),
            resource: Some(ure).into(),
            ..Default::default()
        };

        UriValidator::is_resolved(&uri).then_some(uri)
    }
}
