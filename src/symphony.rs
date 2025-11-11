/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
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
Types and helpers that allow implementing an Eclipse Symphony&trade; Target Provider as a uService
exposed via the Communication Layer API's `RpcServer`.
*/

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use symphony::models::{ComponentResultSpec, ComponentSpec, DeploymentSpec};
use tracing::{debug, error, trace, warn, Level};

use crate::{
    communication::{RequestHandler, RpcServer, ServiceInvocationError, UPayload},
    UAttributes, UPayloadFormat,
};

pub const METHOD_GET_RESOURCE_ID: u16 = 0x0001;
pub const METHOD_UPDATE_RESOURCE_ID: u16 = 0x0002;
pub const METHOD_DELETE_RESOURCE_ID: u16 = 0x0003;

/// Registers RPC endpoints for managing a deployment target via Eclipse Symphony's uProtocol
/// Target Provider.
///
/// This function registers three RPC endpoints that delegate to the provided [`DeploymentTarget`] implementation:
/// - `Get` (resource ID `0x0001`) - Retrieves current component status
/// - `Update` (resource ID `0x0002`) - Updates deployment components  
/// - `Delete` (resource ID `0x0003`) - Removes deployment components
///
/// # Arguments
/// * `rpc_server` - The RPC server to register the endpoints on
/// * `deployment_target` - The deployment target implementation to delegate requests to
///
/// # Errors
/// Returns an error if any of the endpoints cannot be registered on the RPC server.
pub async fn register_target_provider_endpoints<R: RpcServer, T: DeploymentTarget + 'static>(
    rpc_server: &R,
    deployment_target: Arc<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    let get_op = Arc::new(GetOperation {
        target: deployment_target.clone(),
    });
    let apply_op = Arc::new(ApplyOperation {
        target: deployment_target,
    });
    rpc_server
        .register_endpoint(None, METHOD_GET_RESOURCE_ID, get_op)
        .await
        .inspect_err(|e| error!("failed to register Get operation on RPC Server: {e}"))?;
    rpc_server
        .register_endpoint(None, METHOD_UPDATE_RESOURCE_ID, apply_op.clone())
        .await
        .inspect_err(|e| error!("failed to register Update operation on RPC Server: {e}"))?;
    rpc_server
        .register_endpoint(None, METHOD_DELETE_RESOURCE_ID, apply_op)
        .await
        .inspect_err(|e| error!("failed to register Delete operation on RPC Server: {e}"))?;
    Ok(())
}

#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
#[async_trait]
pub trait DeploymentTarget: Send + Sync {
    /// Retrieves the current status of components within a deployment.
    ///
    /// # Arguments
    /// * `components` - The components whose current status should be retrieved
    /// * `deployment_spec` - The deployment context containing these components
    ///
    /// # Returns
    /// A vector of [`ComponentSpec`] representing the currently deployed state of the requested components.
    ///
    /// # Errors
    /// Returns an error if the current deployment status cannot be determined.
    async fn get(
        &self,
        components: Vec<ComponentSpec>,
        deployment_spec: DeploymentSpec,
    ) -> Result<Vec<ComponentSpec>, Box<dyn std::error::Error>>;

    /// Updates the specified components within a deployment.
    ///
    /// # Arguments
    /// * `components_to_update` - The components to be updated
    /// * `deployment_spec` - The deployment context for these components
    ///
    /// # Returns
    /// A map where keys are component identifiers and values are [`ComponentResultSpec`]
    /// indicating the outcome of each component's update operation.
    ///
    /// # Errors
    /// Returns an error if the update operation cannot be performed. Individual component
    /// failures should be reported in the returned map rather than as an overall error.
    async fn update(
        &self,
        components_to_update: Vec<ComponentSpec>,
        deployment_spec: DeploymentSpec,
    ) -> Result<HashMap<String, ComponentResultSpec>, Box<dyn std::error::Error>>;

    /// Removes the specified components from a deployment.
    ///
    /// # Arguments
    /// * `components_to_delete` - The components to be removed
    /// * `deployment_spec` - The deployment context for these components
    ///
    /// # Returns
    /// A map where keys are component identifiers and values are [`ComponentResultSpec`]
    /// indicating the outcome of each component's deletion operation.
    ///
    /// # Errors
    /// Returns an error if the delete operation cannot be performed. Individual component
    /// failures should be reported in the returned map rather than as an overall error.
    async fn delete(
        &self,
        components_to_delete: Vec<ComponentSpec>,
        deployment_spec: DeploymentSpec,
    ) -> Result<HashMap<String, ComponentResultSpec>, Box<dyn std::error::Error>>;
}

