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

use protobuf::{well_known_types::any::Any, Message};

use crate::up_core_api::ucode::UCode as UCodeProto;
use crate::{up_core_api::ustatus::UStatus as UStatusProto, ProtobufMappable, SerializationError};

#[derive(Copy, Debug, Clone, PartialEq)]
#[repr(C)]
pub enum UCode {
    Ok = UCodeProto::OK as isize,
    Cancelled = UCodeProto::CANCELLED as isize,
    Unknown = UCodeProto::UNKNOWN as isize,
    InvalidArgument = UCodeProto::INVALID_ARGUMENT as isize,
    DeadlineExceeded = UCodeProto::DEADLINE_EXCEEDED as isize,
    NotFound = UCodeProto::NOT_FOUND as isize,
    AlreadyExists = UCodeProto::ALREADY_EXISTS as isize,
    PermissionDenied = UCodeProto::PERMISSION_DENIED as isize,
    Unauthenticated = UCodeProto::UNAUTHENTICATED as isize,
    ResourceExhausted = UCodeProto::RESOURCE_EXHAUSTED as isize,
    FailedPrecondition = UCodeProto::FAILED_PRECONDITION as isize,
    Aborted = UCodeProto::ABORTED as isize,
    OutOfRange = UCodeProto::OUT_OF_RANGE as isize,
    Unimplemented = UCodeProto::UNIMPLEMENTED as isize,
    Internal = UCodeProto::INTERNAL as isize,
    Unavailable = UCodeProto::UNAVAILABLE as isize,
    DataLoss = UCodeProto::DATA_LOSS as isize,
}

impl UCode {
    #[must_use]
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            x if x == UCode::Ok as i32 => Some(UCode::Ok),
            x if x == UCode::Cancelled as i32 => Some(UCode::Cancelled),
            x if x == UCode::Unknown as i32 => Some(UCode::Unknown),
            x if x == UCode::InvalidArgument as i32 => Some(UCode::InvalidArgument),
            x if x == UCode::DeadlineExceeded as i32 => Some(UCode::DeadlineExceeded),
            x if x == UCode::NotFound as i32 => Some(UCode::NotFound),
            x if x == UCode::AlreadyExists as i32 => Some(UCode::AlreadyExists),
            x if x == UCode::PermissionDenied as i32 => Some(UCode::PermissionDenied),
            x if x == UCode::Unauthenticated as i32 => Some(UCode::Unauthenticated),
            x if x == UCode::ResourceExhausted as i32 => Some(UCode::ResourceExhausted),
            x if x == UCode::FailedPrecondition as i32 => Some(UCode::FailedPrecondition),
            x if x == UCode::Aborted as i32 => Some(UCode::Aborted),
            x if x == UCode::OutOfRange as i32 => Some(UCode::OutOfRange),
            x if x == UCode::Unimplemented as i32 => Some(UCode::Unimplemented),
            x if x == UCode::Internal as i32 => Some(UCode::Internal),
            x if x == UCode::Unavailable as i32 => Some(UCode::Unavailable),
            x if x == UCode::DataLoss as i32 => Some(UCode::DataLoss),
            _ => None,
        }
    }

    #[must_use]
    pub fn value(&self) -> i32 {
        *self as i32
    }
}

