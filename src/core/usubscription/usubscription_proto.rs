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
Conversion logic for up-rust USubscription message types into and from their corresponding protobuf types.
*/

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use protobuf::{
    well_known_types::{any::Any, timestamp::Timestamp},
    EnumFull, EnumOrUnknown, Message, MessageField,
};

use crate::{
    core::usubscription::{
        FetchSubscriptionsRequest, FetchSubscriptionsResponse, SubscribeRequest, SubscribeResponse,
        SubscriptionInfo, SubscriptionStatus, UnsubscribeRequest, UnsubscribeResponse,
    },
    up_core_api::{
        uri::UUri as UUriProto,
        usubscription::{
            FetchSubscriptionsRequest as FetchSubscriptionsRequestProto,
            FetchSubscriptionsResponse as FetchSubscriptionsResponseProto,
            SubscribeRequest as SubscribeRequestProto, SubscribeResponse as SubscribeResponseProto,
            Subscription as SubscriptionInfoProto,
            SubscriptionStatus::{
                self as SubscriptionStatusProto, STATUS_SUBSCRIBED, STATUS_SUBSCRIBE_PENDING,
                STATUS_UNSUBSCRIBED, STATUS_UNSUBSCRIBE_PENDING,
            },
            UnsubscribeRequest as UnsubscribeRequestProto,
            UnsubscribeResponse as UnsubscribeResponseProto,
        },
    },
    ProtobufMappable, SerializationError, UCode, UStatus, UUri,
};

pub(crate) fn system_time_as_protobuf_timestamp(
    time: Option<SystemTime>,
) -> Result<Option<Timestamp>, UStatus> {
    if let Some(t) = time {
        let (seconds, nanos) = match t.duration_since(UNIX_EPOCH) {
            Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos() as i32),
            Err(before_epoch) => {
                let duration = before_epoch.duration();
                let whole_seconds = duration.as_secs() as i64;
                let subsec_nanos = duration.subsec_nanos();
                // protobuf's Timestamp always uses a non-negative `nanos` offset added to
                // `seconds`, so a non-zero fraction shifts one whole second into it
                if subsec_nanos == 0 {
                    (-whole_seconds, 0)
                } else {
                    (-whole_seconds - 1, 1_000_000_000 - subsec_nanos as i32)
                }
            }
        };
        Ok(Some(Timestamp {
            seconds,
            nanos,
            ..Default::default()
        }))
    } else {
        Ok(None)
    }
}

pub(crate) fn protobuf_timestamp_as_system_time(
    ts: Option<&Timestamp>,
) -> Result<Option<SystemTime>, UStatus> {
    if let Some(ts) = ts {
        let err = || {
            UStatus::fail_with_code(
                UCode::InvalidArgument,
                "invalid timestamp: seconds value out of range",
            )
        };
        if ts.nanos < 0 || ts.nanos >= 1_000_000_000 {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "invalid timestamp: nanos value out of range",
            ));
        }
        // nanos already validated to be in [0, 1_000_000_000), so this cast is safe
        let nanos = ts.nanos as u32;
        let time = if ts.seconds >= 0 {
            UNIX_EPOCH.checked_add(Duration::new(ts.seconds as u64, nanos))
        } else {
            // reverse of the +1s/complementary-nanos shift applied when encoding a
            // pre-epoch instant; `-(ts.seconds + 1)` cannot overflow since ts.seconds < 0
            let whole_seconds_before_epoch = (-(ts.seconds + 1)) as u64;
            UNIX_EPOCH.checked_sub(Duration::new(
                whole_seconds_before_epoch,
                1_000_000_000 - nanos,
            ))
        };
        time.ok_or_else(err).map(Some)
    } else {
        Ok(None)
    }
}

// SubscriptionStatus conversions

impl TryFrom<&SubscriptionStatusProto> for SubscriptionStatus {
    type Error = UStatus;

