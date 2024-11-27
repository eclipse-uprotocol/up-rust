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

use crate::{
    UAttributes, UAttributesValidators, UCode, UMessage, UMessageType, UPayloadFormat, UPriority,
    UStatus, UUri, UUID,
};
use bytes::Bytes;
use cloudevents::{
    event::ExtensionValue, AttributesReader, Data, Event, EventBuilder, EventBuilderV10,
};
use protobuf::EnumOrUnknown;

/// This module contains functions for mapping uProtocol `UMessage`s  to CloudEvents
/// as defined in the uProtocol specification.

pub const CONTENT_TYPE_CLOUDEVENTS_JSON: &str = "application/cloudevents+json";
pub const CONTENT_TYPE_CLOUDEVENTS_PROTOBUF: &str = "application/cloudevents+protobuf";

const EXTENSION_NAME_COMMSTATUS: &str = "commstatus";
const EXTENSION_NAME_PERMISSION_LEVEL: &str = "plevel";
const EXTENSION_NAME_PRIORITY: &str = "priority";
const EXTENSION_NAME_REQUEST_ID: &str = "reqid";
const EXTENSION_NAME_SINK: &str = "sink";
const EXTENSION_NAME_TTL: &str = "ttl";
const EXTENSION_NAME_TOKEN: &str = "token";
const EXTENSION_NAME_TRACEPARENT: &str = "traceparent";

// Creates a CloudEvent from a uProtocol message.
//
// # Arguments
//
// * `message` - The message to create the event from. The message is being consumed by this function.
// * `skip_validation` - `true` if the given message known to be a valid uProtocol message.
//
// # Errors
//
// Returns an error if the given message does not contain the necessary information for creating a CloudEvent.
// Also returns an error if `skip_validation` is `false` and the message is not a valid uProtocol message.
//
// [impl->dsn~cloudevents-umessage-mapping~1]
pub fn get_cloudevent(message: UMessage, skip_validation: bool) -> Result<Event, UStatus> {
    let Some(attributes) = message.attributes.as_ref() else {
        return Err(UStatus::fail_with_code(
            UCode::INVALID_ARGUMENT,
            "message has no attributes",
        ));
    };
    if !skip_validation {
        UAttributesValidators::get_validator_for_attributes(attributes)
            .validate(attributes)
            .map_err(|e| UStatus::fail_with_code(UCode::INVALID_ARGUMENT, e.to_string()))?;
    }
    let mut event = EventBuilderV10::new()
        .id(attributes
            .id
            .as_ref()
            .unwrap_or(&UUID::build())
            .to_hyphenated_string())
        .ty(attributes
            .type_
            .enum_value_or_default()
            .to_cloudevent_type())
        .source(attributes.source.get_or_default().to_uri(false))
        .build()
        .map_err(|_e| {
            UStatus::fail_with_code(UCode::INVALID_ARGUMENT, "cannot map UMessage to Cloudevent")
        })?;
    if let Some(sink) = attributes.sink.as_ref() {
        event.set_extension(EXTENSION_NAME_SINK, sink.to_uri(false));
    }
    if let Ok(priority) = attributes.priority.enum_value() {
        if priority != UPriority::UPRIORITY_UNSPECIFIED {
            event.set_extension(EXTENSION_NAME_PRIORITY, priority.to_priority_code());
        }
    }
    if let Some(ttl) = attributes.ttl {
        event.set_extension(EXTENSION_NAME_TTL, ttl as i64);
    }
    if let Some(token) = attributes.token.as_ref() {
        event.set_extension(EXTENSION_NAME_TOKEN, token.to_owned());
    }
    if let Some(plevel) = attributes.permission_level {
        event.set_extension(EXTENSION_NAME_PERMISSION_LEVEL, plevel as i64);
    }
    if let Some(reqid) = attributes.reqid.as_ref() {
        event.set_extension(EXTENSION_NAME_REQUEST_ID, reqid.to_hyphenated_string());
    }
    if let Some(commstatus) = attributes.commstatus.as_ref() {
        event.set_extension(EXTENSION_NAME_COMMSTATUS, commstatus.value() as i64);
    }
    if let Some(traceparent) = attributes.traceparent.as_ref() {
        event.set_extension(EXTENSION_NAME_TRACEPARENT, traceparent.to_owned());
    }
    if let Some(payload) = message.payload {
        let payload_format = attributes.payload_format.enum_value_or_default();
        let data = match payload_format {
            UPayloadFormat::UPAYLOAD_FORMAT_JSON | UPayloadFormat::UPAYLOAD_FORMAT_TEXT => {
                Data::String(String::from_utf8(payload.to_vec()).unwrap())
            }
            _ => Data::Binary(payload.to_vec()),
        };
        event.set_data(
            payload_format.to_media_type().unwrap_or("".to_string()),
            data,
        );
    }
    Ok(event)
}

