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

use once_cell::sync::Lazy;
use rand::random;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::UUID;

const BITMASK_CLEAR_VERSION: u64 = 0xffffffffffff0fff;
const BITMASK_CLEAR_VARIANT: u64 = 0x3fffffffffffffff;

const MAX_COUNT: u64 = 0xfff;

/// A factory for creating UUIDs that can be used with uProtocol.
///
/// The structure of the UUIDs created by this factory is defined in the
/// [uProtocol specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/uuid.adoc).
pub struct UUIDBuilder {
    msb: AtomicU64,
    lsb: u64,
}

impl UUIDBuilder {
    /// Builds a new UUID with consistent `rand_b` portion no matter which thread or task this is
    /// called from. The `rand_b` portion is what uniquely identifies this uE.
    ///
    /// # Returns
    ///
    /// UUID with consistent `rand_b` portion, which uniquely identifies this uE
    pub fn build() -> UUID {
        static UUIDBUILDER_SINGLETON: Lazy<UUIDBuilder> = Lazy::new(UUIDBuilder::new);
        UUIDBUILDER_SINGLETON.build_internal()
    }

    /// Creates a new builder for creating uProtocol UUIDs.
    ///
    /// The same builder instance can be used to create one or more UUIDs
    /// by means of invoking [`UUIDBuilder::build_internal()`].
    ///
    /// # Note
    ///
    /// For internal testing purposes only. For end-users, please use [`UUIDBuilder::build()`]
    pub(crate) fn new() -> Self {
        UUIDBuilder {
            msb: AtomicU64::new(0),
            lsb: random::<u64>() & BITMASK_CLEAR_VARIANT | crate::uuid::VARIANT_RFC4122,
        }
    }

    /// Creates a new UUID based on a particular instance of [`UUIDBuilder`]
    pub(crate) fn build_internal(&self) -> UUID {
        loop {
            let current_msb = self.msb.load(Ordering::SeqCst);

            let cas_top_of_loop_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("current system time is set to a point in time before UNIX Epoch");
            let cas_top_of_loop_millis = u64::try_from(cas_top_of_loop_time.as_millis())
                .expect("current system time is set to a point in time too far in the future");

            let current_timestamp = current_msb >> 16;
            let new_msb;

            if cas_top_of_loop_millis == current_timestamp {
                // If the timestamp hasn't changed, attempt to increment the counter.
                let current_counter = current_msb & MAX_COUNT;
                if current_counter < MAX_COUNT {
                    // Prepare new msb with incremented counter.
                    new_msb = current_msb + 1;
                } else {
                    // this should never happen in practice because we
                    // do not expect any uEntity to emit more than
                    // 4095 messages/ms
                    // so we simply keep the current counter at MAX_COUNT
                    continue;
                }
            } else {
                // New timestamp, reset counter.
                new_msb = (cas_top_of_loop_millis << 16) & BITMASK_CLEAR_VERSION
                    | crate::uuid::VERSION_CUSTOM;
            }

            // Only return if CAS succeeds
            if self
                .msb
                .compare_exchange(current_msb, new_msb, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return UUID::from_u64_pair(new_msb, self.lsb)
                    .expect("should have been able to create UUID for valid timestamp");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use std::collections::HashSet;

    #[async_std::test]
    async fn test_uuidbuilder_concurrency_safety_with_lsb_check() {
        // create enough UUIDs / task that we're likely to run over the counter
        let num_tasks = 10; // Number of concurrent tasks
        let uuids_per_task = 500; // Adjusted to ensure total exceeds 4095 in a ms burst (10 * 409 = 4090 max UUIDs per ms)

        // Obtain a UUID before spawning tasks to determine the expected LSB and ensure consistent
        // across all threads / tasks
        let expected_lsb = UUIDBuilder::build().lsb;

        let mut tasks = Vec::new();

        for task_id in 0..num_tasks {
            let expected_lsb_clone = expected_lsb;
            let task = task::spawn(async move {
                let mut local_uuids = Vec::new();
                for _ in 0..uuids_per_task {
                    let uuid = UUIDBuilder::build();
                    assert_eq!(
                        uuid.lsb, expected_lsb_clone,
                        "LSB does not match the expected value."
                    );
                    local_uuids.push((task_id, uuid));
                }
                local_uuids
            });
            tasks.push(task);
        }

        // Await all tasks and collect their results
        let results = futures::future::join_all(tasks).await;

        #[allow(clippy::mutable_key_type)]
        let mut all_uuids = HashSet::new();
        let mut duplicates = Vec::new();
        for local_uuids in results {
            for (task_id, uuid) in local_uuids {
                if !all_uuids.insert(uuid.clone()) {
                    duplicates.push((task_id, uuid));
                }
            }
        }

        // triggers if we have overrun the counter of 4095 messages / ms
        assert!(
            duplicates.is_empty(),
            "Found {} duplicates. First duplicate from task {}: {:?}",
            duplicates.len(),
            duplicates
                .first()
                .map(|(task_id, _)| *task_id as isize)
                .unwrap_or(-1),
            duplicates.first().map(|(_, uuid)| uuid)
        );

        // another check which would trigger if we've overrun the counter of 4095 messages / ms
        // since if these are not equal, it means we had duplicate UUIDs
        assert_eq!(
            all_uuids.len(),
            num_tasks * uuids_per_task,
            "Mismatch in the total number of expected UUIDs."
        );
    }
}
