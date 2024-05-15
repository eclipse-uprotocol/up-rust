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

use std::{default::Default, fmt};

use protobuf::{well_known_types::any::Any, MessageFull};

use crate::{RpcClientResult, UPayload, UPayloadFormat, UStatus};

pub type RpcPayloadResult = Result<RpcPayload, RpcMapperError>;

#[derive(Clone)]
pub struct RpcPayload {
    pub status: UStatus,
    pub payload: Option<UPayload>,
}

#[derive(Debug)]
pub enum RpcMapperError {
    UnexpectedError(String),
    InvalidPayload(String),
    UnknownType(String),
    ProtobufError(String),
}

impl fmt::Display for RpcMapperError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RpcMapperError::UnexpectedError(msg) => write!(f, "Unexpected error: {msg}"),
            RpcMapperError::InvalidPayload(msg) => write!(f, "Invalid payload: {msg}",),
            RpcMapperError::UnknownType(msg) => write!(f, "Unknown type: {msg}"),
            RpcMapperError::ProtobufError(msg) => write!(f, "Protobuf error: {msg}"),
        }
    }
}

/// `RpcMapper` is a structure that provides static methods to wrap an RPC request with
/// an RPC response (uP-L2). APIs that return a `Message` assume that the payload is
/// protobuf-serialized `com.google.protobuf.Any` (USerializationHint.PROTOBUF), and will
/// return an error if anything else is passed.
pub struct RpcMapper;

impl RpcMapper {
    /// Maps the payload data returned by a peer to the expected return type of the RPC method.
    ///
    /// # Parameters
    ///
    /// - `response`: A `Result` of type [`RpcClientResult`], representing the response from an RPC call.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The expected return type of the RPC method.
    ///
    /// # Returns
    ///
    /// Returns a `Result` either containing the expected return type of the RPC method wrapped,
    /// or an [`RpcMapperError`].
    ///
    /// # Errors
    ///
    /// This function can return an [`RpcMapperError`] in the following cases:
    ///
    /// - `InvalidPayload`: If the payload received in the response cannot be decoded into the expected return type `T`.
    ///   This error includes the detailed error message from the decoding process.
    ///
    /// - `UnknownType`: If the payload is present but cannot be decoded into a protobuf `Any` type.
    ///   This typically indicates an issue with the payload format or the expected type `T`.
    ///
    pub fn map_response<T>(response: RpcClientResult) -> Result<T, RpcMapperError>
    where
        T: MessageFull + Default,
    {
        let message = response?; // Directly returns in case of error

        let Some(payload) = message.payload.into_option() else {
            return Err(RpcMapperError::InvalidPayload(
                "Payload is empty".to_string(),
            ));
        };
        if payload.data.is_empty() {
            return Err(RpcMapperError::InvalidPayload(
                "Payload is empty".to_string(),
            ));
        }
        Any::try_from(payload)
            .map_err(|_e| {
                RpcMapperError::UnknownType("Couldn't decode payload into Any".to_string())
            })
            .and_then(|any| match any.unpack::<T>() {
                Ok(Some(m)) => Ok(m),
                Ok(None) => Err(RpcMapperError::InvalidPayload(String::from(
                    "Any object is not of expected type",
                ))),
                Err(error) => Err(RpcMapperError::InvalidPayload(error.to_string())),
            })
    }

