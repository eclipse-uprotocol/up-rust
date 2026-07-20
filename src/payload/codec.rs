/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
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

use std::io::Read;

use bytes::Bytes;

use crate::PayloadEncoding;
#[cfg(feature = "protobuf-support")]
use crate::ProtobufMappable;

use super::UWireError;

/// Byte layout requested by a payload codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PayloadLayout {
    len: usize,
    align: usize,
}

impl PayloadLayout {
    /// Creates a payload layout.
    ///
    /// # Errors
    ///
    /// Returns an error when `align` is zero or not a power of two.
    pub fn new(len: usize, align: usize) -> Result<Self, UWireError> {
        if align == 0 {
            return Err(UWireError::invalid_payload(
                "payload alignment must be non-zero",
            ));
        }
        if !align.is_power_of_two() {
            return Err(UWireError::invalid_payload(format!(
                "payload alignment {align} is not a power of two"
            )));
        }
        Ok(Self { len, align })
    }

    /// Returns the payload length in bytes.
    #[must_use]
    pub fn len(self) -> usize {
        self.len
    }

    /// Returns the required payload alignment in bytes.
    #[must_use]
    pub fn align(self) -> usize {
        self.align
    }

    /// Returns whether the payload has zero bytes.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Compile-time identity for an application payload codec.
///
/// ```rust
/// use up_rust::{payload::codec::PayloadFormat, PayloadEncoding, UPayloadFormat};
///
/// struct JsonTelemetry;
///
/// impl PayloadFormat for JsonTelemetry {
///     fn name() -> &'static str {
///         "json-telemetry-v1"
///     }
///
///     fn encoding() -> PayloadEncoding {
///         PayloadEncoding::JSON
///     }
/// }
/// ```
pub trait PayloadFormat {
    /// Stable codec name for logs, diagnostics, and configuration.
    fn name() -> &'static str;

    /// Payload encoding metadata written into frames that use this codec.
    fn encoding() -> PayloadEncoding;
}

/// Payload-layer codec identity used by typed frame helpers.
pub trait PayloadCodec {
    /// Stable codec name for logs, diagnostics, and configuration.
    fn codec_name() -> &'static str;

    /// Payload encoding metadata written into frames that use this codec.
    fn payload_encoding() -> PayloadEncoding;

    /// Verifies frame encoding metadata against this codec.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame is missing payload encoding metadata or if
    /// the metadata is incompatible with this codec.
    fn verify_encoding(actual: Option<&PayloadEncoding>) -> Result<(), UWireError> {
        let expected = Self::payload_encoding();
        let actual = actual.ok_or(UWireError::MissingEncoding)?;
        if !actual.is_compatible_with(&expected) {
            return Err(UWireError::UnsupportedEncoding {
                expected: Box::new(expected),
                actual: Box::new(actual.clone()),
            });
        }
        Ok(())
    }
}

impl<F> PayloadCodec for F
where
    F: PayloadFormat,
{
    fn codec_name() -> &'static str {
        <F as PayloadFormat>::name()
    }

    fn payload_encoding() -> PayloadEncoding {
        <F as PayloadFormat>::encoding()
    }
}

/// Encodes a typed value with a [`PayloadCodec`].
#[diagnostic::on_unimplemented(
    message = "the payload codec `{Self}` cannot encode `{T}`",
    label = "no `EncodePayload<{T}>` implementation",
    note = "wire implementers provide `payload_layout` and `encode_payload`; see the wire-implementer walkthrough"
)]
pub trait EncodePayload<T: ?Sized>: PayloadCodec {
    /// Returns the exact payload layout required to encode `value`.
    ///
    /// This is a measurement operation, not necessarily a constant-time size
    /// lookup. A variable-length codec may serialize or traverse `value` here
    /// and perform the work again in [`Self::encode_payload`]. Benchmarks and
    /// callers that care about this cost should label the probe and write phases
    /// separately. The returned length and alignment are the complete contract
    /// for the destination passed to `encode_payload`.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be measured for this codec.
    fn payload_layout(value: &T) -> Result<PayloadLayout, UWireError>;

    /// Encodes `value` into `dst`.
    ///
    /// # Errors
    ///
    /// Returns an error if `dst` is too small or serialization fails.
    fn encode_payload(value: &T, dst: &mut [u8]) -> Result<(), UWireError>;

    /// Encodes `value` into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn encode_payload_owned(value: &T) -> Result<Bytes, UWireError> {
        let layout = Self::payload_layout(value)?;
        let mut bytes = vec![0_u8; layout.len()];
        Self::encode_payload(value, &mut bytes)?;
        Ok(Bytes::from(bytes))
    }
}