impl From<UCodeProto> for UCode {
    fn from(value: UCodeProto) -> Self {
        match value {
            UCodeProto::OK => UCode::Ok,
            UCodeProto::CANCELLED => UCode::Cancelled,
            UCodeProto::UNKNOWN => UCode::Unknown,
            UCodeProto::INVALID_ARGUMENT => UCode::InvalidArgument,
            UCodeProto::DEADLINE_EXCEEDED => UCode::DeadlineExceeded,
            UCodeProto::NOT_FOUND => UCode::NotFound,
            UCodeProto::ALREADY_EXISTS => UCode::AlreadyExists,
            UCodeProto::PERMISSION_DENIED => UCode::PermissionDenied,
            UCodeProto::UNAUTHENTICATED => UCode::Unauthenticated,
            UCodeProto::RESOURCE_EXHAUSTED => UCode::ResourceExhausted,
            UCodeProto::FAILED_PRECONDITION => UCode::FailedPrecondition,
            UCodeProto::ABORTED => UCode::Aborted,
            UCodeProto::OUT_OF_RANGE => UCode::OutOfRange,
            UCodeProto::UNIMPLEMENTED => UCode::Unimplemented,
            UCodeProto::INTERNAL => UCode::Internal,
            UCodeProto::UNAVAILABLE => UCode::Unavailable,
            UCodeProto::DATA_LOSS => UCode::DataLoss,
        }
    }
}

impl From<UCode> for UCodeProto {
    fn from(value: UCode) -> Self {
        match value {
            UCode::Ok => UCodeProto::OK,
            UCode::Cancelled => UCodeProto::CANCELLED,
            UCode::Unknown => UCodeProto::UNKNOWN,
            UCode::InvalidArgument => UCodeProto::INVALID_ARGUMENT,
            UCode::DeadlineExceeded => UCodeProto::DEADLINE_EXCEEDED,
            UCode::NotFound => UCodeProto::NOT_FOUND,
            UCode::AlreadyExists => UCodeProto::ALREADY_EXISTS,
            UCode::PermissionDenied => UCodeProto::PERMISSION_DENIED,
            UCode::Unauthenticated => UCodeProto::UNAUTHENTICATED,
            UCode::ResourceExhausted => UCodeProto::RESOURCE_EXHAUSTED,
            UCode::FailedPrecondition => UCodeProto::FAILED_PRECONDITION,
            UCode::Aborted => UCodeProto::ABORTED,
            UCode::OutOfRange => UCodeProto::OUT_OF_RANGE,
            UCode::Unimplemented => UCodeProto::UNIMPLEMENTED,
            UCode::Internal => UCodeProto::INTERNAL,
            UCode::Unavailable => UCodeProto::UNAVAILABLE,
            UCode::DataLoss => UCodeProto::DATA_LOSS,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UStatus {
    code: UCode,
    message: String,
}

impl UStatus {
    /// Creates a status representing a success.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status = UStatus::ok();
    /// assert_eq!(status.get_code(), UCode::Ok);
    /// ```
    #[must_use]
    pub fn ok() -> Self {
        UStatus {
            code: UCode::Ok,
            message: String::new(),
        }
    }

    /// Creates a status representing a failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status = UStatus::fail_with_code(UCode::DataLoss, "something went wrong");
    /// assert_eq!(status.get_code(), UCode::DataLoss);
    /// assert_eq!(status.get_message(), "something went wrong");
    /// ```
    pub fn fail_with_code<M: Into<std::string::String>>(code: UCode, msg: M) -> Self {
        let msg_string = msg.into();
        UStatus {
            code,
            message: msg_string,
        }
    }

    /// Checks if this status represents a failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let failed_status = UStatus::fail_with_code(UCode::Internal, "something went wrong");
    /// assert!(failed_status.is_failed());
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(!succeeded_status.is_failed());
    /// ```
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.get_code() != UCode::Ok
    }

    /// Checks if this status represents a success.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(succeeded_status.is_success());
    ///
    /// let failed_status = UStatus::fail_with_code(UCode::Internal, "something went wrong");
    /// assert!(!failed_status.is_success());
    /// ```
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.get_code() == UCode::Ok
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
    /// use up_rust::{UCode, UStatus};
    ///
    /// let failed_status = UStatus::fail_with_code(UCode::Internal, "my error message");
    /// assert_eq!(failed_status.get_message(), "my error message");
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(succeeded_status.get_message().is_empty());
    /// ```
    #[must_use]
    pub fn get_message(&self) -> &str {
        self.message.as_str()
    }

    /// Gets this status' error code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status_with_code = UStatus::fail_with_code(UCode::Internal, "my error message");
    /// assert_eq!(status_with_code.get_code(), UCode::Internal);
    /// ```
    #[must_use]
    pub fn get_code(&self) -> UCode {
        self.code
    }
}

impl std::fmt::Display for UStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UStatus [code: {:?}, message: {}]",
            self.get_code(),
            self.get_message()
        )
    }
}

