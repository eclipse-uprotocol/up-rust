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

use crate::uprotocol::{Remote, UUri};
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
    /// * `uri` - The serialized `UUri`.
    ///
    /// # Returns
    /// Returns a `UUri` object from the serialized format from the wire.
    fn deserialize(uri: T) -> UUri;

    /// Serializes a `UUri` into a specific serialization format.
    ///
    /// # Arguments
    /// * `uri` - The `UUri` object to be serialized into the format `T`.
    ///
    /// # Returns
    /// Returns the `UUri` in the transport serialized format.
    fn serialize(uri: &UUri) -> T;

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

        if let Some(authority) = long_uri.authority {
            if let Some(Remote::Name(name)) = authority.remote {
                auth.remote = Some(Remote::Name(name));
            }
        }
        if let Some(entity) = long_uri.entity {
            ue.name = entity.name;
        }
        if let Some(resource) = micro_uri.resource {
            ure.id = resource.id;
        }

        let uri = UUri {
            authority: Some(auth),
            entity: Some(ue),
            resource: Some(ure),
        };

        UriValidator::is_resolved(&uri).then_some(uri)
    }
}
