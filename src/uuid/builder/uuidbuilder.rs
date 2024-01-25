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
use std::convert::Into;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::uprotocol::UUID;

const MAX_COUNT: u64 = 0xfff;
const MAX_TIMESTAMP_BITS: u8 = 48;
const MAX_TIMESTAMP_MASK: u64 = 0xffff << MAX_TIMESTAMP_BITS;

/// uProtocol `UUIDv8` data model
///
/// `UUIDv8` can only be built using the static factory methods of the class,
/// given that the `UUIDv8` datamodel is based off the previous UUID generated.
///
/// The UUID is based off [draft-ietf-uuidrev-rfc4122bis](https://datatracker.ietf.org/doc/draft-ietf-uuidrev-rfc4122bis/)
/// and `UUIDv7` with some modifications that are discussed below.
/// The diagram below shows the specification for the UUID (top left is the most significant bit,
/// bottom right is the least significant bit):
///
/// ```plaintext
///     0                   1                   2                   3
///     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |                         unix_ts_ms                            |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |           unix_ts_ms          |  ver  |         counter       |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |var|                          rand_b                           |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |                           rand_b                              |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// | Field      | Description |
/// | ---------- | ----------- |
/// | unix_ts_ms | MUST be the 48 bit big-endian unsigned number of Unix epoch timestamp in milliseconds as per Section 6.1  of RFC |
/// | ver        | MUST be 8 per Section 4.2 of draft-ietf-uuidrev-rfc4122bis |
/// | counter    | MUST be a 12 bit counter field that is reset at each unix_ts_ms tick, and incremented for each UUID generated within the 1ms precision of unix_ts_ms The counter provides the ability to generate 4096 events within 1ms however the precision of the clock is still 1ms accuracy |
/// | var        | MUST be the The 2 bit variant defined by Section 4.1 of RFC |
/// | rand_b     | MUST be a 62 bits random number that is generated at initialization time of the uE only and reused otherwise |

pub struct UUIDv8Builder {
    msb: AtomicU64,
    lsb: [u8; 8],
}

impl Default for UUIDv8Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl UUIDv8Builder {
    pub fn new() -> Self {
        UUIDv8Builder {
            // we do not need to explicitly set the version and variant bits
            // because this will be done implicitly by the
            // call to uuid::builder::Builder::from_custom_bytes
            // when creating a UUID using one of the build functions
            msb: AtomicU64::new(0),
            lsb: rand::thread_rng().gen::<u64>().to_be_bytes(),
        }
    }

    /// Creates a new UUID for the current system time.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp (in milliseconds since UNIX EPOCH) to use.
    ///
    /// # Panics
    ///
    /// if the system time
    /// * is set to a point in time before UNIX Epoch, or
    /// * is set to a point in time later than UNIX Epoch + 0xFFFFFFFFFFFF seconds
    pub fn build(&self) -> UUID {
        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
            if let Ok(now) = u64::try_from(now.as_millis()) {
                self.build_with_instant(now)
            } else {
                panic!("current system time is set to a point in time too far in the future");
            }
        } else {
            panic!("current system time is set to a point in time before UNIX Epoch");
        }
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
    fn build_with_instant(&self, timestamp: u64) -> UUID {
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

            self.msb.load(Ordering::SeqCst)
        };

        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&new_msb.to_be_bytes());
        bytes[8..].copy_from_slice(&self.lsb);
        uuid::Builder::from_custom_bytes(bytes).into_uuid().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_creates_valid_uprotocol_uuid() {
        let uuid = UUIDv8Builder::new().build();
        assert!(uuid.is_uprotocol_uuid());
    }

    #[test]
    fn test_build_with_instant_creates_uprotocol_uuid() {
        let instant = 0x18C684468F8u64; // Thu, 14 Dec 2023 12:19:23 GMT
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
        let instant = 0x18C684468F8u64; // Thu, 14 Dec 2023 12:19:23 GMT
        let builder = UUIDv8Builder::new();
        let _uuid_for_instant = builder.build_with_instant(instant);
        let uuid_for_same_instant = builder.build_with_instant(instant);
        assert!(uuid_for_same_instant.is_uprotocol_uuid());

        // same instant, version (8) and _incremented_ counter (001) should show up in UUID
        assert!(uuid_for_same_instant
            .to_hyphenated_string()
            .starts_with("018c6844-68f8-8001-"));
    }

    #[test]
    fn test_obj_to_string_conversions() {
        let uuid1 = UUIDv8Builder::new().build();
        let str1 = uuid1.to_hyphenated_string();
        let uuid2 = uuid::Uuid::parse_str(&str1).unwrap();
        assert_eq!(str1, uuid2.as_hyphenated().to_string());
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
