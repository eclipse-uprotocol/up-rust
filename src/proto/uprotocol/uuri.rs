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

use std::fmt::Display;

use crate::uprotocol::UUri as uproto_Uuri;
use crate::uri::serializer::{LongUriSerializer, MicroUriSerializer, UriSerializer};

impl From<uproto_Uuri> for String {
    fn from(value: uproto_Uuri) -> Self {
        LongUriSerializer::serialize(&value)
    }
}

impl From<&str> for uproto_Uuri {
    fn from(value: &str) -> Self {
        LongUriSerializer::deserialize(value.into())
    }
}

impl From<uproto_Uuri> for Vec<u8> {
    fn from(value: uproto_Uuri) -> Self {
        MicroUriSerializer::serialize(&value)
    }
}

impl From<Vec<u8>> for uproto_Uuri {
    fn from(value: Vec<u8>) -> Self {
        MicroUriSerializer::deserialize(value)
    }
}

impl Display for uproto_Uuri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uri = LongUriSerializer::serialize(self);
        write!(f, "{}", uri)
    }
}
