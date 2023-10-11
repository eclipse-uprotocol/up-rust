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

use crate::uuid::uuidbuilder::{UUIDFactory, UUIDv8Factory};
use chrono::Utc;
use cloudevents::{Event, EventBuilder, EventBuilderV10};
use prost_types::Any;
use url::{ParseError, Url};

use crate::cloudevent::datamodel::ucloudeventattributes::UCloudEventAttributes;
use crate::cloudevent::datamodel::ucloudeventtype::UCloudEventType;

pub struct UCloudEventBuilder;

impl UCloudEventBuilder {
    pub const PROTOBUF_CONTENT_TYPE: &'static str = "application/x-protobuf";

    /// In this module, we provide functions to generate concrete objects of the same type,
    /// adhering to the CloudEvents specification.
    ///
    /// CloudEvents is a specification for describing events in a common way.
    /// We use CloudEvents to formulate all kinds of events (messages)
    /// that will be sent to and from devices.
    ///
    /// This module provides the functionality to generate CloudEvents of the 4 core types: `Request`, `Response`, `Publish`, and `Notify`.

    // Returns a UUIDv8 id.
    fn create_cloudevent_id() -> String {
        UUIDv8Factory::new().create().to_string()
    }

    /// Creates a CloudEvent for an event for the use case of an RPC Request message.
    ///
    /// # Arguments
    ///
    /// * `rpc_uri` - The URI for the application requesting the RPC.
    /// * `service_method_uri` - The URI for the method to be called on the service. For example: `:/body.access/1/rpc.UpdateDoor`.
    /// * `proto_payload` - Protobuf `Any` object with the message command to be executed on the sink service.
    /// * `attributes` - Additional attributes such as ttl, hash, priority, and token.
    ///
    /// # Returns
    ///
    /// * `Event` - Returns a request CloudEvent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use prost_types::Any;
    /// use uprotocol_sdk::cloudevent::datamodel::ucloudeventattributes::{UCloudEventAttributes, Priority};
    /// use uprotocol_sdk::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    ///
    /// let rpc_uri = "http://myapp.com/rpc";
    /// let service_method_uri = ":/body.access/1/rpc.UpdateDoor";
    /// let proto_payload = Any {
    ///     type_url: "type.googleapis.com/google.protobuf.StringValue".to_string(),
    ///     value: vec![104, 101, 108, 108, 111],  // 'hello' in ASCII
    /// };
    /// let attributes = UCloudEventAttributes {
    ///     ttl: Some(60),
    ///     priority: Some(Priority::Standard),
    ///     hash: Some("123456".to_string()),
    ///     token: Some("abcdef".to_string()),
    /// };
    ///
    /// let cloudevent = UCloudEventBuilder::request(rpc_uri, service_method_uri, &proto_payload, &attributes);
    /// ````
    pub fn request(
        rpc_uri: &str,
        service_method_uri: &str,
        proto_payload: &Any,
        attributes: &UCloudEventAttributes,
    ) -> Event {
        let bce = UCloudEventBuilder::build_base_cloud_event(
            &UCloudEventBuilder::create_cloudevent_id(),
            rpc_uri,
            &proto_payload.value,
            &proto_payload.type_url,
            attributes,
        )
        .extension("sink", service_method_uri)
        .ty(UCloudEventType::REQUEST.to_string())
        .build();

        bce.unwrap()
    }