    fn try_from(value: &SubscriptionStatusProto) -> Result<Self, Self::Error> {
        match value {
            STATUS_UNSUBSCRIBED => Ok(SubscriptionStatus::Unsubscribed),
            STATUS_SUBSCRIBE_PENDING => Ok(SubscriptionStatus::SubscribePending),
            STATUS_SUBSCRIBED => Ok(SubscriptionStatus::Subscribed),
            STATUS_UNSUBSCRIBE_PENDING => Ok(SubscriptionStatus::UnsubscribePending),
            _ => Err(UStatus::fail_with_code(
                UCode::OutOfRange,
                format!("invalid subscription status {}", value.descriptor()),
            )),
        }
    }
}

impl From<&SubscriptionStatus> for SubscriptionStatusProto {
    fn from(value: &SubscriptionStatus) -> Self {
        match value {
            SubscriptionStatus::Unsubscribed => STATUS_UNSUBSCRIBED,
            SubscriptionStatus::SubscribePending => STATUS_SUBSCRIBE_PENDING,
            SubscriptionStatus::Subscribed => STATUS_SUBSCRIBED,
            SubscriptionStatus::UnsubscribePending => STATUS_UNSUBSCRIBE_PENDING,
        }
    }
}

// SubscriptionInfo conversions

impl TryFrom<&SubscriptionInfo> for SubscriptionInfoProto {
    type Error = UStatus;

    fn try_from(value: &SubscriptionInfo) -> Result<Self, Self::Error> {
        Ok(SubscriptionInfoProto {
            subscriber: MessageField::some(UUriProto::from(&value.subscriber)),
            topic: MessageField::some(UUriProto::from(&value.topic)),
            status: SubscriptionStatusProto::from(&value.status).into(),
            expiration: system_time_as_protobuf_timestamp(*value.expiration())?.into(),
            sample_period: value
                .min_sample_period
                .map(|p| p.as_millis().clamp(0, u32::MAX as u128) as u32),
            ..Default::default()
        })
    }
}

impl TryFrom<&SubscriptionInfoProto> for SubscriptionInfo {
    type Error = UStatus;

    fn try_from(proto: &SubscriptionInfoProto) -> Result<Self, Self::Error> {
        let subscriber = proto
            .subscriber
            .as_ref()
            .ok_or(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "subscriber missing",
            ))
            .and_then(|s| {
                UUri::try_from(s).map_err(|_| {
                    UStatus::fail_with_code(UCode::InvalidArgument, "invalid subscriber")
                })
            })?;

        Ok(SubscriptionInfo::new(
            require_topic(proto.topic.clone())?,
            subscriber,
            require_status(proto.status)?,
            protobuf_timestamp_as_system_time(proto.expiration.as_ref())?,
            proto.sample_period.map(|p| Duration::from_millis(p as u64)),
        ))
    }
}

impl ProtobufMappable for SubscriptionInfo {
    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError::new(err.to_string()))
            .and_then(|any| match any.unpack::<SubscriptionInfoProto>() {
                Ok(Some(message_proto)) => SubscriptionInfo::try_from(&message_proto)
                    .map_err(|e| crate::SerializationError::new(e.to_string())),
                Ok(None) => Err(crate::SerializationError::new(
                    "protobuf Any does not contain expected type".to_string(),
                )),
                Err(e) => Err(crate::SerializationError::new(format!(
                    "protobuf Any unpack error: {e}"
                ))),
            })
    }

    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        SubscriptionInfo::try_from(&SubscriptionInfoProto::parse_from_bytes(proto)?)
            .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(&SubscriptionInfoProto::try_from(self).map_err(|e| {
            SerializationError::new(format!("failed to serialize to protobuf: {e}"))
        })?)
        .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
        .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(SubscriptionInfoProto::try_from(self)
            .map_err(|e| SerializationError::new(format!("failed to serialize to protobuf: {e}")))?
            .write_to_bytes()?)
    }
}

// SubscribeRequest conversions

impl TryFrom<&SubscribeRequest> for SubscribeRequestProto {
    type Error = UStatus;

    fn try_from(value: &SubscribeRequest) -> Result<Self, Self::Error> {
        Ok(SubscribeRequestProto {
            topic: MessageField::some(UUriProto::from(&value.topic)),
            expiration: system_time_as_protobuf_timestamp(value.expiration)?.into(),
            sample_period: value
                .sample_period
                .map(|p| p.as_millis().clamp(0, u32::MAX as u128) as u32),
            ..Default::default()
        })
    }
}

