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

use bytes::Bytes;
use chrono::{Duration, Utc};
use cloudevents::event::ExtensionValue;
use cloudevents::{AttributesReader, Data, Event, EventBuilder, EventBuilderV10};
use prost::Message;
use prost_types::Any;
use std::time::SystemTime;
use uuid::Uuid;

use crate::transport::datamodel::UCode;
use crate::uuid::builder::UuidUtils;

/// Code to extract information from a CloudEvent
#[derive(Debug)]
pub struct UCloudEvent;

pub trait EventConversionExt {
    // Convert cloudevents::Event to prost_types::Any
    fn to_any(&self) -> prost_types::Any;
    // Convert prost_types::Any to cloudevents::Event
    fn from_any(any: prost_types::Any) -> Result<cloudevents::Event, ConversionError>;
}

// Define a custom error for the conversion
#[derive(Debug)]
pub struct ConversionError(String);

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ConversionError: {}", self.0)
    }
}

impl std::error::Error for ConversionError {}

impl UCloudEvent {
    /// Pretty prints a `CloudEvent` showing only its id, source, type, and possibly a sink.
    ///
    /// This function is primarily intended for logging purposes.
    ///
    /// # Arguments
    ///
    /// * `cloudEvent` - The `CloudEvent` instance that we wish to format as a pretty string.
    ///
    /// # Returns
    ///
    /// Returns a `String` representation of the `CloudEvent` highlighting only its id, source, type, and potentially a sink.
    pub fn to_string(event: &Event) -> String {
        if let Some(sink) = UCloudEvent::get_sink(event) {
            format!(
                "CloudEvent{{id='{}', source='{}', sink='{}', type='{}'}}",
                event.id(),
                event.source(),
                sink,
                event.ty()
            )
        } else {
            format!(
                "CloudEvent{{id='{}', source='{}', type='{}'}}",
                event.id(),
                event.source(),
                event.ty()
            )
        }
    }

    /// Extracts the source from a cloud event.
    ///
    /// The source is a mandatory attribute. The CloudEvent constructor does not
    /// allow creating a cloud event without a source.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the source is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns the `String` value of the `CloudEvent` source attribute.
    pub fn get_source(event: &Event) -> Option<String> {
        Some(event.source().to_string())
    }

    /// Extracts the sink from a cloud event.
    ///
    /// The sink attribute is optional.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the sink is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` value of the `CloudEvent` sink attribute if it exists,
    /// otherwise a `None` is returned.
    pub fn get_sink(event: &Event) -> Option<String> {
        if let Some(sink) = event.extension("sink") {
            return Some(sink.to_string());
        }
        None
    }

    /// Extracts the request id from a cloud event that is a response RPC `CloudEvent`.
    ///
    /// The request id attribute is optional.
    ///
    /// # Arguments
    ///
    /// * `event` - The response RPC `CloudEvent` from which the request id is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` value of the response RPC `CloudEvent` request id attribute if it exists,
    /// otherwise a `None` is returned.
    pub fn get_request_id(event: &Event) -> Option<String> {
        if let Some(reqid) = event.extension("reqid") {
            return Some(reqid.to_string());
        }
        None
    }

    /// Extracts the hash attribute from a cloud event.
    ///
    /// The hash attribute is optional.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the hash is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` value of the `CloudEvent` hash attribute if it exists,
    /// otherwise a `None` is returned.
    pub fn get_hash(event: &Event) -> Option<String> {
        if let Some(hash) = event.extension("hash") {
            return Some(hash.to_string());
        }
        None
    }

    /// Extracts the string value of the priority attribute from a cloud event.
    ///
    /// The priority attribute is optional.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the priority is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` value of the `CloudEvent` priority attribute if it exists,
    /// otherwise a `None` is returned.
    pub fn get_priority(event: &Event) -> Option<String> {
        if let Some(priority) = event.extension("priority") {
            return Some(priority.to_string());
        }
        None
    }

    /// Extracts the integer value of the ttl (time-to-live) attribute from a cloud event.
    ///
    /// The ttl attribute is optional.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the ttl is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns an `Option<u32>` value of the `CloudEvent` ttl attribute if it exists,
    /// otherwise a `None` is returned.
    pub fn get_ttl(event: &Event) -> Option<u32> {
        if let Some(ExtensionValue::Integer(ttl)) = event.extension("ttl") {
            return Some(*ttl as u32);
        }
        None
    }