    /// Creates a CloudEvent for the use case of: RPC Response message.
    ///
    /// # Arguments
    ///
    /// * `applicationUriForRPC` - The destination of the response. The URI for the original application that requested the RPC and this response is for.
    /// * `serviceMethodUri` - The URI for the method that was called on the service, for example: `:/body.access/1/rpc.UpdateDoor`
    /// * `requestId` - The cloud event id from the original request cloud event that this response is for.
    /// * `protoPayload` - The protobuf serialized response message as defined by the application interface or the google.rpc.Status message containing the details of an error.
    /// * `attributes` - Additional attributes such as ttl, hash and priority.
    ///
    /// # Returns
    ///
    /// Returns a response CloudEvent.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    /// use uprotocol_sdk::cloudevent::datamodel::ucloudeventattributes::UCloudEventAttributes;
    /// use prost_types::Any;
    ///
    /// let rpc_uri = "https://example.com/rpc";
    /// let service_method_uri = "https://example.com/service_method";
    /// let request_id = "123456";
    /// let proto_payload = &Any {
    ///     type_url: "type".to_string(),
    ///     value: vec![],
    /// };
    /// let attributes = &UCloudEventAttributes::default();
    ///
    /// let response_event = UCloudEventBuilder::response(
    ///     rpc_uri,
    ///     service_method_uri,
    ///     request_id,
    ///     proto_payload,
    ///     attributes,
    /// );
    /// ```
    pub fn response(
        rpc_uri: &str,
        service_method_uri: &str,
        request_id: &str,
        proto_payload: &Any,
        attributes: &UCloudEventAttributes,
    ) -> Event {
        let bce = UCloudEventBuilder::build_base_cloud_event(
            &UCloudEventBuilder::create_cloudevent_id(),
            service_method_uri,
            &proto_payload.value,
            &proto_payload.type_url,
            attributes,
        )
        .extension("sink", rpc_uri)
        .extension("reqid", request_id)
        .ty(UCloudEventType::RESPONSE.to_string())
        .build();

        bce.unwrap()
    }

    /// Create a CloudEvent for an event for the use case of: RPC Response message that failed.
    ///
    /// # Arguments
    ///
    /// * `applicationUriForRPC` - The destination of the response. The uri for the original application that requested the RPC and this response is for.
    /// * `serviceMethodUri` - The uri for the method that was called on the service Ex.: :/body.access/1/rpc.UpdateDoor
    /// * `requestId` - The cloud event id from the original request cloud event that this response if for.
    /// * `communicationStatus` - A `Code` value that indicates of a platform communication error while delivering this CloudEvent.
    /// * `attributes` - Additional attributes such as ttl, hash and priority.
    ///
    /// # Returns
    ///
    /// Returns a response CloudEvent Response for the use case of RPC Response message that failed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use uprotocol_sdk::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    /// use uprotocol_sdk::cloudevent::datamodel::ucloudeventattributes::UCloudEventAttributes;
    /// use prost_types::Any;
    ///
    /// let application_uri = "http://myapplication.com/rpc";
    /// let service_method_uri = ":/body.access/1/rpc.UpdateDoor";
    /// let request_id = "1234567890";
    /// let communication_status = 1u32; // Replace with actual `Code` value
    /// let attributes = UCloudEventAttributes::default(); // Populate with real data
    /// let proto_payload = Any::default(); // Populate with real data
    ///
    /// let failed_response = UCloudEventBuilder::response_failed(
    ///     application_uri,
    ///     service_method_uri,
    ///     request_id,
    ///     communication_status,
    ///     &attributes,
    /// );
    /// ```
    pub fn response_failed(
        rpc_uri: &str,
        service_method_uri: &str,
        request_id: &str,
        communication_status: u32,
        attributes: &UCloudEventAttributes,
    ) -> Event {
        // NOTE the Java SDK packs the default Empty protobuf thingy into an Any here...
        // main effect is the payload_schema, which is empty in our case but
        // "type.googleapis.com/google.protobuf.Empty" in Java.
        let bce = UCloudEventBuilder::build_base_cloud_event(
            &UCloudEventBuilder::create_cloudevent_id(),
            service_method_uri,
            &Any::default().value,
            &Any::default().type_url,
            attributes,
        )
        .extension("sink", rpc_uri)
        .extension("reqid", request_id)
        .extension("commstatus", communication_status as i64)
        .ty(UCloudEventType::RESPONSE.to_string())
        .build();

        bce.unwrap()
    }

