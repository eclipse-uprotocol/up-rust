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

use crate::uprotocol::UMessageType;

impl From<UMessageType> for String {
    fn from(value: UMessageType) -> Self {
        match value {
            UMessageType::UmessageTypePublish => "pub.v1".to_string(),
            UMessageType::UmessageTypeRequest => "req.v1".to_string(),
            UMessageType::UmessageTypeResponse => "res.v1".to_string(),
            UMessageType::UmessageTypeUnspecified => "unspec.v1".to_string(),
        }
    }
}

impl From<String> for UMessageType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pub.v1" => UMessageType::UmessageTypePublish,
            "req.v1" => UMessageType::UmessageTypeRequest,
            "res.v1" => UMessageType::UmessageTypeResponse,
            _ => UMessageType::UmessageTypeUnspecified,
        }
    }
}