    /// Extracts the string value of the token attribute from a cloud event.
    ///
    /// The token attribute is optional.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the token is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` value of the `CloudEvent` token attribute if it exists,
    /// otherwise a `None` is returned.
    pub fn get_token(event: &Event) -> Option<String> {
        if let Some(token) = event.extension("token") {
            return Some(token.to_string());
        }
        None
    }

    /// Extracts the integer value of the communication status attribute from a cloud event.
    ///
    /// The communication status attribute is optional. If there was a platform communication error
    /// that occurred while delivering this cloud event, it will be indicated in this attribute.
    /// If the attribute does not exist, it is assumed that everything was `UCode::OK_VALUE`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` from which the communication status is to be extracted.
    ///
    /// # Returns
    ///
    /// Returns a `Code` value that indicates a platform communication error while delivering this
    /// `CloudEvent` or `UCode::OK_VALUE`.
    pub fn get_timestamp(event: &Event) -> Option<u64> {
        let id = event.id();
        match Uuid::parse_str(id) {
            Ok(uuid) => UuidUtils::get_time(&uuid),
            Err(_) => None,
        }
    }

    /// Extracts the communication status attribute from the provided `Event`.
    ///
    /// If there was a platform communication error that occurred while delivering this `Event`,
    /// it will be indicated in this attribute. If the attribute does not exist, it is assumed
    /// that everything was `Ok`.
    ///
    /// # Arguments
    ///
    /// * `event` - `Event` to extract the communication status from.
    ///
    /// # Returns
    ///
    /// An `Option<i64>` value that indicates a platform communication error while delivering this `Event`,
    /// or `None` if everything was `Ok`.
    pub fn get_communication_status(event: &Event) -> Option<i64> {
        if let Some(ExtensionValue::Integer(commstatus)) = event.extension("commstatus") {
            return Some(*commstatus);
        }
        Some(UCode::Ok as i64)
    }

    /// Indicates if a platform communication error occurred while trying to deliver the `CloudEvent`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` to be queried for a platform delivery error.
    ///
    /// # Returns
    ///
    /// Returns `true` if the provided `CloudEvent` is marked with having a platform delivery problem.
    pub fn has_communication_problem(event: &Event) -> bool {
        UCloudEvent::get_communication_status(event).is_some_and(|c: i64| c != UCode::Ok as i64)
    }

    /// Returns a new `Event` from the supplied `Event`, with the platform communication added.
    ///
    /// # Arguments
    ///
    /// * `event` - `Event` that the platform delivery error will be added to.
    /// * `communication_status` - the platform delivery error Code to add to the `Event`.
    ///
    /// # Returns
    ///
    /// A new `Event` from the supplied `Event`, with the platform communication added.
    pub fn add_communication_status(event: Event, communication_status: i64) -> Event {
        let ce = EventBuilderV10::from(event);

        ce.extension("commstatus", communication_status)
            .build()
            .unwrap()
    }

    /// Extracts the timestamp from the UUIDV8 `Event` Id, using Unix epoch as the reference.
    ///
    /// # Arguments
    ///
    /// * `event` - `Event` from which the timestamp is to be extracted.
    ///
    /// # Returns
    ///
    /// An `Option<u64>` containing the timestamp from the UUIDV8 `Event` Id or `None` if the timestamp can't be extracted.
    pub fn get_creation_timestamp(event: &Event) -> Option<u64> {
        let cloud_event_id = event.id();
        match Uuid::parse_str(cloud_event_id) {
            Ok(uuid) => Some(UuidUtils::get_time(&uuid).expect("Failed to get time from UUID")),
            Err(_) => None,
        }
    }

    /// Calculates if an `Event` configured with a creation time and a `ttl` attribute is expired.
    ///
    /// The `ttl` attribute configures how long this event should live for after it was generated (in milliseconds).
    ///
    /// # Arguments
    ///
    /// * `event` - The `Event` to inspect for expiration.
    ///
    /// # Returns
    ///
    /// Returns `true` if the `Event` was configured with a `ttl` greater than 0 and a creation time to compare for expiration.
    pub fn is_expired_by_cloud_event_creation_date(event: &Event) -> bool {
        match UCloudEvent::get_ttl(event) {
            Some(ttl) if ttl > 0 => {
                if let Some(cloud_event_creation_time) = event.time() {
                    let now = Utc::now();
                    let creation_time_plus_ttl =
                        *cloud_event_creation_time + Duration::milliseconds(ttl as i64);
                    return now > creation_time_plus_ttl;
                }
            }
            _ => {}
        }
        false
    }

