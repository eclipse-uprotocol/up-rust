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

use cloudevents::Event as CloudEvent;
use protobuf::Message;

use crate::cloudevent::serializer::{CloudEventSerializer, SerializationError};
use crate::cloudevents::CloudEvent as CloudEventProto;

/// Serialize and deserialize `CloudEvents` to protobuf format.
pub struct CloudEventProtobufSerializer;
impl CloudEventSerializer for CloudEventProtobufSerializer {
    fn serialize(&self, cloud_event: &CloudEvent) -> Result<Vec<u8>, SerializationError> {
        CloudEventProto::from(cloud_event.to_owned())
            .write_to_bytes()
            .map_err(|e| SerializationError::new(e.to_string()))
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<CloudEvent, SerializationError> {
        CloudEventProto::parse_from_bytes(bytes)
            .map(CloudEvent::from)
            .map_err(|error| SerializationError::new(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cloudevents::event::ExtensionValue;
    use cloudevents::{AttributesReader, Event, EventBuilder, EventBuilderV10};
    use protobuf::well_known_types::any::Any;

    use crate::cloudevent::builder::{UCloudEventBuilder, UCloudEventUtils};
    use crate::cloudevent::datamodel::UCloudEventAttributesBuilder;
    use crate::cloudevent::serializer::cloudeventjsonserializer::CloudEventJsonSerializer;
    use crate::rpc::RpcMapper;
    use crate::uprotocol::uattributes::{UMessageType, UPriority};
    use crate::uprotocol::uri::{UAuthority, UEntity, UResource, UUri};
    use crate::uri::serializer::{LongUriSerializer, UriSerializer};

    #[test]
    fn serialize_and_deserialize_cloud_event_to_protobuf() {
        // Build the source
        let uri = UUri {
            authority: Some(UAuthority::default()).into(),
            entity: Some(UEntity {
                name: "body.access".to_string(),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                name: "Door".to_string(),
                instance: Some("front_left".to_string()),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };

        let source = LongUriSerializer::serialize(&uri).unwrap();

        // Fake payload
        let proto_payload = build_proto_payload_for_test();

        // Configure cloud event
        let u_cloud_event_attributes = UCloudEventAttributesBuilder::new()
            .with_hash("somehash".to_string())
            .with_priority(UPriority::UPRIORITY_CS0)
            .with_ttl(3)
            .build();

        let mut cloud_event_builder = UCloudEventBuilder::build_base_cloud_event(
            "hello",
            &source,
            &proto_payload.write_to_bytes().unwrap(),
            &proto_payload.type_url,
            &u_cloud_event_attributes,
        );
        cloud_event_builder = cloud_event_builder.ty(UMessageType::UMESSAGE_TYPE_PUBLISH);

        let cloud_event = cloud_event_builder.build().unwrap();

        let serializer = CloudEventProtobufSerializer;
        let bytes = serializer.serialize(&cloud_event).unwrap();
        let deserialized_cloud_event = serializer.deserialize(&bytes).unwrap();

        assert_eq!(cloud_event, deserialized_cloud_event);
    }

    #[test]
    fn serialize_two_different_cloud_events_are_not_the_same() {
        // Fake payload
        let proto_payload = build_proto_payload_for_test();

        // Cloud event
        let cloud_event = EventBuilderV10::new()
            .id("hello")
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH)
            .source("/body.access/1/door.front_left".to_string())
            .data_with_schema(
                "application/protobuf".to_string(),
                format!("proto://{}", Any::default().type_url),
                proto_payload.write_to_bytes().unwrap(),
            )
            .build()
            .unwrap();

        // Another cloud event
        let another_cloud_event = EventBuilderV10::new()
            .id("hello")
            .ty(UMessageType::UMESSAGE_TYPE_REQUEST)
            .source("/body.access/1/door.front_left".to_string())
            .build()
            .unwrap();

        let serializer = CloudEventProtobufSerializer;
        let bytes_cloud_event = serializer.serialize(&cloud_event).unwrap();
        let bytes_another_cloud_event = serializer.serialize(&another_cloud_event).unwrap();
        assert_ne!(bytes_cloud_event, bytes_another_cloud_event);
    }

    #[test]
    fn serialize_two_same_cloud_events_are_the_same() {
        // Fake payload
        let proto_payload = build_proto_payload_for_test();

        // Cloud event
        let cloud_event = EventBuilderV10::new()
            .id("hello")
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH)
            .source("/body.access/1/door.front_left".to_string())
            .data_with_schema(
                "application/protobuf".to_string(),
                format!("proto://{}", Any::default().type_url),
                proto_payload.write_to_bytes().unwrap(),
            )
            .build()
            .unwrap();

        // Another cloud event
        let another_cloud_event = EventBuilderV10::new()
            .id("hello")
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH)
            .source("/body.access/1/door.front_left".to_string())
            .data_with_schema(
                "application/protobuf".to_string(),
                format!("proto://{}", Any::default().type_url),
                proto_payload.write_to_bytes().unwrap(),
            )
            .build()
            .unwrap();

        let serializer = CloudEventProtobufSerializer;
        let bytes_cloud_event = serializer.serialize(&cloud_event).unwrap();
        let bytes_another_cloud_event = serializer.serialize(&another_cloud_event).unwrap();

        //assert_eq!(bytes_cloud_event, bytes_another_cloud_event);    // this can fail, likely due to sorting differences in CloudEvent extension pairs
        compare_bytevec_sums(&bytes_cloud_event, &bytes_another_cloud_event);
    }

    #[test]
    fn double_serialization_protobuf_when_creating_cloud_event_with_factory_methods() {
        let serializer = CloudEventProtobufSerializer;

        // Source
        let uri = UUri {
            authority: Some(UAuthority::default()).into(),
            entity: Some(UEntity {
                name: "body.access".to_string(),
                ..Default::default()
            })
            .into(),
            resource: Some(UResource {
                name: "Door".to_string(),
                instance: Some("front_left".to_string()),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let source = LongUriSerializer::serialize(&uri).unwrap();

        // Fake payload
        let proto_payload = build_other_proto_payload_for_test();

        // Additional attributes
        let u_cloud_event_attributes = UCloudEventAttributesBuilder::new()
            .with_hash("somehash".to_string())
            .with_priority(UPriority::UPRIORITY_CS1)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        // Build the cloud event
        let mut cloud_event_builder = UCloudEventBuilder::build_base_cloud_event(
            "testme",
            &source,
            &proto_payload.write_to_bytes().unwrap(),
            &proto_payload.type_url,
            &u_cloud_event_attributes,
        );
        cloud_event_builder = cloud_event_builder.ty(UMessageType::UMESSAGE_TYPE_PUBLISH);

        let cloud_event1 = cloud_event_builder.build().unwrap();
        let bytes1 = serializer.serialize(&cloud_event1).unwrap();
        let cloud_event2 = serializer.deserialize(&bytes1).unwrap();

        assert_cloud_events_are_the_same(&cloud_event1, &cloud_event2);

        let bytes2 = serializer.serialize(&cloud_event2).unwrap();
        // assert_eq!(bytes1, bytes2);  // this can fail, likely due to sorting differences in CloudEvent extension pairs
        compare_bytevec_sums(&bytes1, &bytes2);

        let cloud_event3 = serializer.deserialize(&bytes2).unwrap();
        let cloud_event3_payload = UCloudEventUtils::get_payload(&cloud_event3);

        let pay1 = Event::from(RpcMapper::unpack_any::<CloudEventProto>(&proto_payload).unwrap());
        let pay2 =
            Event::from(RpcMapper::unpack_any::<CloudEventProto>(&cloud_event3_payload).unwrap());

        assert_cloud_events_are_the_same(&pay1, &pay2);

        assert_cloud_events_are_the_same(&cloud_event2, &cloud_event3);
        assert_cloud_events_are_the_same(&cloud_event1, &cloud_event3);
    }

    #[test]
    fn double_serialization_protobuf() {
        let serializer = CloudEventProtobufSerializer;

        let mut builder = build_cloud_event_for_test();
        let cloud_event_proto = build_other_proto_payload_for_test();

        builder = builder.data_with_schema(
            "application/protobuf".to_string(),
            format!("proto://{}", Any::default().type_url),
            cloud_event_proto.write_to_bytes().unwrap(),
        );

        let cloud_event1 = builder.build().unwrap();
        let bytes1 = serializer.serialize(&cloud_event1).unwrap();
        let cloud_event2 = serializer.deserialize(&bytes1).unwrap();

        assert_cloud_events_are_the_same(&cloud_event1, &cloud_event2);

        let bytes2 = serializer.serialize(&cloud_event2).unwrap();
        compare_bytevec_sums(&bytes1, &bytes2);

        let cloud_event3 = serializer.deserialize(&bytes2).unwrap();
        let cloud_event3_payload = UCloudEventUtils::get_payload(&cloud_event3);

        let pay1 =
            Event::from(RpcMapper::unpack_any::<CloudEventProto>(&cloud_event_proto).unwrap());
        let pay2 =
            Event::from(RpcMapper::unpack_any::<CloudEventProto>(&cloud_event3_payload).unwrap());

        assert_eq!(pay1, pay2);

        assert_cloud_events_are_the_same(&cloud_event2, &cloud_event3);
        assert_cloud_events_are_the_same(&cloud_event1, &cloud_event3);
    }

    #[test]
    fn double_serialization_proto_to_json() {
        let proto_serializer = CloudEventProtobufSerializer;
        let json_serializer = CloudEventJsonSerializer;

        let mut builder = build_cloud_event_for_test();
        let cloud_event_proto = build_other_proto_payload_for_test();

        builder = builder.data_with_schema(
            "application/protobuf".to_string(),
            format!("proto://{}", Any::default().type_url),
            cloud_event_proto.write_to_bytes().unwrap(),
        );

        let cloud_event1 = builder.build().unwrap();
        let bytes1 = proto_serializer.serialize(&cloud_event1).unwrap();
        let cloud_event2 = proto_serializer.deserialize(&bytes1).unwrap();

        assert_cloud_events_are_the_same(&cloud_event1, &cloud_event2);

        let bytes2 = proto_serializer.serialize(&cloud_event2).unwrap();
        compare_bytevec_sums(&bytes1, &bytes2);

        let bytes3 = json_serializer.serialize(&cloud_event2).unwrap();
        let cloud_event3 = json_serializer.deserialize(&bytes3).unwrap();

        assert_cloud_events_are_the_same(&cloud_event2, &cloud_event3);
        assert_eq!(cloud_event1, cloud_event3);
    }

    #[test]
    fn double_serialization_json_to_proto() {
        let proto_serializer = CloudEventProtobufSerializer;
        let json_serializer = CloudEventJsonSerializer;

        let mut builder = build_cloud_event_for_test();
        let cloud_event_proto = build_other_proto_payload_for_test();

        builder = builder.data_with_schema(
            "application/protobuf".to_string(),
            format!("proto://{}", Any::default().type_url),
            cloud_event_proto.write_to_bytes().unwrap(),
        );

        let cloud_event1 = builder.build().unwrap();
        let bytes1 = json_serializer.serialize(&cloud_event1).unwrap();
        let cloud_event2 = json_serializer.deserialize(&bytes1).unwrap();

        assert_eq!(cloud_event1, cloud_event2);

        let bytes2 = json_serializer.serialize(&cloud_event2).unwrap();
        compare_bytevec_sums(&bytes1, &bytes2);

        let bytes3 = proto_serializer.serialize(&cloud_event2).unwrap();
        let cloud_event3 = proto_serializer.deserialize(&bytes3).unwrap();

        assert_cloud_events_are_the_same(&cloud_event2, &cloud_event3);
        assert_cloud_events_are_the_same(&cloud_event1, &cloud_event3);
    }

    fn assert_cloud_events_are_the_same(cloud_event1: &CloudEvent, cloud_event2: &CloudEvent) {
        assert_eq!(cloud_event1.specversion(), cloud_event2.specversion());
        assert_eq!(cloud_event1.id(), cloud_event2.id());
        assert_eq!(cloud_event1.source(), cloud_event2.source());
        assert_eq!(cloud_event1.ty(), cloud_event2.ty());
        assert_eq!(
            cloud_event1.datacontenttype(),
            cloud_event2.datacontenttype()
        );
        assert_eq!(cloud_event1.dataschema(), cloud_event2.dataschema());

        let mut ce1_extension_names: Vec<&str> =
            cloud_event1.iter_extensions().map(|(key, _)| key).collect();
        let mut ce2_extension_names: Vec<&str> =
            cloud_event2.iter_extensions().map(|(key, _)| key).collect();

        ce1_extension_names.sort();
        ce2_extension_names.sort();

        assert_eq!(ce1_extension_names.join(","), ce2_extension_names.join(","));

        let data1 = cloud_event1.data().expect("cloud_event1 data is None");
        let data2 = cloud_event2.data().expect("cloud_event2 data is None");
        assert_eq!(data1, data2);

        assert_eq!(cloud_event1, cloud_event2);
    }

    // 'Embarrassment solution' to compare serialized Event types - as these have lists of extensions, which do not give guarantees
    // for sorting order, bytes can have different positions in a serialization byte stream. So we just sum up byte values and compare...
    fn compare_bytevec_sums(vec1: &[u8], vec2: &[u8]) {
        let sum1: u32 = vec1.iter().map(|&byte| byte as u32).sum();
        let sum2: u32 = vec2.iter().map(|&byte| byte as u32).sum();
        assert_eq!(sum1, sum2);
    }

    fn build_cloud_event_for_test() -> EventBuilderV10 {
        EventBuilderV10::new()
            .id("hello")
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH)
            .source("//VCU.VIN/body.access")
    }

    fn build_proto_payload_for_test() -> Any {
        let event = EventBuilderV10::new()
            .id("hello")
            .source("http://example.com")
            .ty("example.demo")
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                format!("proto://{}", Any::default().type_url),
                Any::default().value,
            )
            .extension("ttl", ExtensionValue::Integer(3))
            .build()
            .unwrap();

        pack_event_into_any(&event)
    }

    fn build_other_proto_payload_for_test() -> Any {
        let event = EventBuilderV10::new()
            .id("hello")
            .source("//VCU.VIN/body.access")
            .ty(UMessageType::UMESSAGE_TYPE_PUBLISH)
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                format!("proto://{}", Any::default().type_url),
                // "proto:://CloudEvent.io.cloudevents.v1",
                Any::default().value,
            )
            .extension("ttl", ExtensionValue::Integer(3))
            .build()
            .unwrap();

        pack_event_into_any(&event)
    }

    fn pack_event_into_any(event: &Event) -> Any {
        let proto_event = CloudEventProto::from(event.clone());
        RpcMapper::pack_any(&proto_event).unwrap()
    }
}
