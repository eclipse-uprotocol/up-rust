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

use crate::uprotocol::uri::UAuthority;

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

    pub fn get_ip(&self) -> Option<&[u8]> {
        self.ip.as_deref()
    }

    pub fn has_ip(&self) -> bool {
        self.ip.is_some()
    }

    pub fn get_id(&self) -> Option<&[u8]> {
        self.id.as_deref()
    }

    pub fn has_id(&self) -> bool {
        self.id.is_some()
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
        let mut validation_errors = Vec::new();

        match &self.remote {
            None => {
                validation_errors.push(ValidationError::new("Has Authority, but no remote"));
            }
            Some(Remote::Ip(ip)) => {
                if !(ip.len() == REMOTE_IPV4_BYTES || ip.len() == REMOTE_IPV6_BYTES) {
                    validation_errors.push(ValidationError::new("IP address is not IPv4 (4 bytes) or IPv6 (16 bytes)"));
                }
            }
            Some(Remote::Id(id)) => {
                if !matches!(
                    id.len(),
                    REMOTE_ID_MINIMUM_BYTES..=REMOTE_ID_MAXIMUM_BYTES
                ) {
                    validation_errors.push(ValidationError::new("ID doesn't fit in bytes allocated"));
                }
            }
            Some(Remote::Name(_)) => {
                validation_errors.push(ValidationError::new("Must use IP address or ID as UAuthority for micro form."));
            }
        }

        if !validation_errors.is_empty() {
            let combined_message = validation_errors
                .into_iter()
                .map(|err| err.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            Err(ValidationError::new(combined_message))
        } else {
            Ok(())
        }
    }
}
