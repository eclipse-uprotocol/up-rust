/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
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

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use url::Url;

// cloudevents-sdk
use cloudevents::{event::ExtensionValue, AttributesReader, AttributesWriter, Data, EventBuilder};

// protoc-generated code from cloudevents.proto
use crate::proto_cloudevents::cloudevents::CloudEvent as ProtoCloudEvent;
use crate::proto_cloudevents::Data as ProtoCloudEventData;
use crate::proto_cloudevents::{cloud_event_attribute_value::Attr, CloudEventAttributeValue};

use crate::{UMessage, UPriority};

mod cloudeventvalidator;
pub use cloudeventvalidator::*;
mod ucloudeventattributes;
pub use ucloudeventattributes::*;
mod ucloudeventbuilder;
pub use ucloudeventbuilder::*;
mod ucloudeventutils;
pub use ucloudeventutils::*;

mod cloudeventjsonserializer;
pub use cloudeventjsonserializer::*;
mod cloudeventprotobufserializer;
pub use cloudeventprotobufserializer::*;
mod cloudeventserializer;
pub use cloudeventserializer::*;

#[derive(Debug)]
pub enum CloudEventError {
    ParsingError(String),
    SerializationError(String),
    ValidationError(String),
}

impl CloudEventError {
    pub fn parsing_error<T>(message: T) -> CloudEventError
    where
        T: Into<String>,
    {
        Self::ParsingError(message.into())
    }

    pub fn serialization_error<T>(message: T) -> CloudEventError
    where
        T: Into<String>,
    {
        Self::SerializationError(message.into())
    }

    pub fn validation_error<T>(message: T) -> CloudEventError
    where
        T: Into<String>,
    {
        Self::ValidationError(message.into())
    }
}

impl std::fmt::Display for CloudEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParsingError(e) => f.write_fmt(format_args!("Parsing error: {}", e)),
            Self::SerializationError(e) => f.write_fmt(format_args!("Serialization error: {}", e)),
            Self::ValidationError(e) => f.write_fmt(format_args!("Validation failure: {}", e)),
        }
    }
}

impl std::error::Error for CloudEventError {}

impl TryFrom<UMessage> for cloudevents::Event {
    type Error = CloudEventError;

    fn try_from(source_event: UMessage) -> Result<Self, Self::Error> {
        let attributes = source_event.attributes.get_or_default();
        let source = attributes.source.get_or_default();
        let payload = source_event.payload.get_or_default();
        let uri = String::try_from(source)
            .map_err(|e| CloudEventError::serialization_error(e.to_string()))?;

        if attributes.id.is_none() {
            return Err(CloudEventError::serialization_error("Empty attributes ID"));
        }
        if attributes.type_.enum_value().is_err() {
            return Err(CloudEventError::serialization_error("Bad attributes type"));
        }

        let event_builder = cloudevents::EventBuilderV10::new()
            .id(attributes.id.get_or_default())
            .ty(attributes.type_.enum_value_or_default().to_type_string())
            .source(uri);
        let mut event = event_builder
            .build()
            .map_err(|e| {
                CloudEventError::serialization_error(format!("Error creating cloudevent: {e}"))
            })
            .unwrap();

        let ctype = payload.format.enum_value_or_default();
        event.set_datacontenttype(ctype.to_media_type());

        if payload.has_value() {
            event.set_data_unchecked(payload.value().to_vec());
        }
        if let Some(ttl) = attributes.ttl {
            event.set_extension("ttl", ExtensionValue::Integer(ttl as i64));
        }
        if let Some(token) = &attributes.token {
            event.set_extension("token", ExtensionValue::String(token.to_string()));
        }
        if attributes
            .priority
            .enum_value()
            .is_ok_and(|p| p != UPriority::UPRIORITY_UNSPECIFIED)
        {
            event.set_extension(
                "priority",
                ExtensionValue::String(
                    attributes.priority.enum_value().unwrap().to_priority_code(),
                ),
            );
        }
        if let Some(sink) = &attributes.sink.clone().into_option() {
            let uri = String::try_from(sink)
                .map_err(|e| CloudEventError::serialization_error(e.to_string()))?;
            event.set_extension("sink", ExtensionValue::String(uri));
        }
        if let Some(commstatus) = attributes.commstatus {
            event.set_extension("commstatus", ExtensionValue::Integer(commstatus as i64));
        }
        if let Some(reqid) = &attributes.reqid.clone().into_option() {
            event.set_extension("reqid", ExtensionValue::String(reqid.to_string()));
        }
        if let Some(plevel) = attributes.permission_level {
            event.set_extension("plevel", ExtensionValue::Integer(plevel as i64));
        }

        Ok(event)
    }
}