    /// Calculates if an `Event` configured with a UUIDv8 id and a `ttl` attribute is expired.
    ///
    /// The `ttl` attribute configures how long this event should live for after it was generated (in milliseconds).
    ///
    /// # Arguments
    ///
    /// * `event` - The `Event` to inspect for expiration.
    ///
    /// # Returns
    ///
    /// Returns `true` if the `Event` was configured with a `ttl` greater than 0 and a UUIDv8 id to compare for expiration.
    pub fn is_expired(event: &Event) -> bool {
        let maybe_ttl = UCloudEvent::get_ttl(event);
        match maybe_ttl {
            Some(ttl) if ttl > 0 => {
                let cloud_event_id = event.id();
                match Uuid::parse_str(cloud_event_id) {
                    Ok(uuid) => {
                        let event_time =
                            UuidUtils::get_time(&uuid).expect("Failed to get time from UUID");
                        let now = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .expect("Time went backwards")
                            .as_millis();
                        let delta = now as u64 - event_time;
                        delta >= ttl as u64
                    }
                    Err(_) => false,
                }
            }
            _ => false,
        }
    }

    /// Checks if an `Event` has a UUIDv8 id.
    ///
    /// # Arguments
    ///
    /// * `event` - The `Event` whose id is to be inspected.
    ///
    /// # Returns
    ///
    /// Returns `true` if the `Event` has a valid UUIDv8 id.
    pub fn is_cloud_event_id(event: &Event) -> bool {
        let cloud_event_id = event.id();
        match Uuid::parse_str(cloud_event_id) {
            Ok(uuid) => UuidUtils::is_valid_uuid(&uuid),
            Err(_) => false,
        }
    }

    /// Extracts the payload from the `Event` as a protobuf `Any` object.
    ///
    /// An "all or nothing" error handling strategy is applied. If any issue arises,
    /// the default instance of the `Any` object will be returned.
    ///
    /// # Arguments
    ///
    /// * `event` - The `Event` containing the payload to be extracted.
    ///
    /// # Returns
    ///
    /// Returns the payload from the `Event` as a Protobuf `Any` object.
    pub fn get_payload(event: &Event) -> Any {
        if let Some(buffer) = UCloudEvent::serialize_event_data_into_bytes(event) {
            if let Ok(any) = prost_types::Any::decode(buffer.as_slice()) {
                return any;
            }
        }
        prost_types::Any::default()
    }

    /// Unpacks the payload from the `Event` as a protobuf `Message` of the provided type `T`.
    ///
    /// The protobuf message of type `T` must be available in the context for this function to work.
    /// An "all or nothing" error handling strategy is applied, and if any issues arise during decoding,
    /// a `DecodeError` will be returned.
    ///
    /// # Arguments
    ///
    /// * `event` - The `Event` containing the payload to be extracted.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the protobuf `Message` to be unpacked. It must implement both `Message` and `Default` traits.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the unpacked message of type `T` or a decoding error.
    pub fn unpack<T: Message + Default>(event: &Event) -> Result<T, prost::DecodeError> {
        let any_payload = UCloudEvent::get_payload(event);
        let buffer = Bytes::from(any_payload.value);

        T::decode(buffer)
    }

    /// Serializes the data of a given `CloudEvent` into a byte vector.
    ///
    /// This function attempts to serialize the data payload of the provided `CloudEvent` into a byte vector (`Vec<u8>`).
    /// If serialization is successful, it returns the byte vector wrapped in `Some`. If the event doesn't contain data
    /// or there's a serialization issue, it returns `None`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `CloudEvent` whose data is to be serialized.
    ///
    /// # Returns
    ///
    /// An `Option<Vec<u8>>` containing the serialized data if successful, or `None` if the serialization fails or data is not present.
    pub fn serialize_event_data_into_bytes(event: &Event) -> Option<Vec<u8>> {
        match event.data() {
            Some(Data::Binary(bytes)) => Some(bytes.clone()),
            Some(Data::String(s)) => Some(s.as_bytes().to_vec()),
            Some(Data::Json(j)) => Some(j.to_string().into_bytes()),
            None => None,
        }
    }

    /// Extracts the `String` value of a specified extension from a `CloudEvent`.
    ///
    /// # Arguments
    ///
    /// * `extensionName` - The name of the desired extension within the `CloudEvent`.
    /// * `cloudEvent` - The `CloudEvent` from which we aim to extract the extension value.
    ///
    /// # Returns
    ///
    /// Returns an `Option<String>` containing the value of the extension if it exists, or `None` if the extension is not present.
    pub fn extract_string_value_from_extension(
        event: &Event,
        extension_name: &str,
    ) -> Option<String> {
        if event.extension(extension_name).is_some() {
            event.extension(extension_name).map(|ext| ext.to_string())
        } else {
            None
        }
    }

