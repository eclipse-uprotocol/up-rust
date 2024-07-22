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

#[cfg(feature = "usubscription")]
pub mod usubscription;

// Types not used by up_rust, but re-exported to up_rust users, keeping them in their respective submodules
#[cfg(feature = "udiscovery")]
pub mod udiscovery;
#[cfg(feature = "utwin")]
pub mod utwin;
