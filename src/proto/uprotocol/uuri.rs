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

use crate::uprotocol::uri::UUri;
use crate::uprotocol::SerializationError;
use crate::uri::serializer::{LongUriSerializer, MicroUriSerializer, UriSerializer};

impl TryFrom<UUri> for String {
    type Error = SerializationError;

    fn try_from(value: UUri) -> Result<Self, Self::Error> {
        LongUriSerializer::serialize(&value)
    }
}

impl TryFrom<&str> for UUri {
    type Error = SerializationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        LongUriSerializer::deserialize(value.into())
    }
}

impl TryFrom<UUri> for Vec<u8> {
    type Error = SerializationError;

    fn try_from(value: UUri) -> Result<Self, Self::Error> {
        MicroUriSerializer::serialize(&value)
    }
}

impl TryFrom<Vec<u8>> for UUri {
    type Error = SerializationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        MicroUriSerializer::deserialize(value)
    }
}