impl From<ProtoCloudEvent> for cloudevents::Event {
    fn from(source_event: ProtoCloudEvent) -> Self {
        let mut subject: Option<String> = None;
        let mut dt: Option<DateTime<Utc>> = None;
        let mut dataschema: Option<Url> = None;
        let mut contenttype: Option<String> = None;

        // extensions
        let mut extensions = HashMap::<String, ExtensionValue>::new();
        for (key, value) in &source_event.attributes {
            match value.attr.as_ref().unwrap() {
                Attr::CeBoolean(b) => {
                    extensions.insert(key.to_string(), ExtensionValue::Boolean(*b));
                }
                Attr::CeBytes(_bytes) => {
                    // TODO not quite sure whether/how to map this to ExtensionValue::String
                }
                Attr::CeInteger(i) => {
                    extensions.insert(key.to_string(), ExtensionValue::Integer(i64::from(*i)));
                }
                Attr::CeString(s) => {
                    // contenttype
                    // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
                    if key.eq("contenttype") {
                        contenttype = Some(s.to_string());
                    } else if key.eq("subject") {
                        subject = Some(s.to_string());
                    } else {
                        extensions.insert(key.to_string(), ExtensionValue::String(s.to_string()));
                    }
                }
                Attr::CeTimestamp(ts) => {
                    // timestamp
                    // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
                    if key.eq("timestamp") {
                        dt = Some(DateTime::from_timestamp(ts.seconds, ts.nanos as u32).unwrap());
                    } else {
                        extensions.insert(key.to_string(), ExtensionValue::String(ts.to_string()));
                    }
                }
                Attr::CeUri(uri) => {
                    // dataschema
                    // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
                    if key.eq("dataschema") {
                        if let Ok(url) = Url::parse(uri.as_str()) {
                            dataschema = Some(url);
                        }
                        // if Url::parse() doesn't work, this attribute is lost
                    } else {
                        extensions.insert(key.to_string(), ExtensionValue::String(uri.to_string()));
                    }
                }
                Attr::CeUriRef(uriref) => {
                    extensions.insert(key.to_string(), ExtensionValue::String(uriref.to_string()));
                }
            }
        }

        // Could discriminate CloudEvent spec versions here, according to event.specversion. But ignored for now, this is all 1.0
        let mut event_builder = cloudevents::EventBuilderV10::new()
            .id(source_event.id)
            .source(source_event.source)
            .ty(source_event.type_);

        if let Some(s) = subject {
            event_builder = event_builder.subject(s);
        }

        if let Some(time) = dt {
            event_builder = event_builder.time(time);
        }
        let mut cloud_event = event_builder.build().unwrap();

        // Extract data - the proto serialization knows a protobuf.Any type!... something there?
        let event_data: Option<Data> = match source_event.data {
            Some(ProtoCloudEventData::BinaryData(data)) => Some(Data::Binary(data)),
            Some(ProtoCloudEventData::TextData(text)) => Some(Data::String(text)),
            _ => None,
        };
        if let Some(data) = event_data {
            cloud_event.set_data_unchecked(data);
        }
        cloud_event.set_datacontenttype(contenttype);
        cloud_event.set_dataschema(dataschema);

        for (key, value) in &extensions {
            cloud_event.set_extension(key, value.clone());
        }

        cloud_event
    }
}

