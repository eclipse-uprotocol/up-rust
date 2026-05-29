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
use crate::up_core_api::uattributes::UPriority as UPriorityProto;
use crate::up_core_api::uoptions::exts::ce_name;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(C)]
pub enum UPriority {
    CS0 = UPriorityProto::UPRIORITY_CS0 as isize,
    CS1 = UPriorityProto::UPRIORITY_CS1 as isize,
    CS2 = UPriorityProto::UPRIORITY_CS2 as isize,
    CS3 = UPriorityProto::UPRIORITY_CS3 as isize,
    CS4 = UPriorityProto::UPRIORITY_CS4 as isize,
    CS5 = UPriorityProto::UPRIORITY_CS5 as isize,
    CS6 = UPriorityProto::UPRIORITY_CS6 as isize,
}

impl From<&UPriority> for UPriorityProto {
    fn from(value: &UPriority) -> Self {
        match value {
            UPriority::CS0 => UPriorityProto::UPRIORITY_CS0,
            UPriority::CS1 => UPriorityProto::UPRIORITY_CS1,
            UPriority::CS2 => UPriorityProto::UPRIORITY_CS2,
            UPriority::CS3 => UPriorityProto::UPRIORITY_CS3,
            UPriority::CS4 => UPriorityProto::UPRIORITY_CS4,
            UPriority::CS5 => UPriorityProto::UPRIORITY_CS5,
            UPriority::CS6 => UPriorityProto::UPRIORITY_CS6,
        }
    }
}

impl TryFrom<UPriorityProto> for UPriority {
    type Error = UAttributesError;

    fn try_from(value: UPriorityProto) -> Result<Self, Self::Error> {
        match value {
            UPriorityProto::UPRIORITY_CS0 => Ok(UPriority::CS0),
            UPriorityProto::UPRIORITY_CS1 => Ok(UPriority::CS1),
            UPriorityProto::UPRIORITY_CS2 => Ok(UPriority::CS2),
            UPriorityProto::UPRIORITY_CS3 => Ok(UPriority::CS3),
            UPriorityProto::UPRIORITY_CS4 => Ok(UPriority::CS4),
            UPriorityProto::UPRIORITY_CS5 => Ok(UPriority::CS5),
            UPriorityProto::UPRIORITY_CS6 => Ok(UPriority::CS6),
            _ => Err(UAttributesError::parsing_error(format!(
                "invalid UPriority value: {}",
                value as i32
            ))),
        }
    }
}

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
    /// assert_eq!(UPriority::CS2.to_priority_code(), "CS2");
    /// ```
    #[must_use]
    pub fn to_priority_code(self) -> String {
        let desc = UPriorityProto::from(&self).descriptor();
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
    /// assert_eq!(priority, UPriority::CS2);
    ///
    /// assert!(UPriority::try_from_priority_code("not-supported").is_err());
    /// ```
    pub fn try_from_priority_code<T>(code: T) -> Result<Self, UAttributesError>
    where
        T: Into<String>,
    {
        let prio: String = code.into();
        UPriorityProto::enum_descriptor()
            .values()
            .find_map(|desc| {
                let proto_desc = desc.proto();

                ce_name
                    .get(proto_desc.options.get_or_default())
                    .and_then(|prio_option_value| {
                        if prio_option_value.eq(prio.as_str()) {
                            desc.cast::<UPriorityProto>()
                                .and_then(|prio_proto| UPriority::try_from(prio_proto).ok())
                        } else {
                            None
                        }
                    })
            })
            .ok_or_else(|| UAttributesError::parsing_error(format!("unknown priority [{prio}]")))
    }
}
