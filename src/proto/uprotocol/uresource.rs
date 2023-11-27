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

impl UResource {
    pub fn has_id(resource: &UResource) -> bool {
        resource.id.is_some()
    }

    pub fn get_message(resource: &UResource) -> Option<&String> {
        match &resource.message {
            Some(message) => Some(message),
            _ => None,
        }
    }

    pub fn get_instance(resource: &UResource) -> Option<&String> {
        match &resource.instance {
            Some(instance) => Some(instance),
            _ => None,
        }
    }
}

impl From<String> for UResource {
    fn from(value: String) -> Self {
        let parts: Vec<&str> = value.split('#').collect();
        let name_and_instance: String = parts[0].to_string();
        let name_and_instance_parts: Vec<&str> = name_and_instance.split('.').collect();

        let resource_name: String = name_and_instance_parts[0].to_string();
        let resource_instance: Option<String> = name_and_instance_parts
            .get(1)
            .map_or_else(|| None, |s| Some(s.to_string()));

        let resource_message: Option<String> =
            parts.get(1).map_or_else(|| None, |s| Some(s.to_string()));

        UResource {
            name: resource_name,
            id: None,
            instance: resource_instance,
            message: resource_message,
        }
    }
}