// [impl->req~usubscription-subscribe-request-signature~1]
impl TryFrom<&SubscribeRequestProto> for SubscribeRequest {
    type Error = UStatus;

    fn try_from(proto: &SubscribeRequestProto) -> Result<Self, Self::Error> {
        Ok(SubscribeRequest {
            // [impl->dsn~usubscription-subscribe-valid-topic-uuris~1]
            topic: require_topic(proto.topic.clone())?,
            // [impl->dsn~usubscription-subscription-expiration-datatype~1]
            expiration: protobuf_timestamp_as_system_time(proto.expiration.as_ref())?,
            // [impl->dsn~usubscription-sample-period-datatype~1]
            sample_period: proto
                .sample_period
                .map(|sp| Duration::from_millis(sp as u64)),
        })
    }
}

impl ProtobufMappable for SubscribeRequest {
    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError::new(err.to_string()))
            .and_then(|any| match any.unpack::<SubscribeRequestProto>() {
                Ok(Some(message_proto)) => SubscribeRequest::try_from(&message_proto)
                    .map_err(|e| crate::SerializationError::new(e.to_string())),
                Ok(None) => Err(crate::SerializationError::new(
                    "protobuf Any does not contain expected type".to_string(),
                )),
                Err(e) => Err(crate::SerializationError::new(format!(
                    "protobuf Any unpack error: {e}"
                ))),
            })
    }

    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        SubscribeRequest::try_from(&SubscribeRequestProto::parse_from_bytes(proto)?)
            .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(&SubscribeRequestProto::try_from(self).map_err(|e| {
            SerializationError::new(format!("failed to serialize to protobuf: {e}"))
        })?)
        .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
        .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(SubscribeRequestProto::try_from(self)
            .map_err(|e| SerializationError::new(format!("failed to serialize to protobuf: {e}")))?
            .write_to_bytes()?)
    }
}

// SubscriptionResponse conversions

impl From<&SubscribeResponse> for SubscribeResponseProto {
    fn from(value: &SubscribeResponse) -> Self {
        SubscribeResponseProto {
            topic: MessageField::some(UUriProto::from(&value.topic)),
            status: SubscriptionStatusProto::from(&value.status).into(),
            ..Default::default()
        }
    }
}

// [impl->req~usubscription-subscribe-response-signature~1]
impl TryFrom<&SubscribeResponseProto> for SubscribeResponse {
    type Error = UStatus;

    fn try_from(proto: &SubscribeResponseProto) -> Result<Self, Self::Error> {
        Ok(SubscribeResponse {
            topic: require_topic(proto.topic.clone())?,
            status: require_status(proto.status)?,
        })
    }
}

impl ProtobufMappable for SubscribeResponse {
    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError::new(err.to_string()))
            .and_then(|any| match any.unpack::<SubscribeResponseProto>() {
                Ok(Some(message_proto)) => SubscribeResponse::try_from(&message_proto)
                    .map_err(|e| crate::SerializationError::new(e.to_string())),
                Ok(None) => Err(crate::SerializationError::new(
                    "protobuf Any does not contain expected type".to_string(),
                )),
                Err(e) => Err(crate::SerializationError::new(format!(
                    "protobuf Any unpack error: {e}"
                ))),
            })
    }

    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        SubscribeResponse::try_from(&SubscribeResponseProto::parse_from_bytes(proto)?)
            .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(&SubscribeResponseProto::from(self))
            .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
            .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(SubscribeResponseProto::from(self).write_to_bytes()?)
    }
}

// UnsubscribeResponse conversions
// (though empty, these empty types are still needed to use RpcClient::invoke_method() with empty request/response objects)
impl From<&UnsubscribeResponse> for UnsubscribeResponseProto {
    fn from(_value: &UnsubscribeResponse) -> Self {
        UnsubscribeResponseProto::default()
    }
}

// [impl->req~usubscription-unsubscribe-response-signature~1]
impl From<&UnsubscribeResponseProto> for UnsubscribeResponse {
    fn from(_proto: &UnsubscribeResponseProto) -> Self {
        UnsubscribeResponse {}
    }
}

impl ProtobufMappable for UnsubscribeResponse {
    fn parse_from_packed_protobuf_bytes(_proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Ok(UnsubscribeResponse {})
    }

