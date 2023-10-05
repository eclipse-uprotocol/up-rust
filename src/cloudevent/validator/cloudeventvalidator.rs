use cloudevents::event::SpecVersion;
use cloudevents::{AttributesReader, Event};
use std::str::FromStr;

use crate::cloudevent::datamodel::ucloudeventtype::UCloudEventType;
use crate::cloudevent::ucloudevent::UCloudEvent;
use crate::cloudevent::validator::validationresult::ValidationResult;
use crate::transport::datamodel::ustatus::{UCode, UStatus};
use crate::uri::datamodel::uuri::UUri;
use crate::uri::serializer::longuriserializer::LongUriSerializer;
use crate::uri::serializer::uriserializer::UriSerializer;

pub trait CloudEventValidator: std::fmt::Display {
    fn validate(&self, cloud_event: &Event) -> UStatus {
        let error_messages: Vec<String> = vec![
            self.validate_version(cloud_event),
            self.validate_id(cloud_event),
            self.validate_source(cloud_event),
            self.validate_type(cloud_event),
            self.validate_sink(cloud_event),
        ]
        .into_iter()
        .filter(|status| status.is_failure())
        .map(|status| status.get_message())
        .collect();

        let error_message = error_messages.join(",");
        if error_message.is_empty() {
            UStatus::ok()
        } else {
            UStatus::fail_with_msg_and_reason(&error_message, UCode::InvalidArgument)
        }
    }

    fn validate_version(&self, cloud_event: &Event) -> ValidationResult {
        let version = cloud_event.specversion();

        if version == SpecVersion::V10 {
            ValidationResult::success()
        } else {
            ValidationResult::failure(&format!(
                "Invalid CloudEvent version [{}]. CloudEvent version must be 1.0.",
                version
            ))
        }
    }

    fn validate_id(&self, cloud_event: &Event) -> ValidationResult {
        if UCloudEvent::is_cloud_event_id(cloud_event) {
            ValidationResult::success()
        } else {
            ValidationResult::failure(&format!(
                "Invalid CloudEvent Id [{}]. CloudEvent Id must be of type UUIDv8.",
                cloud_event.id()
            ))
        }
    }

    fn validate_source(&self, cloud_event: &Event) -> ValidationResult;
    fn validate_type(&self, cloud_event: &Event) -> ValidationResult;

    fn validate_sink(&self, cloud_event: &Event) -> ValidationResult {
        if let Some(sink) = UCloudEvent::get_sink(cloud_event) {
            let uri = LongUriSerializer::deserialize(sink.clone());

            let result = self.validate_entity_uri(&uri);
            if result.is_failure() {
                return ValidationResult::failure(&format!(
                    "Invalid CloudEvent sink [{}]. {}",
                    sink,
                    result.get_message()
                ));
            }
        }
        ValidationResult::Success
    }

    fn validate_entity_uri(&self, uri: &UUri) -> ValidationResult {
        let authority = &uri.authority;

        if authority.is_marked_remote() && authority.device.is_none() {
            return ValidationResult::failure(
                "UriPart is configured to be microRemote and is missing uAuthority device name.",
            );
        }
        if uri.entity.name.trim().is_empty() {
            return ValidationResult::failure("UriPart is missing uSoftware Entity name.");
        }
        ValidationResult::Success
    }

    fn validate_topic_uri(&self, uri: &UUri) -> ValidationResult {
        let result = self.validate_entity_uri(uri);
        if result.is_failure() {
            return result;
        }

        let resource = &uri.resource;
        if resource.name.trim().is_empty() {
            return ValidationResult::failure("UriPart is missing uResource name.");
        }
        if resource.message.is_none() {
            return ValidationResult::failure("UriPart is missing Message information.");
        }
        ValidationResult::Success
    }

    fn validate_rpc_topic_uri(&self, uri: &UUri) -> ValidationResult {
        let result = self.validate_entity_uri(uri);
        if result.is_failure() {
            return ValidationResult::failure(&format!(
                "Invalid RPC uri application response topic. {}",
                result.get_message()
            ));
        }

        let resource = &uri.resource;
        let mut topic = String::from(&resource.name);
        topic.push('.');
        if let Some(instance) = &resource.instance {
            topic.push_str(instance);
        }

        if topic.ne("rpc.response") {
            return ValidationResult::failure(
                "Invalid RPC uri application response topic. UriPart is missing rpc.response.",
            );
        }
        ValidationResult::Success
    }
}

pub enum Validators {
    Response,
    Request,
    Publish,
    Notification,
}

impl Validators {
    pub fn validator(&self) -> Box<dyn CloudEventValidator> {
        match self {
            Validators::Response => Box::new(ResponseValidator),
            Validators::Request => Box::new(RequestValidator),
            Validators::Publish => Box::new(PublishValidator),
            Validators::Notification => Box::new(NotificationValidator),
        }
    }

