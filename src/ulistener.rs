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

use crate::{UMessage, UStatus, UUri};

pub trait UListener {
    fn on_receive(&self, message: Result<UMessage, UStatus>);
}

pub trait NotificationUListener: UListener {}

pub trait PublishUListener: UListener {}

pub trait RequestUListener: UListener {}

pub trait ResponseUListener: UListener {}

pub trait GenericUListener: UListener {}

pub enum ListenerType {
    Notification(Box<dyn NotificationUListener>),
    Publish(Box<dyn PublishUListener>),
    Request(Box<dyn RequestUListener>),
    Response(Box<dyn ResponseUListener>),
    Generic(UUri, Box<dyn GenericUListener>),
}

#[cfg(test)]
mod tests {
    use crate::ulistener::{
        ListenerType, NotificationUListener, PublishUListener, RequestUListener, ResponseUListener,
        UListener,
    };
    use crate::{UMessage, UStatus, UTransport, UUri};
    use async_trait::async_trait;

    struct UpClientFoo;

    #[async_trait]
    impl UTransport for UpClientFoo {
        async fn send(&self, message: UMessage) -> Result<(), UStatus> {
            todo!()
        }

        async fn receive(&self, topic: UUri) -> Result<UMessage, UStatus> {
            todo!()
        }

        async fn register_listener(
            &self,
            topic: UUri,
            listener: ListenerType,
        ) -> Result<String, UStatus> {
            match listener {
                ListenerType::Notification(notificationListener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Notifications
                    todo!()
                }
                ListenerType::Publish(publishListener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Publish
                    todo!()
                }
                ListenerType::Request(requestListener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Request
                    todo!()
                }
                ListenerType::Response(responseListener) => {
                    // implement Foo-specific means of allowing protocol to perform filtering for Response
                    todo!()
                }
                ListenerType::Generic(uuri, genericListener) => {
                    todo!()
                }
            }
        }

        async fn unregister_listener(&self, topic: UUri, listener: &str) -> Result<(), UStatus> {
            todo!()
        }
    }

    struct MyClientFooNotificationListener;

    impl UListener for MyClientFooNotificationListener {
        fn on_receive(&self, message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl NotificationUListener for MyClientFooNotificationListener {}

    struct MyClientFooPublishListener;

    impl UListener for MyClientFooPublishListener {
        fn on_receive(&self, message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl PublishUListener for MyClientFooPublishListener {}

    struct MyClientFooRequestListener;

    impl UListener for MyClientFooRequestListener {
        fn on_receive(&self, message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl RequestUListener for MyClientFooRequestListener {}

    struct MyClientFooResponseListener;

    impl UListener for MyClientFooResponseListener {
        fn on_receive(&self, message: Result<UMessage, UStatus>) {
            todo!()
        }
    }

    impl ResponseUListener for MyClientFooResponseListener {}
}