    fn parse_from_protobuf_bytes(_proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Ok(UnsubscribeResponse {})
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(&UnsubscribeResponseProto::from(self))
            .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
            .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(UnsubscribeResponseProto::from(self).write_to_bytes()?)
    }
}

// UnsubscribeRequest conversions

impl TryFrom<&UnsubscribeRequest> for UnsubscribeRequestProto {
    type Error = UStatus;

    fn try_from(value: &UnsubscribeRequest) -> Result<Self, Self::Error> {
        Ok(UnsubscribeRequestProto {
            topic: MessageField::some(UUriProto::from(&value.topic)),
            ..Default::default()
        })
    }
}

// [impl->req~usubscription-unsubscribe-request-signature~1]
impl TryFrom<&UnsubscribeRequestProto> for UnsubscribeRequest {
    type Error = UStatus;

    fn try_from(proto: &UnsubscribeRequestProto) -> Result<Self, Self::Error> {
        Ok(UnsubscribeRequest {
            // [impl->dsn~usubscription-unsubscribe-valid-topic-uuris~1]
            topic: require_topic(proto.topic.clone())?,
        })
    }
}

impl ProtobufMappable for UnsubscribeRequest {
    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError::new(err.to_string()))
            .and_then(|any| match any.unpack::<UnsubscribeRequestProto>() {
                Ok(Some(message_proto)) => UnsubscribeRequest::try_from(&message_proto)
                    .map_err(|e| crate::SerializationError::new(e.to_string())),
                Ok(None) => Err(crate::SerializationError::new(
                    "protobuf Any does not contain expected type".to_string(),
                )),
                Err(e) => Err(crate::SerializationError::new(format!(
                    "protobuf Any unpack error: {e}"
                ))),
            })
    }

    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        UnsubscribeRequest::try_from(&UnsubscribeRequestProto::parse_from_bytes(proto)?)
            .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(&UnsubscribeRequestProto::try_from(self).map_err(|e| {
            SerializationError::new(format!("failed to serialize to protobuf: {e}"))
        })?)
        .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
        .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(UnsubscribeRequestProto::try_from(self)
            .map_err(|e| SerializationError::new(format!("failed to serialize to protobuf: {e}")))?
            .write_to_bytes()?)
    }
}

// FetchSubscriptionsRequest conversions

impl From<&FetchSubscriptionsRequest> for FetchSubscriptionsRequestProto {
    fn from(value: &FetchSubscriptionsRequest) -> Self {
        FetchSubscriptionsRequestProto {
            topic_filter: value.topic_filter.as_ref().map(UUriProto::from).into(),
            subscriber_filter: value.subscriber_filter.as_ref().map(UUriProto::from).into(),
            ..Default::default()
        }
    }
}

// [impl->req~usubscription-fetch-subscriptions-request-signature~1]
impl TryFrom<&FetchSubscriptionsRequestProto> for FetchSubscriptionsRequest {
    type Error = UStatus;

    fn try_from(value: &FetchSubscriptionsRequestProto) -> Result<Self, Self::Error> {
        // [impl->dsn~usubscription-fetch-subscriptions-invalid-subscriber-filter~1]
        let subscriber_filter = value
            .subscriber_filter
            .as_ref()
            .map(UUri::try_from)
            .transpose()
            .map_err(|_| {
                UStatus::fail_with_code(UCode::InvalidArgument, "invalid subscriber filter")
            })?;

        // [impl->dsn~usubscription-fetch-subscriptions-invalid-topic-filter~1]
        let topic_filter = value
            .topic_filter
            .as_ref()
            .map(UUri::try_from)
            .transpose()
            .map_err(|_| UStatus::fail_with_code(UCode::InvalidArgument, "invalid topic filter"))?;

        Ok(FetchSubscriptionsRequest {
            topic_filter,
            subscriber_filter,
        })
    }
}

