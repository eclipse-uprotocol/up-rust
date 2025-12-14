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
This example illustrates how the functionality from the symphony feature can be used to implement
an Eclipse Symphony Target.
 */

use std::{collections::HashMap, sync::Arc, time::Duration};

use serde_json::json;
use symphony::models::{ComponentResultSpec, ComponentSpec, DeploymentSpec};
use tokio::sync::Notify;
use up_rust::{
    communication::{CallOptions, InMemoryRpcClient, InMemoryRpcServer, RpcClient, UPayload},
    local_transport::LocalTransport,
    symphony::DeploymentTarget,
    StaticUriProvider, UPayloadFormat, UUri,
};

struct ExampleDeploymentTarget(Arc<Notify>, Arc<Notify>, Arc<Notify>);

#[async_trait::async_trait]
impl DeploymentTarget for ExampleDeploymentTarget {
    async fn get(
        &self,
        _components: Vec<ComponentSpec>,
        _deployment_spec: DeploymentSpec,
    ) -> Result<Vec<ComponentSpec>, Box<dyn std::error::Error>> {
        self.0.notify_one();
        Ok(vec![])
    }

    async fn update(
        &self,
        _components_to_update: Vec<ComponentSpec>,
        _deployment_spec: DeploymentSpec,
    ) -> Result<HashMap<String, ComponentResultSpec>, Box<dyn std::error::Error>> {
        self.1.notify_one();
        Ok(HashMap::new())
    }

    async fn delete(
        &self,
        _components_to_delete: Vec<ComponentSpec>,
        _deployment_spec: DeploymentSpec,
    ) -> Result<HashMap<String, ComponentResultSpec>, Box<dyn std::error::Error>> {
        self.2.notify_one();
        Ok(HashMap::new())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(LocalTransport::default());

    let get_method = UUri::try_from_parts(
        "local_authority",
        0xAAA1,
        0x01,
        up_rust::symphony::METHOD_GET_RESOURCE_ID,
    )
    .expect("failed to create get method URI");
    let update_method = UUri::try_from_parts(
        "local_authority",
        0xAAA1,
        0x01,
        up_rust::symphony::METHOD_UPDATE_RESOURCE_ID,
    )
    .expect("failed to create update method URI");
    let delete_method = UUri::try_from_parts(
        "local_authority",
        0xAAA1,
        0x01,
        up_rust::symphony::METHOD_DELETE_RESOURCE_ID,
    )
    .expect("failed to create delete method URI");
    let uri_provider =
        StaticUriProvider::try_from(&get_method).expect("failed to create URI provider");
    let rpc_server = InMemoryRpcServer::new(transport.clone(), Arc::new(uri_provider));

    let get_notify = Arc::new(Notify::new());
    let update_notify = Arc::new(Notify::new());
    let delete_notify = Arc::new(Notify::new());

    let target = ExampleDeploymentTarget(
        get_notify.clone(),
        update_notify.clone(),
        delete_notify.clone(),
    );
    up_rust::symphony::register_target_provider_endpoints(&rpc_server, Arc::new(target))
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
    )?;
    Ok(())
}
