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

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{
    LocalUriProvider, UCode, UListener, UMessage, UMessageBuilder, UStatus, UTransport, UUri, UUID,
};

use super::{
    build_message, CallOptions, RegistrationError, RpcClient, ServiceInvocationError, UPayload,
};

/// Handles an RPC Response message received from the transport layer.
fn handle_response_message(response: UMessage) -> Result<Option<UPayload>, ServiceInvocationError> {
    match response.commstatus() {
        Some(UCode::OK) | None => {
            // successful invocation
            let payload_format = response.payload_format().unwrap_or_default();
            Ok(response
                .payload
                .map(|payload| UPayload::new(payload, payload_format)))
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

    fn handle_response(&self, response_message: UMessage) {
        let reqid = response_message.request_id_unchecked().clone();
        let response_sender = {
            // drop lock as soon as possible
            let Ok(mut pending_requests) = self.pending_requests.lock() else {
                info!(
                    request_id = reqid.to_hyphenated_string(),
                    "failed to process response message, cannot acquire lock for pending requests map"
                );
                return;
            };
            pending_requests.remove(&reqid)
        };
        if let Some(sender) = response_sender {
            if let Err(_e) = sender.send(response_message) {
                // channel seems to be closed already
                debug!(
                    request_id = reqid.to_hyphenated_string(),
                    "failed to deliver RPC Response message, channel already closed"
                );
            } else {
                debug!(
                    request_id = reqid.to_hyphenated_string(),
                    "successfully delivered RPC Response message"
                )
            }
        } else {
            // we seem to have received a duplicate of the response message, ignoring it ...
            debug!(
                request_id = reqid.to_hyphenated_string(),
                "ignoring (duplicate?) RPC Response message with unknown request ID"
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
            .is_ok_and(|pending_requests| pending_requests.contains_key(reqid))
    }
}

#[async_trait]
impl UListener for ResponseListener {
    async fn on_receive(&self, msg: UMessage) {
        // it is sufficient to check if the message is a response
        // because the transport implementation forwards valid UMessages only
        if msg.is_response() {
            self.handle_response(msg);
        } else {
            debug!(
                message_type = msg.type_unchecked().to_cloudevent_type(),
                "ignoring non-response message received by RPC client"
            );
        }
    }
}

/// An [`RpcClient`] which keeps all information about pending requests in memory.
///
/// The client requires an implementations of [`UTransport`] for sending RPC Request messages
/// to the service implementation and receiving its RPC Response messages.
///
/// During [startup](`Self::new`) the client registers a generic [`UListener`] with the transport
/// for receiving all kinds of messages with a _sink_ address matching the client. The listener
/// maintains an in-memory mapping of (pending) request IDs to response message handlers.
///
/// When an [`RPC call`](Self::invoke_method) is made, an RPC Request message is sent to the service
/// implementation and a response handler is created and registered with the listener.
/// When an RPC Response message arrives from the service, the corresponding handler is being looked
/// up and invoked.
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
    ) -> Result<Self, RegistrationError> {
        let response_listener = Arc::new(ResponseListener {
            pending_requests: Mutex::new(HashMap::new()),
        });
        transport
            .register_listener(
                &UUri::any(),
                Some(&uri_provider.get_source_uri()),
                response_listener.clone(),
            )
            .await
            .map_err(RegistrationError::from)?;

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
        let rpc_request_message = build_message(&mut builder, payload)
            .map_err(|e| ServiceInvocationError::InvalidArgument(e.to_string()))?;

        let receiver = self
            .response_listener
            .try_add_pending_request(message_id.clone())?;
        self.transport
            .send(rpc_request_message)
            .await
            .inspect_err(|_e| {
                self.response_listener.remove_pending_request(&message_id);
            })?;
        debug!(
            request_id = message_id.to_hyphenated_string(),
            ttl = call_options.ttl(),
            "successfully sent RPC Request message"
        );

        match timeout(Duration::from_millis(call_options.ttl() as u64), receiver).await {
            Err(_) => {
                debug!(
                    request_id = message_id.to_hyphenated_string(),
                    ttl = call_options.ttl(),
                    "invocation of service operation has timed out"
                );
                self.response_listener.remove_pending_request(&message_id);
                Err(ServiceInvocationError::DeadlineExceeded)
            }
            Ok(result) => match result {
                Ok(response_message) => handle_response_message(response_message),
                Err(_e) => {
                    debug!(
                        request_id = message_id.to_hyphenated_string(),
                        "response listener failed to forward response message"
                    );
                    self.response_listener.remove_pending_request(&message_id);
                    Err(ServiceInvocationError::Internal(
                        "error receiving response message".to_string(),
                    ))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {

    // [utest->dsn~communication-layer-impl-default~1]

    use super::*;

    use protobuf::well_known_types::wrappers::StringValue;
    use tokio::{join, sync::Notify};

    use crate::{utransport::MockTransport, StaticUriProvider, UMessageBuilder, UPriority, UUri};

    fn new_uri_provider() -> Arc<dyn LocalUriProvider> {
        Arc::new(StaticUriProvider::new("", 0x0005, 0x02))
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
    async fn test_registration_of_response_listener_fails() {
        // GIVEN a transport
        let mut mock_transport = MockTransport::default();
        // with the maximum number of listeners already registered
        mock_transport
            .expect_do_register_listener()
            .once()
            .returning(|_source_filter, _sink_filter, _listener| {
                Err(UStatus::fail_with_code(
                    UCode::RESOURCE_EXHAUSTED,
                    "max number of listeners exceeded",
                ))
            });

        // WHEN trying to create an RpcClient for the transport
        let creation_attempt =
            InMemoryRpcClient::new(Arc::new(mock_transport), new_uri_provider()).await;

        // THEN the attempt fails with a MaxListenersExceeded error
        assert!(
            creation_attempt.is_err_and(|e| matches!(e, RegistrationError::MaxListenersExceeded))
        );
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

        let (captured_listener_tx, captured_listener_rx) = tokio::sync::oneshot::channel();
        let request_sent = Arc::new(Notify::new());
        let request_sent_clone = request_sent.clone();

        // GIVEN an RPC client
        let mut mock_transport = MockTransport::default();
        mock_transport
            .expect_do_register_listener()
            .once()
            .return_once(move |_source_filter, _sink_filter, listener| {
                captured_listener_tx
                    .send(listener)
                    .map_err(|_e| UStatus::fail("cannot capture listener"))
            });
        let expected_message_id = message_id.clone();
        mock_transport
            .expect_do_send()
            .once()
            .withf(move |request_message| {
                request_message.id_unchecked() == &expected_message_id
                    && request_message.priority_unchecked() == UPriority::UPRIORITY_CS6
                    && request_message.ttl_unchecked() == 5_000
                    && request_message.token() == Some(&String::from("my_token"))
            })
            .returning(move |_request_message| {
                request_sent_clone.notify_one();
                Ok(())
            });

        let uri_provider = new_uri_provider();
        let rpc_client = Arc::new(
            InMemoryRpcClient::new(Arc::new(mock_transport), uri_provider.clone())
                .await
                .unwrap(),
        );
        let client: Arc<dyn RpcClient> = rpc_client.clone();

        // WHEN invoking a remote service operation
        let response_handle = tokio::spawn(async move {
            let request_payload = StringValue {
                value: "World".to_string(),
                ..Default::default()
            };
            client
                .invoke_proto_method::<_, StringValue>(
                    service_method_uri(),
                    call_options,
                    request_payload,
                )
                .await
        });

        // AND the remote service sends the corresponding RPC Response message
        let response_payload = StringValue {
            value: "Hello World".to_string(),
            ..Default::default()
        };
        let response_message = UMessageBuilder::response(
            uri_provider.get_source_uri(),
            message_id.clone(),
            service_method_uri(),
        )
        .build_with_protobuf_payload(&response_payload)
        .unwrap();

        // wait for the RPC Request message having been sent
        let (response_listener_result, _) = join!(captured_listener_rx, request_sent.notified());
        let response_listener = response_listener_result.unwrap();

        // send the RPC Response message which completes the request
        let cloned_response_message = response_message.clone();
        let cloned_response_listener = response_listener.clone();
        tokio::spawn(async move {
            cloned_response_listener
                .on_receive(cloned_response_message)
                .await
        });

        // THEN the response contains the expected payload
        let response = response_handle.await.unwrap();
        assert!(response.is_ok_and(|payload| payload.value == *"Hello World"));
        assert!(!rpc_client.contains_pending_request(&message_id));

        // AND if the remote service sends its response message again
        response_listener.on_receive(response_message).await;
        // the duplicate response is silently ignored
        assert!(!rpc_client.contains_pending_request(&message_id));
    }

    #[tokio::test]
    async fn test_invoke_method_fails_on_repeated_invocation() {
        let message_id = UUID::build();
        let first_request_sent = Arc::new(Notify::new());
        let first_request_sent_clone = first_request_sent.clone();

        // GIVEN an RPC client
        let mut mock_transport = MockTransport::default();
        mock_transport
            .expect_do_register_listener()
            .once()
            .return_const(Ok(()));
        let expected_message_id = message_id.clone();
        mock_transport
            .expect_do_send()
            .once()
            .withf(move |request_message| request_message.id_unchecked() == &expected_message_id)
            .returning(move |_request_message| {
                first_request_sent_clone.notify_one();
                Ok(())
            });

        let in_memory_rpc_client = Arc::new(
            InMemoryRpcClient::new(Arc::new(mock_transport), new_uri_provider())
                .await
                .unwrap(),
        );
        let rpc_client: Arc<dyn RpcClient> = in_memory_rpc_client.clone();

        // WHEN invoking a remote service operation
        let call_options =
            CallOptions::for_rpc_request(5_000, Some(message_id.clone()), None, None);
        let cloned_call_options = call_options.clone();
        let cloned_rpc_client = rpc_client.clone();

        tokio::spawn(async move {
            let request_payload = StringValue {
                value: "World".to_string(),
                ..Default::default()
            };
            cloned_rpc_client
                .invoke_proto_method::<_, StringValue>(
                    service_method_uri(),
                    cloned_call_options,
                    request_payload,
                )
                .await
        });

        // we wait for the first request message having been sent via the transport
        // in order to be sure that the pending request has been added to the client's
        // internal state
        first_request_sent.notified().await;

        // AND invoking the same operation before the response to the first request has arrived
        let request_payload = StringValue {
            value: "World".to_string(),
            ..Default::default()
        };
        let second_request_handle = tokio::spawn(async move {
            rpc_client
                .invoke_proto_method::<_, StringValue>(
                    service_method_uri(),
                    call_options,
                    request_payload,
                )
                .await
        });

        // THEN the second invocation fails
        let response = second_request_handle.await.unwrap();
        assert!(response.is_err_and(|e| matches!(e, ServiceInvocationError::AlreadyExists(_))));
        // because there is a pending request for the message ID used in both requests
        assert!(in_memory_rpc_client.contains_pending_request(&message_id));
    }

    #[tokio::test]
    async fn test_invoke_method_fails_with_remote_error() {
        let (captured_listener_tx, captured_listener_rx) = std::sync::mpsc::channel();

        // GIVEN an RPC client
        let mut mock_transport = MockTransport::default();
        mock_transport.expect_do_register_listener().returning(
            move |_source_filter, _sink_filter, listener| {
                captured_listener_tx
                    .send(listener)
                    .map_err(|_e| UStatus::fail("cannot capture listener"))
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
                let captured_listener = captured_listener_rx.recv().unwrap().to_owned();
                tokio::spawn(async move { captured_listener.on_receive(response_message).await });
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
