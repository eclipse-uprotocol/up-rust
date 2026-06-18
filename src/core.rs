/********************************************************************************
 * Copyright (c) 2024 Contributors to the Eclipse Foundation
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
/*!
Default client implementations for interacting with [uProtocol's Core entities](https://github.com/eclipse-uprotocol/up-spec/tree/v1.6.0-alpha.7/up-l3#2-core-uprotocol-uentities).
*/

#[cfg(feature = "udiscovery")]
pub mod udiscovery;
#[cfg(feature = "usubscription")]
pub mod usubscription;
