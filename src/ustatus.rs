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

use std::error::Error;

pub use crate::up_core_api::ucode::UCode;
pub use crate::up_core_api::ustatus::UStatus;

impl UStatus {
    /// Creates a status representing a success.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status = UStatus::ok();
    /// assert_eq!(status.code.unwrap(), UCode::OK);
    /// ```
    pub fn ok() -> Self {
        UStatus {
            code: UCode::OK.into(),
            ..Default::default()
        }
    }

    /// Creates a status representing a failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UStatus;
    ///
    /// let status = UStatus::fail("something went wrong");
    /// assert_eq!(status.message.unwrap(), "something went wrong");
    /// ```
    pub fn fail<M: Into<String>>(msg: M) -> Self {
        UStatus {
            code: UCode::UNKNOWN.into(),
            message: Some(msg.into()),
            ..Default::default()
        }
    }

    /// Creates a status representing a failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status = UStatus::fail_with_code(UCode::DATA_LOSS, "something went wrong");
    /// assert_eq!(status.code.unwrap(), UCode::DATA_LOSS);
    /// assert_eq!(status.message.unwrap(), "something went wrong");
    /// ```
    pub fn fail_with_code<M: Into<String>>(code: UCode, msg: M) -> Self {
        UStatus {
            code: code.into(),
            message: Some(msg.into()),
            ..Default::default()
        }
    }

    /// Checks if this status represents a failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UStatus;
    ///
    /// let failed_status = UStatus::fail("something went wrong");
    /// assert!(failed_status.is_failed());
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(!succeeded_status.is_failed());
    /// ```
    pub fn is_failed(&self) -> bool {
        self.get_code() != UCode::OK
    }

    /// Checks if this status represents a success.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UStatus;
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(succeeded_status.is_success());
    ///
    /// let failed_status = UStatus::fail("something went wrong");
    /// assert!(!failed_status.is_success());
    /// ```
    pub fn is_success(&self) -> bool {
        self.get_code() == UCode::OK
    }

    /// Gets this status' error message.
    ///
    /// # Returns
    ///
    /// an empty string if this instance has been created without a message,
    /// i.e. not using one of its factory functions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::UStatus;
    ///
    /// let failed_status = UStatus::fail("my error message");
    /// assert_eq!(failed_status.get_message(), "my error message");
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(succeeded_status.get_message().is_empty());
    /// ```
    pub fn get_message(&self) -> String {
        match self.message.as_ref() {
            Some(msg) => msg.to_owned(),
            None => String::default(),
        }
    }

    /// Gets this status' error code.
    ///
    /// # Returns
    ///
    /// [`UCode::UNKNOWN`] if this status has been created without providing an error code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status = UStatus::fail("my error message");
    /// assert_eq!(status.get_code(), UCode::UNKNOWN);
    ///
    /// let status_with_code = UStatus::fail_with_code(UCode::INTERNAL, "my error message");
    /// assert_eq!(status_with_code.get_code(), UCode::INTERNAL);
    /// ```
    pub fn get_code(&self) -> UCode {
        self.code.enum_value_or_default()
    }
}

impl Error for UStatus {}

#[cfg(test)]
mod tests {
    use super::*;

    use protobuf::{Enum, EnumOrUnknown};

    #[test]
    fn test_is_failed() {
        assert!(!UStatus {
            ..Default::default()
        }
        .is_failed());
        UCode::VALUES.iter().for_each(|code| {
            let ustatus = UStatus {
                code: EnumOrUnknown::from(*code),
                ..Default::default()
            };
            assert_eq!(ustatus.is_failed(), *code != UCode::OK);
        });
    }

    #[test]
    fn test_is_success() {
        assert!(UStatus {
            ..Default::default()
        }
        .is_success());
        UCode::VALUES.iter().for_each(|code| {
            let ustatus = UStatus {
                code: EnumOrUnknown::from(*code),
                ..Default::default()
            };
            assert_eq!(ustatus.is_success(), *code == UCode::OK);
        });
    }
}