fn extract_request_data(
    request_payload: Option<UPayload>,
) -> Result<Value, ServiceInvocationError> {
    let Some(req_payload) = request_payload
        .filter(|req_payload| req_payload.payload_format() == UPayloadFormat::UPAYLOAD_FORMAT_JSON)
    else {
        return Err(ServiceInvocationError::InvalidArgument(
            "request has no JSON payload".to_string(),
        ));
    };

    serde_json::from_slice(req_payload.payload().to_vec().as_slice()).map_err(|err| {
        debug!("failed to deserialize request payload: {:?}", err);
        ServiceInvocationError::InvalidArgument(
            "request payload is not a valid UTF-8 string".to_string(),
        )
    })
}

struct GetOperation<T: DeploymentTarget> {
    target: Arc<T>,
}

#[async_trait::async_trait]
impl<T: DeploymentTarget> RequestHandler for GetOperation<T> {
    // expects a DeploymentSpec in the request and returns an array of ComponentSpecs
    async fn handle_request(
        &self,
        _resource_id: u16,
        message_attributes: &UAttributes,
        request_payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        let source_uri = message_attributes.source_unchecked().to_uri(true);
        if tracing::enabled!(Level::DEBUG) {
            debug!(source = source_uri, "processing GET request");
        }
        let request_data = extract_request_data(request_payload)?;
        if tracing::enabled!(Level::TRACE) {
            trace!(
                source = source_uri,
                "payload: {}",
                serde_json::to_string_pretty(&request_data).expect("failed to serialize Value")
            );
        }
        let deployment_spec: DeploymentSpec =
            serde_json::from_value(request_data["deployment"].clone()).map_err(|err| {
                debug!(
                    source = source_uri,
                    "request does not contain DeploymentSpec: {err}"
                );
                ServiceInvocationError::InvalidArgument(
                    "request does not contain DeploymentSpec".to_string(),
                )
            })?;
        let component_specs: Vec<ComponentSpec> =
            serde_json::from_value(request_data["components"].clone()).map_err(|err| {
                debug!(
                    source = source_uri,
                    "request does not contain ComponentSpec array: {err}"
                );
                ServiceInvocationError::InvalidArgument(
                    "request does not contain ComponentSpec array".to_string(),
                )
            })?;

        let result = self
            .target
            .get(component_specs, deployment_spec)
            .await
            .map_err(|err| {
                warn!(source = source_uri, "error getting component status: {err}");
                ServiceInvocationError::Internal("failed to get component status".to_string())
            })?;
        let serialized_response_data = serde_json::to_vec(&result).map_err(|err| {
            warn!(
                source = source_uri,
                "error serializing ComponentSpec: {err}"
            );
            ServiceInvocationError::Internal("failed to create response payload".to_string())
        })?;
        if tracing::enabled!(Level::TRACE) {
            trace!(
                source = source_uri,
                "returning response: {}",
                serde_json::to_string_pretty(&result).expect("failed to serialize Value")
            );
        }
        let response_payload = UPayload::new(
            serialized_response_data,
            UPayloadFormat::UPAYLOAD_FORMAT_JSON,
        );
        Ok(Some(response_payload))
    }
}

struct ApplyOperation<T: DeploymentTarget> {
    target: Arc<T>,
}

