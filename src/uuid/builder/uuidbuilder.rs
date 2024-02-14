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

use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::uprotocol::UUID;

const BITMASK_CLEAR_VERSION: u64 = 0xffffffffffff0fff;
const BITMASK_CLEAR_VARIANT: u64 = 0x3fffffffffffffff;

const MAX_COUNT: u64 = 0xfff;
const MAX_TIMESTAMP_BITS: u8 = 48;
const MAX_TIMESTAMP_MASK: u64 = 0xffff << MAX_TIMESTAMP_BITS;

/// A factory for creating UUIDs that can be used with uProtocol.
///
/// The structure of the UUIDs created by this factory is defined in the
/// [uProtocol specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/uuid.adoc).
pub struct UUIDv8Builder {
    msb: AtomicU64,
    lsb: u64,
}

impl Default for UUIDv8Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl UUIDv8Builder {
    /// Creates a new builder for creating uProtocol UUIDs.
    ///
    /// The same bulder instance can be used to create one or more UUIDs
    /// by means of invoking [`UUIDBuilder::build`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uuid::builder::UUIDv8Builder;
    ///
    /// let builder = UUIDv8Builder::new();
    /// let uuid1 = builder.build();
    /// assert!(uuid1.is_uprotocol_uuid());
    /// let uuid2 = builder.build();
    /// assert!(uuid2.is_uprotocol_uuid());
    /// assert_ne!(uuid1, uuid2);
    /// ```
    pub fn new() -> Self {
        UUIDv8Builder {
            msb: AtomicU64::new(0),
            // set variant to RFC4122
            lsb: rand::thread_rng().gen::<u64>() & BITMASK_CLEAR_VARIANT
                | crate::proto::uprotocol::uuid::VARIANT_RFC4122,
        }
    }

    /// Creates a new UUID for the current system time.
    ///
    /// # Panics
    ///
    /// if the system time is either
    /// * set to a point in time before UNIX Epoch, or
    /// * set to a point in time later than UNIX Epoch + 0xFFFFFFFFFFFF seconds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uuid::builder::UUIDv8Builder;
    ///
    /// let uuid = UUIDv8Builder::new().build();
    /// assert!(uuid.is_uprotocol_uuid());
    /// assert!(uuid.get_time().is_some());
    /// ```
    pub fn build(&self) -> UUID {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current system time is set to a point in time before UNIX Epoch");
        let now_millis = u64::try_from(now.as_millis())
            .expect("current system time is set to a point in time too far in the future");
        self.build_with_instant(now_millis)
    }

    /// Creates a new UUID for a given timestamp.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp (in milliseconds since UNIX EPOCH) to use.
    ///
    /// # Panics
    ///
    /// * if the given timestamp is greater than 2^48 - 1.
    pub(crate) fn build_with_instant(&self, timestamp: u64) -> UUID {
        assert!(
            timestamp & MAX_TIMESTAMP_MASK == 0,
            "Timestamp of UUID must not exceed 48 bits"
        );

        let new_msb = {
            let current_msb = self.msb.load(Ordering::SeqCst);

            if timestamp == (current_msb >> 16) {
                if (current_msb & MAX_COUNT) < MAX_COUNT {
                    self.msb.fetch_add(1, Ordering::SeqCst);
                } else {
                    // this should never happen in practice because we
                    // do not expect any uEntity to emit more than
                    // 4095 messages/ms
                    // so we simply keep the current counter at MAX_COUNT
                }
            } else {
                self.msb.store(timestamp << 16, Ordering::SeqCst);
            }

            // set UUID's version to 'custom'
            self.msb.load(Ordering::SeqCst) & BITMASK_CLEAR_VERSION
                | crate::proto::uprotocol::uuid::VERSION_CUSTOM
        };

        UUID::from_u64_pair(new_msb, self.lsb)
            .expect("should have been able to create UUID for valid timestamp")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_with_instant_creates_uprotocol_uuid() {
        let instant = 0x18C684468F8_u64; // Thu, 14 Dec 2023 12:19:23 GMT
        let uuid = UUIDv8Builder::new().build_with_instant(instant);
        assert!(uuid.is_uprotocol_uuid());
        assert_eq!(uuid.get_time().unwrap(), instant);

        // instant, version (8) and counter (000) should show up in UUID
        assert!(uuid
            .to_hyphenated_string()
            .starts_with("018c6844-68f8-8000-"));
    }

    #[test]
    fn test_uuid_for_subsequent_generation() {
        let instant = 0x18C684468F8_u64; // Thu, 14 Dec 2023 12:19:23 GMT
        let builder = UUIDv8Builder::new();

        let uuid_for_instant = builder.build_with_instant(instant);
        assert!(uuid_for_instant.is_uprotocol_uuid());
        // instant, version (8) and counter (000) should show up in UUID
        assert!(uuid_for_instant
            .to_hyphenated_string()
            .starts_with("018c6844-68f8-8000-"));

        let uuid_for_same_instant = builder.build_with_instant(instant);
        assert!(uuid_for_same_instant.is_uprotocol_uuid());
        // same instant, version (8) and _incremented_ counter (001) should show up in UUID
        assert!(uuid_for_same_instant
            .to_hyphenated_string()
            .starts_with("018c6844-68f8-8001-"));
    }

    #[test]
    fn test_uuid_for_constant_random() {
        let factory = UUIDv8Builder::new();
        let uuid1 = factory.build();
        let uuid2 = factory.build();
        assert_eq!(uuid1.lsb, uuid2.lsb);
    }

    #[test]
    #[should_panic]
    fn test_uuid_panics_for_invalid_timestamp() {
        // maximum value that can be stored in a 48-bit timestamp (in milliseconds)
        let max_48_bit_unix_ts_ms = (1u64 << 48) - 1;

        // add 1 millisecond to the maximum duration, to overflow
        let overflowed_48_bit_unix_ts_ms = max_48_bit_unix_ts_ms + 1;

        let builder = UUIDv8Builder::new();
        let _uprotocol_uuid_past_max_unix_ts_ms =
            builder.build_with_instant(overflowed_48_bit_unix_ts_ms);
    }
}
