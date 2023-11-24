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

use std::str::FromStr;
use uuid::Uuid;

use crate::uprotocol::Uuid as uproto_Uuid;
use crate::uuid::serializer::uuidserializer::UuidSerializer;

/// UUID Serializer interface used to serialize/deserialize UUIDs to/from a string
pub struct LongUuidSerializer;

impl UuidSerializer<String> for LongUuidSerializer {
    fn serialize(uuid: &uproto_Uuid) -> String {
        uuid.to_string()
    }

    fn deserialize(uuid: String) -> uproto_Uuid {
        match Uuid::from_str(&uuid) {
            Ok(uuid) => uuid.into(),
            Err(_err) => uproto_Uuid::default(),
        }
    }
}
