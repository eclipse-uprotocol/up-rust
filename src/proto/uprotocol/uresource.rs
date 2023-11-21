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

use crate::uprotocol::UResource;

impl From<String> for UResource {
    fn from(value: String) -> Self {
        let parts: Vec<&str> = value.split('#').collect();
        let name_and_instance: String = parts[0].to_string();
        let name_and_instance_parts: Vec<&str> = name_and_instance.split('.').collect();

        let resource_name: String = name_and_instance_parts[0].to_string();
        let resource_instance: String = name_and_instance_parts
            .get(1)
            .map_or_else(|| "".to_string(), |s| s.to_string());

        let resource_message: String = parts
            .get(1)
            .map_or_else(|| "".to_string(), |s| s.to_string());

        UResource {
            name: resource_name,
            id: None,
            instance: Some(resource_instance),
            message: Some(resource_message),
        }
    }
}