    /// Create a CloudEvent for an event for the use case of: Publish generic message.
    ///
    /// # Arguments
    ///
    /// * `source` - The uri of the topic being published.
    /// * `protoPayload` - Protobuf Any object with the Message to be published.
    /// * `attributes` - Additional attributes such as ttl, hash and priority.
    ///
    /// # Returns
    ///
    /// Returns a publish CloudEvent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use uprotocol_sdk::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    /// use uprotocol_sdk::cloudevent::datamodel::ucloudeventattributes::UCloudEventAttributes;
    /// use prost_types::Any;
    ///
    /// let source = "http://myapplication.com/topic";
    /// let attributes = UCloudEventAttributes::default(); // Populate with real data
    /// let proto_payload = Any::default(); // Populate with real data
    ///
    /// let publish_event = UCloudEventBuilder::publish(
    ///     source,
    ///     &proto_payload,
    ///     &attributes,
    /// );
    /// ```
    pub fn publish(source: &str, payload: &Any, attributes: &UCloudEventAttributes) -> Event {
        let bce = UCloudEventBuilder::build_base_cloud_event(
            &UCloudEventBuilder::create_cloudevent_id(),
            source,
            &payload.value,
            &payload.type_url,
            attributes,
        )
        .ty(UCloudEventType::PUBLISH.to_string())
        .build();

        bce.unwrap()
    }

    /// Create a CloudEvent for an event for the use case of: Publish a notification message.
    /// A published event containing the sink (destination) is often referred to as a notification,
    /// it is an event sent to a specific consumer.
    ///
    /// # Arguments
    ///
    /// * `source` - The uri of the topic being published.
    /// * `sink` - The uri of the destination of this notification.
    /// * `protoPayload` - Protobuf Any object with the Message to be published.
    /// * `attributes` - Additional attributes such as ttl, hash and priority.
    ///
    /// # Returns
    ///
    /// Returns a publish CloudEvent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use uprotocol_sdk::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    /// use uprotocol_sdk::cloudevent::datamodel::ucloudeventattributes::UCloudEventAttributes;
    /// use prost_types::Any;
    ///
    /// let source = "http://myapplication.com/topic";
    /// let sink = "http://myapplication.com/destination";
    /// let attributes = UCloudEventAttributes::default(); // Populate with real data
    /// let proto_payload = Any::default(); // Populate with real data
    ///
    /// let notify_event = UCloudEventBuilder::notify(
    ///     source,
    ///     sink,
    ///     &proto_payload,
    ///     &attributes,
    /// );
    /// ```
    pub fn notify(
        source: &str,
        sink: &str,
        payload: &Any,
        attributes: &UCloudEventAttributes,
    ) -> Event {
        let bce = UCloudEventBuilder::build_base_cloud_event(
            &UCloudEventBuilder::create_cloudevent_id(),
            source,
            &payload.value,
            &payload.type_url,
            attributes,
        )
        .extension("sink", sink)
        .ty(UCloudEventType::PUBLISH.to_string())
        .build();

        bce.unwrap()
    }

    /// Base CloudEvent builder that is the same for all CloudEvent types.
    ///
    /// # Arguments
    ///
    /// * `id` - Event unique identifier.
    /// * `source` - Identifies who is sending this event in the format of a uProtocol URI.
    /// * `proto_payload` - The serialized Event data with the content type of "application/x-protobuf".
    /// * `payload_schema` - The schema of the proto payload bytes, for example you can use `proto://proto_payload.type_url` on your service/app object.
    /// * `attributes` - Additional cloud event attributes that can be passed in. All attributes are optional and will be added only if they were configured.
    ///
    /// ATTENTION: This will prefix the payload_schema url with "proto://" if no schema is provided, because the cloudevent builder uses the url crate for this, which will balk if there's no schema provided.
    ///
    /// # Returns
    ///
    /// Returns an `EventBuilderV10` that can be additionally configured and then by calling `.build()` construct an `Event` ready to be serialized and sent to the transport layer.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cloudevents::{Event, EventBuilder, EventBuilderV10};
    /// # use uprotocol_sdk::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    /// # use uprotocol_sdk::cloudevent::datamodel::ucloudeventattributes::UCloudEventAttributes;
    /// # use prost_types::Any;
    ///
    /// let id = "unique_id";
    /// let source = "source_url";
    /// let proto_payload = Any::default();
    /// let attributes = UCloudEventAttributes::default();
    ///
    /// let event_builder = UCloudEventBuilder::build_base_cloud_event(
    ///     id,
    ///     source,
    ///     &proto_payload.value,
    ///     &proto_payload.type_url,
    ///     &attributes,
    /// );
    /// ```
    pub fn build_base_cloud_event(
        id: &str,
        source: &str,
        payload: &[u8],
        payload_schema: &str,
        attributes: &UCloudEventAttributes,
    ) -> EventBuilderV10 {
        let payload_schema = UCloudEventBuilder::payload_schema_prefixed(payload_schema);

        let mut eb = EventBuilderV10::new()
            .id(id)
            .source(source)
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                payload_schema,
                Vec::from(payload),
            )
            .time(Utc::now());

