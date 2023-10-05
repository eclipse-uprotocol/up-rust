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

use mac_address::get_mac_address;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::{Builder, Uuid};

const UUIDV8_VERSION: u64 = 8;
const CLOCK_DRIFT_TOLERANCE: u64 = 10_000_000;
const MAX_COUNT: u64 = 0xfff;
const EMPTY_NODE_ID: [u8; 6] = [0, 0, 0, 0, 0, 0];

// enum Factories {
//     UUIDv6,
//     UProtocol,
// }

pub trait UUIDFactory {
    fn create(&self) -> Uuid;
    fn create_with_instant(&self, instant: u64) -> Uuid;
}

struct UUIDv6Factory;

impl UUIDFactory for UUIDv6Factory {
    fn create(&self) -> Uuid {
        self.create_with_instant(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
    }

    fn create_with_instant(&self, _instant: u64) -> Uuid {
        let result = get_mac_address();
        match result {
            Ok(maybe_mac) => {
                match maybe_mac {
                    Some(mac) => {
                        // MAC address retrieved successfully
                        return Uuid::now_v6(&mac.bytes());
                    }
                    None => {
                        // The function succeeded, but there was no MAC address.
                        println!("No MAC address was found.");
                    }
                }
            }
            Err(e) => {
                // The function returned an error.
                eprintln!(
                    "An error occurred while trying to retrieve the MAC address: {}",
                    e
                );
            }
        }
        Uuid::now_v6(&EMPTY_NODE_ID)
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

pub struct UUIDv8Factory {
    msb: Arc<Mutex<u64>>,
    lsb: u64,
}

impl UUIDv8Factory {
    // The java-sdk implementation uses a signed 64 bit integer here, which can lead to the below operation to overflow. In Rust,
    // we therefore make lsb an unsigned value. To be be identical with the java SDK implementation, _lsb would need to be an i64,
    // and we need the compiler directive to allow overflowing literals: #[allow(overflowing_literals)]
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let _lsb: u64 = (rng.gen::<u64>() & 0x3fffffffffffffff) | 0x8000000000000000;

        UUIDv8Factory {
            msb: Arc::new(Mutex::new(UUIDV8_VERSION << 12)),
            lsb: _lsb,
        }
    }
}

impl Default for UUIDv8Factory {
    fn default() -> Self {
        Self::new()
    }
}

impl UUIDFactory for UUIDv8Factory {
    fn create(&self) -> Uuid {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.create_with_instant(now)
    }

    fn create_with_instant(&self, instant: u64) -> Uuid {
        let new_msb = {
            // Lock the mutex only for the time it takes to perform the calculations
            let mut msb = self.msb.lock().unwrap();

            // Check if the current time is the same as the previous time or has moved
            // backwards after a small system clock adjustment or after a leap second.
            // Drift tolerance = (previous_time - 10s) < current_time <= previous_time
            if instant <= (*msb >> 16) && instant > ((*msb >> 16) - CLOCK_DRIFT_TOLERANCE) {
                // Increment the counter if we are not at MAX_COUNT
                if (*msb & 0xFFF) < MAX_COUNT {
                    *msb += 1;
                } else {
                    panic!("Counters out of bounds");
                }

            // The previous time is not the same tick as the current so we reset msb
            } else {
                *msb = (instant << 16) | (UUIDV8_VERSION << 12);
            }

            // Clone the msb to use outside of this block
            *msb
        };

        let mut bytes = [0u8; 16]; // 8 bytes for msg and 8 bytes for lsb
        bytes[..8].copy_from_slice(&new_msb.to_le_bytes());
        bytes[8..].copy_from_slice(&self.lsb.to_le_bytes());

        let builder = Builder::from_custom_bytes(bytes);
        builder.into_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uuid::uuidutils::UuidUtils;
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
        let uuid_factory = UUIDv8Factory::new();
        let uuid1 = uuid_factory.create();
        let str1 = uuid1.to_string();
        let uuid2 = Uuid::parse_str(&str1).unwrap();
        assert_eq!(str1, uuid2.to_string());
    }

    #[test]
    fn test_uuid_for_constant_random() {
        let factory = UUIDv8Factory::new();
        let uuid1 = factory.create();
        let uuid2 = factory.create();
        assert_eq!(uuid1.to_fields_le().3, uuid2.to_fields_le().3); // Check that the "node" field (least significant 64 bits) is the same
    }

    #[test]
    fn test_uuid_create_test_counters() {
        let uuidv8_factory = UUIDv8Factory::new();
        let mut uuids = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        for _ in 0..4096 {
            let uuid = uuidv8_factory.create_with_instant(now);
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
        let factory = UUIDv8Factory::new();
        let uuid1 = factory.create();

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
        let uuid1 = UUIDv6Factory.create();

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
        let uuidv6_factory = UUIDv6Factory {};
        let mut uuids = Vec::new();

        for _ in 0..4096 {
            let uuid = uuidv6_factory.create();
            uuids.push(uuid);
        }

        // Try adding one more, but there is no counters in UUIDv6 version, so it doesn't cause any errors
        let uuid = uuidv6_factory.create();
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
        let factory = UUIDv8Factory::new();
        let uuid1 = factory.create();
        let bytes = uuid1.as_bytes();

        let encoded = general_purpose::STANDARD.encode(bytes);
        let decoded = general_purpose::STANDARD.decode(encoded.clone()).unwrap();
        let uuid2 = Uuid::from_slice(&decoded).unwrap();

        println!(
            "Size of UUID as string is: {}, Length in binary is: {}",
            uuid1.to_string().len(),
            encoded.len()
        );

        assert_eq!(bytes, &decoded[..]);
        assert_eq!(uuid1.to_string(), uuid2.to_string());
    }
}
