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

// [impl->dsn~communication-layer-impl-default~1]

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    communication::{
        apply_common_options, build_message, CallOptions, PubSubError, Publisher, UPayload,
    },
    LocalUriProvider, UMessageBuilder, UTransport,
};

/// A [`Publisher`] that uses the uProtocol Transport Layer API for publishing events to topics.
pub struct SimplePublisher<T, P> {
    transport: Arc<T>,
    uri_provider: Arc<P>,
}

impl<T: UTransport, P: LocalUriProvider> SimplePublisher<T, P> {
    /// Creates a new client.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for sending messages.
    /// * `uri_provider` - The service to use for creating the event messages' _sink_ address.
    pub fn new(transport: Arc<T>, uri_provider: Arc<P>) -> Self {
        SimplePublisher {
            transport,
            uri_provider,
        }
    }
}

#[async_trait]
impl<T: UTransport, P: LocalUriProvider> Publisher for SimplePublisher<T, P> {
    async fn publish(
        &self,
        resource_id: u16,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<(), PubSubError> {
        let mut builder = UMessageBuilder::publish(self.uri_provider.get_resource_uri(resource_id));
        apply_common_options(call_options, &mut builder);
        match build_message(&mut builder, payload) {
            Ok(publish_message) => self
                .transport
                .send(publish_message)
                .await
                .map_err(Box::from)
                .map_err(PubSubError::PublishError),
            Err(e) => Err(PubSubError::InvalidArgument(format!(
                "failed to create Publish message from parameters: {e}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {

    // [utest->dsn~communication-layer-impl-default~1]

    use super::*;

    use crate::{utransport::MockTransport, StaticUriProvider, UCode, UPriority, UStatus, UUID};

    fn new_uri_provider() -> Arc<StaticUriProvider> {
        Arc::new(StaticUriProvider::new("", 0x0005, 0x02).expect("failed to create URI provider"))
    }

    #[tokio::test]
    async fn test_publish_fails_for_invalid_topic() {
        // GIVEN a publisher
        let uri_provider = new_uri_provider();
        let mut transport = MockTransport::new();

        transport.expect_do_send().never();
        let publisher = SimplePublisher::new(Arc::new(transport), uri_provider);

        // WHEN publishing to an invalid topic
        let options = CallOptions::for_publish(None, None, None);
        let publish_result = publisher
            // resource ID for topic must be >= 0x8000
            .publish(0x1000, options, None)
            .await;

        // THEN publishing fails with an InvalidArgument error
        assert!(publish_result.is_err_and(|e| matches!(e, PubSubError::InvalidArgument(_msg))));
    }

    #[tokio::test]
    async fn test_publish_fails_with_transport_error() {
        let message_id = UUID::build();
        // GIVEN a publisher
        let uri_provider = new_uri_provider();
        let mut transport = MockTransport::new();
        // that is not connected to the underlying messaging infrastructure
        let expected_message_id = message_id.clone();
        transport
            .expect_do_send()
            .once()
            .withf(move |msg| msg.id() == &expected_message_id)
            .returning(|_msg| {
                Err(UStatus::fail_with_code(
                    UCode::Unavailable,
                    "transport not available",
                ))
            });
        let publisher = SimplePublisher::new(Arc::new(transport), uri_provider);

        // WHEN publishing to a valid topic
        let options = CallOptions::for_publish(None, Some(message_id), None);
        let publish_result = publisher.publish(0x9A00, options, None).await;

        // THEN publishing fails with a PublishError
        assert!(publish_result.is_err_and(|e| matches!(e, PubSubError::PublishError(_status))));
    }

    #[tokio::test]
    async fn test_publish_succeeds() {
        // GIVEN a publisher
        let uri_provider = new_uri_provider();
        let mut transport = MockTransport::new();
        let message_id = UUID::build();
        let expected_message_id = message_id.clone();
        let value = b"Hello";

        transport
            .expect_do_send()
            .once()
            .withf(move |message| {
                message.is_publish()
                    && message.id() == &expected_message_id
                    && message.priority_unchecked() == UPriority::CS3
                    && message.ttl_unchecked() == 5_000
                    && message.payload() == Some(value.as_slice().into())
            })
            .returning(|_msg| Ok(()));

        let publisher = SimplePublisher::new(Arc::new(transport), uri_provider);

        // WHEN publishing some data to a valid topic
        let call_options = CallOptions::for_publish(
            Some(5_000),
            Some(message_id.clone()),
            Some(crate::UPriority::CS3),
        );
        let publish_result = publisher
            .publish(
                0x9A00,
                call_options,
                Some(UPayload::new(value.as_slice(), crate::UPayloadFormat::Raw)),
            )
            .await;

        // THEN a corresponding Publish message has been sent via the transport
        assert!(publish_result.is_ok());
    }
}