    pub fn get_validator(cloud_event: &Event) -> Box<dyn CloudEventValidator> {
        if let Ok(event_type) = UCloudEventType::from_str(cloud_event.ty()) {
            match event_type {
                UCloudEventType::RESPONSE => return Box::new(ResponseValidator),
                UCloudEventType::REQUEST => return Box::new(RequestValidator),
                _ => {}
            }
        }
        Box::new(PublishValidator)
    }
}

struct PublishValidator;
impl CloudEventValidator for PublishValidator {
    fn validate_source(&self, cloud_event: &Event) -> ValidationResult {
        let source = LongUriSerializer::deserialize(cloud_event.source().to_string());
        let result = self.validate_topic_uri(&source);
        if result.is_failure() {
            return ValidationResult::failure(&format!(
                "Invalid Publish type CloudEvent source [{}]. {}",
                source,
                result.get_message()
            ));
        }
        ValidationResult::Success
    }

    fn validate_type(&self, cloud_event: &Event) -> ValidationResult {
        if let Ok(event_type) = UCloudEventType::from_str(cloud_event.ty()) {
            if event_type.eq(&UCloudEventType::PUBLISH) {
                return ValidationResult::Success;
            }
        }
        ValidationResult::failure(&format!(
            "Invalid CloudEvent type [{}]. CloudEvent of type Publish must have a type of 'pub.v1'",
            cloud_event.ty(),
        ))
    }
}

impl std::fmt::Display for PublishValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CloudEventValidator.Publish")
    }
}

struct NotificationValidator;
impl CloudEventValidator for NotificationValidator {
    fn validate_source(&self, cloud_event: &Event) -> ValidationResult {
        PublishValidator.validate_source(cloud_event)
    }

    fn validate_type(&self, cloud_event: &Event) -> ValidationResult {
        PublishValidator.validate_type(cloud_event)
    }

    fn validate_sink(&self, cloud_event: &Event) -> ValidationResult {
        if let Some(sink) = UCloudEvent::get_sink(cloud_event) {
            let uri = LongUriSerializer::deserialize(sink.clone());
            let result = self.validate_entity_uri(&uri);
            if result.is_failure() {
                return ValidationResult::failure(&format!(
                    "Invalid Notification type CloudEvent sink [{}]. {}",
                    sink,
                    result.get_message()
                ));
            }
        } else {
            return ValidationResult::failure(
                "Invalid CloudEvent sink. Notification CloudEvent sink must be an uri.",
            );
        }

        ValidationResult::Success
    }
}

impl std::fmt::Display for NotificationValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CloudEventValidator.Notification")
    }
}

struct RequestValidator;
impl CloudEventValidator for RequestValidator {
    fn validate_source(&self, cloud_event: &Event) -> ValidationResult {
        let source = cloud_event.source();
        let uri = LongUriSerializer::deserialize(source.clone());
        let result = self.validate_rpc_topic_uri(&uri);
        if result.is_failure() {
            return ValidationResult::failure(&format!(
                "Invalid RPC Request CloudEvent source [{}]. {}",
                source,
                result.get_message()
            ));
        }
        ValidationResult::Success
    }

    fn validate_sink(&self, cloud_event: &Event) -> ValidationResult {
        if let Some(sink) = UCloudEvent::get_sink(cloud_event) {
            let uri = LongUriSerializer::deserialize(sink.clone());
            let result = self.validate_rpc_topic_uri(&uri);
            if result.is_failure() {
                return ValidationResult::failure(&format!(
                    "Invalid RPC Request CloudEvent sink [{}]. {}",
                    sink,
                    result.get_message()
                ));
            }
        } else {
            return ValidationResult::failure(
                "Invalid RPC Request CloudEvent sink. Request CloudEvent sink must be uri for the method to be called.",
            );
        }

        ValidationResult::Success
    }

    fn validate_type(&self, cloud_event: &Event) -> ValidationResult {
        if let Ok(event_type) = UCloudEventType::from_str(cloud_event.ty()) {
            if event_type.eq(&UCloudEventType::REQUEST) {
                return ValidationResult::Success;
            }
        }
        ValidationResult::failure(&format!(
            "Invalid CloudEvent type [{}]. CloudEvent of type Request must have a type of 'req.v1'",
            cloud_event.ty(),
        ))
    }
}

impl std::fmt::Display for RequestValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CloudEventValidator.Request")
    }
}

struct ResponseValidator;
impl CloudEventValidator for ResponseValidator {
    fn validate_source(&self, cloud_event: &Event) -> ValidationResult {
        let source = cloud_event.source();
        let uri = LongUriSerializer::deserialize(source.clone());
        let result = self.validate_rpc_topic_uri(&uri);
        if result.is_failure() {
            return ValidationResult::failure(&format!(
                "Invalid RPC Response CloudEvent source [{}]. {}",
                source,
                result.get_message()
            ));
        }
        ValidationResult::Success
    }

