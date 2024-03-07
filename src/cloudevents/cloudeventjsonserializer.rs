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

use crate::cloudevents::{CloudEventError, CloudEventSerializer};

///  Serialize and deserialize `CloudEvents` to/from JSON format.
pub struct CloudEventJsonSerializer;
impl CloudEventSerializer for CloudEventJsonSerializer {
    fn serialize(&self, cloud_event: &CloudEvent) -> Result<Vec<u8>, CloudEventError> {
        serde_json::to_vec(cloud_event)
            .map_err(|error| CloudEventError::serialization_error(error.to_string()))
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<CloudEvent, CloudEventError> {
        serde_json::from_slice::<CloudEvent>(bytes)
            .map_err(|error| CloudEventError::serialization_error(error.to_string()))
    }
}
