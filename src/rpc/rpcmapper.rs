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

use prost_types::Any;
use std::default::Default;
use std::fmt;
use std::future::Future;

use crate::rpc::rpcclient::RpcClientResult;
use crate::uprotocol::{Data, UCode, UPayload, UPayloadFormat, UStatus};

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
            RpcMapperError::UnexpectedError(msg) => write!(f, "Unexpected error: {}", msg),
            RpcMapperError::InvalidPayload(msg) => write!(f, "Invalid payload: {}", msg),
            RpcMapperError::UnknownType(msg) => write!(f, "Unknown type: {}", msg),
            RpcMapperError::ProtobufError(msg) => write!(f, "Protobuf error: {}", msg),
        }
    }
}

/// `RpcMapper` is a structure that provides static methods to wrap an RPC request with
/// an RPC response (uP-L2). APIs that return a `Message` assume that the payload is
/// protobuf-serialized `com.google.protobuf.Any` (USerializationHint.PROTOBUF), and will
/// return an error if anything else is passed.
pub struct RpcMapper;

impl RpcMapper {
    /// Maps a `Future` of [`RpcClientResult`]  into a `Future` containing the expected return type of the RPC method, or an [`RpcMapperError`].
    ///
    /// # Parameters
    ///
    /// - `response_future`: A `Future` that resolves to an [`RpcClientResult`].
    ///
    /// # Type Parameters
    ///
    /// - `T`: The declared expected return type of the RPC method. It must implement [`prost::Message`] and [`Default`].
    ///
    /// # Returns
    ///
    /// Returns a `Future` that resolves to a `Result` either containing the expected return type of the RPC method wrapped,
    /// or an [`RpcMapperError`].
    pub async fn map_response<T, F>(response_future: F) -> Result<T, RpcMapperError>
    where
        T: prost::Message + Default,
        F: Future<Output = RpcClientResult>,
    {
        match response_future.await {
            Ok(payload) => {
                // we got some useable response payload
                let any: Any = payload.into();
                if any != Any::default() {
                    match T::decode(any.value.as_slice()) {
                        Ok(result) => {
                            // expected response type could be decoded
                            Ok(result)
                        }
                        Err(error) => {
                            // ... or not
                            Err(RpcMapperError::InvalidPayload(error.to_string()))
                        }
                    }
                } else {
                    // we got a payload, but it's not a protobuf
                    Err(RpcMapperError::UnknownType(
                        "Couldn't decode payload into Any".to_string(),
                    ))
                }
            }
            Err(error) => Err(error),
        }
    }

