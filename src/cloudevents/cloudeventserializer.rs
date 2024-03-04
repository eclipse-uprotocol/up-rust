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

// cloudevents-sdk
use cloudevents::Event as CloudEvent;

use crate::cloudevents::CloudEventError;

/// A trait defining the functionality for serializing and deserializing `CloudEvents`.
///
/// This trait provides methods to serialize a `CloudEvent` into a byte vector and deserialize a byte vector back into a `CloudEvent`.
pub trait CloudEventSerializer {
    /// Serializes a given `CloudEvent` into a byte vector.
    ///
    /// # Arguments
    ///
    /// * `cloud_event` - A reference to the `CloudEvent` that needs to be serialized.
    ///
    /// # Returns
    ///
    /// Returns a serialized byte vector representation of the `CloudEvent`, or a `CloudEventError`.
    ///
    /// # Errors
    ///
    /// Returns a `CloudEventError` if the serialization process fails. This may occur due to issues like
    /// invalid event data, failure in encoding the event into bytes, or other serialization-specific errors.
    fn serialize(&self, cloud_event: &CloudEvent) -> Result<Vec<u8>, CloudEventError>;

    /// Deserializes a byte vector back into a `CloudEvent`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A byte slice representing the serialized form of a `CloudEvent`.
    ///
    /// # Returns
    ///
    /// Returns the deserialized `CloudEvent`, or a `CloudEventError`.
    ///
    /// # Errors
    ///
    /// Returns a `CloudEventError` if the deserialization process fails. This can happen if the byte data
    /// does not represent a valid `CloudEvent`, if necessary information is missing, if the data is corrupted,
    /// or if it fails to meet the expected format or schema.
    fn deserialize(&self, bytes: &[u8]) -> Result<CloudEvent, CloudEventError>;
}
