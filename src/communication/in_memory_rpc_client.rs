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

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::oneshot::{Receiver, Sender};
use tracing::{debug, info};

use crate::{
    LocalUriProvider, UCode, UListener, UMessage, UMessageBuilder, UMessageType, UStatus,
    UTransport, UUri, UUID,
};

use super::{CallOptions, RpcClient, ServiceInvocationError, UPayload};

fn handle_response_message(response: UMessage) -> Result<Option<UPayload>, ServiceInvocationError> {
    let Some(attribs) = response.attributes.as_ref() else {
        return Err(ServiceInvocationError::RpcError(UStatus::fail_with_code(
            UCode::INTERNAL,
            "response message does not contain attributes",
        )));
    };

    match attribs.commstatus.map(|v| v.enum_value_or_default()) {
        Some(UCode::OK) | None => {
            // successful invocation
            response.payload.map_or(Ok(None), |payload| {
                Ok(Some(UPayload::new(
                    payload,
                    attribs.payload_format.enum_value_or_default(),
                )))
            })
        }
        Some(code) => {
            // try to extract UStatus from response payload
            let status = response.extract_protobuf().unwrap_or_else(|_e| {
                UStatus::fail_with_code(code, "failed to invoke service operation")
            });
            Err(ServiceInvocationError::from(status))
        }
    }
}

struct ResponseListener {
    // request ID -> sender for response message
    pending_requests: Mutex<HashMap<UUID, Sender<UMessage>>>,
}

impl ResponseListener {
    fn try_add_pending_request(
        &self,
        reqid: UUID,
    ) -> Result<Receiver<UMessage>, ServiceInvocationError> {
        let Ok(mut pending_requests) = self.pending_requests.lock() else {
            return Err(ServiceInvocationError::Internal(
                "failed to add response handler".to_string(),
            ));
        };

        if let Entry::Vacant(entry) = pending_requests.entry(reqid) {
            let (tx, rx) = tokio::sync::oneshot::channel();
            entry.insert(tx);
            Ok(rx)
        } else {
            Err(ServiceInvocationError::AlreadyExists(
                "RPC request with given ID already pending".to_string(),
            ))
        }
    }

    fn handle_response(&self, reqid: &UUID, response_message: UMessage) {
        let Ok(mut pending_requests) = self.pending_requests.lock() else {
            info!(
                request_id = reqid.to_hyphenated_string(),
                "failed to process response message, cannot acquire lock for pending requests map"
            );
            return;
        };
        if let Some(sender) = pending_requests.remove(reqid) {
            if let Err(_e) = sender.send(response_message) {
                // channel seems to be closed already
                debug!(
                    request_id = reqid.to_hyphenated_string(),
                    "failed to deliver response message, channel already closed"
                );
            }
        } else {
            // we seem to have received a duplicate of the response message, ignoring it ...
            debug!(
                request_id = reqid.to_hyphenated_string(),
                "ignoring response message for unknown request"
            );
        }
    }

    fn remove_pending_request(&self, reqid: &UUID) -> Option<Sender<UMessage>> {
        self.pending_requests
            .lock()
            .map_or(None, |mut pending_requests| pending_requests.remove(reqid))
    }

    #[cfg(test)]
    fn contains(&self, reqid: &UUID) -> bool {
        self.pending_requests
            .lock()
            .map_or(false, |pending_requests| {
                pending_requests.contains_key(reqid)
            })
    }
}

#[async_trait]
impl UListener for ResponseListener {
    async fn on_receive(&self, msg: UMessage) {
        let message_type = msg
            .attributes
            .get_or_default()
            .type_
            .enum_value_or_default();
        if message_type != UMessageType::UMESSAGE_TYPE_RESPONSE {
            debug!(
                message_type = message_type.to_cloudevent_type(),
                "service provider replied with message that is not an RPC Response"
            );
            return;
        }

        if let Some(reqid) = msg
            .attributes
            .as_ref()
            .and_then(|attribs| attribs.reqid.clone().into_option())
        {
            self.handle_response(&reqid, msg);
        } else {
            debug!("ignoring malformed response message not containing request ID");
        }
    }
}

/// An ['RpcClient'] which keeps all information about pending requests in memory.
pub struct InMemoryRpcClient {
    transport: Arc<dyn UTransport>,
    uri_provider: Arc<dyn LocalUriProvider>,
    response_listener: Arc<ResponseListener>,
}

