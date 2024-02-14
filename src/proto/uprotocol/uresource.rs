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

use crate::uprotocol::uri::UResource;

impl UResource {
    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }

    pub fn get_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn get_instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }
}

impl From<&str> for UResource {
    fn from(value: &str) -> Self {
        let mut parts = value.split('#');
        let name_and_instance = parts.next().unwrap_or_default();
        let resource_message = parts.next().map(std::string::ToString::to_string);

        let mut name_and_instance_parts = name_and_instance.split('.');
        let resource_name = name_and_instance_parts
            .next()
            .unwrap_or_default()
            .to_string();
        let resource_instance = name_and_instance_parts
            .next()
            .map(std::string::ToString::to_string);

        let mut resource_id: Option<u32> = None;
        if resource_name.contains("rpc")
            && resource_instance
                .as_ref()
                .is_some_and(|i| i.contains("response"))
        {
            resource_id = Some(0);
        }

        UResource {
            name: resource_name,
            id: resource_id,
            instance: resource_instance,
            message: resource_message,
            ..Default::default()
        }
    }
}

impl From<String> for UResource {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