    /// Extracts the `Integer` value of a specified extension from a `CloudEvent`.
    ///
    /// # Arguments
    ///
    /// * `extensionName` - The name of the desired extension within the `CloudEvent`.
    /// * `cloudEvent` - The `CloudEvent` from which we aim to extract the extension value.
    ///
    /// # Returns
    ///
    /// Returns an `Option<i32>` (or the appropriate integer type) containing the value of the extension if it exists,
    /// or `None` if the extension is not present.
    pub fn extract_integer_value_from_extension(
        event: &Event,
        extension_name: &str,
    ) -> Option<i32> {
        UCloudEvent::extract_string_value_from_extension(event, extension_name)
            .and_then(|s| s.parse::<i32>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloudevent::builder::UCloudEventBuilder;
    use crate::cloudevent::datamodel::ucloudeventattributes::{Priority, UCloudEventAttributes};
    use crate::cloudevent::datamodel::ucloudeventtype::UCloudEventType;
    use crate::proto::CloudEvent;
    use crate::uri::datamodel::{UAuthority, UEntity, UResource, UUri};
    use crate::uri::serializer::{LongUriSerializer, UriSerializer};
    use crate::uuid::builder::UUIDv8Factory;

    use chrono::{offset, TimeZone, Utc};
    use cloudevents::{Data, Event, EventBuilder, EventBuilderV10};
    use prost_types::Any;
    use url::Url;

    #[test]
    fn test_extract_source_from_cloud_event() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let source = UCloudEvent::get_source(&cloud_event);

        assert_eq!(
            Some("/body.access//door.front_left#Door".to_string()),
            source
        );
    }

    #[test]
    fn test_extract_sink_from_cloud_event_when_sink_exists() {
        let sink_for_test = "//bo.cloud/petapp/1/rpc.response";

        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder
            .extension("sink", sink_for_test.to_string())
            .build()
            .expect("Failed to build the cloud event");

        let sink = UCloudEvent::get_sink(&cloud_event);
        assert_eq!(Some(sink_for_test.to_string()), sink);
    }

    #[test]
    fn test_extract_sink_from_cloud_event_when_sink_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let sink = UCloudEvent::get_sink(&cloud_event);

        assert!(sink.is_none());
    }

    #[test]
    fn test_extract_request_id_from_cloud_event_when_request_id_exists() {
        let mut builder = build_base_cloud_event_for_test();
        builder = builder.extension("reqid", "someRequestId".to_string());

        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let request_id = UCloudEvent::get_request_id(&cloud_event);

        assert_eq!(Some("someRequestId".to_string()), request_id);
    }

    #[test]
    fn test_extract_request_id_from_cloud_event_when_request_id_does_not_exist() {
        let builder = build_base_cloud_event_for_test();

        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let request_id = UCloudEvent::get_request_id(&cloud_event);

        assert_eq!(None, request_id);
    }

    #[test]
    fn test_extract_request_id_from_cloud_event_when_request_id_value_is_none() {
        let mut builder = build_base_cloud_event_for_test();

        // Representing "null" in Rust with Option::None
        let reqid: Option<&str> = None;

        if let Some(rid) = reqid {
            builder = builder.extension("reqid", rid);
        }

        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let request_id = UCloudEvent::get_request_id(&cloud_event);

        assert_eq!(None, request_id);
    }

    #[test]
    fn test_extract_hash_from_cloud_event_when_hash_exists() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let hash_value = UCloudEvent::get_hash(&cloud_event);

