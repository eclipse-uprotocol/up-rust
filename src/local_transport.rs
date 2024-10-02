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
Provides a local UTransport which can be used for connecting uEntities running in the same
process.
*/

use std::{collections::HashSet, sync::Arc};

use tokio::sync::RwLock;

use crate::{ComparableListener, UListener, UMessage, UStatus, UTransport, UUri};

#[derive(Eq, PartialEq, Hash)]
struct RegisteredListener {
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: ComparableListener,
}

impl RegisteredListener {
    fn matches(&self, source: &UUri, sink: Option<&UUri>) -> bool {
        if !self.source_filter.matches(source) {
            return false;
        }

        if let Some(pattern) = &self.sink_filter {
            sink.map_or(false, |candidate_sink| pattern.matches(candidate_sink))
        } else {
            sink.is_none()
        }
    }
    fn matches_msg(&self, msg: &UMessage) -> bool {
        if let Some(source) = msg
            .attributes
            .as_ref()
            .and_then(|attribs| attribs.source.as_ref())
        {
            self.matches(
                source,
                msg.attributes
                    .as_ref()
                    .and_then(|attribs| attribs.sink.as_ref()),
            )
        } else {
            false
        }
    }
    async fn on_receive(&self, msg: UMessage) {
        self.listener.on_receive(msg).await
    }
}

/// A [`UTransport`] that can be used to exchange messages within a single process.
///
/// A message sent via [`UTransport::send`] will be dispatched to all registered listeners that
/// match the message's source and sink filters.
#[derive(Default)]
pub struct LocalTransport {
    listeners: RwLock<HashSet<RegisteredListener>>,
}

impl LocalTransport {
    async fn dispatch(&self, message: UMessage) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            if listener.matches_msg(&message) {
                listener.on_receive(message.clone()).await;
            }
        }
    }
}

#[async_trait::async_trait]
impl UTransport for LocalTransport {
    async fn send(&self, message: UMessage) -> Result<(), UStatus> {
        self.dispatch(message).await;
        Ok(())
    }

    async fn register_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let registered_listener = RegisteredListener {
            source_filter: source_filter.to_owned(),
            sink_filter: sink_filter.map(|u| u.to_owned()),
            listener: ComparableListener::new(listener),
        };
        let mut listeners = self.listeners.write().await;
        if listeners.contains(&registered_listener) {
            Err(UStatus::fail_with_code(
                crate::UCode::ALREADY_EXISTS,
                "listener already registered for filters",
            ))
        } else {
            listeners.insert(registered_listener);
            Ok(())
        }
    }

    async fn unregister_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        let registered_listener = RegisteredListener {
            source_filter: source_filter.to_owned(),
            sink_filter: sink_filter.map(|u| u.to_owned()),
            listener: ComparableListener::new(listener),
        };
        let mut listeners = self.listeners.write().await;
        if listeners.remove(&registered_listener) {
            Ok(())
        } else {
            Err(UStatus::fail_with_code(
                crate::UCode::NOT_FOUND,
                "no such listener registered for filters",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utransport::MockUListener, LocalUriProvider, StaticUriProvider, UMessageBuilder};

    #[tokio::test]
    async fn test_send_dispatches_to_matching_listener() {
        const RESOURCE_ID: u16 = 0xa1b3;
        let mut listener = MockUListener::new();
        listener.expect_on_receive().once().return_const(());
        let listener_ref = Arc::new(listener);
        let uri_provider = StaticUriProvider::new("my-vehicle", 0x100d, 0x02);
        let transport = LocalTransport::default();

        transport
            .register_listener(
                &uri_provider.get_resource_uri(RESOURCE_ID),
                None,
                listener_ref.clone(),
            )
            .await
            .unwrap();
        let _ = transport
            .send(
                UMessageBuilder::publish(uri_provider.get_resource_uri(RESOURCE_ID))
                    .build()
                    .unwrap(),
            )
            .await;

        transport
            .unregister_listener(
                &uri_provider.get_resource_uri(RESOURCE_ID),
                None,
                listener_ref,
            )
            .await
            .unwrap();
        let _ = transport
            .send(
                UMessageBuilder::publish(uri_provider.get_resource_uri(RESOURCE_ID))
                    .build()
                    .unwrap(),
            )
            .await;
    }

    #[tokio::test]
    async fn test_send_does_not_dispatch_to_non_matching_listener() {
        const RESOURCE_ID: u16 = 0xa1b3;
        let mut listener = MockUListener::new();
        listener.expect_on_receive().never().return_const(());
        let listener_ref = Arc::new(listener);
        let uri_provider = StaticUriProvider::new("my-vehicle", 0x100d, 0x02);
        let transport = LocalTransport::default();

        transport
            .register_listener(
                &uri_provider.get_resource_uri(RESOURCE_ID + 10),
                None,
                listener_ref.clone(),
            )
            .await
            .unwrap();
        let _ = transport
            .send(
                UMessageBuilder::publish(uri_provider.get_resource_uri(RESOURCE_ID))
                    .build()
                    .unwrap(),
            )
            .await;

        transport
            .unregister_listener(
                &uri_provider.get_resource_uri(RESOURCE_ID + 10),
                None,
                listener_ref,
            )
            .await
            .unwrap();
        let _ = transport
            .send(
                UMessageBuilder::publish(uri_provider.get_resource_uri(RESOURCE_ID))
                    .build()
                    .unwrap(),
            )
            .await;
    }
}
