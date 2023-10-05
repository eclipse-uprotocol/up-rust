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

use crate::proto::Status as ProtoStatus;

/// Enum to contain the status code that we map to `google.rpc.Code`.
///
/// Please refer to [Google's RPC Code Documentation](https://github.com/googleapis/googleapis/blob/master/google/rpc/code.proto)
/// for documentation on the codes listed below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    Unspecified = -1,
}

impl From<i32> for UCode {
    // This differs slightly from Java SDK, as in Rust the from() function always returns
    // a valid value. So we return UCode::Unspecified in case no valid code is provided, where
    // in Java a None is returned.
    // Alternatively, this could be a try_from() trait implementation, but for a from-integer
    // conversion this approach seems more idiomatic.
    fn from(code: i32) -> Self {
        match code {
            0 => UCode::Ok,
            1 => UCode::Cancelled,
            2 => UCode::Unknown,
            3 => UCode::InvalidArgument,
            4 => UCode::DeadlineExceeded,
            5 => UCode::NotFound,
            6 => UCode::AlreadyExists,
            7 => UCode::PermissionDenied,
            8 => UCode::ResourceExhausted,
            9 => UCode::FailedPrecondition,
            10 => UCode::Aborted,
            11 => UCode::OutOfRange,
            12 => UCode::Unimplemented,
            13 => UCode::Internal,
            14 => UCode::Unavailable,
            15 => UCode::DataLoss,
            16 => UCode::Unauthenticated,
            _ => UCode::Unspecified,
        }
    }
}

const OK: &str = "ok";
const FAILED: &str = "failed";

/// UProtocol general Ack requestRpcMessage for all operations.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum UStatus {
    Ok(OkStatus),
    Fail(FailStatus),
}

impl UStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, UStatus::Ok(_))
    }

    pub fn is_failed(&self) -> bool {
        !self.is_success()
    }

    pub fn msg(&self) -> String {
        match self {
            UStatus::Ok(ok_status) => ok_status.id.clone(),
            UStatus::Fail(fail_status) => fail_status.fail_msg.clone(),
        }
    }

    pub fn code(&self) -> UCode {
        match self {
            UStatus::Ok(_) => UCode::Ok,
            UStatus::Fail(fail_status) => fail_status.code,
        }
    }

    pub fn code_as_int(&self) -> i32 {
        match self {
            UStatus::Ok(_) => UCode::Ok as i32,
            UStatus::Fail(fail_status) => fail_status.code as i32,
        }
    }
}

impl fmt::Display for UStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UMessage {} {}",
            if self.is_success() { OK } else { FAILED },
            self.msg()
        )
    }
}

impl From<ProtoStatus> for UStatus {
    fn from(source_status: ProtoStatus) -> Self {
        // TODO this is omitting details() for the moment

        let code = UCode::from(source_status.code);
        match code {
            UCode::Ok => UStatus::ok_with_id(&source_status.message),
            _ => UStatus::fail_with_msg_and_reason(&source_status.message, code),
        }
    }
}

