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

pub trait UListener {
    fn on_receive(&self, message: Result<UMessage, UStatus>);
}

pub trait NotificationUListener: UListener + Send + Sync {}

pub trait PublishUListener: UListener + Send + Sync {}

pub trait RequestUListener: UListener + Send + Sync {}

pub trait ResponseUListener: UListener + Send + Sync {}

pub trait GenericUListener: UListener + Send + Sync {
    fn is_match(&self, message: UMessage) -> bool;
}

pub enum ListenerType {
    Notification(Box<dyn NotificationUListener>),
    Publish(Box<dyn PublishUListener>),
    Request(Box<dyn RequestUListener>),
    Response(Box<dyn ResponseUListener>),
    /// May be left unimplemented if wished to do so and only use protocol-level filtering
    Generic(Box<dyn GenericUListener>),
}

#[cfg(test)]
mod tests {
    use crate::ulistener::{
        GenericUListener, ListenerType, NotificationUListener, PublishUListener, RequestUListener,
        ResponseUListener, UListener,
    };
    use crate::{UCode, UMessage, UPriority, UStatus, UTransport, UUri};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct UpClientFoo {
        listeners: Arc<Mutex<Vec<Box<dyn GenericUListener>>>>,
    }

    #[async_trait]
    impl UTransport for UpClientFoo {
        async fn send(&self, _message: UMessage) -> Result<(), UStatus> {
            todo!()
        }

        async fn receive(&self, _topic: UUri) -> Result<UMessage, UStatus> {
            todo!()
        }

        async fn register_listener(
            &self,
            _topic: UUri,
            listener: ListenerType,
        ) -> Result<String, UStatus> {
            match listener {
                ListenerType::Notification(_notification_listener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Notifications
                    todo!()
                }
                ListenerType::Publish(_publish_listener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Publish
                    todo!()
                }
                ListenerType::Request(_request_listener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Request
                    todo!()
                }
                ListenerType::Response(_response_listener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Response
                    todo!()
                }
                ListenerType::Generic(generic_listener) => {
                    // assume that we have another thread / async task that's being pinged on each
                    // message received and that we skim thru listeners in order to check if each
                    // one is a match
                    //
                    // and if a match is found, we call that genericListener from the Vec's on_receive()
                    let mut listeners = self.listeners.lock().map_err(|_| {
                        UStatus::fail_with_code(UCode::INTERNAL, "Failed to get lock on listeners")
                    })?;

                    listeners.push(generic_listener);
                    Ok("here's your handle".to_string())
                }
            }
        }

        async fn unregister_listener(&self, _topic: UUri, _listener: &str) -> Result<(), UStatus> {
            todo!()
        }
    }

    struct MyClientFooNotificationListener;

    impl UListener for MyClientFooNotificationListener {
        fn on_receive(&self, _message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl NotificationUListener for MyClientFooNotificationListener {}

    struct MyClientFooPublishListener;

    impl UListener for MyClientFooPublishListener {
        fn on_receive(&self, _message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl PublishUListener for MyClientFooPublishListener {}

    struct MyClientFooRequestListener;

    impl UListener for MyClientFooRequestListener {
        fn on_receive(&self, _message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl RequestUListener for MyClientFooRequestListener {}

    struct MyClientFooResponseListener;

    impl UListener for MyClientFooResponseListener {
        fn on_receive(&self, _message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl ResponseUListener for MyClientFooResponseListener {}

    struct MyClientFooPriorityMatcherListener {
        priority_match: UPriority,
    }

    impl UListener for MyClientFooPriorityMatcherListener {
        fn on_receive(&self, _message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl GenericUListener for MyClientFooPriorityMatcherListener {
        fn is_match(&self, message: UMessage) -> bool {
            if let Some(attributes) = message.attributes.as_ref() {
                match attributes.priority.enum_value() {
                    Ok(priority) => {
                        if self.priority_match == priority {
                            return true;
                        }
                    }
                    Err(_e) => {}
                }
            }
            false
        }
    }

    struct UpClientBar {}

    #[async_trait]
    impl UTransport for UpClientBar {
        async fn send(&self, _message: UMessage) -> Result<(), UStatus> {
            todo!()
        }

        async fn receive(&self, _topic: UUri) -> Result<UMessage, UStatus> {
            todo!()
        }

        async fn register_listener(
            &self,
            _topic: UUri,
            listener: ListenerType,
        ) -> Result<String, UStatus> {
            match listener {
                ListenerType::Notification(_notification_listener) => {
                    // implement Bar-specific means of allowing protocol to perform filtering for Notifications
                    todo!()
                }
                ListenerType::Publish(_publish_listener) => {
                    // implement Bar-specific means of allowing protocol to perform filtering for Publish
                    todo!()
                }
                ListenerType::Request(_request_listener) => {
                    // implement Bar-specific means of allowing protocol to perform filtering for Request
                    todo!()
                }
                ListenerType::Response(_response_listener) => {
                    // implement Bar-specific means of allowing protocol to perform filtering for Response
                    todo!()
                }
                ListenerType::Generic(_generic_listener) => {
                    // for UpClientBar we only handle this at the protocol level, with no generic
                    // mechanism to handle incoming messages
                    return Err(UStatus::fail_with_code(
                        UCode::UNIMPLEMENTED,
                        "No generic matching capabilities.",
                    ));
                }
            }
        }

        async fn unregister_listener(&self, _topic: UUri, _listener: &str) -> Result<(), UStatus> {
            todo!()
        }
    }
}
