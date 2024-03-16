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
use std::any::Any;

/// `UListener` is the uP-L1 interface that provides a means to create listeners which are registered to `UTransport`
///
/// Implementations of `UListener` contain the details for what should occur when a message is received
/// For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
pub trait UListener: Any + Send + Sync {
    /// Performs some action on receipt
    ///
    /// # Arguments
    ///
    /// * `received` - Either the message or error `UStatus` received
    fn on_receive(&self, received: Result<UMessage, UStatus>);
}
pub trait CloneBoxUListener {
    fn clone_box(&self) -> Box<dyn ClonableBoxUListener + 'static>;
}

impl<T> CloneBoxUListener for T
where
    T: 'static + ClonableBoxUListener + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn ClonableBoxUListener + 'static> {
        Box::new(self.clone())
    }
}

impl CloneBoxUListener for Box<dyn ClonableBoxUListener> {
    fn clone_box(&self) -> Box<dyn ClonableBoxUListener + 'static> {
        self.as_ref().clone_box()
    }
}

pub trait ClonableBoxUListener: UListener + CloneBoxUListener {}

impl<T> ClonableBoxUListener for T where T: UListener + Clone + Send + Sync {}
