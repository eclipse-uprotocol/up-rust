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

/*!
This example illustrates how client code can use the _Communication Level API_ to invoke a
service operation. It also shows how the corresponding service provider can be implemented.
 */
use std::sync::Arc;

use protobuf::well_known_types::wrappers::StringValue;
use up_rust::{
    communication::{
        CallOptions, InMemoryRpcClient, InMemoryRpcServer, RequestHandler, RpcClient, RpcServer,
        ServiceInvocationError, UPayload,
    },
    local_transport::LocalTransport,
    LocalUriProvider,
};

struct EchoOperation {}

#[async_trait::async_trait]
impl RequestHandler for EchoOperation {
    async fn handle_request(
        &self,
        _resource_id: u16,
        request_payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        if let Some(req_payload) = request_payload {
            Ok(Some(req_payload))
        } else {
            Err(ServiceInvocationError::InvalidArgument(
                "request has no payload".to_string(),
            ))
        }
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const METHOD_RESOURCE_ID: u16 = 0x00a0;
    let transport = Arc::new(LocalTransport::new("my-vehicle", 0xa34b, 0x01));
    let uri_provider: Arc<dyn LocalUriProvider> = transport.clone();

    // create the RpcServer using the local transport
    let rpc_server = InMemoryRpcServer::new(transport.clone(), uri_provider.clone());
    // and register an endpoint for the service operation
    let echo_op = Arc::new(EchoOperation {});

    rpc_server
        .register_endpoint(None, METHOD_RESOURCE_ID, echo_op.clone())
        .await?;

    // now create an RpcClient attached to the same local transport
    let rpc_client = InMemoryRpcClient::new(transport.clone(), uri_provider.clone()).await?;
    // and invoke the service operation without any payload
    match rpc_client
        .invoke_method(
            uri_provider.get_resource_uri(METHOD_RESOURCE_ID),
            CallOptions::for_rpc_request(1_000, None, None, None),
            None, // no payload
        )
        .await
    {
        Err(ServiceInvocationError::InvalidArgument(msg)) => {
            println!("service returned expected error: {}", msg)
        }
        _ => panic!("expected service to return an Invalid Argument error"),
    }

    // now invoke the operaiton with a message in the request payload
    let value = StringValue {
        value: "Hello".to_string(),
        ..Default::default()
    };
    let payload = UPayload::try_from_protobuf(value)?;
    // and make sure that the response contains a message in the payload
    match rpc_client
        .invoke_method(
            uri_provider.get_resource_uri(METHOD_RESOURCE_ID),
            CallOptions::for_rpc_request(1_000, None, None, None),
            Some(payload),
        )
        .await
    {
        Ok(Some(payload)) => {
            let value = payload.extract_protobuf::<StringValue>()?;
            println!("service returned message: {}", value.value);
        }
        _ => panic!("expected service to return response message"),
    }

    // and finally unregister the endpoint
    rpc_server
        .unregister_endpoint(None, METHOD_RESOURCE_ID, echo_op)
        .await?;

    Ok(())
}
