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

use crate::uprotocol::{Remote, UAuthority};

/// Helper functions to deal with UAuthority::Remote structure
impl UAuthority {
    pub fn has_name(authority: &UAuthority) -> bool {
        matches!(authority.remote, Some(Remote::Name(_)))
    }

    pub fn has_ip(authority: &UAuthority) -> bool {
        matches!(authority.remote, Some(Remote::Ip(_)))
    }

    pub fn has_id(authority: &UAuthority) -> bool {
        matches!(authority.remote, Some(Remote::Id(_)))
    }

    pub fn get_name(authority: &UAuthority) -> Option<&String> {
        match &authority.remote {
            Some(Remote::Name(name)) => Some(name),
            _ => None,
        }
    }

    pub fn get_ip(authority: &UAuthority) -> Option<&Vec<u8>> {
        match &authority.remote {
            Some(Remote::Ip(ip)) => Some(ip),
            _ => None,
        }
    }

    pub fn get_id(authority: &UAuthority) -> Option<&Vec<u8>> {
        match &authority.remote {
            Some(Remote::Id(id)) => Some(id),
            _ => None,
        }
    }

    pub fn set_name(authority: &mut UAuthority, name: String) -> &mut UAuthority {
        authority.remote = Some(Remote::Name(name));
        authority
    }

    pub fn set_ip(authority: &mut UAuthority, ip: Vec<u8>) -> &mut UAuthority {
        authority.remote = Some(Remote::Ip(ip));
        authority
    }

    pub fn set_id(authority: &mut UAuthority, id: Vec<u8>) -> &mut UAuthority {
        authority.remote = Some(Remote::Id(id));
        authority
    }
}
