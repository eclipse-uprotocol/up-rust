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

use crate::uattributes::UAttributesError;

const CE_TYPE_PUBLISH: &str = "up-pub.v1";
const CE_TYPE_NOTIFICATION: &str = "up-not.v1";
const CE_TYPE_REQUEST: &str = "up-req.v1";
const CE_TYPE_RESPONSE: &str = "up-res.v1";

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum UMessageType {
    Publish,
    Request,
    Response,
    Notification,
}

impl UMessageType {
    /// Gets this message type's CloudEvent type name.
    ///
    /// # Returns
    ///
    /// The value to use for the *type* property when mapping to a CloudEvent.
    #[must_use]
    pub fn to_cloudevent_type(&self) -> String {
        match self {
            UMessageType::Publish => CE_TYPE_PUBLISH,
            UMessageType::Request => CE_TYPE_REQUEST,
            UMessageType::Response => CE_TYPE_RESPONSE,
            UMessageType::Notification => CE_TYPE_NOTIFICATION,
        }
        .to_string()
    }

    /// Gets the message type for a CloudEvent type name.
    ///
    /// # Errors
    ///
    /// Returns a [`UAttributesError::ParsingError`] if the given name does not match
    /// any of the supported message types.
    pub fn try_from_cloudevent_type<S: Into<String>>(value: S) -> Result<Self, UAttributesError> {
        let type_string = value.into();
        match type_string.as_str() {
            CE_TYPE_PUBLISH => Ok(UMessageType::Publish),
            CE_TYPE_NOTIFICATION => Ok(UMessageType::Notification),
            CE_TYPE_REQUEST => Ok(UMessageType::Request),
            CE_TYPE_RESPONSE => Ok(UMessageType::Response),
            _ => Err(UAttributesError::parsing_error(format!(
                "unknown message type: {type_string}"
            ))),
        }
    }
}

#[cfg(feature = "up-core-types")]
mod core_types_support {
    use super::*;
    use crate::up_core_api::uattributes::UMessageType as UMessageTypeProto;

    impl TryFrom<UMessageTypeProto> for UMessageType {
        type Error = UAttributesError;

        fn try_from(proto_message_type: UMessageTypeProto) -> Result<Self, Self::Error> {
            match proto_message_type {
                UMessageTypeProto::UMESSAGE_TYPE_PUBLISH => Ok(UMessageType::Publish),
                UMessageTypeProto::UMESSAGE_TYPE_REQUEST => Ok(UMessageType::Request),
                UMessageTypeProto::UMESSAGE_TYPE_RESPONSE => Ok(UMessageType::Response),
                UMessageTypeProto::UMESSAGE_TYPE_NOTIFICATION => Ok(UMessageType::Notification),
                _ => Err(UAttributesError::parsing_error(format!(
                    "invalid UMessageType value: {}",
                    proto_message_type as i32
                ))),
            }
        }
    }

    impl From<&UMessageType> for UMessageTypeProto {
        fn from(value: &UMessageType) -> Self {
            match value {
                UMessageType::Publish => UMessageTypeProto::UMESSAGE_TYPE_PUBLISH,
                UMessageType::Request => UMessageTypeProto::UMESSAGE_TYPE_REQUEST,
                UMessageType::Response => UMessageTypeProto::UMESSAGE_TYPE_RESPONSE,
                UMessageType::Notification => UMessageTypeProto::UMESSAGE_TYPE_NOTIFICATION,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::{UAttributesError, UMessageType};

    const TYPE_PUBLISH: &str = "up-pub.v1";
    const TYPE_NOTIFICATION: &str = "up-not.v1";
    const TYPE_REQUEST: &str = "up-req.v1";
    const TYPE_RESPONSE: &str = "up-res.v1";

    #[test_case(UMessageType::Publish, TYPE_PUBLISH; "for PUBLISH")]
    #[test_case(UMessageType::Notification, TYPE_NOTIFICATION; "for NOTIFICATION")]
    #[test_case(UMessageType::Request, TYPE_REQUEST; "for REQUEST")]
    #[test_case(UMessageType::Response, TYPE_RESPONSE; "for RESPONSE")]
    fn test_to_cloudevent_type(message_type: UMessageType, expected_ce_name: &str) {
        assert_eq!(message_type.to_cloudevent_type(), expected_ce_name);
    }

    #[test_case(TYPE_PUBLISH, Some(UMessageType::Publish); "succeeds for PUBLISH")]
    #[test_case(TYPE_NOTIFICATION, Some(UMessageType::Notification); "succeeds for NOTIFICATION")]
    #[test_case(TYPE_REQUEST, Some(UMessageType::Request); "succeeds for REQUEST")]
    #[test_case(TYPE_RESPONSE, Some(UMessageType::Response); "succeeds for RESPONSE")]
    #[test_case("foo.bar", None; "fails for unknown type")]
    fn test_try_from_cloudevent_type(
        cloudevent_type: &str,
        expected_message_type: Option<UMessageType>,
    ) {
        let result = UMessageType::try_from_cloudevent_type(cloudevent_type);
        assert!(result.is_ok() == expected_message_type.is_some());
        if let Some(message_type) = expected_message_type {
            assert_eq!(result.unwrap(), message_type)
        } else {
            assert!(matches!(
                result.unwrap_err(),
                UAttributesError::ParsingError(_msg)
            ))
        }
    }
}
