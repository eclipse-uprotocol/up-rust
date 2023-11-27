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

use prost::Message;
use prost_types::Any;
use std::slice;

use crate::uprotocol::{Data, UPayload};

impl From<Any> for UPayload {
    fn from(value: Any) -> Self {
        let vec = value.encode_to_vec();
        let len = vec.len() as i32;
        UPayload {
            data: Some(Data::Value(vec)),
            length: Some(len),
            ..Default::default()
        }
    }
}

impl From<UPayload> for Any {
    fn from(value: UPayload) -> Self {
        if let Some(bytes) = data_to_slice(&value) {
            if let Ok(any) = Any::decode(bytes) {
                return any;
            }
        }
        Any::default()
    }
}

fn data_to_slice(payload: &UPayload) -> Option<&[u8]> {
    if let Some(data) = &payload.data {
        match data {
            Data::Reference(bytes) => {
                if let Some(length) = payload.length {
                    return Some(read_memory(*bytes, length));
                }
            }
            Data::Value(bytes) => {
                return Some(bytes.as_slice());
            }
        }
    }
    None
}

// Please no one use this...
fn read_memory(address: u64, length: i32) -> &'static [u8] {
    unsafe {
        // Convert the raw address to a pointer
        let ptr = address as *const u8;

        // Create a slice from the pointer and the length
        slice::from_raw_parts(ptr, length as usize)
    }
}
