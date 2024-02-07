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

use crate::uprotocol::uri::UEntity;

pub use crate::uri::validator::ValidationError;

const UENTITY_ID_LENGTH: usize = 16;
const UENTITY_ID_VALID_BITMASK: u32 = 0xffff << UENTITY_ID_LENGTH;
const UENTITY_MAJOR_VERSION_LENGTH: usize = 8;
const UENTITY_MAJOR_VERSION_VALID_BITMASK: u32 = 0xffffff << UENTITY_MAJOR_VERSION_LENGTH;

impl UEntity {
    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }

    /// Returns whether a `UEntity` satisfies the requirements of a micro form URI
    ///
    /// # Returns
    /// Returns a `Result<(), ValidationError>` where the ValidationError will contain the reasons it failed or OK(())
    /// otherwise
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the failure case
    pub fn validate_micro_form(&self) -> Result<(), ValidationError> {
        if let Some(id) = self.id {
            if id & UENTITY_ID_VALID_BITMASK != 0 {
                return Err(ValidationError::new(
                    "ID does not fit within allotted 16 bits in micro form",
                ));
            }
        } else {
            return Err(ValidationError::new("ID must be present"));
        }

        if let Some(major_version) = self.version_major {
            if major_version & UENTITY_MAJOR_VERSION_VALID_BITMASK != 0 {
                return Err(ValidationError::new(
                    "Major version does not fit within 8 allotted bits in micro form",
                ));
            }
        } else {
            return Err(ValidationError::new("Major version must be present"));
        }

        Ok(())
    }
}
