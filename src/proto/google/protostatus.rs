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

impl Name for ProtoStatus {
    const NAME: &'static str = "Status";
    const PACKAGE: &'static str = "google.rpc";
}
