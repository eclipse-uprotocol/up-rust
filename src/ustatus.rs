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

use bytes::Bytes;

use crate::SerializationError;

#[derive(Copy, Debug, Clone, PartialEq)]
#[repr(C)]
pub enum UCode {
    Ok = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,
}

impl UCode {
    pub fn try_from_i32(value: i32) -> Result<Self, SerializationError> {
        match value {
            x if x == UCode::Ok as i32 => Ok(UCode::Ok),
            x if x == UCode::Cancelled as i32 => Ok(UCode::Cancelled),
            x if x == UCode::Unknown as i32 => Ok(UCode::Unknown),
            x if x == UCode::InvalidArgument as i32 => Ok(UCode::InvalidArgument),
            x if x == UCode::DeadlineExceeded as i32 => Ok(UCode::DeadlineExceeded),
            x if x == UCode::NotFound as i32 => Ok(UCode::NotFound),
            x if x == UCode::AlreadyExists as i32 => Ok(UCode::AlreadyExists),
            x if x == UCode::PermissionDenied as i32 => Ok(UCode::PermissionDenied),
            x if x == UCode::ResourceExhausted as i32 => Ok(UCode::ResourceExhausted),
            x if x == UCode::FailedPrecondition as i32 => Ok(UCode::FailedPrecondition),
            x if x == UCode::Aborted as i32 => Ok(UCode::Aborted),
            x if x == UCode::OutOfRange as i32 => Ok(UCode::OutOfRange),
            x if x == UCode::Unimplemented as i32 => Ok(UCode::Unimplemented),
            x if x == UCode::Internal as i32 => Ok(UCode::Internal),
            x if x == UCode::Unavailable as i32 => Ok(UCode::Unavailable),
            x if x == UCode::DataLoss as i32 => Ok(UCode::DataLoss),
            x if x == UCode::Unauthenticated as i32 => Ok(UCode::Unauthenticated),
            _ => Err(SerializationError::new(format!(
                "unknown UCode value: {}",
                value
            ))),
        }
    }

