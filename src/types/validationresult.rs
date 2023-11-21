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

use std::fmt;

use crate::proto::{Code, Status};

///  ValidationResult of success or failure, wrapping the value of a UStatus.
#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    Success,
    Failure(String),
}

impl From<ValidationResult> for Status {
    fn from(value: ValidationResult) -> Self {
        match value {
            ValidationResult::Success => Status {
                code: Code::Ok.into(),
                ..Default::default()
            },
            ValidationResult::Failure(message) => Status {
                code: Code::InvalidArgument.into(),
                message,
                ..Default::default()
            },
        }
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationResult::Success => write!(f, "ValidationResult.Success"),
            ValidationResult::Failure(message) => {
                write!(f, "ValidationResult.Failure(message = '{}')", message)
            }
        }
    }
}

impl ValidationResult {
    // Returns true if ValidationResult is Success
    pub fn is_success(&self) -> bool {
        matches!(self, ValidationResult::Success)
    }

    // Returns true if ValidationResult is Failure
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    // Returns the message of the ValidationResult, if available
    pub fn get_message(&self) -> String {
        match self {
            ValidationResult::Success => String::from(""),
            ValidationResult::Failure(message) => message.clone(),
        }
    }

    pub fn success() -> Self {
        ValidationResult::Success
    }

    pub fn failure(message: &str) -> Self {
        ValidationResult::Failure(message.to_string())
    }

    pub fn to_status(&self) -> Status {
        match self {
            Self::Success => Status {
                code: Code::Ok.into(),
                ..Default::default()
            },
            Self::Failure(error) => Status {
                code: Code::InvalidArgument.into(),
                message: *error,
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_validation_result_to_string() {
        let success = ValidationResult::success();
        assert_eq!("ValidationResult.Success", success.to_string());
    }

    #[test]
    fn test_failure_validation_result_to_string() {
        let failure = ValidationResult::failure("boom");
        assert_eq!(
            "ValidationResult.Failure(message = 'boom')",
            failure.to_string()
        );
    }

    #[test]
    fn test_success_validation_result_is_success() {
        let success = ValidationResult::success();
        assert!(success.is_success());
    }

    #[test]
    fn test_failure_validation_result_is_success() {
        let failure = ValidationResult::failure("boom");
        assert!(!failure.is_success());
    }

    #[test]
    fn test_success_validation_result_message() {
        let success = ValidationResult::success();
        assert!(success.get_message().is_empty());
    }

    #[test]
    fn test_failure_validation_result_message() {
        let failure = ValidationResult::failure("boom");
        assert_eq!("boom", failure.get_message());
    }

    #[test]
    fn test_success_validation_result_to_status() {
        let success = ValidationResult::success();
        let status_success = Status {
            code: Code::Ok.into(),
            ..Default::default()
        };
        assert_eq!(status_success, Status::from(success));
    }

    #[test]
    fn test_failure_validation_result_to_status() {
        let failure = ValidationResult::failure("boom");
        let status_failure = Status {
            code: Code::InvalidArgument.into(),
            message: "boom".into(),
            ..Default::default()
        };
        assert_eq!(status_failure, Status::from(failure));
    }
}