    fn validate_sink(&self, cloud_event: &Event) -> ValidationResult {
        if let Some(sink) = UCloudEvent::get_sink(cloud_event) {
            let uri = LongUriSerializer::deserialize(sink.clone());
            let result = self.validate_rpc_topic_uri(&uri);
            if result.is_failure() {
                return ValidationResult::failure(&format!(
                    "Invalid RPC Response CloudEvent sink [{}]. {}",
                    sink,
                    result.get_message()
                ));
            }
        } else {
            return ValidationResult::failure(
                "Invalid RPC Response CloudEvent sink. Response CloudEvent sink must be uri of the destination of the response.",
            );
        }
        ValidationResult::Success
    }

    fn validate_type(&self, cloud_event: &Event) -> ValidationResult {
        if let Ok(event_type) = UCloudEventType::from_str(cloud_event.ty()) {
            if event_type.eq(&UCloudEventType::RESPONSE) {
                return ValidationResult::Success;
            }
        }
        ValidationResult::failure(&format!(
            "Invalid CloudEvent type [{}]. CloudEvent of type Response must have a type of 'res.v1'",
            cloud_event.ty(),
        ))
    }
}

impl std::fmt::Display for ResponseValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CloudEventValidator.Response")
    }
}

#[cfg(test)]
mod tests {
    use crate::cloudevent::datamodel::ucloudeventattributes::{
        Priority, UCloudEventAttributesBuilder,
    };
    use crate::cloudevent::ucloudeventbuilder::UCloudEventBuilder;
    use crate::uri::datamodel::uauthority::UAuthority;
    use crate::uri::datamodel::uentity::UEntity;
    use crate::uri::datamodel::uresource::UResource;

    use super::*;

    use cloudevents::{Data, EventBuilder, EventBuilderV10};
    use prost::Message;
    use prost_types::Any;

    #[test]
    fn test_get_a_publish_cloud_event_validator() {
        let cloud_event = build_base_cloud_event_for_test();
        let validator: Box<dyn CloudEventValidator> = Validators::get_validator(&cloud_event);
        let status = validator.validate_type(&cloud_event);

        assert_eq!(status, ValidationResult::Success);
        assert_eq!("CloudEventValidator.Publish", validator.to_string());
    }

    #[test]
    fn test_get_a_notification_cloud_event_validator() {
        let mut cloud_event = build_base_cloud_event_for_test();
        cloud_event.set_extension("sink", "//bo.cloud/petapp");

        let validator: Box<dyn CloudEventValidator> = Validators::Notification.validator();
        let status = validator.validate_type(&cloud_event);

        assert_eq!(status, ValidationResult::Success);
        assert_eq!("CloudEventValidator.Notification", validator.to_string());
    }

    #[test]
    fn test_publish_cloud_event_type() {
        let builder = build_base_cloud_event_builder_for_test();
        let event = builder
            .ty(UCloudEventType::RESPONSE.to_string())
            .build()
            .unwrap();

        let validator: Box<dyn CloudEventValidator> = Validators::Publish.validator();
        let status = validator.validate_type(&event).to_status();

        assert_eq!(UCode::InvalidArgument, status.code());
        assert_eq!(
            "Invalid CloudEvent type [res.v1]. CloudEvent of type Publish must have a type of 'pub.v1'",
            status.msg()
        );
    }

    fn build_base_cloud_event_builder_for_test() -> EventBuilderV10 {
        let entity = UEntity::long_format("body.access".to_string(), None);
        let uri = UUri::new(
            Some(UAuthority::LOCAL),
            Some(entity),
            Some(UResource::long_format("door".to_string())),
        );
        let source = LongUriSerializer::serialize(&uri);
        let payload = build_proto_payload_for_test();
        let attributes = UCloudEventAttributesBuilder::new()
            .with_hash("somehash".to_string())
            .with_priority(Priority::Standard)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        UCloudEventBuilder::build_base_cloud_event(
            "testme",
            &source,
            &payload.encode_to_vec(),
            &payload.type_url,
            &attributes,
        )
    }

    fn build_base_cloud_event_for_test() -> Event {
        let mut builder = build_base_cloud_event_builder_for_test();
        builder = builder.ty(UCloudEventType::PUBLISH.to_string());
        builder.build().unwrap()
    }

    fn build_proto_payload_for_test() -> Any {
        let event = EventBuilderV10::new()
            .id("hello")
            .source("/body.access")
            .ty(UCloudEventType::PUBLISH.to_string())
            .data_with_schema(
                "application/octet-stream",
                "proto://type.googleapis.com/example.demo",
                Any::default().value,
            )
            .build()
            .unwrap();

        pack_event_into_any(&event)
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
