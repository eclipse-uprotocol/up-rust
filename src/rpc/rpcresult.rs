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

use crate::uprotocol::{UCode, UStatus};

/// A wrapper for RPC stub calls.
///
/// This enum represents the result of an RPC call, encapsulating either a successful result of a specific type
/// or a failure represented by a `Status` object.
///
/// The successful result type is represented by the type parameter `T`.
#[derive(Debug, Clone)]
pub enum RpcResult<T> {
    Success(T),
    Failure(UStatus),
}

impl<T> RpcResult<T> {
    /// Returns `true` if the `RpcResult` is a `Success` variant, otherwise returns `false`.
    pub fn is_success(&self) -> bool {
        matches!(self, RpcResult::Success(_))
    }

    /// Returns `true` if the `RpcResult` is a `Failure` variant, otherwise returns `false`.
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Unwraps the value if the `RpcResult` is a `Success` variant, otherwise invokes the provided closure `op` and returns its result.
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, op: F) -> T {
        match self {
            RpcResult::Success(val) => val,
            RpcResult::Failure(_) => op(),
        }
    }

    /// Transforms the inner value of a `Success` variant using the provided function `func`.
    /// If the `RpcResult` is a `Failure`, the error status is propagated unchanged.
    pub fn map<U, F>(self, func: F) -> RpcResult<U>
    where
        F: FnOnce(T) -> Result<U, String>,
    {
        match self {
            RpcResult::Success(value) => match func(value) {
                Ok(val) => RpcResult::Success(val),
                Err(e) => RpcResult::Failure(UStatus::fail_with_code(UCode::Unknown, &e)),
            },
            RpcResult::Failure(status) => RpcResult::Failure(status),
        }
    }

    /// Chains a sequence of operations where each operation might fail. If the `RpcResult` is
    /// a `Success`, applies the provided function `func` to the inner value and returns the result.
    /// If the `RpcResult` is a `Failure`, the error status is propagated unchanged.
    /// (was 'flatMap()' in the Java SDK)
    pub fn and_then<U, F>(self, func: F) -> RpcResult<U>
    where
        F: FnOnce(T) -> RpcResult<U>,
    {
        match self {
            RpcResult::Success(value) => func(value),
            RpcResult::Failure(status) => RpcResult::Failure(status),
        }
    }

    /// Validates the inner value of a `Success` variant using the provided predicate function.
    /// If the predicate returns `true`, the original `RpcResult` is returned unchanged.
    /// If it returns `false`, or if the `RpcResult` is a `Failure`,
    /// a `Failure` variant with a `FailedPrecondition` status is returned.
    /// (was 'filter()' in the Java SDK)
    #[must_use]
    pub fn validate<F>(self, predicate: F) -> Self
    where
        F: FnOnce(&T) -> bool,
    {
        match self {
            RpcResult::Success(value) if predicate(&value) => RpcResult::Success(value),
            RpcResult::Success(_) => RpcResult::Failure(UStatus::fail_with_code(
                UCode::FailedPrecondition,
                "Validation failed",
            )),
            failure @ RpcResult::Failure(_) => failure,
        }
    }

    /// Returns the `Code` of the `Failure` variant if present, or `None` if the `RpcResult` is a `Success`.
    pub fn failure_value(&self) -> Option<UCode> {
        match self {
            RpcResult::Failure(status) => {
                Some(UCode::try_from(status.code).unwrap_or(UCode::Unknown))
            }
            RpcResult::Success(_) => None,
        }
    }

    /// Returns a reference to the value inside the `Success` variant if present, or `None` if the `RpcResult` is a `Failure`.
    pub fn success_value(&self) -> Option<&T> {
        match self {
            RpcResult::Success(value) => Some(value),
            RpcResult::Failure(_) => None,
        }
    }

    /// Constructs a new `RpcResult` instance representing a successful operation containing the given value.
    pub fn success(value: T) -> Self {
        RpcResult::Success(value)
    }

    /// Attempts to create an `RpcResult` from another `RpcResult` with a different inner type.
    /// This function is specifically designed to handle the failure cases of `RpcResult`.
    ///
    /// # Parameters
    ///
    /// - `failure`: An `RpcResult` of any type. The function processes the failure variant of this `RpcResult`.
    ///
    /// # Returns
    ///
    /// If the provided `RpcResult` is a failure variant, it returns `Ok` with an `RpcResult::Failure` containing the same status.
    /// Otherwise, it returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided `RpcResult` is not a failure variant. The error is a static string,
    /// `"Expected a Failure variant!"`, indicating that the function expected a failure variant but found a success variant instead.
    ///
    /// This is typically used in contexts where only the failure cases of an `RpcResult` are relevant,
    /// and the presence of a success variant indicates a logical error or unexpected state.
    ///
    pub fn from_failure<U>(failure: RpcResult<U>) -> Result<Self, &'static str> {
        match failure {
            RpcResult::Failure(status) => Ok(RpcResult::Failure(status)),
            RpcResult::Success(_) => Err("Expected a Failure variant!"),
        }
    }

    /// Creates a new `RpcResult::Failure` variant using the provided message and error details.
    /// The combined message is used to construct an internal status representation.
    ///
    /// # Arguments
    ///
    /// * `message`: A descriptive message for the failure.
    /// * `_e`: Additional details or context about the error.
    pub fn failure_with_message(message: &str, e: &str) -> Self {
        let error_message = format!("{message}: {e}");
        let error = Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            error_message,
        ));

        let status = UStatus::fail(&error.to_string());
        RpcResult::Failure(status)
    }

    /// Creates a new `RpcResult::Failure` variant with the specified status code and message.
    ///
    /// # Arguments
    ///
    /// * `code`: The status code indicating the nature of the failure.
    /// * `message`: A descriptive message providing details about the failure.
    pub fn failure(code: UCode, message: &str) -> Self {
        let status = UStatus::fail_with_code(code, message);
        RpcResult::Failure(status)
    }

    /// Flattens a nested `RpcResult` structure into a single `RpcResult`.
    ///
    /// If the outer `RpcResult` is a success variant containing another `RpcResult`,
    /// the inner result is returned. If the outer or inner `RpcResult` is a failure,
    /// a failure is returned.
    ///
    /// # Arguments
    ///
    /// * `result`: The nested `RpcResult` to flatten.
    pub fn flatten<U>(result: RpcResult<RpcResult<U>>) -> RpcResult<U> {
        match result {
            RpcResult::Success(inner_result) => inner_result,
            RpcResult::Failure(status) => RpcResult::Failure(status),
        }
    }
}

