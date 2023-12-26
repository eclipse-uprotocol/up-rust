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

use crate::uri::validator::ValidationError;

pub enum IpConformance {
    NonConformal,
    IPv4,
    IPv6,
}

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

    pub fn remote_ip_conforms(&self) -> Result<IpConformance, ValidationError> {
        if let Some(_remote) = self.remote.as_ref() {
            match &self.remote {
                Some(Remote::Ip(ip)) => Ok(match ip.len() {
                    REMOTE_IPV4_BYTES => IpConformance::IPv4,
                    REMOTE_IPV6_BYTES => IpConformance::IPv6,
                    _ => IpConformance::NonConformal,
                }),
                _ => Err(ValidationError::new("Remote is not IP")),
            }
        } else {
            Err(ValidationError::new("No remote"))
        }
    }

    pub fn remote_id_conforms(&self) -> Result<bool, ValidationError> {
        if let Some(_remote) = self.remote.as_ref() {
            match &self.remote {
                Some(Remote::Id(id)) => Ok(match id.len() {
                    REMOTE_ID_MINIMUM_BYTES..=REMOTE_ID_MAXIMUM_BYTES => true,
                    _ => false,
                }),
                _ => Err(ValidationError::new("Remote is not ID")),
            }
        } else {
            Err(ValidationError::new("No remote"))
        }
    }
}
