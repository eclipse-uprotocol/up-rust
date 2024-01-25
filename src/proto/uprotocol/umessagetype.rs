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

use crate::uprotocol::uattributes::UMessageType;

impl From<UMessageType> for String {
    fn from(value: UMessageType) -> Self {
        value.to_string()
    }
}

impl From<&str> for UMessageType {
    fn from(value: &str) -> Self {
        match value {
            "pub.v1" => UMessageType::UMESSAGE_TYPE_PUBLISH,
            "req.v1" => UMessageType::UMESSAGE_TYPE_REQUEST,
            "res.v1" => UMessageType::UMESSAGE_TYPE_RESPONSE,
            _ => UMessageType::UMESSAGE_TYPE_UNSPECIFIED,
        }
    }
}

impl Display for UMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UMessageType::UMESSAGE_TYPE_PUBLISH => write!(f, "pub.v1"),
            UMessageType::UMESSAGE_TYPE_REQUEST => write!(f, "req.v1"),
            UMessageType::UMESSAGE_TYPE_RESPONSE => write!(f, "res.v1"),
            UMessageType::UMESSAGE_TYPE_UNSPECIFIED => write!(f, "unspec.v1"),
        }
    }
}