    /// This function checks if a `RpcClientResult` contains a protobuf status type,
    /// -  if that is so it extracts the status code from the protobuf status and
    ///   - returns an [`RpcPayloadResult`] result with `UStatus::Ok()` and No(ne) [`UPayload`] if the protobuf status was Ok
    ///   - returns an [`RpcPayloadResult`] result with a failed `UStatus` (mirroring the protobuf status) and No(ne) [`UPayload`] if the protobuf status was not Ok
    /// - if the payload did not contain a protobuf status, return [`RpcPayloadResult`] result with `UStatus::Ok()` and the original payload in Some([`UPayload`])
    ///
    /// The usage idea is to apply this function to a `RpcClient::invoke_method()` result, then match the return to see if it's gotten a(ny) valid response, and
    /// apply `RpcMapper::map_result()` in case a payload was returned and a specific payload type is expected.
    ///
    /// # Errors
    ///
    /// This function can return an `RpcMapperError` in the following cases:
    ///
    /// - `UnknownType`: If the payload is present but cannot be decoded into a protobuf `Any` type. This indicates an issue with the payload format.
    ///
    /// - Other errors propagated from the `RpcClientResult` processing, including failure in unpacking a protobuf status or other issues encountered during processing.
    ///
    /// # Note
    /// There is one conscious deviation from the Java SDK: this implementation returns a `failed` status in every case where there's not a protobuf status
    /// in the payload. In such cases, the payload is still passed on as a function result so it can be used in further decoding attempts. So there are two
    /// things to check with the return from this function:
    /// - is there [`UStatus`] information (transporting info about the status of an operation, sent from a remote service)?
    /// - is there payload data passed in the result, to be decoded by the caller.
    ///
    // TODO This entire thing feels klunky and kludgy; this needs to be revisited...
    pub fn map_response_to_result(response: RpcClientResult) -> RpcPayloadResult {
        let message = response?; // Directly returns in case of error
        let Some(payload) = message.payload.into_option() else {
            return Err(RpcMapperError::InvalidPayload(
                "Payload is empty".to_string(),
            ));
        };
        if payload.data.is_empty() {
            return Err(RpcMapperError::InvalidPayload(
                "Payload is empty".to_string(),
            ));
        }
        Any::try_from(payload)
            .map_err(|_e| {
                RpcMapperError::UnknownType("Couldn't decode payload into Any".to_string())
            })
            .and_then(|any| {
                match Self::unpack_any::<UStatus>(&any) {
                    Ok(proto_status) => Ok(RpcPayload {
                        status: proto_status,
                        payload: None,
                    }),
                    Err(_error) => {
                        // in this branch, we couldn't decode the payload into a protobuf-status, but there is something else there to pass on
                        UPayload::try_from(&any)
                            .map_err(|e| RpcMapperError::InvalidPayload(e.to_string()))
                            .map(|payload| RpcPayload {
                                status: UStatus::fail(format!(
                                    "Unexpected any-payload type {}",
                                    any.type_url
                                )),
                                payload: Some(payload), // get the original payload back to avoid having to .clone() payload, above
                            })
                    }
                }
            })
    }

    /// Packs a protobuf message into a `UPayload` object.
    ///
    /// This function is used to encapsulate a strongly-typed data object into a `UPayload`,
    /// which allows for more generic data handling. It leverages Prost's protobuf encoding for
    /// serializing the data.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The type of the data to be packed.   
    ///
    /// # Parameters
    ///
    /// * `data`: The data to pack.
    ///
    /// # Returns
    ///
    /// The payload containing the packed data.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the protobuf serialization of the data exceeds 2^32 - 1 bytes.
    pub fn pack_payload<T: MessageFull>(data: &T) -> Result<UPayload, RpcMapperError> {
        let buf = data.write_to_bytes().map_err(|_e| {
            RpcMapperError::ProtobufError(String::from("failed to serialize payload to protobuf"))
        })?;
        Ok(UPayload {
            data: buf,
            format: UPayloadFormat::UPAYLOAD_FORMAT_PROTOBUF.into(),
            ..Default::default()
        })
    }

    /// Unpacks a given `UPayload` into a protobuf message.
    ///
    /// This function is used to extract strongly-typed data from a `UPayload` object, leveraging
    /// Prost's protobuf decoding capabilities for deserialization.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked.
    ///
    /// # Parameters
    ///
    /// * `payload`: The `UPayload` object containing the data to be unpacked.
    ///
    /// # Returns
    ///
    /// * `Ok(T)`: The deserialized protobuf message contained in the payload.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the unpacking process fails, for example if the payload could
    /// not be deserialized into the target protobuf type `T`.
    pub fn unpack_payload<T: MessageFull + Default>(
        payload: UPayload,
    ) -> Result<T, RpcMapperError> {
        Any::try_from(payload)
            .map_err(|_e| RpcMapperError::UnknownType("Couldn't decode payload".to_string()))
            .and_then(|any| {
                T::parse_from_bytes(any.value.as_slice())
                    .map_err(|error| RpcMapperError::InvalidPayload(error.to_string()))
            })
    }