impl<T> std::fmt::Display for RpcResult<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcResult::Success(value) => write!(f, "Success({value})"),
            RpcResult::Failure(status) => write!(
                f,
                "Failure(code: {}\nmessage: \"{}\"\n)",
                status.code,
                status.message()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_default() -> i32 {
        5
    }

    fn fun_that_returns_an_error_for_map(x: i32) -> Result<i32, String> {
        Err(format!("{} went boom", x))
    }

    fn fun_that_returns_a_failure_for_and_then(x: i32) -> RpcResult<i32> {
        RpcResult::Failure(UStatus::fail(&format!("{} went boom", x)))
    }

    fn multiply_by_2(x: i32) -> Result<RpcResult<i32>, String> {
        Ok(RpcResult::success(x * 2))
    }

    #[test]
    fn test_is_success_on_success() {
        let result = RpcResult::success(2);
        assert!(result.is_success());
    }

    #[test]
    fn test_is_success_on_failure() {
        let result = RpcResult::<i32>::failure(UCode::InvalidArgument, "boom");
        assert!(!result.is_success());
    }

    #[test]
    fn test_is_failure_on_success() {
        let result = RpcResult::<i32>::success(2);
        assert!(!result.is_failure());
    }

    #[test]
    fn test_is_failure_on_failure() {
        let result = RpcResult::<i32>::failure(UCode::InvalidArgument, "boom");
        assert!(result.is_failure());
    }

    #[test]
    fn test_unwrap_or_else_on_success() {
        let result = RpcResult::success(2);
        assert_eq!(2, result.unwrap_or_else(get_default));
    }
    #[test]

    fn test_unwrap_or_else_on_failure() {
        let result: RpcResult<i32> = RpcResult::failure(UCode::InvalidArgument, "boom");
        assert_eq!(get_default(), result.unwrap_or_else(get_default));
    }

    #[test]
    fn test_unwrap_or_else_on_success_value() {
        let result = RpcResult::success(2);
        assert_eq!(2, result.unwrap_or_else(|| 5));
    }

    #[test]
    fn test_unwrap_or_else_on_failure_value() {
        let result: RpcResult<i32> = RpcResult::failure(UCode::InvalidArgument, "boom");
        assert_eq!(5, result.unwrap_or_else(|| 5));
    }

    #[test]
    fn test_success_value_on_success() {
        let result = RpcResult::success(2);
        assert_eq!(Some(&2), result.success_value());
    }

    #[test]
    fn test_success_value_on_failure() {
        let result: RpcResult<i32> = RpcResult::failure(UCode::InvalidArgument, "boom");
        assert!(result.failure_value().is_some());
    }

    #[test]
    fn test_failure_value_on_success() {
        let result = RpcResult::success(2);
        assert!(result.failure_value().is_none());
    }

    #[test]
    fn test_failure_value_on_failure() {
        let result: RpcResult<i32> = RpcResult::failure(UCode::InvalidArgument, "boom");
        assert_eq!(Some(UCode::InvalidArgument), result.failure_value());
    }

    #[test]
    fn test_map_on_success() {
        let result = RpcResult::success(2);
        let mapped = result.map(|x| Ok(x * 2));
        assert!(mapped.is_success());
        assert_eq!(Some(4), mapped.success_value().copied());
    }

    #[test]
    fn test_map_success_when_function_returns_error() {
        let result: RpcResult<i32> = RpcResult::Success(2);
        let mapped: RpcResult<i32> = result.map(fun_that_returns_an_error_for_map);

        match mapped {
            RpcResult::Success(_) => panic!("Expected failure, but got success."),
            RpcResult::Failure(status) => {
                assert_eq!(UCode::Unknown as i32, status.code);
                assert_eq!("2 went boom", status.message());
            }
        }
    }

    #[test]
    fn test_map_on_failure() {
        let result: RpcResult<i32> =
            RpcResult::Failure(UStatus::fail_with_code(UCode::InvalidArgument, "boom"));
        let mapped: RpcResult<i32> = result.map(|x| Ok(x * 2));

        match mapped {
            RpcResult::Success(_) => panic!("Expected failure, but got success."),
            RpcResult::Failure(status) => {
                assert_eq!(UCode::InvalidArgument as i32, status.code);
                assert_eq!("boom", status.message());
            }
        }
    }

    #[test]
    fn test_and_then_success_when_function_returns_error() {
        let result = RpcResult::Success(2);
        let flat_mapped = result.and_then(fun_that_returns_a_failure_for_and_then);

        match flat_mapped {
            RpcResult::Success(_) => panic!("Expected a failure, but got a success!"),
            RpcResult::Failure(status) => {
                assert_eq!(UCode::Unknown as i32, status.code);
            }
        }
    }

    #[test]
    fn test_and_then_on_success() {
        let result = RpcResult::Success(2);
        let and_then_mapped = result.and_then(|x| RpcResult::Success(x * 2));

        match and_then_mapped {
            RpcResult::Success(val) => assert_eq!(4, val),
            RpcResult::Failure(_) => panic!("Expected a success, but got a failure!"),
        }
    }

    #[test]
    fn test_and_then_on_failure() {
        let result = RpcResult::Failure(UStatus::fail(""));
        let and_then_mapped = result.and_then(|x: i32| RpcResult::Success(x * 2));

        match and_then_mapped {
            RpcResult::Success(_) => panic!("Expected a failure, but got a success!"),
            RpcResult::Failure(status) => {
                assert_eq!(UCode::Unknown as i32, status.code);
            }
        }
    }

    #[test]
    fn test_filter_on_success_that_fails() {
        let result = RpcResult::Success(2);
        let filter_result = result.validate(|&i| i > 5);

        match filter_result {
            RpcResult::Success(_) => panic!("Expected a failure, but got a success!"),
            RpcResult::Failure(status) => {
                assert_eq!(UCode::FailedPrecondition as i32, status.code);
            }
        }
    }

    #[test]
    fn test_filter_on_success_that_succeeds() {
        let result = RpcResult::Success(2);
        let filter_result = result.validate(|&i| i < 5);

        match filter_result {
            RpcResult::Success(val) => assert_eq!(2, val),
            RpcResult::Failure(_) => panic!("Expected a success, but got a failure!"),
        }
    }

    // Omitted, as catching panics in validate() isn't something we want to do in Rust
    // #[test]
    // fn test_filter_on_success_when_function_throws_exception() {
    // }

    #[test]
    fn test_validate_on_failure() {
        let result: RpcResult<i32> = RpcResult::failure(UCode::InvalidArgument, "boom");
        let validated_result = result.validate(|&i| i > 5);

        match validated_result {
            RpcResult::Failure(status) => {
                assert_eq!(UCode::InvalidArgument as i32, status.code);
                assert_eq!("boom", status.message());
            }
            _ => panic!("Expected Failure but found Success"),
        }
    }

    #[test]
    fn test_flatten_on_success() {
        let result = RpcResult::success(2);
        let mapped: RpcResult<RpcResult<i32>> = result.map(|x| Ok(RpcResult::success(x * 2)));
        let flattened: RpcResult<i32> = RpcResult::<RpcResult<i32>>::flatten(mapped);

        match flattened {
            RpcResult::Success(value) => assert_eq!(4, value),
            _ => panic!("Expected Success but found Failure"),
        }
    }

    #[test]
    fn test_flatten_on_success_with_function_that_fails() {
        let result = RpcResult::success(2);
        let mapped = result.map(fun_that_returns_an_error_for_map);

        match mapped {
            RpcResult::Failure(status) => {
                assert_eq!(UCode::Unknown as i32, status.code);
                assert_eq!("2 went boom", status.message());
            }
            _ => panic!("Expected Failure but found Success"),
        }
    }

    #[test]
    fn test_flatten_on_failure() {
        let result: RpcResult<i32> =
            RpcResult::Failure(UStatus::fail_with_code(UCode::InvalidArgument, "boom"));
        let mapped: RpcResult<RpcResult<i32>> = result.map(multiply_by_2);
        let flattened = RpcResult::<RpcResult<i32>>::flatten(mapped);

        match flattened {
            RpcResult::Failure(status) => {
                assert_eq!(UCode::InvalidArgument as i32, status.code);
                assert_eq!("boom", status.message());
            }
            _ => panic!("Expected Failure but found Success"),
        }
    }

    #[test]
    fn test_to_string_success() {
        let result: RpcResult<i32> = RpcResult::success(2);
        assert_eq!("Success(2)", result.to_string());
    }

    #[test]
    fn test_to_string_failure() {
        let result: RpcResult<i32> =
            RpcResult::Failure(UStatus::fail_with_code(UCode::InvalidArgument, "boom"));
        let expected_output = format!(
            "Failure(code: {}\nmessage: \"{}\"\n)",
            UCode::InvalidArgument as i32,
            "boom"
        );
        assert_eq!(expected_output, result.to_string());
    }
}