/// Decodes a typed value from contiguous payload bytes.
pub trait DecodePayload<'a, T>: PayloadCodec {
    /// Decodes `T` from payload bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if `src` is malformed for this codec.
    fn decode_payload(src: &'a [u8]) -> Result<T, UWireError>;
}

/// Decodes a typed value from an ordered payload byte stream.
pub trait ReadDecodePayload<T>: PayloadCodec {
    /// Decodes `T` from `reader`, which must yield exactly `payload_len` bytes.
    ///
    /// Implementations must treat `payload_len` as a finite allocation and read
    /// bound before reserving payload-sized storage. They must consume exactly
    /// that many bytes, distinguish an early EOF from malformed payload data,
    /// and check for an additional byte so overlong input is rejected. Reader
    /// errors and malformed input must return [`UWireError`], not panic. A codec
    /// may enforce a lower implementation-specific maximum, but must report that
    /// limit rather than allocating first and rejecting later.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader fails, yields an unexpected byte count, or
    /// contains malformed payload bytes for this codec.
    fn decode_payload_from_reader<R: Read>(reader: R, payload_len: usize) -> Result<T, UWireError>;
}

/// Marker trait for byte-oriented payload codecs.
pub trait BytePayloadCodec: PayloadCodec {}

/// Built-in raw byte payload codec.
#[derive(Debug)]
pub struct RawBytes;

impl RawBytes {
    /// Returns the raw-byte payload encoding metadata.
    #[must_use]
    pub fn encoding() -> PayloadEncoding {
        <Self as PayloadFormat>::encoding()
    }
}

impl PayloadFormat for RawBytes {
    fn name() -> &'static str {
        "raw-bytes"
    }

    fn encoding() -> PayloadEncoding {
        PayloadEncoding::RAW
    }
}

impl BytePayloadCodec for RawBytes {}

impl EncodePayload<[u8]> for RawBytes {
    fn payload_layout(value: &[u8]) -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(value.len(), 1)
    }

    fn encode_payload(value: &[u8], dst: &mut [u8]) -> Result<(), UWireError> {
        let actual = dst.len();
        let out = dst
            .get_mut(..value.len())
            .ok_or_else(|| UWireError::buffer_too_small(value.len(), actual))?;
        out.copy_from_slice(value);
        Ok(())
    }
}

impl<'a> DecodePayload<'a, &'a [u8]> for RawBytes {
    fn decode_payload(src: &'a [u8]) -> Result<&'a [u8], UWireError> {
        Ok(src)
    }
}

impl DecodePayload<'_, Vec<u8>> for RawBytes {
    fn decode_payload(src: &[u8]) -> Result<Vec<u8>, UWireError> {
        Ok(src.to_vec())
    }
}

impl DecodePayload<'_, Bytes> for RawBytes {
    fn decode_payload(src: &[u8]) -> Result<Bytes, UWireError> {
        Ok(Bytes::copy_from_slice(src))
    }
}

impl ReadDecodePayload<Vec<u8>> for RawBytes {
    fn decode_payload_from_reader<R: Read>(
        reader: R,
        payload_len: usize,
    ) -> Result<Vec<u8>, UWireError> {
        read_exact_payload(reader, payload_len)
    }
}

impl ReadDecodePayload<Bytes> for RawBytes {
    fn decode_payload_from_reader<R: Read>(
        reader: R,
        payload_len: usize,
    ) -> Result<Bytes, UWireError> {
        read_exact_payload(reader, payload_len).map(Bytes::from)
    }
}

/// Protocol Buffers application payload codec.
///
/// `ProtobufPayload` serializes and deserializes only application payload bytes.
/// It does not wrap a complete uProtocol frame and does not serialize frame
/// metadata.
#[derive(Debug)]
pub struct ProtobufPayload;

impl ProtobufPayload {
    /// Returns the generic Protocol Buffers payload encoding metadata.
    #[must_use]
    pub fn encoding() -> PayloadEncoding {
        <Self as PayloadFormat>::encoding()
    }
}

impl PayloadFormat for ProtobufPayload {
    fn name() -> &'static str {
        "protobuf"
    }

    fn encoding() -> PayloadEncoding {
        PayloadEncoding::PROTOBUF
    }
}

