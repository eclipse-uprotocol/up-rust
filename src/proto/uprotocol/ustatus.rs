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

use crate::uprotocol::ustatus::{UCode, UStatus};

impl UStatus {
    pub fn ok() -> Self {
        UStatus {
            code: UCode::OK.into(),
            ..Default::default()
        }
    }

    pub fn fail(msg: &str) -> Self {
        UStatus {
            code: UCode::UNKNOWN.into(),
            message: Some(msg.to_string()),
            ..Default::default()
        }
    }

    pub fn fail_with_code(code: UCode, msg: &str) -> Self {
        UStatus {
            code: code.into(),
            message: Some(msg.to_string()),
            ..Default::default()
        }
    }

    pub fn is_failed(&self) -> bool {
        self.get_code() != UCode::OK
    }

    pub fn is_success(&self) -> bool {
        self.get_code() == UCode::OK
    }

    pub fn get_code(&self) -> UCode {
        self.code.enum_value_or_default()
    }
}