        if let Some(ttl_value) = attributes.ttl {
            eb = eb.extension("ttl", ttl_value as i64);
        }
        if let Some(priority_value) = &attributes.priority {
            eb = eb.extension("priority", priority_value.to_string());
        }
        if let Some(hash_value) = &attributes.hash {
            eb = eb.extension("hash", hash_value.clone());
        }
        if let Some(token_value) = &attributes.token {
            eb = eb.extension("token", token_value.clone());
        }

        eb
    }

    // cloudevent data_with_schema() uses url crate parser, and this requires a schema to accept an url as valid
    pub fn payload_schema_prefixed(url: &str) -> String {
        match Url::parse(url) {
            Ok(_) => url.to_string(),
            Err(ParseError::RelativeUrlWithoutBase) => format!("proto://{}", url),
            Err(_) => url.to_string(), // Handle other parse errors without prefixing.
        }
    }
}

#[cfg(test)]
mod tests {
    use cloudevents::AttributesReader;

    use super::*;
    use crate::cloudevent::datamodel::ucloudeventattributes::Priority;
    use crate::cloudevent::ucloudevent::UCloudEvent;
    use crate::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    use crate::transport::datamodel::ustatus::UCode;
    use crate::uri::datamodel::uauthority::UAuthority;
    use crate::uri::datamodel::uentity::UEntity;
    use crate::uri::datamodel::uresource::UResource;
    use crate::uri::datamodel::uuri::UUri;
    use crate::uri::serializer::longuriserializer::LongUriSerializer;
    use crate::uri::serializer::uriserializer::UriSerializer;

    use cloudevents::{Data, Event, EventBuilder, EventBuilderV10};

    #[test]
    fn test_create_base_cloud_event() {
        let entity = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(entity),
            Some(UResource::new(
                "door".to_string(),
                Some(String::from("front_left")),
                Some(String::from("Door")),
                None,
                false,
            )),
        );
        let source = uri.to_string();
        let proto_payload = pack_event_into_any(&build_proto_payload_for_test());

