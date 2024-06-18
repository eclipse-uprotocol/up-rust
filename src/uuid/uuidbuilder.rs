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

use rand::RngCore;
use std::{
    ops::Sub,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::UUID;

/// A factory for creating UUIDs that can be used with uProtocol.
///
/// The structure of the UUIDs created by this factory is defined in the
/// [uProtocol specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/uuid.adoc).
pub struct UUIDBuilder {}

impl UUIDBuilder {
    /// Creates a new UUID that can be used for uProtocol messages.
    ///
    /// # Panics
    ///
    /// if the system clock is set to an instant before the UNIX Epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use up_rust::{UUID, UUIDBuilder};
    ///
    /// let uuid = UUIDBuilder::build();
    /// assert!(uuid.is_uprotocol_uuid());
    /// ```
    pub fn build() -> UUID {
        let duration_since_unix_epoch = SystemTime::UNIX_EPOCH
            .elapsed()
            .expect("current system time is set to a point in time before UNIX Epoch");
        Self::build_for_timestamp(duration_since_unix_epoch)
    }

    pub(crate) fn build_for_timestamp(duration_since_unix_epoch: Duration) -> UUID {
        let timestamp_millis = u64::try_from(duration_since_unix_epoch.as_millis())
            .expect("system time is set to a time too far in the future");
        // fill upper 48 bits with timestamp
        let mut msb = (timestamp_millis << 16).to_be_bytes();
        // fill remaining bits with random bits
        rand::thread_rng().fill_bytes(&mut msb[6..]);
        // set version (7)
        msb[6] = msb[6] & 0b00001111 | 0b01110000;

        let mut lsb = [0u8; 8];
        // fill lsb with random bits
        rand::thread_rng().fill_bytes(&mut lsb);
        // set variant (RFC4122)
        lsb[0] = lsb[0] & 0b00111111 | 0b10000000;
        UUID::from_bytes_unchecked(msb, lsb)
    }

    /// Creates a UUID n ms in the past.
    ///
    /// # Note
    ///
    /// For internal testing purposes only. For end-users, please use [`UUIDBuilder::build()`]
    #[allow(dead_code)] // used for testing in other modules, but not picked up on so we disable the warning here
    pub(crate) fn build_n_ms_in_past(n_ms_in_past: u64) -> UUID {
        let duration_since_unix_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current system time is set to a point in time before UNIX Epoch");
        Self::build_for_timestamp(
            duration_since_unix_epoch.sub(Duration::from_millis(n_ms_in_past)),
        )
    }
}
