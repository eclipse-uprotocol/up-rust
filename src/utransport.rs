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
    async fn register_listener<T>(&self, topic: UUri, listener: T) -> Result<(), UStatus>
    where
        T: UListener + 'static;

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
    async fn unregister_listener<T>(&self, topic: UUri, listener: T) -> Result<(), UStatus>
    where
        T: UListener + 'static;
}

#[cfg(test)]
mod tests {
    use crate::listener_wrapper::ListenerWrapper;
    use crate::ulistener::UListener;
    use crate::{Number, UAuthority, UCode, UMessage, UStatus, UTransport, UUri};
    use async_std::task;
    use async_trait::async_trait;
    use std::collections::hash_map::Entry;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    struct UPClientFoo {
        #[allow(clippy::type_complexity)]
        listeners: Arc<Mutex<HashMap<UUri, HashSet<ListenerWrapper>>>>,
    }

    impl UPClientFoo {
        pub fn new() -> Self {
            Self {
                listeners: Arc::new(Mutex::new(HashMap::new())),
            }
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

                    if occupied.is_empty() {
                        return Err(UStatus::fail_with_code(
                            UCode::NOT_FOUND,
                            format!("No listeners registered for topic: {:?}", &uuri),
                        ));
                    }

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

        async fn register_listener<T>(&self, topic: UUri, listener: T) -> Result<(), UStatus>
        where
            T: UListener + 'static,
        {
            let mut topics_listeners = self.listeners.lock().unwrap();
            let listeners = topics_listeners.entry(topic).or_default();
            let identified_listener = ListenerWrapper::new(listener);
            listeners.insert(identified_listener);

            Err(UStatus::fail_with_code(
                UCode::OK,
                format!("{}", listeners.len()),
            ))
        }

        async fn unregister_listener<T>(&self, topic: UUri, listener: T) -> Result<(), UStatus>
        where
            T: UListener + 'static,
        {
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
                    let identified_listener = ListenerWrapper::new(listener);
                    occupied.remove(&identified_listener);
                    return Err(UStatus::fail_with_code(
                        UCode::OK,
                        format!("{}", occupied.len()),
                    ));
                }
            }
        }
    }

    #[derive(Debug)]
    struct ListenerBaz;
    impl UListener for ListenerBaz {
        fn on_receive(&self, received: Result<UMessage, UStatus>) {
            println!("Printing from ListenerBaz! received: {:?}", received);
        }
    }

    #[derive(Debug)]
    struct ListenerBar;
    impl UListener for ListenerBar {
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
        let listener_baz = ListenerBaz;
        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_baz));
        assert_eq!(register_res, Err(UStatus::fail_with_code(UCode::OK, "1")));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert_eq!(check_on_receive_res, Ok(()));
    }

    #[test]
    fn test_register_and_unregister() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);
        let listener_baz_for_register = ListenerBaz;
        let register_res = task::block_on(
            up_client_foo.register_listener(uuri_1.clone(), listener_baz_for_register),
        );
        assert_eq!(register_res, Err(UStatus::fail_with_code(UCode::OK, "1")));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert_eq!(check_on_receive_res, Ok(()));

        let listener_baz_for_unregister = ListenerBaz;
        let unregister_res = task::block_on(
            up_client_foo.unregister_listener(uuri_1.clone(), listener_baz_for_unregister),
        );
        assert_eq!(unregister_res, Err(UStatus::fail_with_code(UCode::OK, "0")));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert!(check_on_receive_res.is_err());
    }

    #[test]
    fn test_register_multiple_listeners_on_one_uuri() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);
        let listener_baz = ListenerBaz;
        let listener_bar = ListenerBar;

        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_baz));
        assert_eq!(register_res, Err(UStatus::fail_with_code(UCode::OK, "1")));

        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_bar));
        assert_eq!(register_res, Err(UStatus::fail_with_code(UCode::OK, "2")));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert_eq!(check_on_receive_res, Ok(()));

        let listener_baz_for_unregister = ListenerBaz;
        let listener_bar_for_unregister = ListenerBar;
        let unregister_baz_res = task::block_on(
            up_client_foo.unregister_listener(uuri_1.clone(), listener_baz_for_unregister),
        );
        assert_eq!(
            unregister_baz_res,
            Err(UStatus::fail_with_code(UCode::OK, "1"))
        );
        let unregister_bar_res = task::block_on(
            up_client_foo.unregister_listener(uuri_1.clone(), listener_bar_for_unregister),
        );
        assert_eq!(
            unregister_bar_res,
            Err(UStatus::fail_with_code(UCode::OK, "0"))
        );

        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert!(check_on_receive_res.is_err());
    }

    #[test]
    fn test_if_no_listeners() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);

        assert!(check_on_receive_res.is_err());
    }

    #[test]
    fn test_register_multiple_same_listeners_on_one_uuri() {
        let up_client_foo = UPClientFoo::new();
        let uuri_1 = uuri_factory(1);
        let listener_baz_1 = ListenerBaz;
        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_baz_1));
        assert_eq!(register_res, Err(UStatus::fail_with_code(UCode::OK, "1")));

        let listener_baz_2 = ListenerBaz;
        let register_res =
            task::block_on(up_client_foo.register_listener(uuri_1.clone(), listener_baz_2));
        assert_eq!(register_res, Err(UStatus::fail_with_code(UCode::OK, "1")));

        let umessage = UMessage::default();
        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert_eq!(check_on_receive_res, Ok(()));

        let listener_baz_for_unregister = ListenerBaz;
        let unregister_baz_res = task::block_on(
            up_client_foo.unregister_listener(uuri_1.clone(), listener_baz_for_unregister),
        );
        assert_eq!(
            unregister_baz_res,
            Err(UStatus::fail_with_code(UCode::OK, "0"))
        );

        let check_on_receive_res = up_client_foo.check_on_receive(&uuri_1, &umessage);
        assert!(check_on_receive_res.is_err());
    }
}
