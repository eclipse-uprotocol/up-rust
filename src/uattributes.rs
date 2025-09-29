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

mod uattributesvalidator;
mod upayloadformat;
mod upriority;

use std::time::SystemTime;

pub use uattributesvalidator::*;
pub use upriority::*;

pub use crate::up_core_api::uattributes::*;
use crate::UUID;

pub(crate) const UPRIORITY_DEFAULT: UPriority = UPriority::UPRIORITY_CS1;

#[derive(Debug)]
pub enum UAttributesError {
    ValidationError(String),
    ParsingError(String),
}

impl UAttributesError {
    pub fn validation_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ValidationError(message.into())
    }

    pub fn parsing_error<T>(message: T) -> UAttributesError
    where
        T: Into<String>,
    {
        Self::ParsingError(message.into())
    }
}

impl std::fmt::Display for UAttributesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationError(e) => f.write_fmt(format_args!("Validation failure: {e}")),
            Self::ParsingError(e) => f.write_fmt(format_args!("Parsing error: {e}")),
        }
    }
}

impl std::error::Error for UAttributesError {}

impl UAttributes {
    /// Checks if a given priority class is the default priority class.
    ///
    /// Messages that do not have a priority class set explicity, are assigned to
    /// the default priority class.
    pub(crate) fn is_default_priority(prio: UPriority) -> bool {
        prio == UPRIORITY_DEFAULT
    }

    /// Checks if these are the attributes for a Publish message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_PUBLISH.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_publish());
    /// ```
    pub fn is_publish(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_PUBLISH)
    }

    /// Checks if these are the attributes for an RPC Request message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_REQUEST.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_request());
    /// ```
    pub fn is_request(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_REQUEST)
    }

    /// Checks if these are the attributes for an RPC Response message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_RESPONSE.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_response());
    /// ```
    pub fn is_response(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_RESPONSE)
    }

    /// Checks if these are the attributes for a Notification message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAttributes, UMessageType};
    ///
    /// let attribs = UAttributes {
    ///   type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
    ///   ..Default::default()
    /// };
    /// assert!(attribs.is_notification());
    /// ```
    pub fn is_notification(&self) -> bool {
        self.type_.enum_value() == Ok(UMessageType::UMESSAGE_TYPE_NOTIFICATION)
    }

    /// Checks if the message that is described by these attributes should be considered expired.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::ttl`] (time-to-live) contains a value greater than 0, but
    /// * the current system time cannot be determined, or
    /// * the message has expired according to the timestamp extracted from [`Self::id`] and the time-to-live value.
    pub fn check_expired(&self) -> Result<(), UAttributesError> {
        if let Some(ttl) = self.ttl {
            if ttl == 0 {
                return Ok(());
            }
        }
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_e| {
                UAttributesError::validation_error("Cannot determine current system time")
            })
            .and_then(|duration_since_epoch| {
                self.check_expired_for_reference(duration_since_epoch.as_millis())
            })
    }

    /// Checks if the message that is described by these attributes should be considered expired.
    ///
    /// # Arguments
    /// * `reference_time` - The reference time as a `Duration` since UNIX epoch. The check will be performed in relation to this point in time.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Self::ttl`] (time-to-live) contains a value greater than 0, but
    /// the message has expired according to the timestamp extracted from [`Self::id`], the
    /// time-to-live value and the provided reference time.
    pub fn check_expired_for_reference(
        &self,
        reference_time: u128,
    ) -> Result<(), UAttributesError> {
        let ttl = match self.ttl {
            Some(t) if t > 0 => u128::from(t),
            _ => return Ok(()),
        };

        if let Some(creation_time) = self.id.as_ref().and_then(UUID::get_time) {
            if (creation_time as u128).saturating_add(ttl) <= reference_time {
                return Err(UAttributesError::validation_error("Message has expired"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;
    use test_case::test_case;

    /// Creates a UUID for a given creation time offset.
    ///
    /// # Note
    ///
    /// For internal testing purposes only. For end-users, please use [`UUID::build()`]
    fn build_for_time_offset(offset_millis: i64) -> UUID {
        let duration_since_unix_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current system time is set to a point in time before UNIX Epoch");
        let now_as_millis_since_epoch: u64 = u64::try_from(duration_since_unix_epoch.as_millis())
            .expect("current system time is too far in the future");
        let creation_timestamp = now_as_millis_since_epoch
            .checked_add_signed(offset_millis)
            .unwrap();
        UUID::build_for_timestamp_millis(creation_timestamp)
    }

    #[test_case(None, None, false; "for message without ID nor TTL")]
    #[test_case(None, Some(0), false; "for message without ID with TTL 0")]
    #[test_case(None, Some(500), false; "for message without ID with TTL")]
    #[test_case(Some(build_for_time_offset(-1000)), None, false; "for past message without TTL")]
    #[test_case(Some(build_for_time_offset(-1000)), Some(0), false; "for past message with TTL 0")]
    #[test_case(Some(build_for_time_offset(-1000)), Some(500), true; "for past message with expired TTL")]
    #[test_case(Some(build_for_time_offset(-1000)), Some(2000), false; "for past message with non-expired TTL")]
    #[test_case(Some(build_for_time_offset(1000)), Some(2000), false; "for future message with TTL")]
    #[test_case(Some(build_for_time_offset(1000)), None, false; "for future message without TTL")]
    fn test_is_expired(id: Option<UUID>, ttl: Option<u32>, should_be_expired: bool) {
        let attributes = UAttributes {
            type_: UMessageType::UMESSAGE_TYPE_NOTIFICATION.into(),
            priority: UPriority::UPRIORITY_CS1.into(),
            id: id.into(),
            ttl,
            ..Default::default()
        };

        assert!(attributes.check_expired().is_err() == should_be_expired);
    }
}