    /// Packs a given `data` of type `T` into a protbuf `Any` object.
    ///
    /// This function is useful for converting strongly-typed data into an `Any`
    /// object for use in message-passing scenarios where the type needs to be
    /// encoded as `Any`.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The type of the data to be packed.
    ///
    /// # Parameters
    ///
    /// * `data`: The data of type `T` that will be packed into the returned `Any` object.
    ///
    /// # Returns
    ///
    /// * `Ok(Any)`: A protobuf `Any` object containing the packed `data`.
    /// * `Err(RpcMapperError)`: An error that occurred during the packing process.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the packing process fails.
    pub fn pack_any<T: MessageFull>(data: &T) -> Result<Any, RpcMapperError> {
        Any::pack(data).map_err(|error| RpcMapperError::InvalidPayload(error.to_string()))
    }

    /// Unpacks a given protbuf `Any` object into a data of type `T`.
    ///
    /// This function is used to convert an `Any` object back into its original
    /// strongly-typed data. It's essentially the reverse operation of `pack_any`.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The expected type of the unpacked data. This type must implement `prost::Name`
    ///   for type URL validation and `std::default::Default` for initializing the type.
    ///
    /// # Parameters
    ///
    /// * `any`: The `Any` object that will be unpacked.
    ///
    /// # Returns
    ///
    /// * `Ok(T)`: A `T` object containing the unpacked data.
    /// * `Err(RpcMapperError)`: An error that occurred during the unpacking process.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the unpacking process fails, for example due to type mismatch
    /// or if the data inside `Any` could not be decoded into type `T`.
    pub fn unpack_any<T: MessageFull>(any: &Any) -> Result<T, RpcMapperError> {
        match any.unpack() {
            Err(error) => Err(RpcMapperError::InvalidPayload(error.to_string())),
            Ok(v) => match v {
                Some(msg) => Ok(msg),
                None => Err(RpcMapperError::ProtobufError(String::from(
                    "Any object does not contain payload",
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Buf, BufMut};
    use protobuf::MessageField;

    use crate::{UCode, UMessage};

    fn build_status_response(code: UCode, msg: &str) -> RpcClientResult {
        let status: UStatus = UStatus::fail_with_code(code, msg);
        let any = RpcMapper::pack_any(&status)?;
        let payload =
            UPayload::try_from(any).map_err(|e| RpcMapperError::InvalidPayload(e.to_string()))?;
        let message = UMessage {
            payload: MessageField::some(payload),
            ..Default::default()
        };
        Ok(message)
    }

    fn build_empty_payload_response() -> RpcClientResult {
        let payload = UPayload {
            data: vec![],
            ..Default::default()
        };
        let message = UMessage {
            payload: MessageField::some(payload),
            ..Default::default()
        };
        Ok(message)
    }

    fn build_number_response(number: i32) -> RpcClientResult {
        let any: Any = Any {
            type_url: "type.googleapis.com/Int32Value".to_string(),
            value: number.to_be_bytes().into(),
            ..Default::default()
        };
        let payload =
            UPayload::try_from(any).map_err(|e| RpcMapperError::InvalidPayload(e.to_string()))?;
        let message = UMessage {
            payload: MessageField::some(payload),
            ..Default::default()
        };
        Ok(message)
    }

    #[test]
    fn test_map_response_to_result_happy_path() {
        let result = RpcMapper::map_response_to_result(build_number_response(3)).unwrap();

        assert!(result.status.is_failed()); // TODO this seems strange

        let payload = result.payload.unwrap();
        let any = Any::try_from(payload).unwrap();
        assert_eq!("type.googleapis.com/Int32Value", any.type_url);
        let value = (&any.value[..]).get_i32();
        assert_eq!(value, 3);
    }

    #[test]
    fn test_compose_that_returns_status() {
        let response = build_status_response(UCode::INVALID_ARGUMENT, "boom");

        let result = RpcMapper::map_response_to_result(response).unwrap();

        assert!(result.status.is_failed());
        assert_eq!(result.status.get_code(), UCode::INVALID_ARGUMENT);
        assert_eq!(result.status.message.unwrap(), "boom");
    }

    #[test]
    fn test_compose_with_failure() {
        let response = Err(RpcMapperError::UnexpectedError("Boom".to_string()));
        let result = RpcMapper::map_response_to_result(response);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "Unexpected error: Boom");
    }

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_status_using_map_response_to_rpc_response(
    ) {
        let response = build_status_response(UCode::INVALID_ARGUMENT, "boom");
        let result = RpcMapper::map_response_to_result(response).unwrap();

        assert!(result.status.is_failed());
        assert_eq!(UCode::INVALID_ARGUMENT, result.status.get_code());
        assert_eq!("boom", result.status.message.unwrap());
    }

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_bad_proto_using_map_response_to_rpc_response(
    ) {
        let response = build_number_response(42);
        let result = RpcMapper::map_response_to_result(response).unwrap();

        assert!(result.status.is_failed());
        assert_eq!(
            result.status.message.unwrap(),
            "Unexpected any-payload type type.googleapis.com/Int32Value"
        );
    }

    // Create a generic UMessage for use in test cases
    fn build_umessage_for_test() -> UMessage {
        let arbitrary_proto = crate::up_core_api::file::FileBatch::default();
        let any = RpcMapper::pack_any(&arbitrary_proto).unwrap();

        let payload = UPayload::try_from(any).unwrap();
        UMessage {
            payload: MessageField::some(payload),
            ..Default::default()
        }
    }

    #[test]
    fn test_success_invoke_method_happy_flow_using_map_response_to_rpc_response() {
        let response_message = build_umessage_for_test();
        let result = RpcMapper::map_response_to_result(Ok(response_message.clone())).unwrap();

        assert!(result.status.is_failed());
        assert_eq!(result.payload.unwrap(), response_message.payload.unwrap());
    }

    #[test]
    fn test_success_invoke_method_happy_flow_using_map_response() {
        let response_message = build_umessage_for_test();
        let file_batch =
            RpcMapper::map_response::<crate::up_core_api::file::FileBatch>(Ok(response_message))
                .unwrap();

        assert_eq!(file_batch, crate::up_core_api::file::FileBatch::default());
    }

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_status_using_map_response() {
        let response = build_status_response(UCode::ABORTED, "hello");
        let e = RpcMapper::map_response::<crate::up_core_api::file::FileBatch>(response);

        assert!(e.is_err());
    }

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_bad_proto_using_map_response() {
        let response = build_number_response(42);
        let e = RpcMapper::map_response::<crate::up_core_api::file::FileBatch>(response);

        assert!(e.is_err());
    }

    // all these stub-using tests, what do they add?

    #[test]
    fn test_success_invoke_method_that_has_null_payload_map_response() {
        let response = Err(RpcMapperError::InvalidPayload(
            "not a CloudEvent".to_string(),
        ));
        let result = RpcMapper::map_response::<crate::up_core_api::file::FileBatch>(response);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Invalid payload: not a CloudEvent"
        );
    }

    #[test]
    fn test_success_invoke_method_that_has_null_payload_map_response_to_result() {
        let response = Err(RpcMapperError::InvalidPayload(
            "Invalid payload".to_string(),
        ));
        let result = RpcMapper::map_response_to_result(response);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Invalid payload: Invalid payload"
        );
    }

    #[test]
    fn test_success_invoke_method_happy_flow_that_returns_status_using_map_response() {
        let response = build_status_response(UCode::OK, "all good");
        let s = RpcMapper::map_response::<UStatus>(response).unwrap();
        let ustatus = s;

        assert_eq!(UCode::OK, ustatus.get_code());
        assert_eq!("all good", ustatus.message.unwrap());
    }

    #[test]
    fn test_success_invoke_method_happy_flow_that_returns_status_using_map_response_to_result_to_rpc_response(
    ) {
        let response = build_status_response(UCode::OK, "all good");
        let s = RpcMapper::map_response_to_result(response).unwrap();

        assert!(s.status.is_success());
        assert_eq!(s.status.get_code(), UCode::OK);
    }

    #[test]
    fn test_unpack_payload_failed() {
        let payload = Any {
            type_url: "type.googleapis.com/Int32Value".to_string(),
            value: {
                let mut buf = vec![];
                buf.put_i32(42);
                buf
            },
            ..Default::default()
        };

        let result: Result<UStatus, RpcMapperError> = RpcMapper::unpack_any::<UStatus>(&payload);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_payload_that_is_not_type_any() {
        let response = build_empty_payload_response();
        let result = RpcMapper::map_response::<UStatus>(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_payload_that_is_not_type_any_map_to_result() {
        let response = build_empty_payload_response();
        let result = RpcMapper::map_response_to_result(response);
        assert!(result.is_err());
    }
}
