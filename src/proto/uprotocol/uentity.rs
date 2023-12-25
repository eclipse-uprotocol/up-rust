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

use crate::uri::validator::ValidationError;

const UENTITY_ID_LENGTH: usize = 16;
const UENTITY_ID_VALID_BITMASK: u32 = 0xffff << UENTITY_ID_LENGTH;
const UENTITY_MAJOR_VERSION_LENGTH: usize = 8;
const UENTITY_MAJOR_VERSION_VALID_BITMASK: u32 = 0xffffff << UENTITY_MAJOR_VERSION_LENGTH;

impl UEntity {
    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }

    /// Returns whether a `UEntity`'s `id` can fit within the 16 bits allotted for the micro URI format
    ///
    /// # Returns
    /// Returns a `Result<bool, ValidationError>` where the error means id is empty and happy path tells us whether it fits (true)
    /// or not (false)
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` in the failure case, indicating no id present
    pub fn id_fits_micro_uri(&self) -> Result<bool, ValidationError> {
        if let Some(id) = self.id {
            if id & UENTITY_ID_VALID_BITMASK == 0 {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(ValidationError::new("Missing id"))
        }
    }

    pub fn version_fits_micro_uri(&self) -> Result<bool, ValidationError> {
        if let Some(id) = self.version_major {
            if id & UENTITY_MAJOR_VERSION_VALID_BITMASK == 0 {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(ValidationError::new("Major version must be present"))
        }
    }
}