#[async_trait::async_trait]
impl<T: DeploymentTarget> RequestHandler for ApplyOperation<T> {
    async fn handle_request(
        &self,
        resource_id: u16,
        message_attributes: &UAttributes,
        request_payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        let source_uri = message_attributes.source_unchecked().to_uri(true);
        let sink_uri = message_attributes.sink_unchecked().to_uri(true);
        if tracing::enabled!(Level::DEBUG) {
            debug!(source = source_uri, method = sink_uri, "processing request",);
        }
        let request_data = extract_request_data(request_payload)?;
        if tracing::enabled!(Level::TRACE) {
            let json =
                serde_json::to_string_pretty(&request_data).expect("failed to serialize Value");
            trace!("payload: {}", json);
        }

        let deployment_spec: DeploymentSpec =
            serde_json::from_value(request_data["deployment"].clone()).map_err(|err| {
                debug!(
                    source = source_uri,
                    method = sink_uri,
                    "request does not contain DeploymentSpec: {err}"
                );
                ServiceInvocationError::InvalidArgument(
                    "request does not contain DeploymentSpec".to_string(),
                )
            })?;

        let affected_components: Vec<ComponentSpec> =
            serde_json::from_value(request_data["components"].clone()).map_err(|err| {
                debug!(
                    source = source_uri,
                    method = sink_uri,
                    "request does not contain ComponentSpec array: {err}"
                );
                ServiceInvocationError::InvalidArgument(
                    "request does not contain ComponentSpec array".to_string(),
                )
            })?;

        let result = match resource_id {
            METHOD_UPDATE_RESOURCE_ID => self
                .target
                .update(affected_components, deployment_spec)
                .await
                .map_err(|err| {
                    warn!(
                        source = source_uri,
                        method = sink_uri,
                        "error updating components: {err}"
                    );
                    ServiceInvocationError::Internal("failed to update components".to_string())
                }),
            METHOD_DELETE_RESOURCE_ID => self
                .target
                .delete(affected_components, deployment_spec)
                .await
                .map_err(|err| {
                    warn!(
                        source = source_uri,
                        method = sink_uri,
                        "error deleting components: {err}"
                    );
                    ServiceInvocationError::Internal("failed to delete components".to_string())
                }),
            _ => {
                return Err(ServiceInvocationError::Unimplemented(
                    "no such operation".to_string(),
                ));
            }
        }?;

        let serialized_response_data = serde_json::to_vec(&result).map_err(|err| {
            warn!(
                source = source_uri,
                method = sink_uri,
                "error serializing HashMap: {err}"
            );
            ServiceInvocationError::Internal("failed to create response payload".to_string())
        })?;

        let response_payload = UPayload::new(
            serialized_response_data,
            UPayloadFormat::UPAYLOAD_FORMAT_JSON,
        );
        Ok(Some(response_payload))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::Notify;

    use crate::{
        communication::{
            CallOptions, InMemoryRpcClient, InMemoryRpcServer, MockRpcServerImpl, RpcClient,
        },
        local_transport::LocalTransport,
        StaticUriProvider, UUri,
    };

    use super::*;

    #[tokio::test]
    async fn test_register_target_provider_endpoints_fails() {
        let mut rpc_server = MockRpcServerImpl::new();
        rpc_server
            .expect_do_register_endpoint()
            .returning(|_, _, _| {
                Err(crate::communication::RegistrationError::MaxListenersExceeded)
            });
        let deployment_target = MockDeploymentTarget::new();

        assert!(
            register_target_provider_endpoints(&rpc_server, Arc::new(deployment_target))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_endpoints_delegate_to_deployment_target() {
        let transport = Arc::new(LocalTransport::default());

        let get_method =
            UUri::try_from_parts("local_authority", 0xAAA1, 0x01, METHOD_GET_RESOURCE_ID)
                .expect("failed to create get method URI");
        let update_method =
            UUri::try_from_parts("local_authority", 0xAAA1, 0x01, METHOD_UPDATE_RESOURCE_ID)
                .expect("failed to create update method URI");
        let delete_method =
            UUri::try_from_parts("local_authority", 0xAAA1, 0x01, METHOD_DELETE_RESOURCE_ID)
                .expect("failed to create delete method URI");
        let uri_provider =
            StaticUriProvider::try_from(&get_method).expect("failed to create URI provider");
        let rpc_server = InMemoryRpcServer::new(transport.clone(), Arc::new(uri_provider));

        let mut mock_target = MockDeploymentTarget::default();
        let get_notify = Arc::new(Notify::new());
        let cloned_get_notify = get_notify.clone();
        mock_target.expect_get().returning(move |_, _| {
            cloned_get_notify.notify_one();
            Ok(vec![])
        });
        let update_notify = Arc::new(Notify::new());
        let cloned_update_notify = update_notify.clone();
        mock_target.expect_update().returning(move |_, _| {
            cloned_update_notify.notify_one();
            Ok(HashMap::new())
        });
        let delete_notify = Arc::new(Notify::new());
        let cloned_delete_notify = delete_notify.clone();
        mock_target.expect_delete().returning(move |_, _| {
            cloned_delete_notify.notify_one();
            Ok(HashMap::new())
        });
        register_target_provider_endpoints(&rpc_server, Arc::new(mock_target))
            .await
            .expect("failed to register endpoints");

        let rpc_client = InMemoryRpcClient::new(
            transport.clone(),
            Arc::new(StaticUriProvider::new("local_authority", 0xAAA2, 0x01)),
        )
        .await
        .expect("failed to create RPC client");

        let request_payload = json!({
            "deployment": DeploymentSpec::empty(),
            "components": []
        });
        let payload = UPayload::new(
            serde_json::to_vec(&request_payload).expect("failed to create request payload"),
            UPayloadFormat::UPAYLOAD_FORMAT_JSON,
        );
        let call_options = CallOptions::for_rpc_request(0x1000, None, None, None);
        rpc_client
            .invoke_method(get_method, call_options.clone(), Some(payload.clone()))
            .await
            .expect("Get invocation failed");
        rpc_client
            .invoke_method(update_method, call_options.clone(), Some(payload.clone()))
            .await
            .expect("Update invocation failed");
        rpc_client
            .invoke_method(delete_method, call_options, Some(payload))
            .await
            .expect("Delete invocation failed");

        tokio::try_join!(
            tokio::time::timeout(Duration::from_secs(2), get_notify.notified()),
            tokio::time::timeout(Duration::from_secs(2), update_notify.notified()),
            tokio::time::timeout(Duration::from_secs(2), delete_notify.notified()),
        )
        .expect("failed to receive notification from deployment target");
    }
}
