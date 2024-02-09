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

use crate::uprotocol::uri::UUri as uproto_Uuri;
use crate::uprotocol::SerializationError;
use crate::uri::serializer::{LongUriSerializer, MicroUriSerializer, UriSerializer};

impl TryFrom<uproto_Uuri> for String {
    type Error = SerializationError;

    fn try_from(value: uproto_Uuri) -> Result<Self, Self::Error> {
        LongUriSerializer::serialize(&value)
    }
}

impl TryFrom<&str> for uproto_Uuri {
    type Error = SerializationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        LongUriSerializer::deserialize(value.into())
    }
}

impl TryFrom<uproto_Uuri> for Vec<u8> {
    type Error = SerializationError;

    fn try_from(value: uproto_Uuri) -> Result<Self, Self::Error> {
        MicroUriSerializer::serialize(&value)
    }
}

impl TryFrom<Vec<u8>> for uproto_Uuri {
    type Error = SerializationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        MicroUriSerializer::deserialize(value)
    }
}