// Creates a uProtocol message from a CloudEvent.
//
// # Arguments
//
// * `event` - The CloudEvent to create the message from.
//
// # Errors
//
// Returns an error if the given event does not contain the necessary information for creating a uProtocol message.
// Also returns an error if `skip_validation` is `false` and the resulting message is not a valid uProtocol message.
//
// [impl->dsn~cloudevents-umessage-mapping~1]
pub fn get_umessage(
    event: Event,
    skip_validation: bool,
) -> Result<UMessage, Box<dyn std::error::Error>> {
    let message_type = UMessageType::try_from_cloudevent_type(event.ty())?;
    let id = event.id().parse::<UUID>()?;
    let source_uri = event.source().to_string().parse()?;

    let sink_uri = match event.extension(EXTENSION_NAME_SINK) {
        Some(ExtensionValue::String(sink)) => Some(sink.parse::<UUri>()?),
        _ => None,
    };

    let priority = match event.extension(EXTENSION_NAME_PRIORITY) {
        Some(ExtensionValue::String(code)) => Some(UPriority::try_from_priority_code(code)?),
        _ => None,
    };

    let ttl = match event.extension(EXTENSION_NAME_TTL) {
        Some(ExtensionValue::Integer(ttl)) => Some(u32::try_from(*ttl)?),
        _ => None,
    };

    let token = match event.extension(EXTENSION_NAME_TOKEN) {
        Some(ExtensionValue::String(token)) => Some(token.to_string()),
        _ => None,
    };

    let permission_level = match event.extension(EXTENSION_NAME_PERMISSION_LEVEL) {
        Some(ExtensionValue::Integer(level)) => Some(u32::try_from(*level)?),
        _ => None,
    };

    let reqid = match event.extension(EXTENSION_NAME_REQUEST_ID) {
        Some(ExtensionValue::String(uuid)) => Some(uuid.parse::<UUID>()?),
        _ => None,
    };

    let commstatus = match event.extension(EXTENSION_NAME_COMMSTATUS) {
        Some(ExtensionValue::Integer(code)) => {
            i32::try_from(*code).map(|v| Some(EnumOrUnknown::<UCode>::from_i32(v)))?
        }
        _ => None,
    };

    let traceparent = match event.extension(EXTENSION_NAME_TRACEPARENT) {
        Some(ExtensionValue::String(traceparent)) => Some(traceparent.to_string()),
        _ => None,
    };

    let payload = match event.data() {
        Some(Data::Binary(buf)) => Some(Bytes::copy_from_slice(buf.as_slice())),
        Some(Data::String(text)) => Some(Bytes::copy_from_slice(text.as_bytes())),
        Some(Data::Json(json)) => {
            Some(serde_json::to_vec(json).map(|v| Bytes::copy_from_slice(v.as_slice()))?)
        }
        _ => None,
    };

    let payload_format = match event.datacontenttype() {
        Some(media_type) => Some(UPayloadFormat::from_media_type(media_type)?),
        _ => None,
    };

    let attributes = UAttributes {
        commstatus,
        id: Some(id).into(),
        type_: message_type.into(),
        source: Some(source_uri).into(),
        sink: sink_uri.into(),
        priority: priority.unwrap_or(UPriority::default()).into(),
        ttl,
        permission_level,
        reqid: reqid.into(),
        token,
        traceparent,
        payload_format: payload_format.unwrap_or(UPayloadFormat::default()).into(),
        ..Default::default()
    };
    if !skip_validation {
        UAttributesValidators::get_validator_for_attributes(&attributes).validate(&attributes)?;
    }
    Ok(UMessage {
        attributes: Some(attributes).into(),
        payload,
        ..Default::default()
    })
}