    /// This function checks if a RpcClientResult contains a protobuf status type,
    /// -  if that is so it extracts the status code from the protobuf status and
    ///   - returns an [`RpcPayloadResult`] result with `UStatus::Ok()` and No(ne) [`UPayload`] if the protobuf status was Ok
    ///   - returns an [`RpcPayloadResult`] result with a failed UStatus (mirroring the protobuf status) and No(ne) [`UPayload`] if the protobuf status was not Ok
    /// - if the payload did not contain a protobuf status, return [`RpcPayloadResult`] result with `UStatus::Ok()` and the original payload in Some([`UPayload`])
    ///
    /// The usage idea is to apply this function to a RpcClient::invoke_method() result, then match the return to see if it's gotten a(ny) valid response, and
    /// apply RpcMapper::map_result() in case a payload was returned and a specific payload type is expected.
    ///
    /// Types used:
    /// - RpcPayloadResult = Result<RpcPayload, RpcMapperError>
    /// - pub struct RpcPayload {
    ///     status: UStatus,
    ///     payload: `Option<UPayload>`,
    ///   }
    ///
    /// # Note
    /// There is one conscious deviation from the Java SDK: this implementation returns a `failed` status in every case where there's not a protobuf status
    /// in the payload. In such cases, the payload is still passed on as a function result so it can be used in further decoding attempts. So there are two
    /// things to check with the return from this function:
    /// - is there [`UStatus`] information (transporting info about the status of an operation, sent from a remote service)?
    /// - is there payload data passed in the result, to be decoded by the caller.
    ///
    /// This entire thing feels klunky and kludgy; this this needs to be revisited...
    pub async fn map_response_to_result<F>(response_future: F) -> RpcPayloadResult
    where
        F: Future<Output = RpcClientResult>,
    {
        match response_future.await {
            Ok(payload) => {
                let any: Any = payload.into();
                if any != Any::default() {
                    match Self::unpack_any::<UStatus>(any.clone()) {
                        // in this branch, we have successfully unpacked a protobuf-status from the (now consumed) payload
                        Ok(proto_status) => {
                            match UCode::try_from(proto_status.code).unwrap_or(UCode::Unknown) {
                                UCode::Ok => Ok(RpcPayload {
                                    status: UStatus::ok(),
                                    payload: None,
                                }),
                                _ => Ok(RpcPayload {
                                    status: proto_status,
                                    payload: None,
                                }),
                            }
                        }
                        // in this branch, we couldn't decode the payload into a protobuf-status, but there is something else there to pass on
                        Err(_error) => {
                            Ok(RpcPayload {
                                status: UStatus::fail(&format!(
                                    "Unexpected any-payload type {}",
                                    any.type_url
                                )),
                                payload: Some(any.into()), // get the original payload back to avoid having to .clone() payload, above
                            })
                        }
                    }
                } else {
                    // we got a payload, but it's not a protobuf
                    Err(RpcMapperError::UnknownType(
                        "Couldn't decode payload into Any".to_string(),
                    ))
                }
            }
            Err(error) => {
                // in this branch, we didn't get anything useful from the response_future
                Err(error)
            }
        }
    }

    /// Packs a given data of type `T` into a `UPayload` object.
    ///
    /// This function is used to encapsulate a strongly-typed data object into a `UPayload`,
    /// which allows for more generic data handling. It leverages Prost's protobuf encoding for
    /// serializing the data.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The type of the data to be packed. Must implement `prost::Message` for protobuf
    ///   serialization.
    ///
    /// # Parameters
    ///
    /// * `data`: The data of type `T` that will be packed into `UPayload`.
    ///
    /// # Returns
    ///
    /// * `Ok(UPayload)`: A `UPayload` object containing the packed data.
    /// * `Err(RpcMapperError)`: An error that occurred during the packing process.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the packing process fails, for example if the data could
    /// not be serialized into protobuf format.
    pub fn pack_payload<T: prost::Message>(data: T) -> Result<UPayload, RpcMapperError> {
        let mut buf = vec![];
        match data.encode(&mut buf) {
            Ok(_) => {
                let len = buf.len() as i32;
                Ok(UPayload {
                    data: Some(Data::Value(buf)),
                    length: Some(len),
                    format: UPayloadFormat::UpayloadFormatRaw as i32,
                })
            }
            Err(error) => Err(RpcMapperError::InvalidPayload(error.to_string())),
        }
    }

    /// Unpacks a given `UPayload` into a data object of type `T`.
    ///
    /// This function is used to extract strongly-typed data from a `UPayload` object, leveraging
    /// Prost's protobuf decoding capabilities for deserialization.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The target type of the data to be unpacked. Must implement `prost::Message` for protobuf
    ///   deserialization and `Default` for initialization.
    ///
    /// # Parameters
    ///
    /// * `payload`: The `UPayload` object containing the data to be unpacked.
    ///
    /// # Returns
    ///
    /// * `Ok(T)`: A `T` object containing the unpacked data.
    /// * `Err(RpcMapperError)`: An error that occurred during the unpacking process.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the unpacking process fails, for example if the payload could
    /// not be deserialized into the target protobuf type `T`.
    pub fn unpack_payload<T: prost::Message + std::default::Default>(
        payload: UPayload,
    ) -> Result<T, RpcMapperError> {
        let any: Any = payload.into();
        if any != Any::default() {
            match T::decode(any.value.as_slice()) {
                Ok(result) => Ok(result),
                Err(error) => Err(RpcMapperError::InvalidPayload(error.to_string())),
            }
        } else {
            Err(RpcMapperError::UnknownType(
                "Couldn't decode payload".to_string(),
            ))
        }
    }

