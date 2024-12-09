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

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    communication::build_message, LocalUriProvider, UAttributes, UAttributesError,
    UAttributesValidators, UCode, UListener, UMessage, UMessageBuilder, UStatus, UTransport, UUri,
};

use super::{RegistrationError, RequestHandler, RpcServer, ServiceInvocationError, UPayload};

struct RequestListener {
    request_handler: Arc<dyn RequestHandler>,
    transport: Arc<dyn UTransport>,
}

impl RequestListener {
    async fn process_valid_request(&self, resource_id: u16, request_message: UMessage) {
        let transport_clone = self.transport.clone();
        let request_handler_clone = self.request_handler.clone();

        let request_id = request_message
            .attributes
            .get_or_default()
            .id
            .get_or_default();
        let request_timeout = request_message
            .attributes
            .get_or_default()
            .ttl
            .unwrap_or(10_000);
        let payload = request_message.payload;
        let payload_format = request_message
            .attributes
            .get_or_default()
            .payload_format
            .enum_value_or_default();
        let request_payload = payload.map(|data| UPayload::new(data, payload_format));

        debug!(ttl = request_timeout, id = %request_id, "processing RPC request");

        let invocation_result_future = request_handler_clone.handle_request(
            resource_id,
            &request_message.attributes,
            request_payload,
        );
        let outcome = tokio::time::timeout(
            Duration::from_millis(request_timeout as u64),
            invocation_result_future,
        )
        .await
        .map_err(|_e| {
            info!(ttl = request_timeout, "request handler timed out");
            ServiceInvocationError::DeadlineExceeded
        })
        .and_then(|v| v);

        let response = match outcome {
            Ok(response_payload) => {
                let mut builder = UMessageBuilder::response_for_request(
                    request_message.attributes.get_or_default(),
                );
                build_message(&mut builder, response_payload)
            }
            Err(e) => {
                let error = UStatus::from(e);
                UMessageBuilder::response_for_request(request_message.attributes.get_or_default())
                    .with_comm_status(error.get_code())
                    .build_with_protobuf_payload(&error)
            }
        };

        match response {
            Ok(response_message) => {
                if let Err(e) = transport_clone.send(response_message).await {
                    info!(ucode = e.code.value(), "failed to send response message");
                }
            }
            Err(e) => {
                info!("failed to create response message: {}", e);
            }
        }
    }

    async fn process_invalid_request(
        &self,
        validation_error: UAttributesError,
        request_attributes: &UAttributes,
    ) {
        // all we need is a valid source address and a message ID to be able to send back an error message
        let (Some(id), Some(source_address)) = (
            request_attributes.id.to_owned().into_option(),
            request_attributes
                .source
                .to_owned()
                .into_option()
                .filter(|uri| uri.is_rpc_response()),
        ) else {
            debug!("invalid request message does not contain enough data to create response");
            return;
        };

        debug!(id = %id, "processing invalid request message");

        let response_payload =
            UStatus::fail_with_code(UCode::INVALID_ARGUMENT, validation_error.to_string());
        let Ok(response_message) = UMessageBuilder::response(
            source_address,
            id,
            request_attributes.sink.get_or_default().to_owned(),
        )
        .with_comm_status(response_payload.get_code())
        .build_with_protobuf_payload(&response_payload) else {
            info!("failed to create error message");
            return;
        };

        if let Err(e) = self.transport.send(response_message).await {
            info!(ucode = e.code.value(), "failed to send error response");
        }
    }
}

#[async_trait]
impl UListener for RequestListener {
    async fn on_receive(&self, msg: UMessage) {
        let Some(attributes) = msg.attributes.as_ref() else {
            debug!("ignoring invalid message having no attributes");
            return;
        };

        let validator = UAttributesValidators::Request.validator();
        if let Err(e) = validator.validate(attributes) {
            self.process_invalid_request(e, attributes).await;
        } else if let Some(resource_id) = attributes
            .sink
            .as_ref()
            .and_then(|uri| u16::try_from(uri.resource_id).ok())
        {
            // the conversion cannot fail because request message validation has succeeded
            self.process_valid_request(resource_id, msg).await;
        }
    }
}

