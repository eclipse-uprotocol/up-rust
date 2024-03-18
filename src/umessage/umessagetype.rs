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

use protobuf::EnumFull;

use crate::uattributes::UAttributesError;
pub use crate::up_core_api::uattributes::UMessageType;
use crate::up_core_api::uprotocol_options::exts::ce_name;

impl UMessageType {
    /// Gets this message type's CloudEvent type name.
    ///
    /// # Returns
    ///
    /// The value to use for the *type* property when mapping to a CloudEvent.
    pub fn to_cloudevent_type(&self) -> String {
        let desc = self.descriptor();
        let desc_proto = desc.proto();
        ce_name
            .get(desc_proto.options.get_or_default())
            .unwrap_or_default()
    }

    /// Gets the message type for a CloudEvent type name.
    ///
    /// # Errors
    ///
    /// Returns a [`UAttributesError::ParsingError`] if the given name does not match
    /// any of the supported message types.
    pub fn try_from_cloudevent_type<S: Into<String>>(value: S) -> Result<Self, UAttributesError> {
        let type_string = value.into();

        Self::enum_descriptor()
            .values()
            .find_map(|desc| {
                let proto_desc = desc.proto();

                ce_name
                    .get(proto_desc.options.get_or_default())
                    .and_then(|prio_option_value| {
                        if prio_option_value.eq(type_string.as_str()) {
                            desc.cast::<Self>()
                        } else {
                            None
                        }
                    })
            })
            .ok_or_else(|| {
                UAttributesError::parsing_error(format!("unknown message type: {}", type_string))
            })
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::{UAttributesError, UMessageType};

    const PUBLISH_TYPE: &str = "pub.v1";
    const REQUEST_TYPE: &str = "req.v1";
    const RESPONSE_TYPE: &str = "res.v1";

    #[test_case(UMessageType::UMESSAGE_TYPE_PUBLISH, PUBLISH_TYPE; "for PUBLISH")]
    #[test_case(UMessageType::UMESSAGE_TYPE_REQUEST, REQUEST_TYPE; "for REQUEST")]
    #[test_case(UMessageType::UMESSAGE_TYPE_RESPONSE, RESPONSE_TYPE; "for RESPONSE")]
    fn test_to_cloudevent_type(message_type: UMessageType, expected_ce_name: &str) {
        assert_eq!(message_type.to_cloudevent_type(), expected_ce_name);
    }

    #[test_case(PUBLISH_TYPE, Some(UMessageType::UMESSAGE_TYPE_PUBLISH); "succeeds for PUBLISH")]
    #[test_case(REQUEST_TYPE, Some(UMessageType::UMESSAGE_TYPE_REQUEST); "succeeds for REQUEST")]
    #[test_case(RESPONSE_TYPE, Some(UMessageType::UMESSAGE_TYPE_RESPONSE); "succeeds for RESPONSE")]
    #[test_case("foo.bar", None; "fails for unknown type")]
    fn test_try_from_cloudevent_type(
        cloudevent_type: &str,
        expected_message_type: Option<UMessageType>,
    ) {
        let result = UMessageType::try_from_cloudevent_type(cloudevent_type);
        assert!(result.is_ok() == expected_message_type.is_some());
        if expected_message_type.is_some() {
            assert_eq!(result.unwrap(), expected_message_type.unwrap())
        } else {
            assert!(matches!(
                result.unwrap_err(),
                UAttributesError::ParsingError(_msg)
            ))
        }
    }
}
