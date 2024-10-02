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

// [impl->req~up-language-comm-api-default-impl~1]

use std::sync::Arc;

use async_trait::async_trait;

use crate::{LocalUriProvider, UListener, UMessageBuilder, UTransport, UUri};

use super::{
    apply_common_options, build_message, CallOptions, NotificationError, Notifier,
    RegistrationError, UPayload,
};

/// A [`Notifier`] that uses the uProtocol Transport Layer API to send and receive
/// notifications to/from (other) uEntities.
pub struct SimpleNotifier {
    transport: Arc<dyn UTransport>,
    uri_provider: Arc<dyn LocalUriProvider>,
}

impl SimpleNotifier {
    /// Creates a new Notifier for a given transport.
    ///
    /// # Arguments
    ///
    /// * `transport` - The uProtocol Transport Layer implementation to use for sending and receiving notification messages.
    /// * `uri_provider` - The helper for creating URIs that represent local resources.
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
        topic: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError> {
        topic
            .verify_no_wildcards()
            .map_err(|e| RegistrationError::InvalidFilter(e.to_string()))?;
        self.transport
            .register_listener(topic, Some(&self.uri_provider.get_source_uri()), listener)
            .await
            .map_err(RegistrationError::from)
    }

    async fn stop_listening(
        &self,
        topic: &UUri,
        listener: Arc<dyn UListener>,
    ) -> Result<(), RegistrationError> {
        topic
            .verify_no_wildcards()
            .map_err(|e| RegistrationError::InvalidFilter(e.to_string()))?;
        self.transport
            .unregister_listener(topic, Some(&self.uri_provider.get_source_uri()), listener)
            .await
            .map_err(RegistrationError::from)
    }
}

#[cfg(test)]
mod tests {

    // [utest->req~up-language-comm-api-default-impl~1]

    use super::*;

    use protobuf::well_known_types::wrappers::StringValue;

    use crate::{
        utransport::{MockTransport, MockUListener},
        StaticUriProvider, UCode, UPriority, UStatus, UUri, UUID,
    };

    fn new_uri_provider() -> Arc<dyn LocalUriProvider> {
        Arc::new(StaticUriProvider::new("", 0x0005, 0x02))
    }

    #[tokio::test]
    async fn test_start_stop_listening_rejects_wildcard_topic() {
        let mut transport = MockTransport::new();
        transport.expect_do_register_listener().never();
        let uri_provider = new_uri_provider();
        let notifier = SimpleNotifier::new(Arc::new(transport), uri_provider);

        let invalid_topic = UUri::try_from("up://my-vin/A15B/1/FFFF").unwrap();
        let mut listener = MockUListener::new();
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

        let mut listener = MockUListener::new();
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

        let mut listener = MockUListener::new();
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
        // resource ID of origin address must not be 0
        let result = notifier.notify(0x0000, &destination, options, None).await;
        assert!(result.is_err_and(|e| matches!(e, NotificationError::InvalidArgument(_))));
    }
}
