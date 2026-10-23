/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
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
Convenience conversion wrappers up-rust/protobuf message conversion for uSubscription server functions.
*/

use crate::{
    communication::UPayload,
    core::usubscription::{
        FetchSubscriptionsRequest, FetchSubscriptionsResponse, SubscribeRequest, SubscribeResponse,
        UnsubscribeRequest, RESOURCE_ID_FETCH_SUBSCRIPTIONS,
        RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_RESET, RESOURCE_ID_SUBSCRIBE,
        RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS, RESOURCE_ID_UNSUBSCRIBE,
    },
    up_core_api::usubscription::{
        FetchSubscriptionsRequest as FetchSubscriptionsRequestProto,
        FetchSubscriptionsResponse as FetchSubscriptionsResponseProto,
        SubscribeRequest as SubscribeRequestProto, UnsubscribeRequest as UnsubscribeRequestProto,
    },
    UCode, UStatus,
};

/// A decoded uSubscription request, tagged by the operation it belongs to.
///
/// Returned by [`extract_usubscription_request`] so that a server can `match` on
/// the operation and know the associated up-rust message type.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum USubscriptionRequest {
    /// A [`RESOURCE_ID_SUBSCRIBE`] request.
    Subscribe(SubscribeRequest),
    /// A [`RESOURCE_ID_UNSUBSCRIBE`] request.
    Unsubscribe(UnsubscribeRequest),
    /// A [`RESOURCE_ID_FETCH_SUBSCRIPTIONS`] request.
    FetchSubscriptions(FetchSubscriptionsRequest),
    /// A [`RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS`] request.
    RegisterForNotification(()),
    /// A [`RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS`] request.
    UnregisterForNotification(()),
    /// A [`RESOURCE_ID_RESET`] request.
    Reset(()),
}

/// A decoded uSubscription response, tagged by the operation it belongs to.
///
/// Returned by [`pack_usubscription_response`] so that a server can `match` on
/// the operation and know the associated up-rust message type.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum USubscriptionResponse {
    /// A [`RESOURCE_ID_SUBSCRIBE`] request.
    Subscribe(SubscribeResponse),
    /// A [`RESOURCE_ID_UNSUBSCRIBE`] request.
    Unsubscribe(()),
    /// A [`RESOURCE_ID_FETCH_SUBSCRIPTIONS`] request.
    FetchSubscriptions(FetchSubscriptionsResponse),
    /// A [`RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS`] request.
    RegisterForNotification(()),
    /// A [`RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS`] request.
    UnregisterForNotification(()),
    /// A [`RESOURCE_ID_RESET`] request.
    Reset(()),
}

/// Decodes a uSubscription protobuf request into its corresponding up-rust representation.
///
/// Used by a uSubscription service to turn a raw protobuf payload into fully unpacked, native Rust data.
///
/// # Errors
///
/// Returns an error if `resource_id` does not identify a supported operation, if
/// the payload is missing, or if it cannot be deserialized into the expected type.
pub fn extract_usubscription_request(
    resource_id: u16,
    request_payload: Option<UPayload>,
) -> Result<USubscriptionRequest, UStatus> {
    let payload = request_payload.ok_or_else(|| {
        UStatus::fail_with_code(UCode::InvalidArgument, "missing request payload")
    })?;

    match resource_id {
        // [impl->req~usubscription-subscribe-request-signature~1]
        RESOURCE_ID_SUBSCRIBE => {
            let request_proto = payload
                .extract_protobuf::<SubscribeRequestProto>()
                .map_err(|err| UStatus::fail_with_code(UCode::Internal, err.to_string()))?;
            Ok(SubscribeRequest::try_from(&request_proto).map(USubscriptionRequest::Subscribe)?)
        }
        // [impl->req~usubscription-unsubscribe-request-signature~1]
        RESOURCE_ID_UNSUBSCRIBE => {
            let request_proto = payload
                .extract_protobuf::<UnsubscribeRequestProto>()
                .map_err(|err| UStatus::fail_with_code(UCode::Internal, err.to_string()))?;
            Ok(UnsubscribeRequest::try_from(&request_proto)
                .map(USubscriptionRequest::Unsubscribe)?)
        }
        // [impl->req~usubscription-fetch-subscriptions-request-signature~1]
        RESOURCE_ID_FETCH_SUBSCRIPTIONS => {
            let request_proto = payload
                .extract_protobuf::<FetchSubscriptionsRequestProto>()
                .map_err(|err| UStatus::fail_with_code(UCode::Internal, err.to_string()))?;
            Ok(FetchSubscriptionsRequest::try_from(&request_proto)
                .map(USubscriptionRequest::FetchSubscriptions)?)
        }
        // [impl->req~usubscription-register-notifications-request-signature~1]
        RESOURCE_ID_REGISTER_FOR_NOTIFICATIONS => {
            Ok(USubscriptionRequest::RegisterForNotification(()))
        }
        // [impl->req~usubscription-unregister-notifications-request-signature~1]
        RESOURCE_ID_UNREGISTER_FOR_NOTIFICATIONS => {
            Ok(USubscriptionRequest::UnregisterForNotification(()))
        }
        // [impl->req~usubscription-reset-request-signature~1]
        RESOURCE_ID_RESET => Ok(USubscriptionRequest::Reset(())),
        _ => Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            format!("unsupported uSubscription resource id: {resource_id:#06x}"),
        )),
    }
}

/// Encodes a up-rust uSubscription response into its corresponding protobuf representation.
///
/// Used by a uSubscription service to turn function response messages into raw protobuf payload data
///
/// # Errors
///
/// Returns an error if `resource_id` does not identify a supported operation,
/// or if the response object cannot be serialized into the expected type.
pub fn pack_usubscription_response(
    usubscription_response: USubscriptionResponse,
) -> Result<Option<UPayload>, UStatus> {
    match usubscription_response {
        // [impl->req~usubscription-subscribe-response-signature~1]
        USubscriptionResponse::Subscribe(response) => Ok(Some(
            UPayload::try_from_protobuf(response)
                .map_err(|err| UStatus::fail_with_code(UCode::Internal, err.to_string()))?,
        )),
        // [impl->req~usubscription-unsubscribe-response-signature~1]
        USubscriptionResponse::Unsubscribe(_) => Ok(None),
        // [impl->req~usubscription-fetch-subscriptions-response-signature~1]
        USubscriptionResponse::FetchSubscriptions(response) => {
            let r = FetchSubscriptionsResponseProto::try_from(&response)
                .map_err(|err| UStatus::fail_with_code(UCode::Internal, err.to_string()))?;
            Ok(Some(UPayload::try_from_protobuf(r).map_err(|err| {
                UStatus::fail_with_code(UCode::Internal, err.to_string())
            })?))
        }
        // [impl->req~usubscription-register-notifications-response-signature~1]
        USubscriptionResponse::RegisterForNotification(_) => Ok(None),
        // [impl->req~usubscription-unregister-notifications-response-signature~1]
        USubscriptionResponse::UnregisterForNotification(_) => Ok(None),
        // [impl->req~usubscription-reset-response-signature~1]
        USubscriptionResponse::Reset(_) => Ok(None),
    }
}