// [utest->dsn~cloudevents-umessage-mapping~1]
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cloudevents::event::SpecVersion;
    use protobuf::Enum;

    use crate::UMessageBuilder;

    use super::*;

    const MESSAGE_ID: &str = "00000000-0001-7000-8010-101010101a1a";
    const TOPIC: &str = "//my-vehicle/A81B/1/A9BA";
    const METHOD: &str = "//my-vehicle/A000/2/1";
    const REPLY_TO: &str = "//my-vehicle/A81B/1/0";
    const DESTINATION: &str = "//my-vehicle/A000/2/0";
    const PERMISSION_LEVEL: u32 = 5;
    const PRIORITY: UPriority = UPriority::UPRIORITY_CS4;
    const TTL: u32 = 15_000;
    const TRACEPARENT: &str = "traceparent";
    const DATA: [u8; 4] = [0x00, 0x01, 0x02, 0x03];

    //
    // tests asserting conversion of UMessage -> CloudEvent
    //

    fn assert_standard_cloudevent_attributes(
        event: &Event,
        message_type: &str,
        source: &str,
        sink: Option<String>,
    ) {
        assert_eq!(event.specversion(), SpecVersion::V10);
        assert_eq!(event.ty(), message_type);
        assert_eq!(event.id(), MESSAGE_ID);
        assert_eq!(event.source().as_str(), source);
        assert_eq!(
            event.extension(EXTENSION_NAME_SINK).map(|v| v.to_string()),
            sink
        );
        assert_eq!(
            event.extension(EXTENSION_NAME_PRIORITY),
            Some(&ExtensionValue::String(PRIORITY.to_priority_code()))
        );
        assert_eq!(
            event.extension(EXTENSION_NAME_TTL),
            Some(&ExtensionValue::Integer(TTL as i64))
        );
        assert_eq!(
            event.extension(EXTENSION_NAME_TRACEPARENT),
            Some(&ExtensionValue::String(TRACEPARENT.to_string()))
        );
    }

    #[test]
    fn test_get_cloudevent_fails_for_invalid_message() {
        let invalid_attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            id: Some(UUID::build()).into(),
            source: Some(TOPIC.parse::<UUri>().unwrap()).into(),
            ..Default::default()
        };
        let invalid_message = UMessage {
            attributes: Some(invalid_attributes).into(),
            ..Default::default()
        };
        assert!(get_cloudevent(invalid_message.clone(), false)
            .is_err_and(|e| e.get_code() == UCode::INVALID_ARGUMENT));
        assert!(get_cloudevent(invalid_message, true).is_ok());
    }

    #[test]
    fn test_get_cloudevent_for_publish_message_succeeds() {
        let message_id = MESSAGE_ID
            .parse::<UUID>()
            .expect("failed to parse message ID");
        let message =
            UMessageBuilder::publish(UUri::from_str(TOPIC).expect("failed to create topic URI"))
                .with_message_id(message_id)
                .with_priority(PRIORITY)
                .with_ttl(TTL)
                .with_traceparent(TRACEPARENT)
                .build_with_payload("test".as_bytes(), UPayloadFormat::UPAYLOAD_FORMAT_TEXT)
                .expect("failed to create message");

        let event =
            get_cloudevent(message, false).expect("failed to create CloudEvent from UMessage");
        assert_standard_cloudevent_attributes(&event, "pub.v1", TOPIC, None);
        assert_eq!(
            event.datacontenttype().map(|v| v.to_string()),
            UPayloadFormat::UPAYLOAD_FORMAT_TEXT.to_media_type()
        );
        match event.data() {
            Some(Data::String(payload)) => {
                assert_eq!(payload, "test")
            }
            _ => panic!("unexpected payload format"),
        }
    }

    #[test]
    fn test_get_cloudevent_for_notification_message_succeeds() {
        let message_id = MESSAGE_ID
            .parse::<UUID>()
            .expect("failed to parse message ID");
        let message = UMessageBuilder::notification(
            UUri::from_str(TOPIC).expect("failed to create source URI"),
            UUri::from_str(DESTINATION).expect("failed to create sink URI"),
        )
        .with_message_id(message_id)
        .with_priority(PRIORITY)
        .with_ttl(TTL)
        .with_traceparent(TRACEPARENT)
        .build_with_payload(
            "{\"count\": 5}".as_bytes(),
            UPayloadFormat::UPAYLOAD_FORMAT_JSON,
        )
        .expect("failed to create message");

        let event =
            get_cloudevent(message, false).expect("failed to create CloudEvent from UMessage");
        assert_standard_cloudevent_attributes(
            &event,
            "not.v1",
            TOPIC,
            Some(DESTINATION.to_string()),
        );
        assert_eq!(
            event.datacontenttype().map(|v| v.to_string()),
            UPayloadFormat::UPAYLOAD_FORMAT_JSON.to_media_type()
        );
        match event.data() {
            Some(Data::String(payload)) => {
                assert_eq!(payload, "{\"count\": 5}")
            }
            _ => panic!("unexpected payload format"),
        }
    }

    #[test]
    fn test_get_cloudevent_for_request_message_succeeds() {
        let message_id = MESSAGE_ID
            .parse::<UUID>()
            .expect("failed to parse message ID");
        let token = "my-token";
        let message = UMessageBuilder::request(
            UUri::from_str(METHOD).expect("failed to create sink URI"),
            UUri::from_str(REPLY_TO).expect("failed to create source URI"),
            TTL,
        )
        .with_message_id(message_id)
        .with_priority(PRIORITY)
        .with_permission_level(PERMISSION_LEVEL)
        .with_traceparent(TRACEPARENT)
        .with_token(token)
        .build()
        .expect("failed to create message");

        let event =
            get_cloudevent(message, false).expect("failed to create CloudEvent from UMessage");
        assert_standard_cloudevent_attributes(&event, "req.v1", REPLY_TO, Some(METHOD.to_string()));
        assert_eq!(
            event.extension(EXTENSION_NAME_TOKEN),
            Some(&ExtensionValue::String(token.to_string()))
        );
        assert_eq!(
            event.extension(EXTENSION_NAME_PERMISSION_LEVEL),
            Some(&ExtensionValue::Integer(PERMISSION_LEVEL as i64))
        );
        assert!(event.datacontenttype().is_none());
        assert!(event.data().is_none());
    }

    #[test]
    fn test_get_cloudevent_for_response_message_succeeds() {
        let message_id = MESSAGE_ID
            .parse::<UUID>()
            .expect("failed to parse message ID");
        let request_id = UUID::build();

        let message = UMessageBuilder::response(
            UUri::from_str(REPLY_TO).expect("failed to create sink URI"),
            request_id.clone(),
            UUri::from_str(METHOD).expect("failed to create source URI"),
        )
        .with_message_id(message_id)
        .with_ttl(TTL)
        .with_priority(PRIORITY)
        .with_comm_status(UCode::OK)
        .with_traceparent(TRACEPARENT)
        .build_with_payload(DATA.to_vec(), UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF)
        .expect("failed to create message");

        let event =
            get_cloudevent(message, false).expect("failed to create CloudEvent from UMessage");
        assert_standard_cloudevent_attributes(&event, "res.v1", METHOD, Some(REPLY_TO.to_string()));
        assert_eq!(
            event.extension(EXTENSION_NAME_COMMSTATUS),
            Some(&ExtensionValue::Integer(UCode::OK.value() as i64))
        );
        assert_eq!(
            event.datacontenttype().map(|v| v.to_string()),
            UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF.to_media_type()
        );
        match event.data() {
            Some(Data::Binary(payload)) => {
                assert_eq!(payload, &DATA.to_vec())
            }
            _ => panic!("unexpected payload format"),
        }
    }

    //
    // tests asserting conversion of CloudEvent -> UMessage
    //

    fn assert_standard_umessage_attributes(
        attribs: &UAttributes,
        message_type: UMessageType,
        source: &str,
        sink: Option<String>,
    ) {
        assert_eq!(attribs.type_.enum_value_or_default(), message_type);
        assert_eq!(
            attribs.id.get_or_default().to_hyphenated_string(),
            MESSAGE_ID
        );
        assert_eq!(attribs.source.get_or_default().to_uri(false), source);
        assert_eq!(attribs.sink.as_ref().map(|uuri| uuri.to_uri(false)), sink);
        assert_eq!(
            attribs.priority.enum_value_or_default(),
            UPriority::UPRIORITY_CS4
        );
        assert_eq!(attribs.ttl, Some(TTL));
        assert_eq!(attribs.traceparent, Some(TRACEPARENT.to_string()));
    }

    #[test]
    fn test_get_umessage_fails_for_cloudevent_with_missing_sink() {
        let event = cloudevents::EventBuilderV10::new()
            .ty(UMessageType::UMESSAGE_TYPE_NOTIFICATION.to_cloudevent_type())
            .id(MESSAGE_ID)
            .source(TOPIC)
            .build()
            .expect("failed to create CloudEvent");

        assert!(get_umessage(event.clone(), false).is_err());
        assert!(
            get_umessage(event, true).is_ok(),
            "skipping validation should have ignored the missing destination"
        );
    }

    #[test]
    fn test_get_umessage_for_publish_cloudevent_succeeds() {
        let event = cloudevents::EventBuilderV10::new()
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH.to_cloudevent_type())
            .id(MESSAGE_ID)
            .source(TOPIC)
            .extension(
                EXTENSION_NAME_PRIORITY,
                UPriority::UPRIORITY_CS4.to_priority_code(),
            )
            .extension(EXTENSION_NAME_TTL, TTL as i64)
            .extension(EXTENSION_NAME_TRACEPARENT, TRACEPARENT)
            .data(
                UPayloadFormat::UPAYLOAD_FORMAT_TEXT
                    .to_media_type()
                    .unwrap(),
                Data::String("test".to_string()),
            )
            .build()
            .expect("failed to create CloudEvent");

        let umessage =
            get_umessage(event, false).expect("failed to create UMessage from CloudEvent");
        let attribs = umessage.attributes.get_or_default();
        assert_standard_umessage_attributes(
            attribs,
            UMessageType::UMESSAGE_TYPE_PUBLISH,
            TOPIC,
            None,
        );
        assert_eq!(
            attribs.payload_format.enum_value_or_default(),
            UPayloadFormat::UPAYLOAD_FORMAT_TEXT
        );
        assert_eq!(umessage.payload, Some("test".as_bytes().to_vec().into()))
    }

    #[test]
    fn test_get_umessage_for_notification_cloudevent_succeeds() {
        let event = cloudevents::EventBuilderV10::new()
            .ty(UMessageType::UMESSAGE_TYPE_NOTIFICATION.to_cloudevent_type())
            .id(MESSAGE_ID)
            .source(TOPIC)
            .extension(EXTENSION_NAME_SINK, DESTINATION)
            .extension(
                EXTENSION_NAME_PRIORITY,
                UPriority::UPRIORITY_CS4.to_priority_code(),
            )
            .extension(EXTENSION_NAME_TTL, TTL as i64)
            .extension(EXTENSION_NAME_TRACEPARENT, TRACEPARENT)
            .data(
                UPayloadFormat::UPAYLOAD_FORMAT_JSON
                    .to_media_type()
                    .unwrap(),
                Data::String("{\"count\": 5}".to_string()),
            )
            .build()
            .expect("failed to create CloudEvent");

        let umessage =
            get_umessage(event, false).expect("failed to create UMessage from CloudEvent");
        let attribs = umessage.attributes.get_or_default();
        assert_standard_umessage_attributes(
            attribs,
            UMessageType::UMESSAGE_TYPE_NOTIFICATION,
            TOPIC,
            Some(DESTINATION.to_string()),
        );
        assert_eq!(
            attribs.payload_format.enum_value_or_default(),
            UPayloadFormat::UPAYLOAD_FORMAT_JSON
        );
        assert_eq!(
            umessage.payload,
            Some("{\"count\": 5}".as_bytes().to_vec().into())
        )
    }

    #[test]
    fn test_get_umessage_for_request_cloudevent_succeeds() {
        let event = cloudevents::EventBuilderV10::new()
            .ty(UMessageType::UMESSAGE_TYPE_REQUEST.to_cloudevent_type())
            .id(MESSAGE_ID)
            .source(REPLY_TO)
            .extension(EXTENSION_NAME_SINK, METHOD)
            .extension(
                EXTENSION_NAME_PRIORITY,
                UPriority::UPRIORITY_CS4.to_priority_code(),
            )
            .extension(EXTENSION_NAME_PERMISSION_LEVEL, PERMISSION_LEVEL as i64)
            .extension(EXTENSION_NAME_TOKEN, "my-token")
            .extension(EXTENSION_NAME_TTL, TTL as i64)
            .extension(EXTENSION_NAME_TRACEPARENT, TRACEPARENT)
            .build()
            .expect("failed to create CloudEvent");

        let umessage =
            get_umessage(event, false).expect("failed to create UMessage from CloudEvent");
        let attribs = umessage.attributes.get_or_default();
        assert_standard_umessage_attributes(
            attribs,
            UMessageType::UMESSAGE_TYPE_REQUEST,
            REPLY_TO,
            Some(METHOD.to_string()),
        );
        assert_eq!(attribs.permission_level, Some(PERMISSION_LEVEL));
        assert_eq!(attribs.token, Some("my-token".to_string()));
    }

    #[test]
    fn test_get_umessage_for_response_cloudevent_succeeds() {
        let request_id = UUID::build();
        let event = cloudevents::EventBuilderV10::new()
            .ty(UMessageType::UMESSAGE_TYPE_RESPONSE.to_cloudevent_type())
            .id(MESSAGE_ID)
            .source(METHOD)
            .extension(EXTENSION_NAME_SINK, REPLY_TO)
            .extension(
                EXTENSION_NAME_PRIORITY,
                UPriority::UPRIORITY_CS4.to_priority_code(),
            )
            .extension(EXTENSION_NAME_COMMSTATUS, UCode::OK.value() as i64)
            .extension(EXTENSION_NAME_TTL, TTL as i64)
            .extension(EXTENSION_NAME_TRACEPARENT, TRACEPARENT)
            .extension(EXTENSION_NAME_REQUEST_ID, request_id.to_hyphenated_string())
            .data(
                UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF
                    .to_media_type()
                    .unwrap(),
                Data::Binary(DATA.to_vec()),
            )
            .build()
            .expect("failed to create CloudEvent");

        let umessage =
            get_umessage(event, false).expect("failed to create UMessage from CloudEvent");
        let attribs = umessage.attributes.get_or_default();
        assert_standard_umessage_attributes(
            attribs,
            UMessageType::UMESSAGE_TYPE_RESPONSE,
            METHOD,
            Some(REPLY_TO.to_string()),
        );
        assert_eq!(attribs.commstatus, Some(UCode::OK.into()));
        assert_eq!(attribs.reqid, Some(request_id).into());
        assert_eq!(
            attribs.payload_format.enum_value_or_default(),
            UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF
        );
        assert_eq!(umessage.payload, Some(DATA.to_vec().into()))
    }
}