#[cfg(feature = "protobuf-support")]
impl<T> EncodePayload<T> for ProtobufPayload
where
    T: ProtobufMappable,
{
    fn payload_layout(value: &T) -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(Self::encode_payload_owned(value)?.len(), 1)
    }

    fn encode_payload(value: &T, dst: &mut [u8]) -> Result<(), UWireError> {
        copy_encoded_payload(Self::encode_payload_owned(value)?, dst)
    }

    fn encode_payload_owned(value: &T) -> Result<Bytes, UWireError> {
        value
            .write_to_protobuf_bytes()
            .map(Bytes::from)
            .map_err(|error| UWireError::serialization_error(error.to_string()))
    }
}

#[cfg(feature = "protobuf-support")]
impl<'a, T> DecodePayload<'a, T> for ProtobufPayload
where
    T: ProtobufMappable,
{
    fn decode_payload(src: &'a [u8]) -> Result<T, UWireError> {
        T::parse_from_protobuf_bytes(src)
            .map_err(|error| UWireError::invalid_payload(error.to_string()))
    }
}

#[cfg(feature = "protobuf-support")]
impl<T> ReadDecodePayload<T> for ProtobufPayload
where
    T: ProtobufMappable,
{
    fn decode_payload_from_reader<R: Read>(reader: R, payload_len: usize) -> Result<T, UWireError> {
        let bytes = read_exact_payload(reader, payload_len)?;
        T::parse_from_protobuf_bytes(&bytes)
            .map_err(|error| UWireError::invalid_payload(error.to_string()))
    }
}

/// Protocol Buffers `google.protobuf.Any` application payload codec.
#[derive(Debug)]
pub struct ProtobufAnyPayload;

impl ProtobufAnyPayload {
    /// Returns the protobuf-Any payload encoding metadata.
    #[must_use]
    pub fn encoding() -> PayloadEncoding {
        <Self as PayloadFormat>::encoding()
    }
}

impl PayloadFormat for ProtobufAnyPayload {
    fn name() -> &'static str {
        "protobuf-any"
    }

    fn encoding() -> PayloadEncoding {
        PayloadEncoding::PROTOBUF_WRAPPED_IN_ANY
    }
}

#[cfg(feature = "protobuf-support")]
impl<T> EncodePayload<T> for ProtobufAnyPayload
where
    T: ProtobufMappable,
{
    fn payload_layout(value: &T) -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(Self::encode_payload_owned(value)?.len(), 1)
    }

    fn encode_payload(value: &T, dst: &mut [u8]) -> Result<(), UWireError> {
        copy_encoded_payload(Self::encode_payload_owned(value)?, dst)
    }

    fn encode_payload_owned(value: &T) -> Result<Bytes, UWireError> {
        value
            .write_to_packed_protobuf_bytes()
            .map(Bytes::from)
            .map_err(|error| UWireError::serialization_error(error.to_string()))
    }
}

#[cfg(feature = "protobuf-support")]
impl<'a, T> DecodePayload<'a, T> for ProtobufAnyPayload
where
    T: ProtobufMappable,
{
    fn decode_payload(src: &'a [u8]) -> Result<T, UWireError> {
        T::parse_from_packed_protobuf_bytes(src)
            .map_err(|error| UWireError::invalid_payload(error.to_string()))
    }
}

#[cfg(feature = "protobuf-support")]
impl<T> ReadDecodePayload<T> for ProtobufAnyPayload
where
    T: ProtobufMappable,
{
    fn decode_payload_from_reader<R: Read>(reader: R, payload_len: usize) -> Result<T, UWireError> {
        let bytes = read_exact_payload(reader, payload_len)?;
        T::parse_from_packed_protobuf_bytes(&bytes)
            .map_err(|error| UWireError::invalid_payload(error.to_string()))
    }
}

#[cfg(feature = "protobuf-support")]
fn copy_encoded_payload(bytes: Bytes, dst: &mut [u8]) -> Result<(), UWireError> {
    let actual = dst.len();
    let out = dst
        .get_mut(..bytes.len())
        .ok_or_else(|| UWireError::buffer_too_small(bytes.len(), actual))?;
    out.copy_from_slice(&bytes);
    Ok(())
}

