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

use chrono::{DateTime, NaiveDateTime, Utc};
use cloudevents::event::ExtensionValue;
use cloudevents::{AttributesReader, AttributesWriter, Data, EventBuilder};

use std::collections::HashMap;
use url::Url;

use crate::cloudevents::cloud_event::cloud_event_attribute_value::Attr;
use crate::cloudevents::cloud_event::CloudEventAttributeValue;
use crate::cloudevents::cloud_event::Data as CloudEventData;
use crate::cloudevents::CloudEvent as CloudEventProto;

impl From<CloudEventProto> for cloudevents::Event {
    fn from(source_event: CloudEventProto) -> Self {
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
                        #[allow(clippy::cast_sign_loss)]
                        let naive =
                            NaiveDateTime::from_timestamp_opt(ts.seconds, ts.nanos as u32).unwrap();
                        dt = Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
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
            Some(CloudEventData::BinaryData(data)) => Some(Data::Binary(data)),
            Some(CloudEventData::TextData(text)) => Some(Data::String(text)),
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

impl From<cloudevents::Event> for CloudEventProto {
    fn from(source_event: cloudevents::Event) -> Self {
        let mut ext_list = HashMap::<String, CloudEventAttributeValue>::new();

        // subject
        // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
        if let Some(subject) = source_event.subject() {
            let s = CloudEventAttributeValue {
                attr: Some(Attr::CeString(subject.to_string())),
                ..Default::default()
            };
            ext_list.insert("subject".to_string(), s);
        }

        // timestamp
        // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
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
        // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
        if let Some(schema) = source_event.dataschema() {
            let ds = CloudEventAttributeValue {
                attr: Some(Attr::CeUri(schema.to_string())),
                ..Default::default()
            };
            ext_list.insert("dataschema".to_string(), ds);
        }

        // contenttype
        // TODO how is this serialized by eg the Java libraries, considering cloudevent.proto is missing dedicated attributes for this?
        if let Some(contenttype) = source_event.datacontenttype() {
            let ct = CloudEventAttributeValue {
                attr: Some(Attr::CeString(contenttype.to_string())),
                ..Default::default()
            };
            ext_list.insert("contenttype".to_string(), ct);
        }

        // Extract data - the proto serialization knows a protobuf.Any type!... something there?
        let event_data = match source_event.data() {
            Some(Data::Binary(bytes)) => Some(CloudEventData::BinaryData(bytes.clone())),
            Some(Data::String(s)) => Some(CloudEventData::TextData(s.to_string())),
            Some(Data::Json(j)) => Some(CloudEventData::TextData(j.to_string())),
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
        CloudEventProto {
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
    use crate::cloudevent::builder::UCloudEventBuilder;
    use crate::cloudevent::datamodel::UCloudEventAttributes;
    use crate::uprotocol::uattributes::{UMessageType, UPriority};
    use crate::uprotocol::uri::{UEntity, UResource, UUri};
    use crate::uri::serializer::{LongUriSerializer, UriSerializer};

    use cloudevents::{Data, Event, EventBuilder, EventBuilderV10};
    use protobuf::well_known_types::any::Any;

    #[test]
    fn test_cloudevent_to_proto() {
        let origin = build_base_cloud_event_for_test().build().unwrap();
        let proto = CloudEventProto::from(origin.clone());
        let dest = cloudevents::Event::from(proto);

        assert_eq!(origin, dest);
    }

    fn build_base_cloud_event_for_test() -> EventBuilderV10 {
        let uri = UUri {
            entity: Some(UEntity {
                name: "body.access".to_string(),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                name: "door".to_string(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let source = LongUriSerializer::serialize(&uri).unwrap();

        // fake payload
        let payload = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let attributes = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(UPriority::UPRIORITY_CS0)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        let event = UCloudEventBuilder::build_base_cloud_event(
            "testme",
            &source,
            &payload.value,
            payload.type_url.as_str(),
            &attributes,
        );
        event.ty(UMessageType::UMESSAGE_TYPE_PUBLISH.to_type_string())
    }

    fn pack_event_into_any(event: &Event) -> Any {
        let data_bytes: Vec<u8> = match event.data() {
            Some(Data::Binary(bytes)) => bytes.clone(),
            Some(Data::String(s)) => s.as_bytes().to_vec(),
            Some(Data::Json(j)) => j.to_string().into_bytes(),
            None => Vec::new(),
        };

        // The cloudevent crate uses the url crate for storing dataschema, which needs a schema prefix to work,
        // which gets added in UCloudEventBuilder::build_base_cloud_event() or in related test cases.
        // And this schema prefix needs to be removed again here:
        let schema = {
            let temp_schema = event.dataschema().unwrap().to_string();
            temp_schema
                .strip_prefix("proto://")
                .unwrap_or(&temp_schema)
                .to_string()
        };

        Any {
            type_url: schema,
            value: data_bytes,
            ..Default::default()
        }
    }

    fn build_proto_payload_for_test() -> Event {
        EventBuilderV10::new()
            .id("hello")
            .source("//VCU.MY_CAR_VIN/body.access//door.front_left#Door")
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH.to_type_string())
            .data_with_schema(
                "application/octet-stream",
                "proto://type.googleapis.com/example.demo",
                Any::default().value,
            )
            .build()
            .unwrap()
    }
}
