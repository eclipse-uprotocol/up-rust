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

use crate::uprotocol::uri::UAuthority;

/// Helper functions to deal with `UAuthority::Remote` structure
impl UAuthority {
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn get_ip(&self) -> Option<&[u8]> {
        self.ip.as_deref()
    }

    pub fn has_ip(&self) -> bool {
        self.ip.is_some()
    }

    pub fn get_id(&self) -> Option<&[u8]> {
        self.id.as_deref()
    }

    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }
}
