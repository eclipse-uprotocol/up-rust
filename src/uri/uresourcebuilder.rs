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

use crate::UResource;

const MAX_RPC_ID: u32 = 1000;

pub struct UResourceBuilder {}

impl UResourceBuilder {
    /// Builds a `UResource` for an RPC response.
    ///
    /// # Returns
    /// Returns a `UResource` for an RPC response.
    pub fn for_rpc_response() -> UResource {
        UResource {
            name: String::from("rpc"),
            instance: Some(String::from("response")),
            id: Some(0),
            message: None,
            ..Default::default()
        }
    }

    /// Builds a `UResource` for an RPC request with an ID and method name.
    ///
    /// # Arguments
    /// * `method` - The method to be invoked. Pass `None` if no method is specified.
    /// * `id` - The ID of the request. Pass `None` if no ID is specified.
    ///
    /// # Returns
    /// Returns a `UResource` for an RPC request.
    pub fn for_rpc_request(method: Option<String>, id: Option<u32>) -> UResource {
        UResource {
            name: String::from("rpc"),
            instance: method,
            id,
            message: None,
            ..Default::default()
        }
    }

    /// Builds a `UResource` from an ID.
    ///
    /// This method determines the type of `UResource` to create based on the ID value.
    /// If the ID is less than `MAX_RPC_ID`, it is considered an ID for an RPC request
    /// and a corresponding `UResource` for an RPC request is created. Otherwise, a
    /// generic `UResource` is created with the provided ID and default values for other fields.
    ///
    /// # Arguments
    /// * `id` - The ID used to determine the type of `UResource` to create.
    ///
    /// # Returns
    /// Returns a `UResource` instance corresponding to the given ID.
    pub fn from_id(id: u32) -> UResource {
        if id < MAX_RPC_ID {
            return Self::for_rpc_request(None, Some(id));
        }

        UResource {
            name: String::new(),
            instance: None,
            id: Some(id),
            message: None,
            ..Default::default()
        }
    }
}
