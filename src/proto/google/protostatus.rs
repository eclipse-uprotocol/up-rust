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

use crate::proto::Status as ProtoStatus;
use crate::uprotocol::UStatus;

impl Name for ProtoStatus {
    const NAME: &'static str = "Status";
    const PACKAGE: &'static str = "google.rpc";
}

impl From<UStatus> for ProtoStatus {
    fn from(status: UStatus) -> Self {
        ProtoStatus {
            code: status.code,
            message: status.message.unwrap_or_default(),
            details: status.details,
        }
    }
}

impl From<ProtoStatus> for UStatus {
    fn from(status: ProtoStatus) -> Self {
        UStatus {
            code: status.code,
            message: Some(status.message),
            details: status.details,
        }
    }
}
