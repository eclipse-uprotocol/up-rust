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

use prost::Message;
use prost_types::Any;

use crate::transport::datamodel::userializationhint::USerializationHint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UPayload {
    pub data: Vec<u8>,
    pub hint: Option<USerializationHint>,
}

#[derive(Debug)]
pub enum DecodeError {
    WrongSerialization(String),
    DecodeError(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::DecodeError(error) => {
                write!(f, "Decoding error: {}", error)
            }
            DecodeError::WrongSerialization(error) => {
                write!(f, "Wrong serialization: {}", error)
            }
        }
    }
}

impl UPayload {
    /// Create a new UPayload with the given data.
    pub fn new(data: Vec<u8>, hint: Option<USerializationHint>) -> Self {
        UPayload { data, hint }
    }

    /// Get the actual serialized or raw data, which can be deserialized or simply used as is.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get an empty representation of UPayload.
    pub fn empty() -> Self {
        UPayload {
            data: vec![],
            hint: None,
        }
    }

    /// Check if the data in the UPayload is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn from_any(any: Any) -> Self {
        Self::from(any)
    }

    pub fn to_any(&self) -> Result<Any, DecodeError> {
        match self.hint {
            // Attempt decoding even if hint is 'unknown' or missing
            Some(USerializationHint::Protobuf) | Some(USerializationHint::Unknown) | None => {
                match Any::decode(self.data.as_ref()) {
                    Ok(result) => Ok(result),
                    Err(error) => Err(DecodeError::DecodeError(error.to_string())),
                }
            }
            // Every other case, we know can't work
            Some(hint) => Err(DecodeError::WrongSerialization(format!(
                "Can't decode to protobuf:Any from serialization {}",
                hint
            ))),
        }
    }
}

impl fmt::Display for UPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UPayload{{ data: {:?} }}", self.data)
    }
}

impl From<Any> for UPayload {
    fn from(any: Any) -> Self {
        UPayload::new(any.encode_to_vec(), Some(USerializationHint::Protobuf))
    }
}

/// Converts a `UPayload` into a `prost_types::Any`.
///
/// # Note
///
/// In case of any errors during the conversion, this function will return `Any::default()`.
/// Please use `UPayload::to_any()` if you need proper error handling.
impl From<UPayload> for Any {
    fn from(value: UPayload) -> Self {
        match value.hint {
            Some(USerializationHint::Protobuf) => {
                if let Ok(result) = Any::decode(value.data.as_ref()) {
                    result
                } else {
                    Any::default()
                }
            }
            _ => Any::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str;

    use super::UPayload;
    use crate::transport::datamodel::userializationhint::USerializationHint;

    #[test]
    fn test_to_string_with_empty() {
        let u_payload = UPayload::empty();
        assert_eq!("UPayload{ data: [] }", u_payload.to_string());
    }

    #[test]
    fn test_to_string_with_hello() {
        let u_payload = UPayload::new("hello".as_bytes().to_vec(), None);
        assert_eq!(
            "UPayload{ data: [104, 101, 108, 108, 111] }",
            u_payload.to_string()
        );
    }

    #[test]
    fn test_is_empty_with_empty() {
        let u_payload = UPayload::empty();
        assert!(u_payload.is_empty());
    }

    #[test]
    fn test_is_empty_with_data() {
        let u_payload = UPayload::new(vec![1, 2, 3], None);
        assert!(!u_payload.is_empty());
    }

    #[test]
    fn test_equality() {
        let u_payload1 = UPayload::new(vec![1, 2, 3], None);
        let u_payload2 = UPayload::new(vec![1, 2, 3], None);
        let u_payload3 = UPayload::new(vec![4, 5, 6], None);

        assert_eq!(u_payload1, u_payload2);
        assert_ne!(u_payload1, u_payload3);
    }

    #[test]
    fn create_an_empty_upayload() {
        let u_payload = UPayload::empty();
        assert_eq!(0, u_payload.data().len());
        assert!(u_payload.is_empty());
    }

    // Note: Rust doesn't allow null for Vec<u8>
    // #[test]
    // fn create_upayload_with_null() {}

    #[test]
    fn create_upayload_from_bytes() {
        let string_data = "hello";
        let u_payload = UPayload::new(string_data.as_bytes().to_vec(), None);
        assert_eq!(string_data.len(), u_payload.data().len());
        assert!(!u_payload.is_empty());

        // Convert bytes back to a string for comparison
        let data_as_str = str::from_utf8(u_payload.data()).unwrap();
        assert_eq!(string_data, data_as_str);
    }

    #[test]
    fn create_upayload_from_a_string() {
        let string_data = "hello world";
        let u_payload = UPayload::new(string_data.as_bytes().to_vec(), None);
        assert_eq!(string_data.len(), u_payload.data().len());
        assert!(!u_payload.is_empty());

        // Convert bytes back to a string for comparison
        let data_as_str = str::from_utf8(u_payload.data()).unwrap();
        assert_eq!(string_data, data_as_str);
    }

    #[test]
    fn create_upayload_from_a_string_with_a_hint() {
        let string_data = "hello world";
        let u_payload = UPayload::new(
            string_data.as_bytes().to_vec(),
            Some(USerializationHint::Json),
        );

        assert_eq!(string_data.len(), u_payload.data().len());
        assert!(!u_payload.is_empty());

        // Convert bytes back to a string for comparison
        let data_as_str = str::from_utf8(u_payload.data()).unwrap();
        assert_eq!(string_data, data_as_str);

        assert_eq!(USerializationHint::Json, u_payload.hint.unwrap());
    }
}
