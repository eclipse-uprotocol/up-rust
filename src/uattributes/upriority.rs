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
pub use crate::up_core_api::uattributes::UPriority;
use crate::up_core_api::uoptions::exts::ce_name;

impl UPriority {
    /// Encodes this priority to a string.
    ///
    /// The encoding of priorities to strings is defined in the
    /// [uProtocol Core API](https://github.com/eclipse-uprotocol/up-core-api/blob/main/uprotocol/uattributes.proto).
    ///
    /// # Examples
    ///
    /// ```
    /// use up_rust::UPriority;
    ///
    /// assert_eq!(UPriority::UPRIORITY_CS2.to_priority_code(), "CS2");
    /// ```
    pub fn to_priority_code(self) -> String {
        let desc = self.descriptor();
        let desc_proto = desc.proto();
        ce_name
            .get(desc_proto.options.get_or_default())
            .unwrap_or_default()
    }

    /// Gets the priority for a string.
    ///
    /// The encoding of priorities to strings is defined in the
    /// [uProtocol Core API](https://github.com/eclipse-uprotocol/up-core-api/blob/main/uprotocol/uattributes.proto).
    ///
    /// # Errors
    ///
    /// Returns an error if the given string does not match a supported priority.
    ///
    /// # Examples
    ///
    /// ```
    /// use up_rust::UPriority;
    ///
    /// let priority = UPriority::try_from_priority_code("CS2").unwrap();
    /// assert_eq!(priority, UPriority::UPRIORITY_CS2);
    ///
    /// assert!(UPriority::try_from_priority_code("not-supported").is_err());
    /// ```
    pub fn try_from_priority_code<T>(code: T) -> Result<Self, UAttributesError>
    where
        T: Into<String>,
    {
        let prio: String = code.into();
        Self::enum_descriptor()
            .values()
            .find_map(|desc| {
                let proto_desc = desc.proto();

                ce_name
                    .get(proto_desc.options.get_or_default())
                    .and_then(|prio_option_value| {
                        if prio_option_value.eq(prio.as_str()) {
                            desc.cast::<Self>()
                        } else {
                            None
                        }
                    })
            })
            .ok_or_else(|| UAttributesError::parsing_error(format!("unknown priority [{}]", prio)))
    }
}
