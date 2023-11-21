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
use crate::uri::serializer::{LongUriSerializer, UriSerializer};

impl Display for uproto_Uuri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uri = LongUriSerializer::serialize(self);
        write!(f, "{}", uri)
    }
}
