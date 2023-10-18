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

#[derive(Debug)]
pub struct CloudEventSerializationError(pub String);

pub trait CloudEventSerializer {
    fn serialize(&self, cloud_event: &CloudEvent) -> Result<Vec<u8>, CloudEventSerializationError>;
    fn deserialize(&self, bytes: &[u8]) -> Result<CloudEvent, CloudEventSerializationError>;
}
