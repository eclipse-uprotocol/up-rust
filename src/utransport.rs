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

use async_trait::async_trait;

use crate::ulistener::UListener;
use crate::{UMessage, UStatus, UUri};

/// `UTransport` is the uP-L1 interface that provides a common API for uE developers to send and receive messages.
///
/// Implementations of `UTransport` contain the details for connecting to the underlying transport technology and
/// sending `UMessage` using the configured technology. For more information, please refer to
/// [uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/blob/main/up-l1/README.adoc).
#[async_trait]
pub trait UTransport {
    /// Sends a message using this transport's message exchange mechanism.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send. The `type`, `source` and`sink` properties of the [`crate::UAttributes`] contained
    ///   in the message determine the addressing semantics:
    ///   * `source` - The origin of the message being sent. The address must be resolved. The semantics of the address
    ///     depends on the value of the given [attributes' type](crate::UAttributes::type_) property .
    ///     * For a [`PUBLISH`](crate::UMessageType::UMESSAGE_TYPE_PUBLISH) message, this is the topic that the message should be published to,
    ///     * for a [`REQUEST`](crate::UMessageType::UMESSAGE_TYPE_REQUEST) message, this is the *reply-to* address that the sender expects to receive the response at, and
    ///     * for a [`RESPONSE`](crate::UMessageType::UMESSAGE_TYPE_RESPONSE) message, this identifies the method that has been invoked.
    ///   * `sink` - For a `notification`, an RPC `request` or RPC `response` message, the (resolved) address that the message
    ///     should be sent to.
    ///
    /// # Errors
    ///
    /// Returns an error if the message could not be sent.
    async fn send(&self, message: UMessage) -> Result<(), UStatus>;

    /// Receives a message from the transport.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to receive the message from.
    ///
    /// # Errors
    ///
    /// Returns an error if no message could be received. Possible reasons are that the topic does not exist
    /// or that no message is available from the topic.
    async fn receive(&self, topic: UUri) -> Result<UMessage, UStatus>;

    /// Registers a listener to be called for each message that is received on a given address.
    ///
    /// # Arguments
    ///
    /// * `address` - The (resolved) address to register the listener for.
    /// * `listener` - The listener to invoke.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener could not be registered.
    async fn register_listener(
        &self,
        topic: UUri,
        listener: Box<dyn UListener>,
    ) -> Result<(), UStatus>;

    /// Unregisters a listener for a given topic.
    ///
    /// Messages arriving on this topic will no longer be processed by this listener.
    ///
    /// # Arguments
    ///
    /// * `topic` - Resolved topic uri where the listener was registered originally.
    /// * `listener` - Identifier of the listener that should be unregistered.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener could not be unregistered, for example if the given listener does not exist.
    async fn unregister_listener(
        &self,
        topic: UUri,
        listener: Box<dyn UListener>,
    ) -> Result<(), UStatus>;
}

#[cfg(test)]
mod tests {
    use crate::ulistener::UListener;
    use crate::{Number, UAuthority, UCode, UMessage, UStatus, UTransport, UUri};
    use async_std::task;
    use async_trait::async_trait;
    use std::any::Any;
    use std::collections::hash_map::Entry;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    struct UPClientFoo {
        #[allow(clippy::type_complexity)]
        listeners: Arc<Mutex<HashMap<UUri, HashSet<Box<dyn UListener>>>>>,
    }