    #[must_use]
    pub fn value(&self) -> i32 {
        *self as i32
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UAny {
    type_url: String,
    value: Bytes,
}

impl UAny {
    pub fn new<V: Into<Bytes>>(type_url: String, value: V) -> Self {
        UAny {
            type_url,
            value: value.into(),
        }
    }

    pub fn get_type_url(&self) -> &str {
        &self.type_url
    }

    pub fn get_value(&self) -> &[u8] {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct UStatus {
    code: UCode,
    message: Option<String>,
    details: Vec<UAny>,
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
            message: None,
            details: vec![],
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
    /// assert_eq!(status.get_message().unwrap(), "something went wrong");
    /// ```
    pub fn fail_with_code<M: Into<std::string::String>>(code: UCode, msg: M) -> Self {
        Self::new(code, Some(msg), None)
    }

    /// Creates a status from a code and an optional message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let status = UStatus::new(UCode::DataLoss, Some("something went wrong"), None);
    /// assert_eq!(status.get_code(), UCode::DataLoss);
    /// assert_eq!(status.get_message().unwrap(), "something went wrong");
    /// assert!(status.get_details().is_empty());
    /// ```
    pub fn new<M: Into<std::string::String>>(
        code: UCode,
        msg: Option<M>,
        details: Option<Vec<UAny>>,
    ) -> Self {
        UStatus {
            code,
            message: msg.map(|m| m.into()),
            details: details.unwrap_or_default(),
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
    /// an empty string if this instance has been created without a message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let failed_status = UStatus::fail_with_code(UCode::Internal, "my error message");
    /// assert_eq!(failed_status.get_message(), Some("my error message"));
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert!(succeeded_status.get_message().is_none());
    /// ```
    #[must_use]
    pub fn get_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Gets this status' error message or a default value if none is set.
    ///
    /// # Arguments
    ///
    /// * `default` - The default value to return if this status has no message.
    ///
    /// # Returns
    ///
    /// the error message if set, otherwise the provided default value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UCode, UStatus};
    ///
    /// let failed_status = UStatus::fail_with_code(UCode::Internal, "my error message");
    /// assert_eq!(failed_status.get_message_or_default("default"), "my error message");
    ///
    /// let succeeded_status = UStatus::ok();
    /// assert_eq!(succeeded_status.get_message_or_default("default"), "default");
    /// ```
    #[must_use]
    pub fn get_message_or_default<'a>(&'a self, default: &'a str) -> &'a str {
        self.get_message().unwrap_or(default)
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

    /// Gets this status' details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{UAny, UCode, UStatus};
    ///
    /// let details = vec![UAny::new("type.googleapis.com/google.protobuf.StringValue".to_string(), b"the string".as_ref())];
    /// let status_with_details = UStatus::new(UCode::Internal, Some("my error message"), Some(details.clone()));
    /// assert_eq!(status_with_details.get_details(), details.as_slice());
    ///
    /// let status_without_details = UStatus::fail_with_code(UCode::Internal, "my error message");
    /// assert!(status_without_details.get_details().is_empty());
    /// ```
    #[must_use]
    pub fn get_details(&self) -> &[UAny] {
        &self.details
    }
}

impl std::fmt::Display for UStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("UStatus [");
        s.push_str(&format!("code: {:?}", self.get_code()));
        if let Some(msg) = self.get_message() {
            s.push_str(&format!(", message: {msg}"));
        }
        if !self.get_details().is_empty() {
            s.push_str(&format!(", details: {:?}", self.get_details()));
        }
        s.push(']');
        write!(f, "{}", s)
    }
}

impl Error for UStatus {}

#[cfg(feature = "up-core-types")]
mod core_types_support {
    use protobuf::{well_known_types::any::Any, Message};

    use super::*;

    use crate::up_core_api::{ucode::UCode as UCodeProto, ustatus::UStatus as UStatusProto};
    use crate::{ProtobufMappable, SerializationError};

    impl From<UCodeProto> for UCode {
        fn from(ucode_proto: UCodeProto) -> Self {
            match ucode_proto {
                UCodeProto::OK => UCode::Ok,
                UCodeProto::CANCELLED => UCode::Cancelled,
                UCodeProto::UNKNOWN => UCode::Unknown,
                UCodeProto::INVALID_ARGUMENT => UCode::InvalidArgument,
                UCodeProto::DEADLINE_EXCEEDED => UCode::DeadlineExceeded,
                UCodeProto::NOT_FOUND => UCode::NotFound,
                UCodeProto::ALREADY_EXISTS => UCode::AlreadyExists,
                UCodeProto::PERMISSION_DENIED => UCode::PermissionDenied,
                UCodeProto::RESOURCE_EXHAUSTED => UCode::ResourceExhausted,
                UCodeProto::FAILED_PRECONDITION => UCode::FailedPrecondition,
                UCodeProto::ABORTED => UCode::Aborted,
                UCodeProto::OUT_OF_RANGE => UCode::OutOfRange,
                UCodeProto::UNIMPLEMENTED => UCode::Unimplemented,
                UCodeProto::INTERNAL => UCode::Internal,
                UCodeProto::UNAVAILABLE => UCode::Unavailable,
                UCodeProto::DATA_LOSS => UCode::DataLoss,
                UCodeProto::UNAUTHENTICATED => UCode::Unauthenticated,
            }
        }
    }

    impl From<UCode> for UCodeProto {
        fn from(ucode: UCode) -> Self {
            match ucode {
                UCode::Ok => UCodeProto::OK,
                UCode::Cancelled => UCodeProto::CANCELLED,
                UCode::Unknown => UCodeProto::UNKNOWN,
                UCode::InvalidArgument => UCodeProto::INVALID_ARGUMENT,
                UCode::DeadlineExceeded => UCodeProto::DEADLINE_EXCEEDED,
                UCode::NotFound => UCodeProto::NOT_FOUND,
                UCode::AlreadyExists => UCodeProto::ALREADY_EXISTS,
                UCode::PermissionDenied => UCodeProto::PERMISSION_DENIED,
                UCode::ResourceExhausted => UCodeProto::RESOURCE_EXHAUSTED,
                UCode::FailedPrecondition => UCodeProto::FAILED_PRECONDITION,
                UCode::Aborted => UCodeProto::ABORTED,
                UCode::OutOfRange => UCodeProto::OUT_OF_RANGE,
                UCode::Unimplemented => UCodeProto::UNIMPLEMENTED,
                UCode::Internal => UCodeProto::INTERNAL,
                UCode::Unavailable => UCodeProto::UNAVAILABLE,
                UCode::DataLoss => UCodeProto::DATA_LOSS,
                UCode::Unauthenticated => UCodeProto::UNAUTHENTICATED,
            }
        }
    }

    impl From<Any> for UAny {
        fn from(value: Any) -> Self {
            UAny {
                type_url: value.type_url,
                value: value.value.into(),
            }
        }
    }

    impl From<&UAny> for Any {
        fn from(value: &UAny) -> Self {
            Any {
                type_url: value.type_url.clone(),
                value: value.value.clone().into(),
                ..Default::default()
            }
        }
    }

    impl TryFrom<UStatusProto> for UStatus {
        type Error = SerializationError;

        fn try_from(status_proto: UStatusProto) -> Result<Self, Self::Error> {
            // an unsupported UCode value is considered a serialization error because we cannot
            // simply map it to OK (value 0) or UNKNOWN (value 2) without risking data loss
            let code = status_proto
                .code
                .enum_value()
                .map_err(|e| SerializationError::new(format!("unsupported UCode {e}")))
                .map(UCode::from)?;
            let details = status_proto
                .details
                .into_iter()
                .map(UAny::from)
                .collect::<Vec<_>>();
            Ok(UStatus::new(code, status_proto.message, details.into()))
        }
    }

    impl From<&UStatus> for UStatusProto {
        fn from(value: &UStatus) -> Self {
            UStatusProto {
                code: UCodeProto::from(value.get_code()).into(),
                message: value.get_message().map(|m| m.to_string()),
                details: value.get_details().iter().map(Any::from).collect(),
                ..Default::default()
            }
        }
    }

    impl ProtobufMappable for UStatus {
        fn parse_from_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
            let proto = UStatusProto::parse_from_bytes(proto)?;
            UStatus::try_from(proto)
        }

        fn parse_from_packed_protobuf_bytes(proto: &[u8]) -> Result<Self, SerializationError> {
            let any = Any::parse_from_bytes(proto)?;
            match any.unpack::<UStatusProto>() {
                Ok(Some(v)) => UStatus::try_from(v),
                Ok(None) => Err(SerializationError::new(
                    "cannot unpack UStatus, type mismatch",
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
            assert!(ustatus.get_code() == *code && ustatus.get_message() == Some("the message"));
        }
    }

    #[test]
    // [utest->req~ustatus-data-model-proto~1]
    #[cfg(feature = "up-core-types")]
    fn test_proto_serialization() {
        use crate::ProtobufMappable;

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
