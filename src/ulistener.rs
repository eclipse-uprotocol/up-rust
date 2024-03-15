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
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// `UListener` is the uP-L1 interface that provides a means to create listeners which are registered to `UTransport`
///
/// Implementations of `UListener` contain the details for what should occur when a message is received
/// For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
pub trait UListener: Debug + Any + Send + Sync {
    /// This function is necessary to disambiguate concrete implementations of UListener
    /// Arises from limitations surrounding Rust's type system
    ///
    /// Each concrete implementation will implement this function in the same way:
    /// ```
    /// use std::any::Any;
    /// use up_rust::ulistener::UListener;
    /// use up_rust::{UMessage, UStatus};
    ///
    /// #[derive(Debug)]
    /// struct ListenerFoo;
    /// impl UListener for ListenerFoo {
    ///     fn as_any(&self) -> &dyn Any {
    ///         self
    ///     }
    ///
    ///     fn on_receive(&self, received: Result<UMessage, UStatus>) {
    ///         todo!()
    ///     }
    ///
    /// }
    /// ```
    fn as_any(&self) -> &dyn Any;

    /// Performs some action on receipt
    ///
    /// # Arguments
    ///
    /// * `received` - Either the message or error `UStatus` received
    fn on_receive(&self, received: Result<UMessage, UStatus>);
}

impl Hash for dyn UListener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_any().type_id().hash(state);
    }
}

impl PartialEq for dyn UListener {
    fn eq(&self, other: &Self) -> bool {
        Any::type_id(self) == Any::type_id(other)
    }
}

impl Eq for dyn UListener {}