impl Error for UStatus {}

impl From<UStatusProto> for UStatus {
    fn from(value: UStatusProto) -> Self {
        let message = value.message.unwrap_or_default();
        UStatus {
            code: UCode::from(value.code.enum_value_or_default()),
            message,
        }
    }
}

impl From<&UStatus> for UStatusProto {
    fn from(value: &UStatus) -> Self {
        UStatusProto {
            code: UCodeProto::from(value.code).into(),
            message: Some(value.message.as_str().to_string()),
            ..Default::default()
        }
    }
}

impl ProtobufMappable for UStatus {
    fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
        let proto = UStatusProto::parse_from_bytes(proto)?;
        Ok(proto.into())
    }

    fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
        let any = Any::parse_from_bytes(proto)?;
        match any.unpack::<UStatusProto>() {
            Ok(Some(v)) => Ok(v.into()),
            Ok(None) => Err(SerializationError(
                "cannot unpack UStatus, type mismatch".to_string(),
            )),
            Err(e) => Err(SerializationError::from(e)),
        }
    }

    fn write_to_protobuf_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        UStatusProto::from(self)
            .write_to_bytes()
            .map_err(SerializationError::from)
    }

    fn write_to_packed_protobuf_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        Any::pack(&UStatusProto::from(self))
            .map_err(SerializationError::from)
            .and_then(|any| any.write_to_protobuf_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_UCODES: &[UCode] = &[
        UCode::Ok,
        UCode::Cancelled,
        UCode::Unknown,
        UCode::InvalidArgument,
        UCode::DeadlineExceeded,
        UCode::NotFound,
        UCode::AlreadyExists,
        UCode::PermissionDenied,
        UCode::Unauthenticated,
        UCode::ResourceExhausted,
        UCode::FailedPrecondition,
        UCode::Aborted,
        UCode::OutOfRange,
        UCode::Unimplemented,
        UCode::Internal,
        UCode::Unavailable,
        UCode::DataLoss,
    ];

    #[test]
    // [utest->req~ustatus-data-model-impl~1]
    fn test_ustatus_fail_with_code() {
        for code in ALL_UCODES {
            let ustatus = UStatus::fail_with_code(*code, "the message");
            assert!(ustatus.get_code() == *code && ustatus.get_message() == "the message");
        }
    }

    #[test]
    // [utest->req~ustatus-data-model-proto~1]
    fn test_proto_serialization() {
        let ustatus = UStatus::fail_with_code(UCode::Cancelled, "the message");
        let proto = ustatus
            .write_to_protobuf_bytes()
            .expect("failed to serialize to protobuf");
        let deserialized_status = UStatus::parse_from_protobuf_bytes(proto.as_slice())
            .expect("failed to deserialize protobuf");
        assert_eq!(ustatus, deserialized_status);
    }

    #[test]
    fn test_is_failed() {
        assert!(!UStatus::ok().is_failed());
        for code in ALL_UCODES {
            let ustatus = UStatus::fail_with_code(*code, "the message");
            assert_eq!(ustatus.is_failed(), *code != UCode::Ok);
        }
    }

    #[test]
    fn test_is_success() {
        assert!(UStatus::ok().is_success());
        for code in ALL_UCODES {
            let ustatus = UStatus::fail_with_code(*code, "the message");
            assert_eq!(ustatus.is_success(), *code == UCode::Ok);
        }
    }
}
