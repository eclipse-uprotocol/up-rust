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
use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};


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


pub trait UListenerTypeTag {
    fn as_any(&self) -> &dyn Any;
}

pub struct ListenerWrapper<T: UListener + 'static> {
    listener: T,
}

impl<T: UListener + 'static> ListenerWrapper<T> {
    pub fn new(listener: T) -> Self {
        ListenerWrapper { listener }
    }
}

impl<T: UListener + 'static> UListener for ListenerWrapper<T> {
    fn on_receive(&self, received: Result<UMessage, UStatus>) {
        self.listener.on_receive(received)
    }
}

impl<T: UListener + 'static> UListenerTypeTag for ListenerWrapper<T> {
    fn as_any(&self) -> &dyn Any {
        &self.listener
    }
}

pub trait AnyListener: Send + Sync {
    fn on_receive(&self, received: Result<UMessage, UStatus>);
}

impl<T: UListener + 'static> AnyListener for ListenerWrapper<T> {
    fn on_receive(&self, received: Result<UMessage, UStatus>) {
        self.listener.on_receive(received);
    }
}

pub struct TypeIdentifiedListener {
    pub listener: Box<dyn AnyListener>,
    type_id: TypeId,
}

impl TypeIdentifiedListener {
    pub fn new<T>(listener: T) -> Self
        where
            T: UListener + 'static,
    {
        let any_listener = Box::new(ListenerWrapper::new(listener)) as Box<dyn AnyListener>;
        TypeIdentifiedListener {
            listener: any_listener,
            type_id: TypeId::of::<T>(),
        }
    }
}

impl PartialEq for TypeIdentifiedListener {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for TypeIdentifiedListener {}

impl Hash for TypeIdentifiedListener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}