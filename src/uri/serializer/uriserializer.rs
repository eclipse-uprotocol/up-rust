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

pub type UriSerializationError = ();

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

    fn build_resolved(long_uri: &str, micro_uri: &[u8]) -> Option<UUri> {
        if long_uri.is_empty() && micro_uri.is_empty() {
            return Some(UUri {
                ..Default::default()
            });
        }

        todo!()
    }
}

// default Optional<UUri> buildResolved(String longUri, byte[] microUri) {

//     if ((longUri == null || longUri.isEmpty()) && (microUri == null || microUri.length == 0)) {
//         return Optional.of(UUri.getDefaultInstance());
//     }

//     UUri longUUri = LongUriSerializer.instance().deserialize(longUri);
//     UUri microUUri = MicroUriSerializer.instance().deserialize(microUri);

//     final UAuthority.Builder uAuthorityBuilder =
//         UAuthority.newBuilder(microUUri.getAuthority())
//             .setName(longUUri.getAuthority().getName());

//     final UEntity.Builder uEntityBuilder = UEntity.newBuilder(microUUri.getEntity())
//         .setName(longUUri.getEntity().getName());

//     final UResource.Builder uResourceBuilder = UResource.newBuilder(longUUri.getResource())
//         .setId(microUUri.getResource().getId());

//     UUri uUri = UUri.newBuilder()
//         .setAuthority(uAuthorityBuilder)
//         .setEntity(uEntityBuilder)
//         .setResource(uResourceBuilder)
//         .build();
//     return UriValidator.isResolved(uUri) ? Optional.of(uUri) : Optional.empty();
// }
