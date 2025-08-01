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

use protobuf::well_known_types::wrappers::StringValue;
use up_rust::{
    communication::{CallOptions, Publisher, SimplePublisher, UPayload},
    local_transport::LocalTransport,
    LocalUriProvider, StaticUriProvider, UListener, UMessage, UTransport,
};

struct ConsolePrinter {}

#[async_trait::async_trait]
impl UListener for ConsolePrinter {
    async fn on_receive(&self, msg: UMessage) {
        if let Ok(payload) = msg.extract_protobuf::<StringValue>() {
            println!("received event: {}", payload.value);
        }
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const ORIGIN_RESOURCE_ID: u16 = 0xb4c1;
    let uri_provider = Arc::new(StaticUriProvider::new("my-vehicle", 0xa34b, 0x01));
    // using the LocalTransport here allows us to run the client and server in the same process
    // without any network communication, which is useful for testing purposes
    // in a real application, you would use a transport that employs the network to communicate
    // between the client and server, such as the MQTT5 or the Eclipse Zenoh transport
    let transport = Arc::new(LocalTransport::default());
    let publisher = SimplePublisher::new(transport.clone(), uri_provider.clone());
    let listener = Arc::new(ConsolePrinter {});

    transport
        .register_listener(
            &uri_provider.get_resource_uri(ORIGIN_RESOURCE_ID),
            None,
            listener.clone(),
        )
        .await?;

    let value = StringValue {
        value: "Hello".to_string(),
        ..Default::default()
    };
    let payload = UPayload::try_from_protobuf(value)?;
    publisher
        .publish(
            ORIGIN_RESOURCE_ID,
            CallOptions::for_publish(None, None, None),
            Some(payload),
        )
        .await?;

    transport
        .unregister_listener(
            &uri_provider.get_resource_uri(ORIGIN_RESOURCE_ID),
            None,
            listener,
        )
        .await?;

    Ok(())
}