        let ucloud_event_attributes = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(Priority::Standard)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        let cloud_event = UCloudEventBuilder::build_base_cloud_event(
            "testme",
            &source,
            &proto_payload.value,
            &proto_payload.type_url,
            &ucloud_event_attributes,
        )
        .ty(UCloudEventType::PUBLISH.to_string())
        .build()
        .unwrap();

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert_eq!("testme", cloud_event.id());
        assert_eq!(source, cloud_event.source().to_string());
        assert_eq!(UCloudEventType::PUBLISH.to_string(), cloud_event.ty());
        assert!(!cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Standard.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!("3", cloud_event.extension("ttl").unwrap().to_string());
        assert_eq!(
            "someOAuthToken",
            cloud_event.extension("token").unwrap().to_string()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_base_cloud_event_without_attributes() {
        let entity = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(entity),
            Some(UResource::new(
                "door".to_string(),
                Some(String::from("front_left")),
                Some(String::from("Door")),
                None,
                false,
            )),
        );
        let source = uri.to_string();
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());
        let ucloud_event_attributes = UCloudEventAttributes::default();
        let cloud_event = UCloudEventBuilder::build_base_cloud_event(
            "testme",
            &source,
            &proto_payload.value,
            &proto_payload.type_url,
            &ucloud_event_attributes,
        )
        .ty(UCloudEventType::PUBLISH.to_string())
        .build()
        .unwrap();

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert_eq!("testme", cloud_event.id());
        assert_eq!(source, cloud_event.source().to_string());
        assert_eq!(UCloudEventType::PUBLISH.to_string(), cloud_event.ty());
        assert!(!cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert!(!cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("hash")));
        assert!(!cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("priority")));
        assert!(!cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("ttl")));
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_publish_cloud_event() {
        let entity = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(entity),
            Some(UResource::new(
                "door".to_string(),
                Some(String::from("front_left")),
                Some(String::from("Door")),
                None,
                false,
            )),
        );
        let source = uri.to_string();

        // fake payload
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Standard),
            ttl: Some(3),
            token: None,
        };

        let cloud_event =
            UCloudEventBuilder::publish(&source, &proto_payload, &ucloud_event_attributes);

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(source, cloud_event.source().to_string());
        assert_eq!(UCloudEventType::PUBLISH.to_string(), cloud_event.ty());
        assert!(!cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Standard.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_notification_cloud_event() {
        // source
        let entity = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(entity),
            Some(UResource::new(
                "door".to_string(),
                Some(String::from("front_left")),
                Some(String::from("Door")),
                None,
                false,
            )),
        );
        let source = uri.to_string();

        // sink
        let sink_entity = UEntity::long_format("petapp".to_string(), None);
        let sink_uri = UUri::new(
            Some(UAuthority::long_remote(
                "com.gm.bo".to_string(),
                "bo".to_string(),
            )),
            Some(sink_entity),
            Some(UResource::long_format("OK".to_string())),
        );
        let sink = sink_uri.to_string();

        // fake payload
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: None,
        };

        let cloud_event =
            UCloudEventBuilder::notify(&source, &sink, &proto_payload, &ucloud_event_attributes);

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(source, cloud_event.source().to_string());
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(sink, cloud_event.extension("sink").unwrap().to_string());
        assert_eq!(UCloudEventType::PUBLISH.to_string(), cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_request_cloud_event_from_local_use() {
        // Uri for the application requesting the RPC
        let source_use = UEntity::long_format("petapp".to_string(), None);
        let application_uri_for_rpc =
            LongUriSerializer::serialize(&UUri::rpc_response(UAuthority::LOCAL, source_use));

        // service Method Uri
        let method_software_entity_service = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let method_uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(method_software_entity_service),
            Some(UResource::for_rpc_request(
                Some("UpdateDoor".to_string()),
                None,
            )),
        );
        let service_method_uri = method_uri.to_string();

        // fake payload
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: Some("someOAuthToken".to_string()),
        };

        let cloud_event = UCloudEventBuilder::request(
            &application_uri_for_rpc,
            &service_method_uri,
            &proto_payload,
            &ucloud_event_attributes,
        );

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!("/petapp//rpc.response", cloud_event.source().to_string());
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            "/body.access/1/rpc.UpdateDoor",
            cloud_event.extension("sink").unwrap().to_string()
        );
        assert_eq!("req.v1", cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            "someOAuthToken",
            cloud_event.extension("token").unwrap().to_string()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_request_cloud_event_from_remote_use() {
        // Uri for the application requesting the RPC
        let source_use_authority = UAuthority::long_remote("bo".to_string(), "cloud".to_string());
        let source_use = UEntity::new("petapp".to_string(), Some("1".to_string()), None, false);
        let application_uri_for_rpc =
            LongUriSerializer::serialize(&UUri::rpc_response(source_use_authority, source_use));

        // service Method Uri
        let method_software_entity_service = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let method_uri = UUri::new(
            Some(UAuthority::long_remote(
                "VCU".to_string(),
                "MY_CAR_VIN".to_string(),
            )),
            Some(method_software_entity_service),
            Some(UResource::for_rpc_request(
                Some("UpdateDoor".to_string()),
                None,
            )),
        );
        let service_method_uri = method_uri.to_string();

        // fake payload
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: Some("someOAuthToken".to_string()),
        };

        let cloud_event = UCloudEventBuilder::request(
            &application_uri_for_rpc,
            &service_method_uri,
            &proto_payload,
            &ucloud_event_attributes,
        );

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(
            "//bo.cloud/petapp/1/rpc.response",
            cloud_event.source().to_string()
        );
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/rpc.UpdateDoor",
            cloud_event.extension("sink").unwrap().to_string()
        );
        assert_eq!("req.v1", cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            "someOAuthToken",
            cloud_event.extension("token").unwrap().to_string()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_response_cloud_event_originating_from_local_use() {
        // Uri for the application requesting the RPC
        let source_use = UEntity::new("petapp".to_string(), Some("1".to_string()), None, false);
        let application_uri_for_rpc =
            LongUriSerializer::serialize(&UUri::rpc_response(UAuthority::LOCAL, source_use));

        // service Method Uri
        let method_software_entity_service = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let method_uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(method_software_entity_service),
            Some(UResource::for_rpc_request(
                Some("UpdateDoor".to_string()),
                None,
            )),
        );
        let service_method_uri = method_uri.to_string();

        // fake payload
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: None, // There's no token in this case
        };

        let cloud_event = UCloudEventBuilder::response(
            &application_uri_for_rpc,
            &service_method_uri,
            "requestIdFromRequestCloudEvent",
            &proto_payload,
            &ucloud_event_attributes,
        );

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(
            "/body.access/1/rpc.UpdateDoor",
            cloud_event.source().to_string()
        );
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            "/petapp/1/rpc.response",
            cloud_event.extension("sink").unwrap().to_string()
        );
        assert_eq!("res.v1", cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            "requestIdFromRequestCloudEvent",
            cloud_event.extension("reqid").unwrap().to_string()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_response_cloud_event_originating_from_remote_use() {
        // Uri for the application requesting the RPC
        let source_use_authority = UAuthority::long_remote("bo".to_string(), "cloud".to_string());
        let source_use = UEntity::long_format("petapp".to_string(), None);
        let application_uri_for_rpc =
            LongUriSerializer::serialize(&UUri::rpc_response(source_use_authority, source_use));

        // service Method Uri
        let method_software_entity_service = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let method_uri = UUri::new(
            Some(UAuthority::long_remote(
                "VCU".to_string(),
                "MY_CAR_VIN".to_string(),
            )),
            Some(method_software_entity_service),
            Some(UResource::for_rpc_request(
                Some("UpdateDoor".to_string()),
                None,
            )),
        );
        let service_method_uri = method_uri.to_string();

        // fake payload
        let proto_payload: Any = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: None, // There's no token in this case
        };

        let cloud_event = UCloudEventBuilder::response(
            &application_uri_for_rpc,
            &service_method_uri,
            "requestIdFromRequestCloudEvent",
            &proto_payload,
            &ucloud_event_attributes,
        );

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/rpc.UpdateDoor",
            cloud_event.source().to_string()
        );
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            "//bo.cloud/petapp//rpc.response",
            cloud_event.extension("sink").unwrap().to_string()
        );
        assert_eq!("res.v1", cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - not entirely sure why the Any.pack() method choses this instead of the dataschema
            // that is available in the event object, but in any case this specific string doesn't make sense in Rust context.
            // "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
            "proto://type.googleapis.com/example.demo",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            "requestIdFromRequestCloudEvent",
            cloud_event.extension("reqid").unwrap().to_string()
        );
        assert_eq!(
            proto_payload.value,
            UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap()
        );
    }

    #[test]
    fn test_create_a_failed_response_cloud_event_originating_from_local_use() {
        // Uri for the application requesting the RPC
        let source_use = UEntity::new("petapp".to_string(), Some("1".to_string()), None, false);
        let application_uri_for_rpc =
            LongUriSerializer::serialize(&UUri::rpc_response(UAuthority::LOCAL, source_use));

        // Service Method Uri
        let method_software_entity_service = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let method_uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(method_software_entity_service),
            Some(UResource::for_rpc_request(
                Some("UpdateDoor".to_string()),
                None,
            )),
        );
        let service_method_uri = method_uri.to_string();

        // Additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: None,
        };

        let cloud_event = UCloudEventBuilder::response_failed(
            &application_uri_for_rpc,
            &service_method_uri,
            "requestIdFromRequestCloudEvent",
            UCode::InvalidArgument as u32,
            &ucloud_event_attributes,
        );

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(
            "/body.access/1/rpc.UpdateDoor",
            cloud_event.source().to_string()
        );
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            "/petapp/1/rpc.response",
            cloud_event.extension("sink").unwrap().to_string()
        );
        assert_eq!("res.v1", cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        // The Java SDK tests for this string - no sure yet what to do here
        // "type.googleapis.com/google.protobuf.Empty",
        assert_eq!("proto://", cloud_event.dataschema().unwrap().to_string());
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            UCode::InvalidArgument as i32,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "commstatus").unwrap()
        );
        assert_eq!(
            "requestIdFromRequestCloudEvent",
            cloud_event.extension("reqid").unwrap().to_string()
        );
    }

    #[test]
    fn test_create_a_failed_response_cloud_event_originating_from_remote_use() {
        // Uri for the application requesting the RPC
        let source_use_authority = UAuthority::long_remote("bo".to_string(), "cloud".to_string());
        let source_use = UEntity::long_format("petapp".to_string(), None);
        let application_uri_for_rpc =
            LongUriSerializer::serialize(&UUri::rpc_response(source_use_authority, source_use));

        // Service Method Uri
        let method_software_entity_service = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        let method_uri = UUri::new(
            Some(UAuthority::long_remote(
                "VCU".to_string(),
                "MY_CAR_VIN".to_string(),
            )),
            Some(method_software_entity_service),
            Some(UResource::for_rpc_request(
                Some("UpdateDoor".to_string()),
                None,
            )),
        );
        let service_method_uri = method_uri.to_string();

        // Additional attributes
        let ucloud_event_attributes = UCloudEventAttributes {
            hash: Some("somehash".to_string()),
            priority: Some(Priority::Operations),
            ttl: Some(3),
            token: None,
        };

        let cloud_event = UCloudEventBuilder::response_failed(
            &application_uri_for_rpc,
            &service_method_uri,
            "requestIdFromRequestCloudEvent",
            UCode::InvalidArgument as u32,
            &ucloud_event_attributes,
        );

        assert_eq!("1.0", cloud_event.specversion().to_string());
        assert!(!cloud_event.id().is_empty());
        assert_eq!(
            "//vcu.my_car_vin/body.access/1/rpc.UpdateDoor",
            cloud_event.source().to_string()
        );
        assert!(cloud_event
            .iter_extensions()
            .any(|(name, _value)| name.contains("sink")));
        assert_eq!(
            "//bo.cloud/petapp//rpc.response",
            cloud_event.extension("sink").unwrap().to_string()
        );
        assert_eq!("res.v1", cloud_event.ty());
        assert_eq!(
            UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
            cloud_event.datacontenttype().unwrap()
        );
        assert_eq!(
            // The Java SDK tests for this string - no sure yet what to do here
            // "type.googleapis.com/google.protobuf.Empty",
            "proto://",
            cloud_event.dataschema().unwrap().to_string()
        );
        assert_eq!(
            "somehash",
            cloud_event.extension("hash").unwrap().to_string()
        );
        assert_eq!(
            Priority::Operations.to_string(),
            cloud_event.extension("priority").unwrap().to_string()
        );
        assert_eq!(
            3,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "ttl").unwrap()
        );
        assert_eq!(
            UCode::InvalidArgument as i32,
            UCloudEvent::extract_integer_value_from_extension(&cloud_event, "commstatus").unwrap()
        );
        assert_eq!(
            "requestIdFromRequestCloudEvent",
            cloud_event.extension("reqid").unwrap().to_string()
        );
    }

    fn build_proto_payload_for_test() -> Event {
        EventBuilderV10::new()
            .id("hello")
            .source("https://example.com")
            .ty(UCloudEventType::PUBLISH.to_string())
            .data_with_schema(
                "application/octet-stream",
                "proto://type.googleapis.com/example.demo",
                Any::default().value,
            )
            .build()
            .unwrap()
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

        prost_types::Any {
            type_url: schema,
            value: data_bytes,
        }
    }
}
