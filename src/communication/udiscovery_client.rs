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

use async_trait::async_trait;

use crate::up_core_api::{
    udiscovery::{
        FindServicesRequest, FindServicesResponse, GetServiceTopicsRequest,
        GetServiceTopicsResponse,
    },
    uri::UUri as UUriProto,
};
use crate::{
    core::udiscovery::{
        TopicInfo, UDiscovery, RESOURCE_ID_FIND_SERVICES, RESOURCE_ID_GET_SERVICE_TOPICS,
        UDISCOVERY_TYPE_ID, UDISCOVERY_VERSION_MAJOR,
    },
    UStatus, UUri,
};

use super::{CallOptions, RpcClient};

/// Gets a UUri referring to one of the _local_ uDiscovery service's resources.
fn udiscovery_uri(resource_id: u16) -> UUri {
    UUri::try_from_parts(
        "",
        UDISCOVERY_TYPE_ID,
        UDISCOVERY_VERSION_MAJOR,
        resource_id,
    )
    .unwrap()
}

/// A [`UDiscovery`] client implementation for invoking operations of a local uDiscovery service.
///
/// The client requires an [`RpcClient`] for performing the remote procedure calls.
pub struct RpcClientUDiscovery {
    rpc_client: Arc<dyn RpcClient>,
}

impl RpcClientUDiscovery {
    /// Creates a new uDiscovery client for a given transport.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - The client to use for performing the remote procedure calls on the service.
    pub fn new(rpc_client: Arc<dyn RpcClient>) -> Self {
        RpcClientUDiscovery { rpc_client }
    }

    fn default_call_options() -> CallOptions {
        CallOptions::for_rpc_request(5_000, None, None, None)
    }
}

#[async_trait]
impl UDiscovery for RpcClientUDiscovery {
    async fn find_services(
        &self,
        uri_pattern: UUri,
        recursive: bool,
    ) -> Result<Vec<UUri>, UStatus> {
        let request_message = FindServicesRequest {
            uri: Some(UUriProto::from(&uri_pattern)).into(),
            recursive,
            ..Default::default()
        };
        self.rpc_client
            .invoke_proto_method::<_, FindServicesResponse>(
                udiscovery_uri(RESOURCE_ID_FIND_SERVICES),
                Self::default_call_options(),
                request_message,
            )
            .await
            .map(|response_message| {
                response_message.uris.as_ref().map_or(vec![], |batch| {
                    batch
                        .uris
                        .iter()
                        .filter_map(|uri| UUri::try_from(uri).ok())
                        .collect()
                })
            })
            .map_err(UStatus::from)
    }

    async fn get_service_topics(
        &self,
        topic_pattern: UUri,
        recursive: bool,
    ) -> Result<Vec<TopicInfo>, UStatus> {
        let request_message = GetServiceTopicsRequest {
            topic: Some(UUriProto::from(&topic_pattern)).into(),
            recursive,
            ..Default::default()
        };
        self.rpc_client
            .invoke_proto_method::<_, GetServiceTopicsResponse>(
                udiscovery_uri(RESOURCE_ID_GET_SERVICE_TOPICS),
                Self::default_call_options(),
                request_message,
            )
            .await
            .map(|response_message| {
                response_message
                    .topics
                    .iter()
                    .filter_map(|topic| TopicInfo::try_from(topic).ok())
                    .collect()
            })
            .map_err(UStatus::from)
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;

    use super::*;
    use crate::{
        communication::{rpc::MockRpcClient, UPayload},
        up_core_api::{udiscovery::ServiceTopicInfo, uoptions::UServiceTopic, uri::UUriBatch},
        UCode, UUri,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_find_services_invokes_rpc_client() {
        let service_pattern_uri = UUri::try_from_parts("other", 0xFFFF_D5A3, 0x01, 0xFFFF)
            .expect("failed to create service pattern URI");
        let request = FindServicesRequest {
            uri: Some(UUriProto::from(&service_pattern_uri)).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &udiscovery_uri(RESOURCE_ID_FIND_SERVICES) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<FindServicesRequest>()
                    .unwrap();
                request == expected_request && method == &udiscovery_uri(RESOURCE_ID_FIND_SERVICES)
            })
            .returning(move |_method, _options, _payload| {
                let response = FindServicesResponse {
                    uris: Some(UUriBatch {
                        uris: vec![UUriProto {
                            authority_name: "other".to_string(),
                            ue_id: 0x0004_D5A3,
                            ue_version_major: 0x01,
                            resource_id: 0xD3FE,
                            ..Default::default()
                        }],
                        ..Default::default()
                    })
                    .into(),
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let udiscovery_client = RpcClientUDiscovery::new(Arc::new(rpc_client));

        assert!(udiscovery_client
            .find_services(service_pattern_uri.clone(), false)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(udiscovery_client
            .find_services(service_pattern_uri.clone(), false)
            .await
            .is_ok_and(
                |result| result.len() == 1 && service_pattern_uri.matches(result.first().unwrap())
            ));
    }

    #[tokio::test]
    async fn test_get_service_topics_invokes_rpc_client() {
        let topic_pattern_uri = UUri::try_from_parts("*", 0xFFFF_D5A3, 0x01, 0xFFFF)
            .expect("failed to create topic pattern URI");
        let request = GetServiceTopicsRequest {
            topic: Some((&topic_pattern_uri).into()).into(),
            ..Default::default()
        };
        let expected_request = request.clone();
        let mut rpc_client = MockRpcClient::new();
        let mut seq = Sequence::new();
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(|method, _options, payload| {
                method == &udiscovery_uri(RESOURCE_ID_GET_SERVICE_TOPICS) && payload.is_some()
            })
            .return_const(Err(crate::communication::ServiceInvocationError::Internal(
                "internal error".to_string(),
            )));
        rpc_client
            .expect_invoke_method()
            .once()
            .in_sequence(&mut seq)
            .withf(move |method, _options, payload| {
                let request = payload
                    .to_owned()
                    .unwrap()
                    .extract_protobuf::<GetServiceTopicsRequest>()
                    .unwrap();
                request == expected_request
                    && method == &udiscovery_uri(RESOURCE_ID_GET_SERVICE_TOPICS)
            })
            .returning(move |_method, _options, _payload| {
                let topic_info = ServiceTopicInfo {
                    topic: Some(UUriProto {
                        authority_name: "other".to_string(),
                        ue_id: 0x0004_D5A3,
                        ue_version_major: 0x01,
                        resource_id: 0xD3FE,
                        ..Default::default()
                    })
                    .into(),
                    info: Some(UServiceTopic {
                        id: 0x9000,
                        name: "TestTopic".to_string(),
                        message: "TestTopicMessage".to_string(),
                        ..Default::default()
                    })
                    .into(),
                    ttl: 600,
                    ..Default::default()
                };
                let response = GetServiceTopicsResponse {
                    topics: vec![topic_info],
                    ..Default::default()
                };
                Ok(Some(UPayload::try_from_protobuf(response).unwrap()))
            });

        let udiscovery_client = RpcClientUDiscovery::new(Arc::new(rpc_client));

        assert!(udiscovery_client
            .get_service_topics(topic_pattern_uri.clone(), false)
            .await
            .is_err_and(|e| e.get_code() == UCode::Internal));
        assert!(udiscovery_client
            .get_service_topics(topic_pattern_uri.clone(), false)
            .await
            .is_ok_and(|result| result.len() == 1
                && topic_pattern_uri.matches(result.first().map(|r| r.topic()).unwrap())));
    }
}