impl ProtobufMappable for FetchSubscriptionsRequest {
    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError::new(err.to_string()))
            .and_then(|any| match any.unpack::<FetchSubscriptionsRequestProto>() {
                Ok(Some(message_proto)) => FetchSubscriptionsRequest::try_from(&message_proto)
                    .map_err(|e| crate::SerializationError::new(e.to_string())),
                Ok(None) => Err(crate::SerializationError::new(
                    "protobuf Any does not contain expected type".to_string(),
                )),
                Err(e) => Err(crate::SerializationError::new(format!(
                    "protobuf Any unpack error: {e}"
                ))),
            })
    }

    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        FetchSubscriptionsRequest::try_from(&FetchSubscriptionsRequestProto::parse_from_bytes(
            proto,
        )?)
        .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(&FetchSubscriptionsRequestProto::from(self))
            .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
            .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(FetchSubscriptionsRequestProto::from(self).write_to_bytes()?)
    }
}

// FetchSubscriptionsResponse conversions

impl TryFrom<&FetchSubscriptionsResponse> for FetchSubscriptionsResponseProto {
    type Error = UStatus;

    fn try_from(value: &FetchSubscriptionsResponse) -> Result<Self, Self::Error> {
        Ok(FetchSubscriptionsResponseProto {
            subscriptions: value
                .subscriptions
                .iter()
                .map(SubscriptionInfoProto::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            ..Default::default()
        })
    }
}

// [impl->req~usubscription-fetch-subscriptions-response-signature~1]
impl TryFrom<&FetchSubscriptionsResponseProto> for FetchSubscriptionsResponse {
    type Error = UStatus;

