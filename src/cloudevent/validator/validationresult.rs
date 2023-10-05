use std::fmt;

use crate::transport::datamodel::ustatus::{UCode, UStatus};

#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    Success,
    Failure(String),
}

impl From<ValidationResult> for UStatus {
    fn from(value: ValidationResult) -> Self {
        match value {
            ValidationResult::Success => UStatus::ok(),
            ValidationResult::Failure(message) => {
                UStatus::fail_with_msg_and_reason(&message, UCode::InvalidArgument)
            }
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

    pub fn to_status(&self) -> UStatus {
        match self {
            Self::Success => UStatus::ok(),
            Self::Failure(error) => {
                UStatus::fail_with_msg_and_reason(error, UCode::InvalidArgument)
            }
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
        let status_success = UStatus::ok();
        assert_eq!(status_success, UStatus::from(success));
    }

    #[test]
    fn test_failure_validation_result_to_status() {
        let failure = ValidationResult::failure("boom");
        let status_failure = UStatus::fail_with_msg_and_reason("boom", UCode::InvalidArgument);
        assert_eq!(status_failure, UStatus::from(failure));
    }
}