impl From<cloudevents::Event> for ProtoCloudEvent {
    fn from(source_event: cloudevents::Event) -> Self {
        let mut ext_list = HashMap::<String, CloudEventAttributeValue>::new();

        // subject
        if let Some(subject) = source_event.subject() {
            let s = CloudEventAttributeValue {
                attr: Some(Attr::CeString(subject.to_string())),
                ..Default::default()
            };
            ext_list.insert("subject".to_string(), s);
        }

        // timestamp
        if source_event.time().is_some() {
            let time = *source_event.time().unwrap();
            let sys_time: std::time::SystemTime = time.into();

            let timesstamp = CloudEventAttributeValue {
                attr: Some(Attr::CeTimestamp(
                    protobuf::well_known_types::timestamp::Timestamp::from(sys_time),
                )),
                ..Default::default()
            };
            ext_list.insert("timestamp".to_string(), timesstamp);
        }

        // dataschema
        if let Some(schema) = source_event.dataschema() {
            let ds = CloudEventAttributeValue {
                attr: Some(Attr::CeUri(schema.to_string())),
                ..Default::default()
            };
            ext_list.insert("dataschema".to_string(), ds);
        }

        // contenttype
        if let Some(contenttype) = source_event.datacontenttype() {
            let ct = CloudEventAttributeValue {
                attr: Some(Attr::CeString(contenttype.to_string())),
                ..Default::default()
            };
            ext_list.insert("contenttype".to_string(), ct);
        }

        // Extract data - the proto serialization knows a protobuf.Any type!... something there?
        let event_data = match source_event.data() {
            Some(Data::Binary(bytes)) => Some(ProtoCloudEventData::BinaryData(bytes.clone())),
            Some(Data::String(s)) => Some(ProtoCloudEventData::TextData(s.to_string())),
            Some(Data::Json(j)) => Some(ProtoCloudEventData::TextData(j.to_string())),
            None => None,
        };

        // Do extensions
        for (key, value) in source_event.iter_extensions() {
            match value {
                ExtensionValue::Boolean(b) => {
                    let ext = CloudEventAttributeValue {
                        attr: Some(Attr::CeBoolean(*b)),
                        ..Default::default()
                    };
                    ext_list.insert(key.to_string(), ext);
                }
                #[allow(clippy::cast_possible_truncation)]
                ExtensionValue::Integer(i) => {
                    let ext = CloudEventAttributeValue {
                        attr: Some(Attr::CeInteger(*i as i32)),
                        ..Default::default()
                    };
                    ext_list.insert(key.to_string(), ext);
                }
                ExtensionValue::String(s) => {
                    let ext = CloudEventAttributeValue {
                        attr: Some(Attr::CeString(s.to_string())),
                        ..Default::default()
                    };
                    ext_list.insert(key.to_string(), ext);
                }
            }
        }

        // Construct target event
        ProtoCloudEvent {
            spec_version: cloudevents::event::SpecVersion::V10.to_string(),
            id: source_event.id().to_string(),
            source: source_event.source().to_string(),
            type_: source_event.ty().to_string(),
            data: event_data,
            attributes: ext_list,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::up_core_api::uattributes::UAttributes;
    use crate::up_core_api::uattributes::UMessageType;
    use crate::up_core_api::upayload::{UPayload, UPayloadFormat};
    use crate::uri::{UAuthority, UEntity, UResource, UUri};
    use crate::uuid::UUIDBuilder;

    use cloudevents::{Event, EventBuilder, EventBuilderV10};
    use protobuf::well_known_types::any::Any;

    #[test]
    fn test_umessage_to_event() {
        let (message, event, _event_proto) = build_message_formats_for_test();
        let dest = Event::try_from(message).unwrap();

        assert_eq!(event, dest);
    }

    #[test]
    fn test_event_to_eventproto() {
        let (_message, event, event_proto) = build_message_formats_for_test();
        let dest = ProtoCloudEvent::from(event);

        assert_eq!(event_proto, dest);
    }

    #[test]
    fn test_eventproto_to_event() {
        let (_message, event, event_proto) = build_message_formats_for_test();
        let dest = Event::from(event_proto);

        assert_eq!(event, dest);
    }

    // create different event message objects with equivalent content
    fn build_message_formats_for_test() -> (UMessage, Event, ProtoCloudEvent) {
        // common parts
        let uri = UUri {
            authority: Some(UAuthority {
                name: Some("VCU.MY_CAR_VIN".into()),
                ..Default::default()
            })
            .into(),
            entity: Some(UEntity {
                name: "body.access".into(),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                name: "door".to_string(),
                instance: Some("front_left".into()),
                message: Some("Door".into()),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let uuid = UUIDBuilder::new().build();
        let payload = UPayload {
            data: Some(crate::up_core_api::upayload::upayload::Data::Value(
                Any::default().value,
            )),
            format: UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY.into(),
            ..Default::default()
        };
        let attributes = UAttributes {
            source: Some(uri.clone()).into(),
            id: Some(uuid.clone()).into(),
            type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
            ..Default::default()
        };

        // CloudEvent
        let event = EventBuilderV10::new()
            .id(uuid.to_hyphenated_string())
            .source(String::try_from(&uri).unwrap())
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH.to_type_string())
            .data("application/x-protobuf", Any::default().value)
            .build()
            .unwrap();

        // UMessage
        let message = UMessage {
            attributes: Some(attributes).into(),
            payload: Some(payload).into(),
            ..Default::default()
        };

        // protobuf CloudEvent
        let mut attr_list = HashMap::<String, CloudEventAttributeValue>::new();
        attr_list.insert(
            "contenttype".into(),
            CloudEventAttributeValue {
                attr: Some(Attr::CeString("application/x-protobuf".into())),
                ..Default::default()
            },
        );

        let event_proto = ProtoCloudEvent {
            id: uuid.to_hyphenated_string(),
            source: String::try_from(&uri).unwrap(),
            type_: UMessageType::UMESSAGE_TYPE_PUBLISH
                .to_type_string()
                .to_string(),
            attributes: attr_list,
            data: Some(ProtoCloudEventData::BinaryData(Any::default().value)),
            spec_version: "1.0".into(),
            ..Default::default()
        };

        (message, event, event_proto)
    }
}
