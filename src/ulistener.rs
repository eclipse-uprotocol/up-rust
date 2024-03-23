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

use crate::{UMessage, UStatus};

/// `UListener` is the uP-L1 interface that provides a means to create listeners which are registered to `UTransport`
///
/// Implementations of `UListener` contain the details for what should occur when a message is received
/// For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
pub trait UListener: 'static + Send + Sync {
    /// Performs some action on receipt of a message
    ///
    /// # Parameters
    ///
    /// * `msg` - The message
    fn on_receive(&self, msg: UMessage);

    /// Performs some action on receipt of an error
    ///
    /// # Parameters
    ///
    /// * `err` - The error as `UStatus`
    fn on_error(&self, err: UStatus);
}
