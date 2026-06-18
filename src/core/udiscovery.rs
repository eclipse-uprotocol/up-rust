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

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use crate::{UStatus, UUri};

#[cfg(all(feature = "up-l2-rpc-client", feature = "up-core-types"))]
mod udiscovery_client;
#[cfg(all(feature = "up-l2-rpc-client", feature = "up-core-types"))]
pub use udiscovery_client::RpcClientUDiscovery;

/// The uEntity (type) identifier of the uDiscovery service.
pub const UDISCOVERY_TYPE_ID: u32 = 0x0000_0001;
/// The (latest) major version of the uDiscovery service.
pub const UDISCOVERY_VERSION_MAJOR: u8 = 0x03;
/// The resource identifier of uDiscovery's _find services_ operation.
pub const RESOURCE_ID_FIND_SERVICES: u16 = 0x0001;
/// The resource identifier of uDiscovery's _get service topics_ operation.
pub const RESOURCE_ID_GET_SERVICE_TOPICS: u16 = 0x0002;

type MessageTypeString = String;

#[derive(Clone, Debug)]
#[repr(C)]
pub struct TopicInfo {
    pub(crate) topic: UUri,
    pub(crate) message_type: MessageTypeString,
    pub(crate) permission_level: Option<u32>,
    pub(crate) ttl: u32,
}

impl TopicInfo {
    /// Gets the topic URI.
    pub fn topic(&self) -> &UUri {
        &self.topic
    }

    /// Gets the message type string.
    pub fn message_type(&self) -> &str {
        self.message_type.as_str()
    }

    /// Gets the permission level, if any.
    pub fn permission_level(&self) -> Option<u32> {
        self.permission_level
    }

    /// Gets the time-to-live (TTL) value in seconds.
    pub fn ttl(&self) -> u32 {
        self.ttl
    }
}

/// The uProtocol Application Layer client interface to the uDiscovery service.
///
/// Please refer to the [uDiscovery service specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l3/udiscovery/v3/client.adoc)
/// for details.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait UDiscovery: Send + Sync {
    /// Finds service instances based on search criteria.
    ///
    /// # Parameters
    ///
    /// * `uri_pattern` - The URI pattern to use for looking up service instances.
    /// * `recursive` - Flag indicating whether the service should extend the search to its parent uDiscovery node.
    ///
    /// # Returns
    ///
    /// The service instances matching the given search criteria.
    async fn find_services(&self, uri_pattern: UUri, recursive: bool)
        -> Result<Vec<UUri>, UStatus>;

    /// Gets information about topic(s) that a service (instance) publishes messages to.
    ///
    /// # Parameters
    ///
    /// * `topic_pattern` - The URI pattern to use for looking up topic information.
    /// * `recursive` - Flag indicating whether the service should extend the search to its parent uDiscovery node.
    ///
    /// # Returns
    ///
    /// The topics.
    async fn get_service_topics(
        &self,
        topic_pattern: UUri,
        recursive: bool,
    ) -> Result<Vec<TopicInfo>, UStatus>;
}