        assert_eq!(Some("somehash".to_string()), hash_value);
    }

    #[test]
    fn test_extract_hash_from_cloud_event_when_hash_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let mut cloud_event: Event = builder.build().expect("Failed to build the cloud event");
        cloud_event.remove_extension("hash");

        let hash_value = UCloudEvent::get_hash(&cloud_event);

        assert!(hash_value.is_none());
    }

    #[test]
    fn test_extract_priority_from_cloud_event_when_priority_exists() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let priority = UCloudEvent::get_priority(&cloud_event);

        assert_eq!(Some(Priority::Standard.to_string()), priority);
    }

    #[test]
    fn test_extract_priority_from_cloud_event_when_priority_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let mut cloud_event: Event = builder.build().expect("Failed to build the cloud event");
        cloud_event.remove_extension("priority");

        let priority = UCloudEvent::get_priority(&cloud_event);

        assert!(priority.is_none());
    }

    #[test]
    fn test_extract_ttl_from_cloud_event_when_ttl_exists() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let ttl = UCloudEvent::get_ttl(&cloud_event);

        assert_eq!(Some(3), ttl);
    }

    #[test]
    fn test_extract_ttl_from_cloud_event_when_ttl_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let mut cloud_event: Event = builder.build().expect("Failed to build the cloud event");
        cloud_event.remove_extension("ttl");

        let ttl = UCloudEvent::get_ttl(&cloud_event);

        assert!(ttl.is_none());
    }

    #[test]
    fn test_extract_token_from_cloud_event_when_token_exists() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let token = UCloudEvent::get_token(&cloud_event);

        assert_eq!(Some("someOAuthToken".to_string()), token);
    }

    #[test]
    fn test_extract_token_from_cloud_event_when_token_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let mut cloud_event: Event = builder.build().expect("Failed to build the cloud event");
        cloud_event.remove_extension("token");

        let token = UCloudEvent::get_token(&cloud_event);

        assert!(token.is_none());
    }

    #[test]
    fn test_cloud_event_has_platform_error_when_platform_error_exists() {
        let builder = build_base_cloud_event_for_test();
        let mut cloud_event: Event = builder.build().expect("Failed to build the cloud event");
        cloud_event.set_extension("commstatus", UCode::Aborted as i64);

        let has_communication_problem = UCloudEvent::has_communication_problem(&cloud_event);
        let communication_status = UCloudEvent::get_communication_status(&cloud_event);

        assert!(has_communication_problem);
        assert_eq!(Some(UCode::Aborted as i64), communication_status);
    }

    #[test]
    fn test_cloud_event_has_no_platform_error_when_platform_error_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let has_communication_problem = UCloudEvent::has_communication_problem(&cloud_event);
        let communication_status = UCloudEvent::get_communication_status(&cloud_event);

        assert!(!has_communication_problem);
        assert_eq!(Some(UCode::Ok as i64), communication_status);
    }

    #[test]
    fn test_extract_platform_error_from_cloud_event_when_error_exists_in_wrong_format() {
        let mut builder = build_base_cloud_event_for_test();
        builder = builder.extension("commstatus", "boom");
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let has_communication_problem = UCloudEvent::has_communication_problem(&cloud_event);
        let communication_status = UCloudEvent::get_communication_status(&cloud_event);

        assert!(!has_communication_problem);
        assert_eq!(Some(UCode::Ok as i64), communication_status);
    }

    #[test]
    fn test_extract_platform_error_from_cloud_event_when_error_exists() {
        let mut builder = build_base_cloud_event_for_test();
        builder = builder.extension("commstatus", UCode::InvalidArgument as i64);
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let communication_status = UCloudEvent::get_communication_status(&cloud_event);

        assert_eq!(Some(UCode::InvalidArgument as i64), communication_status);
    }

    #[test]
    fn test_extract_platform_error_from_cloud_event_when_error_does_not_exist() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let communication_status = UCloudEvent::get_communication_status(&cloud_event);

        assert_eq!(Some(UCode::Ok as i64), communication_status);
    }

    #[test]
    fn test_adding_platform_error_to_existing_cloud_event() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        assert_eq!(
            Some(UCode::Ok as i64),
            UCloudEvent::get_communication_status(&cloud_event)
        );

        let updated_cloud_event = UCloudEvent::add_communication_status(
            cloud_event.clone(),
            UCode::DeadlineExceeded as i64,
        );

        assert_eq!(
            Some(UCode::DeadlineExceeded as i64),
            UCloudEvent::get_communication_status(&updated_cloud_event)
        );
        assert_eq!(
            Some(UCode::Ok as i64),
            UCloudEvent::get_communication_status(&cloud_event)
        );
    }

    #[test]
    fn test_extract_creation_timestamp_from_cloud_event_uuid_id_when_not_a_uuidv8_id() {
        let builder = build_base_cloud_event_for_test();
        let cloud_event: Event = builder.build().expect("Failed to build the cloud event");

        let creation_timestamp = UCloudEvent::get_timestamp(&cloud_event);

        assert!(creation_timestamp.is_none());
    }

    #[test]
    fn test_extract_creation_timestamp_from_cloud_event_uuidv8_id_when_uuidv8_id_is_valid() {
        let uuid = UUIDv8Factory::new().build();
        let builder = build_base_cloud_event_for_test();
        let cloud_event = builder.id(uuid.to_string()).build().unwrap();

        let maybe_creation_timestamp = UCloudEvent::get_timestamp(&cloud_event);
        assert!(maybe_creation_timestamp.is_some());

        let creation_timestamp = maybe_creation_timestamp.unwrap();
        let now = offset::Utc::now();

        // Convert the creation_timestamp to a DateTime.
        let creation_timestamp_datetime =
            Utc.timestamp_millis_opt(creation_timestamp as i64).unwrap();

        // Verify the equality of the two timestamps at the seconds precision.
        assert_eq!(creation_timestamp_datetime.timestamp(), now.timestamp());
    }

    #[test]
    fn test_cloudevent_is_not_expired_cd_when_no_ttl_configured() {
        let builder = build_base_cloud_event_for_test();
        let mut cloud_event: Event = builder.build().expect("Failed to build the cloud event");
        cloud_event.remove_extension("ttl");

        assert!(!UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_not_expired_cd_when_ttl_is_zero() {
        let mut builder = build_base_cloud_event_for_test();
        builder = builder.extension("ttl", 0);
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_not_expired_cd_when_ttl_is_minus_one() {
        let mut builder = build_base_cloud_event_for_test();
        builder = builder.extension("ttl", -1);
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_not_expired_cd_when_ttl_3_mili_no_creation_date() {
        let proto_payload = build_proto_payload_for_test();

        let builder = cloudevents::EventBuilderV10::new()
            .id("id")
            .ty(UCloudEventType::PUBLISH)
            .source("/body.accss//door.front_left#Door")
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                proto_payload.dataschema().unwrap().to_string(),
                pack_event_into_any(&proto_payload).value,
            )
            .extension("ttl", 3);
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_not_expired_cd_when_ttl_500_mili_with_creation_date_of_now() {
        let builder = build_base_cloud_event_for_test()
            .time(chrono::Utc::now())
            .extension("ttl", 500);
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_expired_cd_when_ttl_500_mili_with_creation_date_of_yesterday() {
        let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
        let builder = build_base_cloud_event_for_test()
            .time(yesterday)
            .extension("ttl", 500);
        let cloud_event = builder.build().unwrap();

        assert!(UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_not_expired_cd_when_ttl_500_mili_with_creation_date_of_tomorrow() {
        let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
        let builder = build_base_cloud_event_for_test()
            .time(tomorrow)
            .extension("ttl", 500);
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired_by_cloud_event_creation_date(
            &cloud_event
        ));
    }

    #[test]
    fn test_cloudevent_is_not_expired_when_no_ttl_configured() {
        let uuid = uuid::Uuid::new_v4();
        let builder = build_base_cloud_event_for_test().id(uuid.to_string());
        let mut cloud_event = builder.build().unwrap();
        cloud_event.remove_extension("ttl");

        assert!(!UCloudEvent::is_expired(&cloud_event));
    }

    #[test]
    fn test_cloudevent_is_not_expired_when_ttl_is_zero() {
        let uuid = uuid::Uuid::new_v4();
        let builder = build_base_cloud_event_for_test()
            .extension("ttl", 0)
            .id(uuid.to_string());
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired(&cloud_event));
    }

    #[test]
    fn test_cloudevent_is_not_expired_when_ttl_is_minus_one() {
        let uuid = UUIDv8Factory::new().build();
        let builder = build_base_cloud_event_for_test()
            .extension("ttl", -1)
            .id(uuid.to_string());
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired(&cloud_event));
    }

    #[test]
    fn test_cloudevent_is_not_expired_when_ttl_is_large_number_mili() {
        let uuid = UUIDv8Factory::new().build();
        let builder = build_base_cloud_event_for_test()
            .extension("ttl", i64::MAX)
            .id(uuid.to_string());
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_expired(&cloud_event));
    }

    #[test]
    fn test_cloudevent_is_expired_when_ttl_1_mili() {
        use std::thread;
        use std::time::Duration;

        let uuid = UUIDv8Factory::new().build();
        let builder = build_base_cloud_event_for_test()
            .extension("ttl", 1)
            .id(uuid.to_string());
        let cloud_event = builder.build().unwrap();

        thread::sleep(Duration::from_millis(800));

        assert!(UCloudEvent::is_expired(&cloud_event));
    }

    #[test]
    fn test_cloudevent_has_a_v8_uuid() {
        let uuid = UUIDv8Factory::new().build();
        let builder = build_base_cloud_event_for_test().id(uuid.to_string());
        let cloud_event = builder.build().unwrap();

        assert!(UCloudEvent::is_cloud_event_id(&cloud_event));
    }

    #[test]
    fn test_cloudevent_does_not_have_a_v8_uuid() {
        let uuid = Uuid::new_v4();
        let builder = build_base_cloud_event_for_test()
            .extension("ttl", 3)
            .id(uuid.to_string());
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_cloud_event_id(&cloud_event));
    }

    #[test]
    fn test_cloudevent_does_not_have_a_uuid_just_some_string() {
        let builder = build_base_cloud_event_for_test().extension("ttl", 3);
        let cloud_event = builder.build().unwrap();

        assert!(!UCloudEvent::is_cloud_event_id(&cloud_event));
    }

    #[test]
    fn test_extract_payload_from_cloud_event_as_any_proto_object() {
        let proto_payload = build_proto_payload_for_test();
        let any_payload = pack_event_into_any(&proto_payload);
        let any_bytes = any_payload.encode_to_vec();

        let builder = cloudevents::EventBuilderV10::new()
            .id("someid")
            .ty(UCloudEventType::PUBLISH)
            .source("/body.accss//door.front_left#Door")
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                format!("proto://{}", any_payload.type_url.clone()),
                any_bytes,
            )
            .extension("ttl", 3);
        let cloud_event = builder.build().unwrap();

        let extracted = UCloudEvent::get_payload(&cloud_event);

        assert_eq!(any_payload, extracted);
    }

    #[test]
    fn test_extract_payload_from_cloud_event_when_payload_is_not_an_any_proto_object() {
        let proto_payload = build_proto_payload_for_test();
        let any_payload = pack_event_into_any(&proto_payload);
        let any_bytes = any_payload.encode_to_vec();

        let cloud_event = cloudevents::EventBuilderV10::new()
            .id("someId")
            .ty(UCloudEventType::PUBLISH)
            // The url crate does not accept URLs without a base
            .source(Url::parse("up:/body.access/1/door.front_left#Door").unwrap())
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                "proto://type.googleapis.com/io.cloudevents.v1.CloudEvent",
                any_bytes,
            )
            .build()
            .unwrap();

        let buffer = UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap();
        let parsed_any = prost_types::Any::decode(buffer.as_slice()).ok().unwrap();

        let payload_any = UCloudEvent::get_payload(&cloud_event);

        assert_eq!(parsed_any, payload_any);
    }

    #[test]
    fn test_extract_payload_from_cloud_event_when_payload_is_bad_proto_object() {
        let cloud_event = cloudevents::EventBuilderV10::new()
            .id("someId")
            .ty(UCloudEventType::PUBLISH)
            // The url crate does not accept URLs without a base
            .source(Url::parse("up:/body.access/1/door.front_left#Door").unwrap())
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                Url::parse("proto://type.googleapis.com/io.cloudevents.v1.CloudEvent").unwrap(),
                cloudevents::event::Data::Binary(
                    "<html><head></head><body><p>Hello</p></body></html>"
                        .as_bytes()
                        .to_vec(),
                ),
            )
            .build()
            .unwrap();

        let extracted = UCloudEvent::get_payload(&cloud_event);

        assert_eq!(Any::default(), extracted);
    }

    #[test]
    fn test_extract_payload_from_cloud_event_as_any_proto_object_when_no_schema() {
        let payload_for_cloud_event = build_proto_payload_for_test();
        let cloud_event_data =
            UCloudEvent::serialize_event_data_into_bytes(&payload_for_cloud_event).unwrap();

        let cloud_event = cloudevents::EventBuilderV10::new()
            .id("someId")
            .ty(UCloudEventType::PUBLISH)
            .source(Url::parse("up:/body.access/1/door.front_left#Door").unwrap())
            .data(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                cloudevents::event::Data::Binary(cloud_event_data.clone()),
            )
            .build()
            .unwrap();

        let extracted = UCloudEvent::get_payload(&cloud_event);

        assert_eq!(cloud_event_data, extracted.encode_to_vec());
    }

    #[test]
    fn test_extract_payload_from_cloud_event_as_any_proto_object_when_no_data() {
        let payload_for_cloud_event = build_proto_payload_for_test();

        let cloud_event = cloudevents::EventBuilderV10::new()
            .id("someId")
            .ty(UCloudEventType::PUBLISH)
            .source(Url::parse("up:/body.access/1/door.front_left#Door").unwrap())
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                payload_for_cloud_event.dataschema().unwrap().to_string(),
                "",
            )
            .build()
            .unwrap();

        let extracted = UCloudEvent::get_payload(&cloud_event);

        assert_eq!(Any::default(), extracted);
    }

    #[test]
    fn test_unpack_payload_by_class_from_cloud_event_proto_message_object() {
        // Creating a protobuf CloudEvent message
        let source_event = cloudevents::EventBuilderV10::new()
            .id("hello")
            .ty(UCloudEventType::PUBLISH)
            .source(Url::parse("up://VCU.MY_CAR_VIN/someService").unwrap())
            .ty("example.demo")
            .data(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                Any::default().value,
            )
            .build()
            .unwrap();

        let proto_event = CloudEvent::from(source_event.clone());
        let bytes = proto_event.encode_to_vec();

        // Creating the CloudEvent
        let cloud_event = EventBuilderV10::new()
            .id("someId")
            .ty(UCloudEventType::PUBLISH)
            .source(Url::parse("up:/body.access/1/door.front_left#Door").unwrap())
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                Url::parse("proto://type.googleapis.com/io.cloudevents.v1.CloudEvent").unwrap(),
                Data::Binary(bytes),
            )
            .build()
            .unwrap();

        let nb = UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap();
        let extracted: Result<CloudEvent, prost::DecodeError> = CloudEvent::decode(nb.as_slice());

        assert!(extracted.is_ok());
        let unpacked_event = extracted.unwrap();

        assert_eq!(
            cloudevents::event::SpecVersion::V10.to_string(),
            unpacked_event.spec_version
        );
        assert_eq!("hello", unpacked_event.id);
        assert_eq!("example.demo", unpacked_event.r#type);
        assert_eq!("up://VCU.MY_CAR_VIN/someService", unpacked_event.source);
    }

    #[test]
    fn test_unpack_payload_from_cloud_event_when_not_valid_message() {
        // Create the CloudEvent with some non-protobuf binary data
        let cloud_event = EventBuilderV10::new()
            .id("someId")
            .ty("pub.v1")
            .source(Url::parse("up:/body.access/1/door.front_left#Door").unwrap())
            .data_with_schema(
                UCloudEventBuilder::PROTOBUF_CONTENT_TYPE,
                Url::parse("proto://type.googleapis.com/io.cloudevents.v1.CloudEvent").unwrap(),
                Data::Binary(b"<html><head></head><body><p>Hello</p></body></html>".to_vec()),
            )
            .build()
            .unwrap();

        // Try to unpack the event data into a proto::CloudEventProto
        let nb = UCloudEvent::serialize_event_data_into_bytes(&cloud_event).unwrap();
        let extracted: Result<CloudEvent, prost::DecodeError> = CloudEvent::decode(nb.as_slice());

        // Assert that the extraction was unsuccessful (since we used non-protobuf data)
        assert!(extracted.is_err());
    }

    #[test]
    fn test_pretty_printing_a_cloudevent_with_a_sink() {
        let sink_for_test = "//bo.cloud/petapp/1/rpc.response";

        let cloud_event = build_base_cloud_event_for_test()
            .extension("sink", sink_for_test)
            .build()
            .unwrap();

        let pretty_print = UCloudEvent::to_string(&cloud_event);

        let expected = "CloudEvent{id='testme', source='/body.access//door.front_left#Door', \
                    sink='//bo.cloud/petapp/1/rpc.response', type='pub.v1'}";

        assert_eq!(expected, pretty_print);
    }

    #[test]
    fn test_pretty_printing_a_cloudevent_without_a_sink() {
        let cloud_event = build_base_cloud_event_for_test().build().unwrap();

        let pretty_print = UCloudEvent::to_string(&cloud_event);

        let expected =
            "CloudEvent{id='testme', source='/body.access//door.front_left#Door', type='pub.v1'}";

        assert_eq!(expected, pretty_print);
    }

    fn build_base_cloud_event_for_test() -> EventBuilderV10 {
        let ue = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(ue),
            Some(UResource::new(
                "door".to_string(),
                Some(String::from("front_left")),
                Some(String::from("Door")),
                None,
                false,
            )),
        );
        let source = LongUriSerializer::serialize(&uri);

        // fake payload
        let payload = pack_event_into_any(&build_proto_payload_for_test());

        // additional attributes
        let attributes = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(Priority::Standard)
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
        event.ty(UCloudEventType::PUBLISH)
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

    fn build_proto_payload_for_test() -> Event {
        EventBuilderV10::new()
            .id("hello")
            .source("//VCU.MY_CAR_VIN/body.access//door.front_left#Door")
            .ty(UCloudEventType::PUBLISH)
            .data_with_schema(
                "application/octet-stream",
                "proto://type.googleapis.com/example.demo",
                Any::default().value,
            )
            .build()
            .unwrap()
    }
}