/// An [`RpcServer`] which keeps all information about registered endpoints in memory.
///
/// The server requires an implementations of [`UTransport`] for receiving RPC Request messages
/// from clients and sending back RPC Response messages.
///
/// For each [endpoint being registered](`Self::register_endpoint`), a [`UListener`] is created for
/// the given request handler and registered with the underlying transport. The listener is also
/// mapped to the endpoint's method resource ID in order to prevent registration of multiple
/// request handlers for the same method.
pub struct InMemoryRpcServer {
    transport: Arc<dyn UTransport>,
    uri_provider: Arc<dyn LocalUriProvider>,
    request_listeners: tokio::sync::Mutex<HashMap<u16, Arc<dyn UListener>>>,
}

impl InMemoryRpcServer {
    /// Creates a new RPC server for a given transport.
    pub fn new(transport: Arc<dyn UTransport>, uri_provider: Arc<dyn LocalUriProvider>) -> Self {
        InMemoryRpcServer {
            transport,
            uri_provider,
            request_listeners: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    fn validate_sink_filter(filter: &UUri) -> Result<(), RegistrationError> {
        if !filter.is_rpc_method() {
            return Err(RegistrationError::InvalidFilter(
                "RPC endpoint's resource ID must be in range [0x0001, 0x7FFF]".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_origin_filter(filter: Option<&UUri>) -> Result<(), RegistrationError> {
        if let Some(uri) = filter {
            if !uri.is_rpc_response() {
                return Err(RegistrationError::InvalidFilter(
                    "origin filter's resource ID must be 0".to_string(),
                ));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    async fn contains_endpoint(&self, resource_id: u16) -> bool {
        let listener_map = self.request_listeners.lock().await;
        listener_map.contains_key(&resource_id)
    }
}

#[async_trait]
impl RpcServer for InMemoryRpcServer {
    async fn register_endpoint(
        &self,
        origin_filter: Option<&UUri>,
        resource_id: u16,
        request_handler: Arc<dyn RequestHandler>,
    ) -> Result<(), RegistrationError> {
        Self::validate_origin_filter(origin_filter)?;
        let sink_filter = self.uri_provider.get_resource_uri(resource_id);
        Self::validate_sink_filter(&sink_filter)?;

        let mut listener_map = self.request_listeners.lock().await;
        if let Entry::Vacant(e) = listener_map.entry(resource_id) {
            let listener = Arc::new(RequestListener {
                request_handler,
                transport: self.transport.clone(),
            });
            self.transport
                .register_listener(
                    origin_filter.unwrap_or(&UUri::any_with_resource_id(
                        crate::uri::RESOURCE_ID_RESPONSE,
                    )),
                    Some(&sink_filter),
                    listener.clone(),
                )
                .await
                .map(|_| {
                    e.insert(listener);
                })
                .map_err(RegistrationError::from)
        } else {
            Err(RegistrationError::MaxListenersExceeded)
        }
    }

    async fn unregister_endpoint(
        &self,
        origin_filter: Option<&UUri>,
        resource_id: u16,
        _request_handler: Arc<dyn RequestHandler>,
    ) -> Result<(), RegistrationError> {
        Self::validate_origin_filter(origin_filter)?;
        let sink_filter = self.uri_provider.get_resource_uri(resource_id);
        Self::validate_sink_filter(&sink_filter)?;

        let mut listener_map = self.request_listeners.lock().await;
        if let Entry::Occupied(entry) = listener_map.entry(resource_id) {
            let listener = entry.get().to_owned();
            self.transport
                .unregister_listener(
                    origin_filter.unwrap_or(&UUri::any_with_resource_id(
                        crate::uri::RESOURCE_ID_RESPONSE,
                    )),
                    Some(&sink_filter),
                    listener,
                )
                .await
                .map(|_| {
                    entry.remove();
                })
                .map_err(RegistrationError::from)
        } else {
            Err(RegistrationError::NoSuchListener)
        }
    }
}

#[cfg(test)]
mod tests {

    // [utest->req~up-language-comm-api-default-impl~1]

    use super::*;

    use protobuf::well_known_types::wrappers::StringValue;
    use test_case::test_case;
    use tokio::sync::Notify;

    use crate::{
        communication::rpc::MockRequestHandler, utransport::MockTransport, StaticUriProvider,
        UAttributes, UMessageType, UPriority, UUri, UUID,
    };

    fn new_uri_provider() -> Arc<dyn LocalUriProvider> {
        Arc::new(StaticUriProvider::new("", 0x0005, 0x02))
    }

    #[test_case(None, 0x4A10; "for empty origin filter")]
    #[test_case(Some(UUri::try_from_parts("authority", 0xBF1A, 0x01, 0x0000).unwrap()), 0x4A10; "for specific origin filter")]
    #[test_case(Some(UUri::try_from_parts("*", 0xFFFF, 0x01, 0x0000).unwrap()), 0x7091; "for wildcard origin filter")]
    #[tokio::test]
    async fn test_register_endpoint_succeeds(origin_filter: Option<UUri>, resource_id: u16) {
        // GIVEN an RpcServer for a transport
        let request_handler = Arc::new(MockRequestHandler::new());
        let mut transport = MockTransport::new();
        let uri_provider = new_uri_provider();
        let expected_source_filter = origin_filter
            .clone()
            .unwrap_or(UUri::any_with_resource_id(0));
        let param_check = move |source_filter: &UUri,
                                sink_filter: &Option<&UUri>,
                                _listener: &Arc<dyn UListener>| {
            source_filter == &expected_source_filter
                && sink_filter.map_or(false, |uri| uri.resource_id == resource_id as u32)
        };
        transport
            .expect_do_register_listener()
            .once()
            .withf(param_check.clone())
            .returning(|_source_filter, _sink_filter, _listener| Ok(()));
        transport
            .expect_do_unregister_listener()
            .once()
            .withf(param_check)
            .returning(|_source_filter, _sink_filter, _listener| Ok(()));

        let rpc_server = InMemoryRpcServer::new(Arc::new(transport), uri_provider);

        // WHEN registering a request handler
        let register_result = rpc_server
            .register_endpoint(origin_filter.as_ref(), resource_id, request_handler.clone())
            .await;
        // THEN registration succeeds
        assert!(register_result.is_ok());
        assert!(rpc_server.contains_endpoint(resource_id).await);

        // and the handler can be unregistered again
        let unregister_result = rpc_server
            .unregister_endpoint(origin_filter.as_ref(), resource_id, request_handler)
            .await;
        assert!(unregister_result.is_ok());
        assert!(!rpc_server.contains_endpoint(resource_id).await);
    }

    #[test_case(None, 0x0000; "for resource ID 0")]
    #[test_case(None, 0x8000; "for resource ID out of range")]
    #[test_case(Some(UUri::try_from_parts("*", 0xFFFF, 0xFF, 0x0001).unwrap()), 0x4A10; "for source filter with invalid resource ID")]
    #[tokio::test]
    async fn test_register_endpoint_fails(origin_filter: Option<UUri>, resource_id: u16) {
        // GIVEN an RpcServer for a transport
        let request_handler = Arc::new(MockRequestHandler::new());
        let mut transport = MockTransport::new();
        let uri_provider = new_uri_provider();
        transport.expect_do_register_listener().never();
        transport.expect_do_unregister_listener().never();

        let rpc_server = InMemoryRpcServer::new(Arc::new(transport), uri_provider);

        // WHEN registering a request handler using invalid parameters
        let register_result = rpc_server
            .register_endpoint(origin_filter.as_ref(), resource_id, request_handler.clone())
            .await;
        // THEN registration fails
        assert!(register_result.is_err_and(|e| matches!(e, RegistrationError::InvalidFilter(_v))));
        assert!(!rpc_server.contains_endpoint(resource_id).await);

        // and an attempt to unregister the handler using the same invalid parameters also fails with the same error
        let unregister_result = rpc_server
            .unregister_endpoint(origin_filter.as_ref(), resource_id, request_handler)
            .await;
        assert!(unregister_result.is_err_and(|e| matches!(e, RegistrationError::InvalidFilter(_v))));
    }

    #[tokio::test]
    async fn test_register_endpoint_fails_for_duplicate_endpoint() {
        // GIVEN an RpcServer for a transport
        let request_handler = Arc::new(MockRequestHandler::new());
        let mut transport = MockTransport::new();
        let uri_provider = new_uri_provider();
        transport
            .expect_do_register_listener()
            .once()
            .return_const(Ok(()));

        let rpc_server = InMemoryRpcServer::new(Arc::new(transport), uri_provider);

        // WHEN registering a request handler for an already existing endpoint
        assert!(rpc_server
            .register_endpoint(None, 0x5000, request_handler.clone())
            .await
            .is_ok());
        let result = rpc_server
            .register_endpoint(None, 0x5000, request_handler)
            .await;

        // THEN registration of the additional handler fails
        assert!(result.is_err_and(|e| matches!(e, RegistrationError::MaxListenersExceeded)));
        // but the original endpoint is still registered
        assert!(rpc_server.contains_endpoint(0x5000).await);
    }

    #[tokio::test]
    async fn test_unregister_endpoint_fails_for_non_existing_endpoint() {
        // GIVEN an RpcServer for a transport
        let request_handler = Arc::new(MockRequestHandler::new());
        let mut transport = MockTransport::new();
        let uri_provider = new_uri_provider();
        transport.expect_do_unregister_listener().never();

        let rpc_server = InMemoryRpcServer::new(Arc::new(transport), uri_provider);

        // WHEN trying to unregister a non existing endpoint
        assert!(!rpc_server.contains_endpoint(0x5000).await);
        let result = rpc_server
            .unregister_endpoint(None, 0x5000, request_handler)
            .await;

        // THEN registration fails
        assert!(result.is_err_and(|e| matches!(e, RegistrationError::NoSuchListener)));
    }

    #[tokio::test]
    async fn test_request_listener_returns_response_for_invalid_request() {
        // GIVEN an RpcServer for a transport
        let mut request_handler = MockRequestHandler::new();
        let mut transport = MockTransport::new();
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let message_id = UUID::build();
        let request_id = message_id.clone();

        request_handler.expect_handle_request().never();
        transport
            .expect_do_send()
            .once()
            .withf(move |response_message| {
                if !response_message.is_response() {
                    return false;
                }
                if response_message
                    .attributes
                    .get_or_default()
                    .reqid
                    .get_or_default()
                    != &request_id
                {
                    return false;
                }
                let error: UStatus = response_message.extract_protobuf().unwrap();
                error.get_code() == UCode::INVALID_ARGUMENT
                    && response_message
                        .attributes
                        .get_or_default()
                        .commstatus
                        .map_or(false, |v| v.enum_value_or_default() == error.get_code())
            })
            .returning(move |_msg| {
                notify_clone.notify_one();
                Ok(())
            });

        // WHEN the server receives a message on an endpoint which is not a
        // valid RPC Request message but contains enough information to
        // create a response
        let invalid_request_attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
            sink: UUri::try_from("up://localhost/A200/1/7000").ok().into(),
            source: UUri::try_from("up://localhost/A100/1/0").ok().into(),
            id: Some(message_id.clone()).into(),
            priority: UPriority::UPRIORITY_CS5.into(),
            ..Default::default()
        };
        assert!(
            UAttributesValidators::Request
                .validator()
                .validate(&invalid_request_attributes)
                .is_err(),
            "request message attributes are supposed to be invalid (no TTL)"
        );
        let invalid_request_message = UMessage {
            attributes: Some(invalid_request_attributes).into(),
            ..Default::default()
        };

        let request_listener = RequestListener {
            request_handler: Arc::new(request_handler),
            transport: Arc::new(transport),
        };
        request_listener.on_receive(invalid_request_message).await;

        // THEN the listener sends an error message in response to the invalid request
        let result = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_listener_ignores_invalid_request() {
        // GIVEN an RpcServer for a transport
        let mut request_handler = MockRequestHandler::new();
        request_handler.expect_handle_request().never();
        let mut transport = MockTransport::new();
        transport.expect_do_send().never();

        // WHEN the server receives a message on an endpoint which is not a
        // valid RPC Request message which does not contain enough information to
        // create a response
        let invalid_request_attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
            sink: UUri::try_from("up://localhost/A200/1/7000").ok().into(),
            source: UUri::try_from("up://localhost/A100/1/0").ok().into(),
            ttl: Some(5_000),
            id: None.into(),
            priority: UPriority::UPRIORITY_CS5.into(),
            ..Default::default()
        };
        assert!(
            UAttributesValidators::Request
                .validator()
                .validate(&invalid_request_attributes)
                .is_err(),
            "request message attributes are supposed to be invalid (no ID)"
        );
        let invalid_request_message = UMessage {
            attributes: Some(invalid_request_attributes).into(),
            ..Default::default()
        };

        let request_listener = RequestListener {
            request_handler: Arc::new(request_handler),
            transport: Arc::new(transport),
        };
        request_listener.on_receive(invalid_request_message).await;

        // THEN the listener ignores the invalid request
        // let result = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_listener_invokes_operation_successfully() {
        let mut request_handler = MockRequestHandler::new();
        let mut transport = MockTransport::new();
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let request_payload = StringValue {
            value: "Hello".to_string(),
            ..Default::default()
        };
        let message_id = UUID::build();
        let message_id_clone = message_id.clone();
        let message_source = UUri::try_from("up://localhost/A100/1/0").unwrap();
        let message_source_clone = message_source.clone();

        request_handler
            .expect_handle_request()
            .once()
            .withf(move |resource_id, message_attributes, request_payload| {
                if let Some(pl) = request_payload {
                    let message_source = message_attributes.source.as_ref().unwrap();
                    let msg: StringValue = pl.extract_protobuf().unwrap();
                    msg.value == *"Hello"
                        && *resource_id == 0x7000_u16
                        && *message_source == message_source_clone
                } else {
                    false
                }
            })
            .returning(|_resource_id, _message_attributes, _request_payload| {
                let response_payload = UPayload::try_from_protobuf(StringValue {
                    value: "Hello World".to_string(),
                    ..Default::default()
                })
                .unwrap();
                Ok(Some(response_payload))
            });
        transport
            .expect_do_send()
            .once()
            .withf(move |response_message| {
                let msg: StringValue = response_message.extract_protobuf().unwrap();
                msg.value == *"Hello World"
                    && response_message.is_response()
                    && response_message
                        .attributes
                        .get_or_default()
                        .commstatus
                        .map_or(true, |v| v.enum_value_or_default() == UCode::OK)
                    && response_message
                        .attributes
                        .get_or_default()
                        .reqid
                        .get_or_default()
                        == &message_id_clone
            })
            .returning(move |_msg| {
                notify_clone.notify_one();
                Ok(())
            });
        let request_message = UMessageBuilder::request(
            UUri::try_from("up://localhost/A200/1/7000").unwrap(),
            message_source,
            5_000,
        )
        .with_message_id(message_id)
        .build_with_protobuf_payload(&request_payload)
        .unwrap();

        let request_listener = RequestListener {
            request_handler: Arc::new(request_handler),
            transport: Arc::new(transport),
        };
        request_listener.on_receive(request_message).await;
        let result = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_listener_invokes_operation_erroneously() {
        let mut request_handler = MockRequestHandler::new();
        let mut transport = MockTransport::new();
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let message_id = UUID::build();
        let message_id_clone = message_id.clone();

        request_handler
            .expect_handle_request()
            .once()
            .withf(|resource_id, _message_attributes, _request_payload| *resource_id == 0x7000_u16)
            .returning(|_resource_id, _message_attributes, _request_payload| {
                Err(ServiceInvocationError::NotFound(
                    "no such object".to_string(),
                ))
            });
        transport
            .expect_do_send()
            .once()
            .withf(move |response_message| {
                let error: UStatus = response_message.extract_protobuf().unwrap();
                error.get_code() == UCode::NOT_FOUND
                    && response_message.is_response()
                    && response_message
                        .attributes
                        .get_or_default()
                        .commstatus
                        .map_or(false, |v| v.enum_value_or_default() == error.get_code())
                    && response_message
                        .attributes
                        .get_or_default()
                        .reqid
                        .get_or_default()
                        == &message_id_clone
            })
            .returning(move |_msg| {
                notify_clone.notify_one();
                Ok(())
            });
        let request_message = UMessageBuilder::request(
            UUri::try_from("up://localhost/A200/1/7000").unwrap(),
            UUri::try_from("up://localhost/A100/1/0").unwrap(),
            5_000,
        )
        .with_message_id(message_id)
        .build()
        .unwrap();

        let request_listener = RequestListener {
            request_handler: Arc::new(request_handler),
            transport: Arc::new(transport),
        };
        request_listener.on_receive(request_message).await;
        let result = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_listener_times_out() {
        // we need to manually implement the RequestHandler
        // because from within the MockRequestHandler's expectation
        // we cannot yield the current task (we can only use the blocking
        // thread::sleep function)
        struct NonRespondingHandler;
        #[async_trait]
        impl RequestHandler for NonRespondingHandler {
            async fn handle_request(
                &self,
                resource_id: u16,
                _message_attributes: &UAttributes,
                _request_payload: Option<UPayload>,
            ) -> Result<Option<UPayload>, ServiceInvocationError> {
                assert_eq!(resource_id, 0x7000);
                // this will yield the current task and allow the
                // RequestListener to run into the timeout
                tokio::time::sleep(Duration::from_millis(2000)).await;
                Ok(None)
            }
        }

        let request_handler = NonRespondingHandler {};
        let mut transport = MockTransport::new();
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let message_id = UUID::build();
        let message_id_clone = message_id.clone();

        transport
            .expect_do_send()
            .once()
            .withf(move |response_message| {
                let error: UStatus = response_message.extract_protobuf().unwrap();
                error.get_code() == UCode::DEADLINE_EXCEEDED
                    && response_message.is_response()
                    && response_message
                        .attributes
                        .get_or_default()
                        .commstatus
                        .map_or(false, |v| v.enum_value_or_default() == error.get_code())
                    && response_message
                        .attributes
                        .get_or_default()
                        .reqid
                        .get_or_default()
                        == &message_id_clone
            })
            .returning(move |_msg| {
                notify_clone.notify_one();
                Ok(())
            });
        let request_message = UMessageBuilder::request(
            UUri::try_from("up://localhost/A200/1/7000").unwrap(),
            UUri::try_from("up://localhost/A100/1/0").unwrap(),
            // make sure this request times out very quickly
            100,
        )
        .with_message_id(message_id)
        .build()
        .expect("should have been able to create RPC Request message");

        let request_listener = RequestListener {
            request_handler: Arc::new(request_handler),
            transport: Arc::new(transport),
        };
        request_listener.on_receive(request_message).await;
        let result = tokio::time::timeout(Duration::from_secs(2), notify.notified()).await;
        assert!(result.is_ok());
    }
}
