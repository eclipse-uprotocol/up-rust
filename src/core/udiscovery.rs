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

pub use crate::up_core_api::udiscovery::{
    FindServicesRequest, FindServicesResponse, GetServiceTopicsRequest, GetServiceTopicsResponse,
    ServiceTopicInfo,
};
use crate::{UStatus, UUri};

/// The uEntity (type) identifier of the uDiscovery service.
pub const UDISCOVERY_TYPE_ID: u32 = 0x0000_0001;
/// The (latest) major version of the uDiscovery service.
pub const UDISCOVERY_VERSION_MAJOR: u8 = 0x03;
/// The resource identifier of uDiscovery's _find services_ operation.
pub const RESOURCE_ID_FIND_SERVICES: u16 = 0x0001;
/// The resource identifier of uDiscovery's _get service topics_ operation.
pub const RESOURCE_ID_GET_SERVICE_TOPICS: u16 = 0x0002;

/// Gets a UUri referring to one of the local uDiscovery service's resources.
///
/// # Examples
///
/// ```rust
/// use up_rust::core::udiscovery;
///
/// let uuri = udiscovery::udiscovery_uri(udiscovery::RESOURCE_ID_FIND_SERVICES);
/// assert_eq!(uuri.resource_id, 0x0001);
/// ```
pub fn udiscovery_uri(resource_id: u16) -> UUri {
    UUri::try_from_parts(
        "",
        UDISCOVERY_TYPE_ID,
        UDISCOVERY_VERSION_MAJOR,
        resource_id,
    )
    .unwrap()
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
    ) -> Result<Vec<ServiceTopicInfo>, UStatus>;
}
