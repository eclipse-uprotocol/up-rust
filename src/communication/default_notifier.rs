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

use std::sync::Arc;

use async_trait::async_trait;

use crate::{LocalUriProvider, UListener, UMessageBuilder, UTransport, UUri};

use super::{
    apply_common_options, build_message, CallOptions, NotificationError, Notifier,
    RegistrationError, UPayload,
};

pub struct SimpleNotifier {
    transport: Arc<dyn UTransport>,
    uri_provider: Arc<dyn LocalUriProvider>,
}

impl SimpleNotifier {
    pub fn new(transport: Arc<dyn UTransport>, uri_provider: Arc<dyn LocalUriProvider>) -> Self {
        SimpleNotifier {
            transport,
            uri_provider,
        }
    }
}

#[async_trait]
impl Notifier for SimpleNotifier {
    async fn notify(
        &self,
        resource_id: u16,
        destination: &UUri,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<(), NotificationError> {
        let mut builder = UMessageBuilder::notification(
            self.uri_provider.get_resource_uri(resource_id),
            destination.to_owned(),
        );
        apply_common_options(call_options, &mut builder);
        let msg = build_message(&mut builder, payload)
            .map_err(|e| NotificationError::InvalidArgument(e.to_string()))?;
        self.transport
            .send(msg)
            .await
            .map_err(NotificationError::NotifyError)
    }

    async fn start_listening(
        &self,
        origin_filter: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError> {
        origin_filter
            .verify_no_wildcards()
            .map_err(|e| RegistrationError::InvalidFilter(e.to_string()))?;
        self.transport
            .register_listener(
                origin_filter,
                Some(&self.uri_provider.get_source_uri()),
                listener,
            )
            .await
            .map_err(RegistrationError::from)
    }

    async fn stop_listening(
        &self,
        origin_filter: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError> {
        origin_filter
            .verify_no_wildcards()
            .map_err(|e| RegistrationError::InvalidFilter(e.to_string()))?;
        self.transport
            .unregister_listener(
                origin_filter,
                Some(&self.uri_provider.get_source_uri()),
                listener,
            )
            .await
            .map_err(RegistrationError::from)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use mockall::mock;
    use protobuf::well_known_types::wrappers::StringValue;

    use crate::{UCode, UMessage, UPriority, UStatus, UUri, UUID};

    mock! {
        pub NotificationListener {}
        #[async_trait]
        impl UListener for NotificationListener {
            async fn on_receive(&self, message: UMessage);
        }
    }

    mock! {
        pub UriLocator {}
        impl LocalUriProvider for UriLocator {
            fn get_authority(&self) -> String;
            fn get_resource_uri(&self, resource_id: u16) -> UUri;
            fn get_source_uri(&self) -> UUri;
        }
    }

    mock! {
        pub Transport {
            async fn do_send(&self, message: UMessage) -> Result<(), UStatus>;
            async fn do_register_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus>;
            async fn do_unregister_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus>;
        }
    }

    #[async_trait]
    impl UTransport for MockTransport {
        async fn send(&self, message: UMessage) -> Result<(), UStatus> {
            self.do_send(message).await
        }
        async fn register_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UListener>,
        ) -> Result<(), UStatus> {
            self.do_register_listener(source_filter, sink_filter, listener)
                .await
        }
        async fn unregister_listener(
            &self,
            source_filter: &UUri,
            sink_filter: Option<&UUri>,
            listener: Arc<dyn UListener>,
        ) -> Result<(), UStatus> {
            self.do_unregister_listener(source_filter, sink_filter, listener)
                .await
        }
    }

    fn new_uri_provider() -> Arc<dyn LocalUriProvider> {
        let mut mock_uri_locator = MockUriLocator::new();
        mock_uri_locator
            .expect_get_resource_uri()
            .returning(|resource_id| UUri {
                ue_id: 0x0005,
                ue_version_major: 0x02,
                resource_id: resource_id as u32,
                ..Default::default()
            });
        mock_uri_locator.expect_get_source_uri().returning(|| UUri {
            ue_id: 0x0005,
            ue_version_major: 0x02,
            resource_id: 0x0000,
            ..Default::default()
        });
        Arc::new(mock_uri_locator)
    }

    #[tokio::test]
    async fn test_start_stop_listening_rejects_wildcard_topic() {
        let mut transport = MockTransport::new();
        transport.expect_do_register_listener().never();
        let uri_provider = new_uri_provider();
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let invalid_topic = UUri::try_from("up://my-vin/A15B/1/FFFF").unwrap();
        let mut listener = MockNotificationListener::new();
        listener.expect_on_receive().never();
        let wrapped_listener = Arc::new(listener);

        let result = notifier
            .start_listening(&invalid_topic, wrapped_listener.clone())
            .await;
        assert!(result.is_err_and(|e| matches!(e, RegistrationError::InvalidFilter(_))));

        let result = notifier
            .stop_listening(&invalid_topic, wrapped_listener)
            .await;
        assert!(result.is_err_and(|e| matches!(e, RegistrationError::InvalidFilter(_))));
    }

    #[tokio::test]
    async fn test_start_listening_succeeds() {
        let uri_provider = new_uri_provider();
        let topic = UUri::try_from("up://my-vin/A15B/1/B10F").unwrap();
        let expected_source_filter = topic.clone();
        let expected_sink_filter = uri_provider.get_source_uri();
        let mut transport = MockTransport::new();
        transport
            .expect_do_register_listener()
            .once()
            .withf(move |source_filter, sink_filter, _listener| {
                source_filter == &expected_source_filter
                    && *sink_filter == Some(&expected_sink_filter)
            })
            .return_const(Ok(()));
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let mut listener = MockNotificationListener::new();
        listener.expect_on_receive().never();
        let result = notifier.start_listening(&topic, Arc::new(listener)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_listening_succeeds() {
        let uri_provider = new_uri_provider();
        let topic = UUri::try_from("up://my-vin/A15B/1/B10F").unwrap();
        let expected_source_filter = topic.clone();
        let expected_sink_filter = uri_provider.get_source_uri();
        let mut transport = MockTransport::new();
        transport
            .expect_do_unregister_listener()
            .once()
            .withf(move |source_filter, sink_filter, _listener| {
                source_filter == &expected_source_filter
                    && *sink_filter == Some(&expected_sink_filter)
            })
            .return_const(Ok(()));
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let mut listener = MockNotificationListener::new();
        listener.expect_on_receive().never();
        let result = notifier.stop_listening(&topic, Arc::new(listener)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_succeeds() {
        let message_id = UUID::build();
        let uri_provider = new_uri_provider();
        let destination = UUri::try_from("up://other-vin/A15B/1/0").unwrap();
        let expected_message_id = message_id.clone();
        let expected_sink = destination.clone();
        let expected_source = uri_provider.get_resource_uri(0xB10F);
        let mut transport = MockTransport::new();
        transport
            .expect_do_send()
            .once()
            .withf(move |message| {
                let Ok(payload) = message.extract_protobuf::<StringValue>() else {
                    return false;
                };
                let Some(attribs) = message.attributes.as_ref() else {
                    return false;
                };
                attribs.is_notification()
                    && attribs.id.get_or_default() == &expected_message_id
                    && attribs.source.get_or_default() == &expected_source
                    && attribs.sink.get_or_default() == &expected_sink
                    && attribs.ttl == Some(10_000)
                    && attribs.priority.enum_value_or_default() == UPriority::UPRIORITY_CS2
                    && payload.value == *"Hello"
            })
            .return_const(Ok(()));
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let mut v = StringValue::new();
        v.value = "Hello".to_string();
        let payload = UPayload::try_from_protobuf(v).unwrap();
        let options = CallOptions::for_notification(
            Some(10_000),
            Some(message_id),
            Some(UPriority::UPRIORITY_CS2),
        );
        let result = notifier
            .notify(0xB10F, &destination, options, Some(payload))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_fails_for_transport_error() {
        let uri_provider = new_uri_provider();
        let destination = UUri::try_from("up://other-vin/A15B/1/0").unwrap();
        let mut transport = MockTransport::new();
        transport
            .expect_do_send()
            .once()
            .return_const(Err(UStatus::fail_with_code(
                crate::UCode::UNAVAILABLE,
                "connection lost",
            )));
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let options = CallOptions::for_notification(None, None, None);
        let result = notifier.notify(0xB10F, &destination, options, None).await;
        assert!(result.is_err_and(|e| match e {
            NotificationError::NotifyError(status) => status.get_code() == UCode::UNAVAILABLE,
            _ => false,
        }));
    }

    #[tokio::test]
    async fn test_publish_fails_for_invalid_destination() {
        let uri_provider = new_uri_provider();
        // destination has resource ID != 0
        let destination = UUri::try_from("up://other-vin/A15B/1/10").unwrap();
        let mut transport = MockTransport::new();
        transport.expect_do_send().never();
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let options = CallOptions::for_notification(None, None, None);
        let result = notifier.notify(0xB10F, &destination, options, None).await;
        assert!(result.is_err_and(|e| matches!(e, NotificationError::InvalidArgument(_))));
    }

    #[tokio::test]
    async fn test_publish_fails_for_invalid_resource_id() {
        let uri_provider = new_uri_provider();
        let destination = UUri::try_from("up://other-vin/A15B/1/0").unwrap();
        let mut transport = MockTransport::new();
        transport.expect_do_send().never();
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let options = CallOptions::for_notification(None, None, None);
        let result = notifier.notify(0x0000, &destination, options, None).await;
        assert!(result.is_err_and(|e| matches!(e, NotificationError::InvalidArgument(_))));
    }
}
