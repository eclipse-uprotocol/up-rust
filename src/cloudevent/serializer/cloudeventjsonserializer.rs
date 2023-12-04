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

use cloudevents::Event as CloudEvent;

use crate::cloudevent::serializer::{CloudEventSerializationError, CloudEventSerializer};

///  Serialize and deserialize `CloudEvents` to/from JSON format.
pub struct CloudEventJsonSerializer;
impl CloudEventSerializer for CloudEventJsonSerializer {
    fn serialize(&self, cloud_event: &CloudEvent) -> Result<Vec<u8>, CloudEventSerializationError> {
        match serde_json::to_vec(cloud_event) {
            Ok(bytes) => Ok(bytes),
            Err(error) => Err(CloudEventSerializationError(error.to_string())),
        }
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<CloudEvent, CloudEventSerializationError> {
        match serde_json::from_slice::<CloudEvent>(bytes) {
            Ok(event) => Ok(event),
            Err(error) => Err(CloudEventSerializationError(error.to_string())),
        }
    }
}
