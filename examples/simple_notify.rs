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

use bytes::Bytes;
use up_rust::{
    communication::{CallOptions, Notifier, SimpleNotifier, UPayload},
    local_transport::LocalTransport,
    LocalUriProvider, StaticUriProvider, UListener, UMessage,
};

struct ConsolePrinter {}

#[async_trait::async_trait]
impl UListener for ConsolePrinter {
    async fn on_receive(&self, msg: UMessage) {
        if let Some(payload) = msg.payload() {
            let msg = String::from_utf8_lossy(payload.as_ref());
            println!("received notification: {}", msg);
        }
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const ORIGIN_RESOURCE_ID: u16 = 0xd100;

    let uri_provider = Arc::new(StaticUriProvider::new("my-vehicle", 0xa34b, 0x01)?);
    // using the LocalTransport here allows us to run the client and server in the same process
    // without any network communication, which is useful for testing purposes
    // in a real application, you would use a transport that employs the network to communicate
    // between the client and server, such as the MQTT5 or the Eclipse Zenoh transport
    let transport = Arc::new(LocalTransport::default());
    let notifier = SimpleNotifier::new(transport, uri_provider.clone());
    let topic = uri_provider.get_resource_uri(ORIGIN_RESOURCE_ID);
    let listener = Arc::new(ConsolePrinter {});

    notifier.start_listening(&topic, listener.clone()).await?;

    let payload = UPayload::new(Bytes::from("Hello"), up_rust::UPayloadFormat::Text);
    notifier
        .notify(
            ORIGIN_RESOURCE_ID,
            &uri_provider.get_source_uri(),
            CallOptions::for_notification(None, None, None),
            Some(payload),
        )
        .await?;

    // At this point we can be sure that all notifications have been processed already.
    // This is because the LocalTransport dispatches all messages to listeners on the same
    // thread that has been used to send the messages.
    // When using an asynchronous transport, such as MQTT5 or Eclipse Zenoh, we would need to
    // notify the sender from within the listener, e.g. by means of a Channel, before stopping
    // the listener.
    notifier.stop_listening(&topic, listener).await?;
    Ok(())
}
