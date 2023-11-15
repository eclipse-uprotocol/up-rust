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

use mac_address::{get_mac_address, MacAddress};
use rand::Rng;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::{Builder, ClockSequence, Timestamp, Uuid};

const UUIDV8_VERSION: u64 = 8;
const MAX_COUNT: u64 = 0xfff;
const EMPTY_NODE_ID: [u8; 6] = [0, 0, 0, 0, 0, 0];

pub struct UuidClockSequence {
    counter: AtomicU16,
}

impl UuidClockSequence {
    pub fn new() -> Self {
        UuidClockSequence {
            counter: AtomicU16::new(0),
        }
    }
}

impl Default for UuidClockSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockSequence for UuidClockSequence {
    type Output = u16;

    fn generate_sequence(&self, _seconds: u64, _subsec_nanos: u32) -> Self::Output {
        // For simplicity, we're currently not using seconds or subsec_nanos

        // Increment and wrap the counter safely using atomic operations
        self.counter.fetch_add(1, Ordering::SeqCst) & 0x3FFF
    }
}

pub struct UUIDv6Builder {
    address: MacAddress,
    counter: UuidClockSequence,
}

impl Default for UUIDv6Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl UUIDv6Builder {
    pub fn new() -> Self {
        let address_bytes = match get_mac_address() {
            Ok(Some(mac)) => mac.bytes(),
            _ => EMPTY_NODE_ID,
        };

        UUIDv6Builder {
            address: MacAddress::from(address_bytes),
            counter: UuidClockSequence::new(),
        }
    }

    pub fn with_mac_address(mut self, address: MacAddress) -> Self {
        self.address = address;
        self
    }

    pub fn build(&self) -> Uuid {
        Uuid::now_v6(&self.address.bytes())
    }

    pub fn build_with_instant(&self, instant: u64) -> Uuid {
        let instant = Timestamp::from_rfc4122(instant, self.counter.generate_sequence(0, 0));
        Uuid::new_v6(instant, &self.address.bytes())
    }
}

/// uProtocol UUIDv8 data model
///
/// UUIDv8 can only be built using the static factory methods of the class,
/// given that the UUIDv8 datamodel is based off the previous UUID generated.
///
/// The UUID is based off the draft-ietf-uuidrev-rfc4122bis and UUIDv7 with
/// some modifications that are discussed below. The diagram below shows the
/// specification for the UUID:
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
/// | Field      | RFC2119 |
/// | -----      | --------|
/// | unix_ts_ms | 48 bit big-endian unsigned number of Unix epoch timestamp in milliseconds as per Section 6.1  of RFC
/// | ver        | MUST be 8 per Section 4.2 of draft-ietf-uuidrev-rfc4122bis
/// | counter    | MUST be a 12 bit counter field that is reset at each unix_ts_ms tick, and incremented for each UUID generated within the 1ms precision of unix_ts_ms The counter provides the ability to generate 4096 events within 1ms however the precision of the clock is still 1ms accuracy
/// | var        | MUST be the The 2 bit variant defined by Section 4.1 of RFC
/// | rand_b     | MUST 62 bits random number that is generated at initialization time of the uE only and reused otherwise |

pub struct UUIDv8Builder {
    msb: Arc<Mutex<u64>>,
    lsb: u64,
}

impl Default for UUIDv8Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl UUIDv8Builder {
    // The java-sdk implementation uses a signed 64 bit integer here, which can lead to the below operation to overflow. In Rust,
    // we therefore make lsb an unsigned value. To be be identical with the java SDK implementation, _lsb would need to be an i64,
    // and we need the compiler directive to allow overflowing literals: #[allow(overflowing_literals)]
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let _lsb: u64 = (rng.gen::<u64>() & 0x3fffffffffffffff) | 0x8000000000000000;