    fn try_from(value: &FetchSubscriptionsResponseProto) -> Result<Self, Self::Error> {
        Ok(FetchSubscriptionsResponse {
            subscriptions: value
                .subscriptions
                .iter()
                .map(SubscriptionInfo::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl ProtobufMappable for FetchSubscriptionsResponse {
    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        Any::parse_from_bytes(proto)
            .map_err(|err| crate::SerializationError::new(err.to_string()))
            .and_then(
                |any| match any.unpack::<FetchSubscriptionsResponseProto>() {
                    Ok(Some(message_proto)) => FetchSubscriptionsResponse::try_from(&message_proto)
                        .map_err(|e| crate::SerializationError::new(e.to_string())),
                    Ok(None) => Err(crate::SerializationError::new(
                        "protobuf Any does not contain expected type".to_string(),
                    )),
                    Err(e) => Err(crate::SerializationError::new(format!(
                        "protobuf Any unpack error: {e}"
                    ))),
                },
            )
    }

    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, crate::SerializationError> {
        FetchSubscriptionsResponse::try_from(&FetchSubscriptionsResponseProto::parse_from_bytes(
            proto,
        )?)
        .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Any::pack(
            &FetchSubscriptionsResponseProto::try_from(self).map_err(|e| {
                SerializationError::new(format!("failed to serialize to protobuf: {e}"))
            })?,
        )
        .map_err(|e| crate::SerializationError::new(format!("failed to pack: {e}")))
        .and_then(|any| any.write_to_protobuf_bytes())
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, crate::SerializationError> {
        Ok(FetchSubscriptionsResponseProto::try_from(self)
            .map_err(|e| SerializationError::new(format!("failed to serialize to protobuf: {e}")))?
            .write_to_bytes()?)
    }
}

/// Extracts and validates the topic from a protobuf request, failing if it is
/// absent or not a well-formed URI.
fn require_topic(topic: MessageField<UUriProto>) -> Result<UUri, UStatus> {
    let topic = topic
        .into_option()
        .ok_or_else(|| UStatus::fail_with_code(UCode::InvalidArgument, "missing topic"))?;
    UUri::try_from(&topic).map_err(|e| {
        UStatus::fail_with_code(UCode::InvalidArgument, format!("invalid topic URI: {e}"))
    })
}

/// Extracts and validates the status from a protobuf request, failing if it is
/// absent or invalid.
fn require_status(
    status: EnumOrUnknown<SubscriptionStatusProto>,
) -> Result<SubscriptionStatus, UStatus> {
    SubscriptionStatus::try_from(&status.enum_value().map_err(|_| {
        UStatus::fail_with_code(UCode::InvalidArgument, "subscription status missing")
    })?)
}

#[cfg(feature = "up-core-types")]
#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::well_known_types::timestamp::Timestamp;

    #[test]
    fn test_system_time_as_protobuf_timestamp() {
        assert!(system_time_as_protobuf_timestamp(None).is_ok_and(|ts| ts.is_none()));

        let time = UNIX_EPOCH + Duration::new(1, 234_000_000);
        assert!(
            system_time_as_protobuf_timestamp(Some(time)).is_ok_and(|ts| {
                ts == Some(Timestamp {
                    seconds: 1,
                    nanos: 234_000_000,
                    ..Default::default()
                })
            })
        );
    }

    #[test]
    fn test_protobuf_timestamp_as_system_time_maps_none() {
        assert!(protobuf_timestamp_as_system_time(None).is_ok_and(|dt| dt.is_none()));
    }

    #[test_case::test_case(10, 234_000_000 => matches Ok(Some(_)); "succeeds for valid timestamp")]
    #[test_case::test_case(-10, 234_000_000 => matches Ok(Some(_)); "succeeds for timestamp before Unix epoch")]
    #[test_case::test_case(10, -1 => matches Err(UStatus {..}); "fails for nanos exceeding lower bound")]
    #[test_case::test_case(10, 1_000_000_000 => matches Err(UStatus {..}); "fails for nanos exceeding upper bound")]
    fn test_protobuf_timestamp_as_system_time(
        seconds: i64,
        nanos: i32,
    ) -> Result<Option<SystemTime>, UStatus> {
        let timestamp = Timestamp {
            seconds,
            nanos,
            ..Default::default()
        };
        protobuf_timestamp_as_system_time(Some(&timestamp))
    }

    // [utest->dsn~usubscription-subscription-expiration-datatype~1]
    #[test]
    fn test_timestamp_conversion_round_trip() {
        let time = UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
        let timestamp = system_time_as_protobuf_timestamp(Some(time))
            .expect("conversion to protobuf timestamp should succeed");
        let round_tripped = protobuf_timestamp_as_system_time(timestamp.as_ref())
            .expect("conversion back to system time should succeed");
        assert_eq!(round_tripped, Some(time));
    }

    // [utest->dsn~usubscription-fetch-subscriptions-invalid-subscriber-filter~1]
    #[test]
    fn test_fetch_subscriptions_request_rejects_invalid_subscriber_filter() {
        let proto = FetchSubscriptionsRequestProto {
            subscriber_filter: MessageField::some(UUriProto {
                // construct an invalid UUri, with a too-large resource ID
                resource_id: 0xFFFF_FFFF,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = FetchSubscriptionsRequest::try_from(&proto);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().get_code(), UCode::InvalidArgument);
    }

    // [utest->dsn~usubscription-fetch-subscriptions-invalid-topic-filter~1]
    #[test]
    fn test_fetch_subscriptions_request_rejects_invalid_topic_filter() {
        let proto = FetchSubscriptionsRequestProto {
            topic_filter: MessageField::some(UUriProto {
                // construct an invalid UUri, with a too-large resource ID
                resource_id: 0xFFFF_FFFF,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = FetchSubscriptionsRequest::try_from(&proto);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().get_code(), UCode::InvalidArgument);
    }

    // [utest->dsn~usubscription-subscribe-valid-topic-uuris~1]
    #[test]
    fn test_subscribe_request_rejects_missing_topic() {
        let proto = SubscribeRequestProto::default();
        let result = SubscribeRequest::try_from(&proto);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().get_code(), UCode::InvalidArgument);
    }

    // [utest->dsn~usubscription-unsubscribe-valid-topic-uuris~1]
    #[test]
    fn test_unsubscribe_request_rejects_missing_topic() {
        let proto = UnsubscribeRequestProto::default();
        let result = UnsubscribeRequest::try_from(&proto);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().get_code(), UCode::InvalidArgument);
    }

    // [utest->dsn~usubscription-sample-period-datatype~1]
    #[test]
    fn test_subscribe_request_preserves_sample_period() {
        let proto = SubscribeRequestProto {
            topic: MessageField::some(UUriProto::from(
                &UUri::try_from_parts("", 0x1234, 1, 0x8000).unwrap(),
            )),
            sample_period: Some(500),
            ..Default::default()
        };
        let request = SubscribeRequest::try_from(&proto).unwrap();
        assert_eq!(request.sample_period, Some(Duration::from_millis(500)));
    }
}