impl InMemoryRpcClient {
    /// Creates a new RPC client for a given transport.
    ///
    /// # Arguments
    ///
    /// * `transport` - The uProtocol Transport Layer implementation to use for invoking service operations.
    /// * `uri_provider` - The helper for creating URIs that represent local resources.
    ///
    /// # Errors
    ///
    /// Returns an error if the generic RPC Response listener could not be
    /// registered with the given transport.
    pub async fn new(
        transport: Arc<dyn UTransport>,
        uri_provider: Arc<dyn LocalUriProvider>,
    ) -> Result<Self, UStatus> {
        let response_listener = Arc::new(ResponseListener {
            pending_requests: Mutex::new(HashMap::new()),
        });
        transport
            .register_listener(
                &UUri::any(),
                Some(&uri_provider.get_source_uri()),
                response_listener.clone(),
            )
            .await?;

        Ok(InMemoryRpcClient {
            transport,
            uri_provider,
            response_listener,
        })
    }

    #[cfg(test)]
    fn contains_pending_request(&self, reqid: &UUID) -> bool {
        self.response_listener.contains(reqid)
    }
}

#[async_trait]
impl RpcClient for InMemoryRpcClient {
    async fn invoke_method(
        &self,
        method: UUri,
        call_options: CallOptions,
        payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        let message_id = call_options.message_id().unwrap_or_else(UUID::build);

        let mut builder = UMessageBuilder::request(
            method.clone(),
            self.uri_provider.get_source_uri(),
            call_options.ttl(),
        );
        builder.with_message_id(message_id.clone());
        if let Some(token) = call_options.token() {
            builder.with_token(token.to_owned());
        }
        if let Some(priority) = call_options.priority() {
            builder.with_priority(priority);
        }
        let rpc_request_message = if let Some(pl) = payload {
            let format = pl.payload_format();
            builder.build_with_payload(pl.payload(), format)
        } else {
            builder.build()
        }
        .map_err(|e| ServiceInvocationError::InvalidArgument(e.to_string()))?;

        let receiver = self
            .response_listener
            .try_add_pending_request(message_id.clone())?;
        self.transport
            .send(rpc_request_message)
            .await
            .map_err(|e| {
                self.response_listener.remove_pending_request(&message_id);
                e
            })?;

        if let Ok(Ok(response_message)) =
            tokio::time::timeout(Duration::from_millis(call_options.ttl() as u64), receiver).await
        {
            handle_response_message(response_message)
        } else {
            self.response_listener.remove_pending_request(&message_id);
            Err(ServiceInvocationError::DeadlineExceeded)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use mockall::mock;
    use protobuf::{well_known_types::wrappers::StringValue, Enum};

    use crate::{UMessageBuilder, UPriority, UUri};

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
        mock_uri_locator.expect_get_source_uri().returning(|| UUri {
            ue_id: 0x0005,
            ue_version_major: 0x02,
            resource_id: 0x0000,
            ..Default::default()
        });
        Arc::new(mock_uri_locator)
    }

    fn service_method_uri() -> UUri {
        UUri {
            ue_id: 0x0001,
            ue_version_major: 0x01,
            resource_id: 0x1000,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_invoke_method_fails_with_transport_error() {
        // GIVEN an RPC client
        let mut mock_transport = MockTransport::default();
        mock_transport
            .expect_do_register_listener()
            .once()
            .returning(|_source_filter, _sink_filter, _listener| Ok(()));
        // with a transport that fails with an error when invoking a method
        mock_transport
            .expect_do_send()
            .returning(|_request_message| {
                Err(UStatus::fail_with_code(
                    UCode::UNAVAILABLE,
                    "transport not available",
                ))
            });
        let client = InMemoryRpcClient::new(Arc::new(mock_transport), new_uri_provider())
            .await
            .unwrap();

        // WHEN invoking a remote service operation
        let message_id = UUID::build();
        let call_options =
            CallOptions::for_rpc_request(5_000, Some(message_id.clone()), None, None);
        let response = client
            .invoke_method(service_method_uri(), call_options, None)
            .await;

        // THEN the invocation fails with the error caused at the Transport Layer
        assert!(response.is_err_and(|e| matches!(e, ServiceInvocationError::Unavailable(_msg))));
        assert!(!client.contains_pending_request(&message_id));
    }

    #[tokio::test]
    async fn test_invoke_method_succeeds() {
        let message_id = UUID::build();
        let call_options = CallOptions::for_rpc_request(
            5_000,
            Some(message_id.clone()),
            Some("my_token".to_string()),
            Some(crate::UPriority::UPRIORITY_CS6),
        );

        // GIVEN an RPC client
        let listener_ref: Arc<Mutex<Option<Arc<dyn UListener>>>> = Arc::new(Mutex::new(None));
        let response_listener = listener_ref.clone();

        let mut mock_transport = MockTransport::default();
        mock_transport.expect_do_register_listener().returning(
            move |_source_filter, _sink_filter, listener| {
                let mut v = listener_ref.lock().unwrap();
                *v = Some(listener);
                Ok(())
            },
        );
        let expected_message_id = message_id.clone();
        mock_transport
            .expect_do_send()
            .withf(move |request_message| {
                request_message
                    .attributes
                    .as_ref()
                    .map_or(false, |attribs| {
                        attribs.id.as_ref() == Some(&expected_message_id)
                            && attribs.priority.value() == UPriority::UPRIORITY_CS6.value()
                            && attribs.ttl == Some(5_000)
                            && attribs.token == Some("my_token".to_string())
                    })
            })
            .returning(move |request_message| {
                let request_payload: StringValue = request_message.extract_protobuf().unwrap();
                let response_payload = StringValue {
                    value: format!("Hello {}", request_payload.value),
                    ..Default::default()
                };

                let response_message = UMessageBuilder::response_for_request(
                    request_message.attributes.as_ref().unwrap(),
                )
                .build_with_protobuf_payload(&response_payload)
                .unwrap();
                let cloned_listener = response_listener.lock().unwrap().take().unwrap().clone();
                tokio::spawn(async move { cloned_listener.on_receive(response_message).await });
                Ok(())
            });

        let rpc_client = Arc::new(
            InMemoryRpcClient::new(Arc::new(mock_transport), new_uri_provider())
                .await
                .unwrap(),
        );
        let client: Arc<dyn RpcClient> = rpc_client.clone();

        // WHEN invoking a remote service operation
        let request_payload = StringValue {
            value: "World".to_string(),
            ..Default::default()
        };
        let response: StringValue = client
            .invoke_proto_method(service_method_uri(), call_options, request_payload)
            .await
            .expect("invoking method should have succeeded");
        // THEN the response contains the expected payload
        assert_eq!(response.value, "Hello World");
        assert!(!rpc_client.contains_pending_request(&message_id));
    }

    #[tokio::test]
    async fn test_invoke_method_fails_with_remote_error() {
        // GIVEN an RPC client
        let listener_ref: Arc<Mutex<Option<Arc<dyn UListener>>>> = Arc::new(Mutex::new(None));
        let response_listener = listener_ref.clone();

        let mut mock_transport = MockTransport::default();
        mock_transport.expect_do_register_listener().returning(
            move |_source_filter, _sink_filter, listener| {
                let mut v = listener_ref.lock().unwrap();
                *v = Some(listener);
                Ok(())
            },
        );
        // and a remote service operation that returns an error
        mock_transport
            .expect_do_send()
            .returning(move |request_message| {
                let error = UStatus::fail_with_code(UCode::NOT_FOUND, "no such object");
                let response_message = UMessageBuilder::response_for_request(
                    request_message.attributes.as_ref().unwrap(),
                )
                .with_comm_status(UCode::NOT_FOUND)
                .build_with_protobuf_payload(&error)
                .unwrap();
                let cloned_listener = response_listener.lock().unwrap().take().unwrap().clone();
                tokio::spawn(async move { cloned_listener.on_receive(response_message).await });
                Ok(())
            });

        let client = InMemoryRpcClient::new(Arc::new(mock_transport), new_uri_provider())
            .await
            .unwrap();

        // WHEN invoking the remote service operation
        let message_id = UUID::build();
        let call_options =
            CallOptions::for_rpc_request(5_000, Some(message_id.clone()), None, None);
        let response = client
            .invoke_method(service_method_uri(), call_options, None)
            .await;

        // THEN the invocation has failed with the error returned from the service
        assert!(response.is_err_and(|e| { matches!(e, ServiceInvocationError::NotFound(_msg)) }));
        assert!(!client.contains_pending_request(&message_id));
    }

    #[tokio::test]
    async fn test_invoke_method_times_out() {
        // GIVEN an RPC client
        let mut mock_transport = MockTransport::default();
        mock_transport
            .expect_do_register_listener()
            .returning(|_source_filter, _sink_filter, _listener| Ok(()));
        // and a remote service operation that does not return a response
        mock_transport
            .expect_do_send()
            .returning(|_request_message| Ok(()));

        let client = InMemoryRpcClient::new(Arc::new(mock_transport), new_uri_provider())
            .await
            .unwrap();

        // WHEN invoking the remote service operation
        let message_id = UUID::build();
        let call_options = CallOptions::for_rpc_request(20, Some(message_id.clone()), None, None);
        let response = client
            .invoke_method(service_method_uri(), call_options, None)
            .await;

        // THEN the invocation times out
        assert!(response.is_err_and(|e| { matches!(e, ServiceInvocationError::DeadlineExceeded) }));
        assert!(!client.contains_pending_request(&message_id));
    }
}