impl From<UStatus> for ProtoStatus {
    fn from(source_status: UStatus) -> Self {
        // TODO this is omitting details() for the moment

        ProtoStatus {
            code: source_status.code_as_int(),
            message: source_status.msg().to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// A successful UStatus
pub struct OkStatus {
    pub id: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// A failed UStatus
pub struct FailStatus {
    pub fail_msg: String,
    pub code: UCode,
}

// Associated functions to construct UStatus
impl UStatus {
    pub fn ok() -> Self {
        UStatus::Ok(OkStatus { id: OK.to_string() })
    }

    pub fn ok_with_id(id: &str) -> Self {
        UStatus::Ok(OkStatus { id: id.to_string() })
    }

    pub fn fail() -> Self {
        UStatus::Fail(FailStatus {
            fail_msg: FAILED.to_string(),
            code: UCode::Unknown,
        })
    }

    pub fn fail_with_msg(msg: &str) -> Self {
        UStatus::Fail(FailStatus {
            fail_msg: msg.to_string(),
            code: UCode::Unknown,
        })
    }

    pub fn fail_with_msg_and_reason(msg: &str, failure_code: UCode) -> Self {
        UStatus::Fail(FailStatus {
            fail_msg: msg.to_string(),
            code: failure_code,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    // Utility function to calculate hash value for testing
    fn hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn test_hash_code_equals() {
        let u_status_1 = UStatus::ok();
        let u_status_2 = UStatus::ok();
        let u_status_3 = UStatus::fail();

        assert_eq!(hash(&u_status_1), hash(&u_status_2));
        assert_ne!(hash(&u_status_1), hash(&u_status_3));
    }

    #[test]
    fn test_hash_code_equals_ok_scenarios() {
        let mut statuses = HashSet::new();

        statuses.insert(UStatus::ok());
        statuses.insert(UStatus::ok());
        statuses.insert(UStatus::ok_with_id("ackId"));
        statuses.insert(UStatus::fail());

        assert_eq!(3, statuses.len());
    }

    #[test]
    fn test_hash_code_equals_fail_scenarios() {
        let mut statuses = HashSet::new();

        statuses.insert(UStatus::fail());
        statuses.insert(UStatus::fail());

        statuses.insert(UStatus::fail_with_msg("boom"));
        statuses.insert(UStatus::fail_with_msg_and_reason("boom", UCode::Unknown));

        statuses.insert(UStatus::fail_with_msg("bam"));
        statuses.insert(UStatus::fail_with_msg_and_reason(
            "boom",
            UCode::Unspecified,
        ));
        statuses.insert(UStatus::fail_with_msg_and_reason(
            "boom",
            UCode::InvalidArgument,
        ));

        assert_eq!(5, statuses.len());
    }

    #[test]
    fn test_to_string_for_ok_status() {
        let ok = UStatus::ok();
        assert_eq!("UMessage ok ok", format!("{}", ok));
    }

    #[test]
    fn test_to_string_for_ok_status_with_id() {
        let ok = UStatus::ok_with_id("boo");
        assert_eq!("UMessage ok boo", format!("{}", ok));
    }

    #[test]
    fn test_to_string_for_failed_status() {
        let failed = UStatus::fail();
        assert_eq!("UMessage failed failed", format!("{}", failed));
    }

    #[test]
    fn test_to_string_for_failed_status_with_message() {
        let failed = UStatus::fail_with_msg("boom");
        assert_eq!("UMessage failed boom", format!("{}", failed));
    }

    #[test]
    fn test_to_string_for_failed_status_with_message_and_failure_reason() {
        let failed = UStatus::fail_with_msg_and_reason("boom", UCode::InvalidArgument);
        assert_eq!("UMessage failed boom", format!("{}", failed));
    }

    #[test]
    fn test_to_string_for_failed_status_with_message_and_code() {
        let failed = UStatus::fail_with_msg_and_reason("boom", UCode::InvalidArgument);
        assert_eq!("UMessage failed boom", format!("{}", failed));
    }

    #[test]
    fn create_ok_status() {
        let ok = UStatus::ok();
        assert!(ok.is_success());
        assert!(!ok.is_failed());
        assert_eq!("ok", ok.msg());
        assert_eq!(UCode::Ok as i32, ok.code_as_int());
    }

    #[test]
    fn create_ok_status_with_id() {
        let ok = UStatus::ok_with_id("boo");
        assert!(ok.is_success());
        assert!(!ok.is_failed());
        assert_eq!("boo", ok.msg());
        assert_eq!(UCode::Ok as i32, ok.code_as_int());
    }

    #[test]
    fn create_failed_status() {
        let failed = UStatus::fail();
        assert!(!failed.is_success());
        assert!(failed.is_failed());
        assert_eq!("failed", failed.msg());
        assert_eq!(UCode::Unknown as i32, failed.code_as_int());
    }

    #[test]
    fn create_failed_status_with_message() {
        let failed = UStatus::fail_with_msg("boom");
        assert!(!failed.is_success());
        assert!(failed.is_failed());
        assert_eq!("boom", failed.msg());
        assert_eq!(UCode::Unknown as i32, failed.code_as_int());
    }

    #[test]
    fn create_failed_status_with_message_and_failure_reason() {
        let failed = UStatus::fail_with_msg_and_reason("boom", UCode::InvalidArgument);
        assert!(!failed.is_success());
        assert!(failed.is_failed());
        assert_eq!("boom", failed.msg());
        assert_eq!(UCode::InvalidArgument as i32, failed.code_as_int());
    }

    // In the Rust SDK, this is redundant with above test
    // #[test]
    // fn create_failed_status_with_message_and_code() {
    //     let failed = UStatus::fail_with_msg_and_reason("boom", UCode::InvalidArgument);
    //     assert!(!failed.is_success());
    //     assert!(failed.is_failed());
    //     assert_eq!("boom", failed.msg());
    //     assert_eq!(UCode::InvalidArgument as i32, failed.code());
    // }

    #[test]
    fn code_from_a_known_int_code() {
        let code = UCode::from(4);
        assert_eq!(UCode::DeadlineExceeded, code);
    }

    #[test]
    fn code_from_a_unknown_int_code() {
        let code = UCode::from(299);
        assert_eq!(UCode::Unspecified, code);
    }

    // #[test]
    // fn code_from_a_known_google_code() {
    //     let gcode = GCode::InvalidArgument;
    //     let code = UCode::from(gcode);
    //     assert_eq!(UCode::InvalidArgument, code);
    // }

    // Cannot do a null object here in Rust
    // #[test]
    // fn code_from_a_null_google_code() {}

    // #[test]
    // fn code_from_a_unrecognized_google_code() {
    //     let google_code = GCode::Unimplemented;
    //     let code = UCode::from(google_code);
    //     assert_eq!(UCode::Unimplemented, code);
    // }
}