        UUIDv8Builder {
            msb: Arc::new(Mutex::new(UUIDV8_VERSION << 12)),
            lsb: _lsb,
        }
    }

    pub fn build(&self) -> Uuid {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.build_with_instant(now)
    }

    pub fn build_with_instant(&self, instant: u64) -> Uuid {
        let new_msb = {
            let mut msb = self.msb.lock().unwrap();

            if instant == (*msb >> 16) {
                // Increment the counter if we are not at MAX_COUNT
                if (*msb & 0xFFFu64) < MAX_COUNT {
                    *msb += 1;
                }
            } else {
                *msb = (instant << 16) | (8u64 << 12);
            }

            *msb
        };

        let mut bytes = [0u8; 16]; // 8 bytes for msg and 8 bytes for lsb
        bytes[..8].copy_from_slice(&new_msb.to_le_bytes());
        bytes[8..].copy_from_slice(&self.lsb.to_le_bytes());
        Builder::from_custom_bytes(bytes).into_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uuid::builder::uuidutils::UuidUtils;
    use base64::{engine::general_purpose, Engine as _};

    #[test]
    fn test_string_to_obj_conversions() {
        let str1 = "01868381-1590-8000-cfe2-68135f43b363";
        let uuid = Uuid::parse_str(str1).unwrap();
        let str2 = uuid.to_string();
        assert_eq!(str1, str2);
    }

    #[test]
    fn test_obj_to_string_conversions() {
        let uuid_factory = UUIDv8Builder::new();
        let uuid1 = uuid_factory.build();
        let str1 = uuid1.to_string();
        let uuid2 = Uuid::parse_str(&str1).unwrap();
        assert_eq!(str1, uuid2.to_string());
    }

    #[test]
    fn test_uuid_for_constant_random() {
        let factory = UUIDv8Builder::new();
        let uuid1 = factory.build();
        let uuid2 = factory.build();
        assert_eq!(uuid1.to_fields_le().3, uuid2.to_fields_le().3); // Check that the "node" field (least significant 64 bits) is the same
    }

    #[test]
    fn test_uuid_create_test_counters() {
        let uuidv8_factory = UUIDv8Builder::new();
        let mut uuids = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        for _ in 0..4096 {
            let uuid = uuidv8_factory.build_with_instant(now);
            uuids.push(uuid);

            // Assert that the timestamp is the same as the first UUID
            assert_eq!(UuidUtils::get_time(&uuids[0]), UuidUtils::get_time(&uuid));

            // Assert that the random part is the same as the first UUID
            assert_eq!(
                &uuids[0].hyphenated().to_string()[19..],
                &uuid.hyphenated().to_string()[19..]
            );
        }
    }

    #[test]
    fn test_uuid_byte_obj_conversions() {
        let factory = UUIDv8Builder::new();
        let uuid1 = factory.build();

        // Convert the UUID to a byte array
        let bytes = uuid1.as_bytes().to_vec();

        // Convert the byte array back to a UUID
        let uuid2 = Uuid::from_slice(&bytes).unwrap();

        // Compare the bytes from the original and the re-converted UUID
        assert_eq!(bytes, uuid2.as_bytes().to_vec());

        // Compare the string representations of the original and re-converted UUID
        assert_eq!(uuid1.to_string(), uuid2.to_string());
    }

    #[test]
    fn test_uuid6_byte_obj_conversions() {
        let uuid1 = UUIDv6Builder::new().build();

        // Convert the UUID to a byte array
        let bytes = uuid1.as_bytes().to_vec();

        // Convert the byte array back to a UUID
        let uuid2 = Uuid::from_slice(&bytes).unwrap();

        // Compare the bytes from the original and the re-converted UUID
        assert_eq!(bytes, uuid2.as_bytes().to_vec());

        // Compare the string representations of the original and re-converted UUID
        assert_eq!(uuid1.to_string(), uuid2.to_string());
    }

    #[test]
    fn test_uuid6_build_many() {
        let uuidv6_factory = UUIDv6Builder::new();
        let mut uuids = Vec::new();

        for _ in 0..4096 {
            let uuid = uuidv6_factory.build();
            uuids.push(uuid);
        }

        // Try adding one more, but there is no counters in UUIDv6 version, so it doesn't cause any errors
        let uuid = uuidv6_factory.build();
        uuids.push(uuid);

        // Now we should have 4097 UUIDs
        assert_eq!(uuids.len(), 4097);

        // Now we check that the time components of the first and last UUIDs are not the same
        let first_uuid = uuids.first().unwrap();
        let last_uuid = uuids.last().unwrap();

        assert_ne!(first_uuid.to_string(), last_uuid.to_string());
    }

    // The following test are strange/unnecessary with the Rust uuid crate's strong type guarantees,
    // so have not been ported:
    //  - test_uuid1_gettime
    //  - test_us_uuid_version_checks

    #[test]
    fn test_uuid_size() {
        let factory = UUIDv8Builder::new();
        let uuid1 = factory.build();
        let bytes = uuid1.as_bytes();

        let encoded = general_purpose::STANDARD.encode(bytes);
        let decoded = general_purpose::STANDARD.decode(encoded.clone()).unwrap();
        let uuid2 = Uuid::from_slice(&decoded).unwrap();

        assert_eq!(bytes, &decoded[..]);
        assert_eq!(uuid1.to_string(), uuid2.to_string());
    }
}
