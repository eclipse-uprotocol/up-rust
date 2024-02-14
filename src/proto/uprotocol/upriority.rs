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

use protobuf::EnumFull;

use crate::uprotocol::UPriority;

impl UPriority {
    pub fn to_priority_code(&self) -> String {
        let desc = self.descriptor();
        let desc_proto = desc.proto();
        crate::uprotocol::uprotocol_options::exts::ce_name
            .get(desc_proto.options.get_or_default())
            .unwrap_or_default()
    }
}