    /// Packs a given `data` of type `T` into a `prost_types::Any` object.
    ///
    /// This function is useful for converting strongly-typed data into an `Any`
    /// object for use in message-passing scenarios where the type needs to be
    /// encoded as `Any`.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The type of the data to be packed. Must implement `prost::Name` to provide
    ///   type URL information.
    ///
    /// # Parameters
    ///
    /// * `data`: The data of type `T` that will be packed into the returned `Any` object.
    ///
    /// # Returns
    ///
    /// * `Ok(Any)`: A `prost_types::Any` object containing the packed `data`.
    /// * `Err(RpcMapperError)`: An error that occurred during the packing process.
    ///
    /// # Errors
    ///
    /// Returns an `RpcMapperError` if the packing process fails.
    pub fn pack_any<T: prost::Name>(data: T) -> Result<Any, RpcMapperError> {
        let result = Any::from_msg(&data);

        match result {
            Ok(any) => Ok(any),
            Err(error) => Err(RpcMapperError::InvalidPayload(error.to_string())),
        }
    }

    /// Unpacks a given `prost_types::Any` object into a data of type `T`.
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
    /// * `any`: The `prost_types::Any` object that will be unpacked.
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
    pub fn unpack_any<T: prost::Name + std::default::Default>(
        any: Any,
    ) -> Result<T, RpcMapperError> {
        let result = any.to_msg();

        match result {
            Ok(value) => Ok(value),
            Err(error) => Err(RpcMapperError::InvalidPayload(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bytes::{Buf, BufMut};
    use cloudevents::{Event, EventBuilder, EventBuilderV10};

    use crate::proto::CloudEvent as CloudEventProto;
    use crate::rpc::RpcClient;
    use crate::transport::builder::UAttributesBuilder;
    use crate::uprotocol::{UAttributes, UEntity, UMessageType, UUri};
    use crate::uri::serializer::{LongUriSerializer, UriSerializer};

    struct ULinkReturnsNumber3;

    #[async_trait]
    impl RpcClient for ULinkReturnsNumber3 {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            let any: Any = Any {
                type_url: "type.googleapis.com/Int32Value".to_string(),
                value: {
                    let mut buf = vec![];
                    buf.put_i32(3);
                    buf
                },
            };
            let payload = any.into();
            Ok(payload)
        }
    }

    struct ULinkHappyPath;

    #[async_trait]
    impl RpcClient for ULinkHappyPath {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            let payload = build_upayload_for_test();
            Ok(payload)
        }
    }

    struct ULinkWithStatusCodeInsteadOfHappyPath;

    #[async_trait]
    impl RpcClient for ULinkWithStatusCodeInsteadOfHappyPath {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            let status = UStatus::fail_with_code(UCode::InvalidArgument, "boom");

            let any = RpcMapper::pack_any(status).unwrap();
            let payload = any.into();

            Ok(payload)
        }
    }

    struct ULinkWithStatusCodeHappyPath;

    #[async_trait]
    impl RpcClient for ULinkWithStatusCodeHappyPath {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            let status = UStatus::fail_with_code(UCode::Ok, "all good");

            let any = RpcMapper::pack_any(status).unwrap();
            let payload = any.into();

            Ok(payload)
        }
    }

    struct ULinkThatCompletesWithAnError;

