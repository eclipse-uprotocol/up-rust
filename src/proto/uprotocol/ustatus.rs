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

use prost::Name;

use crate::uprotocol::{UCode, UStatus};

impl Name for UStatus {
    const NAME: &'static str = "UStatus";
    const PACKAGE: &'static str = "uprotocol.v1";
}

impl UStatus {
    pub fn ok() -> Self {
        UStatus {
            code: UCode::Ok.into(),
            ..Default::default()
        }
    }

    pub fn fail(msg: &str) -> Self {
        UStatus {
            code: UCode::Unknown.into(),
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
        self.code != UCode::Ok as i32
    }

    pub fn is_success(&self) -> bool {
        self.code == UCode::Ok as i32
    }

    pub fn get_code(&self) -> UCode {
        if let Ok(code) = UCode::try_from(self.code) {
            code
        } else {
            UCode::Unknown
        }
    }
}