fn read_exact_payload<R: Read>(mut reader: R, payload_len: usize) -> Result<Vec<u8>, UWireError> {
    let mut bytes = Vec::with_capacity(payload_len);
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
    if bytes.len() != payload_len {
        return Err(UWireError::invalid_payload(format!(
            "payload reader yielded {} bytes but payload_len returned {payload_len} bytes",
            bytes.len()
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "zero-copy-transport")]
    use std::io::Chain;
    #[cfg(any(feature = "protobuf-support", feature = "zero-copy-transport"))]
    use std::io::Cursor;

    #[cfg(feature = "protobuf-support")]
    use protobuf::well_known_types::wrappers::StringValue;

    #[cfg(feature = "zero-copy-transport")]
    use crate::{UFrameMetadata, UFrameView, UMessageBuilder, UUri};

    use super::*;

    #[cfg(feature = "zero-copy-transport")]
    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic")
    }

    #[cfg(feature = "protobuf-support")]
    fn message(value: &str) -> StringValue {
        StringValue {
            value: value.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn raw_bytes_owned_encode_decode_round_trips() {
        let payload = b"raw payload".as_slice();

        let encoded = RawBytes::encode_payload_owned(payload).expect("encode raw bytes");
        let decoded: Vec<u8> = RawBytes::decode_payload(&encoded).expect("decode raw bytes");

        assert_eq!(encoded.as_ref(), payload);
        assert_eq!(decoded, payload);
        assert_eq!(RawBytes::encoding(), PayloadEncoding::RAW);
    }

    #[test]
    fn raw_bytes_rejects_too_small_output_buffer() {
        let error = RawBytes::encode_payload(b"payload".as_slice(), &mut [0_u8; 3]).unwrap_err();

        assert_eq!(
            error,
            UWireError::BufferTooSmall {
                expected: 7,
                actual: 3,
            }
        );
    }

    #[cfg(feature = "protobuf-support")]
    #[test]
    fn protobuf_payload_encode_decode_round_trips() {
        let input = message("protobuf payload");

        let encoded = ProtobufPayload::encode_payload_owned(&input).expect("encode protobuf");
        let decoded: StringValue =
            ProtobufPayload::decode_payload(&encoded).expect("decode protobuf");

        assert_eq!(decoded.value, input.value);
        assert_eq!(ProtobufPayload::encoding(), PayloadEncoding::PROTOBUF);
    }

    #[cfg(feature = "protobuf-support")]
    #[test]
    fn protobuf_any_payload_encode_decode_round_trips() {
        let input = message("protobuf any payload");

        let encoded = ProtobufAnyPayload::encode_payload_owned(&input).expect("encode any");
        let decoded: StringValue =
            ProtobufAnyPayload::decode_payload(&encoded).expect("decode any");

        assert_eq!(decoded.value, input.value);
        assert_eq!(
            ProtobufAnyPayload::encoding(),
            PayloadEncoding::PROTOBUF_WRAPPED_IN_ANY
        );
    }

    #[cfg(feature = "protobuf-support")]
    #[test]
    fn protobuf_payload_reader_decode_round_trips() {
        let input = message("protobuf reader payload");
        let encoded = ProtobufPayload::encode_payload_owned(&input).expect("encode protobuf");

        let decoded: StringValue = ProtobufPayload::decode_payload_from_reader(
            Cursor::new(encoded.as_ref()),
            encoded.len(),
        )
        .expect("decode protobuf from reader");

        assert_eq!(decoded.value, input.value);
    }

    #[cfg(feature = "zero-copy-transport")]
    #[test]
    fn segmented_frame_view_decodes_from_reader_without_contiguous_payload() {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(PayloadEncoding::RAW),
        )
        .expect("metadata");
        let frame = SegmentedFrame {
            metadata,
            first: b"seg".to_vec(),
            second: b"mented".to_vec(),
        };

        let decoded: Vec<u8> = frame
            .decode_payload_from_reader_as::<RawBytes, _>()
            .expect("decode segmented payload");

        assert_eq!(decoded, b"segmented".as_slice());
        assert!(frame.try_contiguous_payload().is_none());
    }

    #[cfg(feature = "zero-copy-transport")]
    struct SegmentedFrame {
        metadata: UFrameMetadata,
        first: Vec<u8>,
        second: Vec<u8>,
    }

    #[cfg(feature = "zero-copy-transport")]
    impl UFrameView for SegmentedFrame {
        type PayloadReader<'a>
            = Chain<Cursor<&'a [u8]>, Cursor<&'a [u8]>>
        where
            Self: 'a;
        type PayloadSlices<'a>
            = std::array::IntoIter<&'a [u8], 2>
        where
            Self: 'a;

        fn metadata(&self) -> &UFrameMetadata {
            &self.metadata
        }

        fn payload_len(&self) -> usize {
            self.first.len() + self.second.len()
        }

        fn has_payload(&self) -> bool {
            true
        }

        fn payload_reader(&self) -> Self::PayloadReader<'_> {
            Cursor::new(self.first.as_slice()).chain(Cursor::new(self.second.as_slice()))
        }

        fn payload_slices(&self) -> Self::PayloadSlices<'_> {
            [self.first.as_slice(), self.second.as_slice()].into_iter()
        }
    }
}