    #[async_trait]
    impl RpcClient for ULinkThatCompletesWithAnError {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            Err(RpcMapperError::UnexpectedError("Boom".to_string()))
        }
    }

    struct ULinkWithCrappyPayload;

    #[async_trait]
    impl RpcClient for ULinkWithCrappyPayload {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            let payload = UPayload {
                data: Some(Data::Value(vec![])),
                ..Default::default()
            };
            Ok(payload)
        }
    }

    struct ULinkWithInvalidPayload;

    #[async_trait]
    impl RpcClient for ULinkWithInvalidPayload {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            Err(RpcMapperError::InvalidPayload(
                "Invalid payload".to_string(),
            ))
        }
    }

    struct ULinkThatReturnsTheWrongProto;

    #[async_trait]
    impl RpcClient for ULinkThatReturnsTheWrongProto {
        async fn invoke_method(
            _topic: UUri,
            _payload: UPayload,
            _attributes: UAttributes,
        ) -> RpcClientResult {
            let any: Any = Any {
                type_url: "type.googleapis.com/Int32Value".to_string(),
                value: {
                    let mut buf = vec![];
                    buf.put_i32(42);
                    buf
                },
            };

            let payload = any.into();
            Ok(payload)
        }
    }

    #[test]
    fn test_compose_happy_path() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkReturnsNumber3::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let result = RpcMapper::map_response_to_result(rpc_response)
                .await
                .unwrap();

            assert!(result.status.is_failed());

            let payload = result.payload.unwrap();
            let any = Any::from(payload);
            assert_eq!("type.googleapis.com/Int32Value", any.type_url);
            let value = (&any.value[..]).get_i32();
            assert_eq!(value, 3);
        });
    }

    #[test]
    fn test_compose_that_returns_status() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithStatusCodeInsteadOfHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response_to_result(rpc_response)
                .await
                .unwrap();

            assert!(response.status.is_failed());
            assert_eq!(response.status.code, UCode::InvalidArgument as i32);
            assert_eq!(response.status.message(), "boom");
        });
    }

    #[test]
    fn test_compose_with_failure() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkThatCompletesWithAnError::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response_to_result(rpc_response).await;

            assert!(response.is_err());
            assert_eq!(
                response.err().unwrap().to_string(),
                "Unexpected error: Boom"
            );
        });
    }

    // This seems to exclusively test this .exceptionally() method on the Java side, which we don't have here
    // (and also, which does only very distantly have anything to do with the uProtocol stuff)
    // #[test]
    // fn test_compose_with_failure_transform_exception() {}

    #[test]
    fn test_success_invoke_method_happy_flow_using_map_response_to_rpc_response() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response_to_result(rpc_response)
                .await
                .unwrap();

            assert!(response.status.is_failed());
            let pft = build_upayload_for_test();
            assert_eq!(response.payload.unwrap(), pft);
        });
    }

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_status_using_map_response_to_rpc_response(
    ) {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithStatusCodeInsteadOfHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response_to_result(rpc_response)
                .await
                .unwrap();

            assert!(response.status.is_failed());
            assert_eq!(UCode::InvalidArgument as i32, response.status.code);
            assert_eq!("boom", response.status.message());
        });
    }

    // No exceptions in Rust
    // #[test]
    // fn test_fail_invoke_method_when_invoke_method_threw_an_exception_using_map_response_to_rpc_response()

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_bad_proto_using_map_response_to_rpc_response(
    ) {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkThatReturnsTheWrongProto::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response_to_result(rpc_response)
                .await
                .unwrap();

            assert!(response.status.is_failed());
            assert_eq!(
                response.status.message(),
                "Unexpected any-payload type type.googleapis.com/Int32Value"
            );
        });
    }

    #[test]
    fn test_success_invoke_method_happy_flow_using_map_response() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let e = RpcMapper::map_response::<CloudEventProto, _>(rpc_response)
                .await
                .unwrap();

            let event = Event::from(e);
            let pft = build_cloud_event_for_test();

            assert_eq!(event, pft);
        });
    }

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_status_using_map_response() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithStatusCodeInsteadOfHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let e = RpcMapper::map_response::<CloudEventProto, _>(rpc_response).await;

            assert!(e.is_err());
            assert_eq!(e.err().unwrap().to_string(), "Invalid payload: failed to decode Protobuf message: CloudEvent.id: invalid wire type: Varint (expected LengthDelimited)");
        });
    }

    // We don't do exceptions
    // #[test]
    // fn test_fail_invoke_method_when_invoke_method_threw_an_exception_using_map_response()

    #[test]
    fn test_fail_invoke_method_when_invoke_method_returns_a_bad_proto_using_map_response() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkThatReturnsTheWrongProto::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let e = RpcMapper::map_response::<CloudEventProto, _>(rpc_response).await;

            assert!(e.is_err());
            assert_eq!(
                e.err().unwrap().to_string(),
                "Invalid payload: failed to decode Protobuf message: invalid tag value: 0"
            );
        });
    }

    // all these stub-using tests, what do they add?

    #[test]
    fn test_success_invoke_method_that_has_null_payload_map_response() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithInvalidPayload::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response::<CloudEventProto, _>(rpc_response).await;

            assert!(response.is_err());
            assert_eq!(
                response.err().unwrap().to_string(),
                "Invalid payload: Invalid payload"
            );
        });
    }

    #[test]
    fn test_success_invoke_method_that_has_null_payload_map_response_to_result() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithInvalidPayload::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let response = RpcMapper::map_response_to_result(rpc_response).await;

            assert!(response.is_err());
            assert_eq!(
                response.err().unwrap().to_string(),
                "Invalid payload: Invalid payload"
            );
        });
    }

    #[test]
    fn test_success_invoke_method_happy_flow_that_returns_status_using_map_response() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithStatusCodeHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let s = RpcMapper::map_response::<UStatus, _>(rpc_response)
                .await
                .unwrap();
            let ustatus = s;

            assert_eq!(UCode::Ok as i32, ustatus.code);
            assert_eq!("all good", ustatus.message());
        });
    }

    #[test]
    fn test_success_invoke_method_happy_flow_that_returns_status_using_map_response_to_result_to_rpc_response(
    ) {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithStatusCodeHappyPath::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let s = RpcMapper::map_response_to_result(rpc_response)
                .await
                .unwrap();

            assert!(s.status.is_success());
            assert_eq!(s.status.code, UCode::Ok as i32);
        });
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
        };

        let result: Result<UStatus, RpcMapperError> = RpcMapper::unpack_any::<UStatus>(payload);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_payload_that_is_not_type_any() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithCrappyPayload::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let result = RpcMapper::map_response::<UStatus, _>(rpc_response).await;
            assert!(result.is_err());
            assert!(result
                .err()
                .unwrap()
                .to_string()
                .contains("Couldn't decode payload into Any"));
        })
    }

    #[test]
    fn test_invalid_payload_that_is_not_type_any_map_to_result() {
        let mut runtime = futures::executor::LocalPool::new();

        runtime.run_until(async {
            let rpc_response = ULinkWithCrappyPayload::invoke_method(
                build_topic(),
                build_upayload_for_test(),
                build_attributes(),
            );

            let result = RpcMapper::map_response_to_result(rpc_response).await;
            assert!(result.is_err());
            assert!(result
                .err()
                .unwrap()
                .to_string()
                .contains("Couldn't decode payload into Any"));
        })
    }

    fn build_cloud_event_for_test() -> Event {
        EventBuilderV10::new()
            .id("hello")
            .ty(UMessageType::UmessageTypeRequest)
            .source("http://example.com")
            .build()
            .unwrap()
    }

    fn build_upayload_for_test() -> UPayload {
        let event = build_cloud_event_for_test();
        let proto_event = CloudEventProto::from(event);
        let any = RpcMapper::pack_any(proto_event).unwrap();

        any.into()
    }

    fn build_topic() -> UUri {
        LongUriSerializer::deserialize("//vcu.vin/hartley/1/rpc.Raise".to_string())
    }

    fn build_attributes() -> UAttributes {
        UAttributesBuilder::request(
            crate::uprotocol::UPriority::UpriorityCs4,
            UUri {
                entity: Some(UEntity {
                    name: "hartley".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            1000,
        )
        .build()
    }
}
