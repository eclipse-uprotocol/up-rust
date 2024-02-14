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

use crate::uprotocol::uri::uauthority::Number;
use crate::uprotocol::UAuthority;

pub use crate::uri::validator::ValidationError;

const REMOTE_IPV4_BYTES: usize = 4;
const REMOTE_IPV6_BYTES: usize = 16;
const REMOTE_ID_MINIMUM_BYTES: usize = 1;
const REMOTE_ID_MAXIMUM_BYTES: usize = 255;

/// Helper functions to deal with `UAuthority::Remote` structure
impl UAuthority {
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns whether a `UAuthority` satisfies the requirements of a micro form URI
    ///
    /// # Returns
    /// Returns a `Result<(), ValidationError>` where the ValidationError will contain the reasons it failed or OK(())
    /// otherwise
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the failure case
    pub fn validate_micro_form(&self) -> Result<(), ValidationError> {
        let Some(number) = &self.number else {
            return Err(ValidationError::new(
                "Must have IP address or ID set as UAuthority for micro form. Neither are set.",
            ));
        };

        match number {
            Number::Ip(ip) => {
                if !(ip.len() == REMOTE_IPV4_BYTES || ip.len() == REMOTE_IPV6_BYTES) {
                    return Err(ValidationError::new(
                        "IP address is not IPv4 (4 bytes) or IPv6 (16 bytes)",
                    ));
                }
            }
            Number::Id(id) => {
                if !matches!(id.len(), REMOTE_ID_MINIMUM_BYTES..=REMOTE_ID_MAXIMUM_BYTES) {
                    return Err(ValidationError::new("ID doesn't fit in bytes allocated"));
                }
            }
        }
        Ok(())
    }
}