    impl UPClientFoo {
        pub fn new() -> Self {
            Self {
                listeners: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        #[allow(clippy::type_complexity)]
        pub fn get_hashmap(&self) -> Arc<Mutex<HashMap<UUri, HashSet<Box<dyn UListener>>>>> {
            self.listeners.clone()
        }

        pub fn check_on_receive(&self, uuri: &UUri, umessage: &UMessage) -> Result<(), UStatus> {
            let mut topics_listeners = self.listeners.lock().unwrap();
            let listeners = topics_listeners.entry(uuri.clone());
            match listeners {
                Entry::Vacant(_) => {
                    return Err(UStatus::fail_with_code(
                        UCode::NOT_FOUND,
                        format!("No listeners registered for topic: {:?}", &uuri),
                    ))
                }
                Entry::Occupied(mut e) => {
                    let occupied = e.get_mut();
                    for listener in occupied.iter() {
                        listener.on_receive(Ok(umessage.clone()));
                    }
                }
            }

            Ok(())
        }
    }

    #[async_trait]
    impl UTransport for UPClientFoo {
        async fn send(&self, _message: UMessage) -> Result<(), UStatus> {
            todo!()
        }

        async fn receive(&self, _topic: UUri) -> Result<UMessage, UStatus> {
            todo!()
        }

        async fn register_listener(
            &self,
            topic: UUri,
            listener: Box<dyn UListener>,
        ) -> Result<(), UStatus> {
            let mut topics_listeners = self.listeners.lock().unwrap();
            let listeners = topics_listeners.entry(topic).or_default();
            listeners.insert(listener);

            Ok(())
        }

        async fn unregister_listener(
            &self,
            topic: UUri,
            listener: Box<dyn UListener>,
        ) -> Result<(), UStatus> {
            let mut topics_listeners = self.listeners.lock().unwrap();
            let listeners = topics_listeners.entry(topic.clone());
            match listeners {
                Entry::Vacant(_) => {
                    return Err(UStatus::fail_with_code(
                        UCode::NOT_FOUND,
                        format!("No listeners registered for topic: {:?}", &topic),
                    ))
                }
                Entry::Occupied(mut e) => {
                    let occupied = e.get_mut();
                    occupied.remove(&listener);
                }
            }

            Ok(())
        }
    }

    #[derive(Debug)]
    struct ListenerBaz;
    impl UListener for ListenerBaz {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn on_receive(&self, received: Result<UMessage, UStatus>) {
            println!("Printing from ListenerBaz! received: {:?}", received);
        }
    }

    #[derive(Debug)]
    struct ListenerBar;
    impl UListener for ListenerBar {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn on_receive(&self, received: Result<UMessage, UStatus>) {
            println!("Printing from ListenerBar! received: {:?}", received);
        }
    }

    fn uuri_factory(uuri_index: u8) -> UUri {
        match uuri_index {
            1 => UUri {
                authority: Some(UAuthority {
                    name: Some("uuri_1".to_string()),
                    number: Some(Number::Ip(vec![192, 168, 1, 200])),
                    ..Default::default()
                })
                .into(),
                ..Default::default()
            },
            _ => UUri::default(),
        }
    }

    #[test]
    fn test_register_and_receive() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);
        let listener_baz: Box<dyn UListener> = Box::new(ListenerBaz);
        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_baz));
        let listeners_after_ops = up_client_foo.get_hashmap();
        println!("{listeners_after_ops:#?}");
        assert_eq!(register_res, Ok(()));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert_eq!(check_on_receive_res, Ok(()));
    }

    #[test]
    fn test_register_and_unregister() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);
        let listener_baz_for_register: Box<dyn UListener> = Box::new(ListenerBaz);
        let register_res = task::block_on(
            up_client_foo.register_listener(uuri_1.clone(), listener_baz_for_register),
        );

        let listeners_after_register = up_client_foo.get_hashmap();
        println!("{listeners_after_register:#?}");
        assert_eq!(register_res, Ok(()));

        let listener_baz_for_unregister: Box<dyn UListener> = Box::new(ListenerBaz);
        let unregister_res = task::block_on(
            up_client_foo.unregister_listener(uuri_1.clone(), listener_baz_for_unregister),
        );
        let listeners_after_unregister = up_client_foo.get_hashmap();
        println!("{listeners_after_unregister:#?}");
        assert_eq!(unregister_res, Ok(()));
    }

    #[test]
    fn test_register_multiple_listeners_on_one_uuri() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);
        let listener_baz: Box<dyn UListener> = Box::new(ListenerBaz);
        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_baz));
        let listeners_after_ops = up_client_foo.get_hashmap();
        println!("{listeners_after_ops:#?}");
        assert_eq!(register_res, Ok(()));

        let listener_bar: Box<dyn UListener> = Box::new(ListenerBar);
        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_bar));
        let listeners_after_ops = up_client_foo.get_hashmap();
        println!("{listeners_after_ops:#?}");
        assert_eq!(register_res, Ok(()));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert_eq!(check_on_receive_res, Ok(()));
    }

    #[test]
    fn test_if_no_listeners() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);

        assert!(check_on_receive_res.is_err());
    }
}
