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
use uuid::Uuid;

use crate::uprotocol::Uuid as uproto_Uuid;

impl From<uproto_Uuid> for Uuid {
    fn from(value: uproto_Uuid) -> Self {
        Uuid::from_u64_pair(value.msb, value.lsb)
    }
}

impl From<Uuid> for uproto_Uuid {
    fn from(value: Uuid) -> Self {
        uproto_Uuid {
            msb: value.as_u64_pair().0,
            lsb: value.as_u64_pair().1,
        }
    }
}

impl Display for uproto_Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Uuid::from(self.clone()))
    }
}
