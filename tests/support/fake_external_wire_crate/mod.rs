/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::{io::Read, mem};

use up_rust::{
    DecodePayload, EncodePayload, LoanPayload, PayloadEncoding, PayloadFormat, PayloadLayout,
    PreparedTxLoanSpec, ReadDecodePayload, UFrameMetadata, UMessageBuilder, UStatus, UUri,
    UVecTxBuffer, UWire, UWireError, UWirePayload, UZeroCopyTransportCore, WireIdentity,
    NATIVE_PREFIX_METADATA_LAYOUT_ID,
};

/// Non-production external selected wire used only by `USR-05B` compile fixtures.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FakeExternalWire;

impl UWire for FakeExternalWire {
    const WIRE_ID: WireIdentity = WireIdentity::new("test.userializer.fake-external-wire", 0x8005);
    const PAYLOAD_FAMILY_ID: WireIdentity =
        WireIdentity::new("test.userializer.fake-external-payload", 0x8006);
    const METADATA_LAYOUT_ID: WireIdentity = NATIVE_PREFIX_METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = 1;
}

impl PayloadFormat for FakeExternalWire {
    fn name() -> &'static str {
        "fake-external-wire-fixture"
    }

    fn encoding() -> PayloadEncoding {
        PayloadEncoding::custom(
            "test.fake-external-v1",
            "application/vnd.uprotocol.fake-external;type=ExternalFixture;version=1",
        )
        .expect("fake external payload encoding")
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExternalFixture {
    pub signal_id: u16,
    pub value: u16,
}

impl EncodePayload<ExternalFixture> for FakeExternalWire {
    fn payload_layout(_value: &ExternalFixture) -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(4, 1)
    }

    fn encode_payload(value: &ExternalFixture, dst: &mut [u8]) -> Result<(), UWireError> {
        if dst.len() < 4 {
            return Err(UWireError::buffer_too_small(4, dst.len()));
        }
        let signal_id = value.signal_id.to_le_bytes();
        let value = value.value.to_le_bytes();
        dst[0] = signal_id[0];
        dst[1] = signal_id[1];
        dst[2] = value[0];
        dst[3] = value[1];
        Ok(())
    }
}

impl<'a> DecodePayload<'a, ExternalFixture> for FakeExternalWire {
    fn decode_payload(src: &'a [u8]) -> Result<ExternalFixture, UWireError> {
        if src.len() != 4 {
            return Err(UWireError::invalid_payload_length(4, src.len()));
        }
        Ok(ExternalFixture {
            signal_id: u16::from_le_bytes([src[0], src[1]]),
            value: u16::from_le_bytes([src[2], src[3]]),
        })
    }
}

impl ReadDecodePayload<ExternalFixture> for FakeExternalWire {
    fn decode_payload_from_reader<R: Read>(
        mut reader: R,
        payload_len: usize,
    ) -> Result<ExternalFixture, UWireError> {
        if payload_len != 4 {
            return Err(UWireError::invalid_payload_length(4, payload_len));
        }
        let mut bytes = [0_u8; 4];
        reader
            .read_exact(&mut bytes)
            .map_err(|error| UWireError::invalid_payload(error.to_string()))?;
        Self::decode_payload(&bytes)
    }
}

// SAFETY: `ExternalFixture` is `repr(C)`, contains only integer fields, has no
// padding, and every written bit pattern is valid for its fields.
unsafe impl LoanPayload<ExternalFixture> for FakeExternalWire {
    fn loan_layout() -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(
            mem::size_of::<ExternalFixture>(),
            mem::align_of::<ExternalFixture>(),
        )
    }

    fn loan_payload(dst: &mut [u8]) -> Result<&mut ExternalFixture, UWireError> {
        let expected_len = mem::size_of::<ExternalFixture>();
        if dst.len() != expected_len {
            return Err(UWireError::invalid_payload_length(expected_len, dst.len()));
        }
        let ptr = dst.as_mut_ptr();
        let align = mem::align_of::<ExternalFixture>();
        if ptr.align_offset(align) != 0 {
            return Err(UWireError::invalid_payload(format!(
                "payload address is not aligned to {align}"
            )));
        }

        let ptr = ptr.cast::<ExternalFixture>();
        // SAFETY: The length and alignment checks above prove the destination can
        // hold one `ExternalFixture`; the type has no invalid integer bit patterns.
        unsafe {
            ptr.write(ExternalFixture::default());
            Ok(&mut *ptr)
        }
    }
}

impl UWirePayload<ExternalFixture> for FakeExternalWire {
    type Codec = Self;
}

pub fn metadata() -> UFrameMetadata {
    let topic = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic URI");
    let message = UMessageBuilder::publish(topic).build().expect("message");
    message
        .attributes()
        .to_frame_metadata(FakeExternalWire::encoding())
        .expect("metadata")
}

#[derive(Default)]
pub struct ExternalTxCore;

#[async_trait::async_trait]
impl UZeroCopyTransportCore for ExternalTxCore {
    type Tx = UVecTxBuffer;
    type Rx = EmptyRx;

    async fn loan_prepared_tx(&self, spec: PreparedTxLoanSpec) -> Result<Self::Tx, UStatus> {
        UVecTxBuffer::with_alignment(
            spec.metadata().clone(),
            spec.payload_len(),
            spec.payload_alignment(),
        )
    }

    async fn send_prepared_zero_copy(&self, _buffer: Self::Tx) -> Result<(), UStatus> {
        Ok(())
    }
}

pub struct EmptyRx;

impl up_rust::UEncodedRxFrame for EmptyRx {
    type PayloadReader<'a>
        = std::io::Cursor<&'a [u8]>
    where
        Self: 'a;
    type PayloadSlices<'a>
        = std::iter::Empty<&'a [u8]>
    where
        Self: 'a;

    fn encoded_metadata(&self) -> &[u8] {
        &[]
    }

    fn payload_len(&self) -> usize {
        0
    }

    fn payload_reader(&self) -> Self::PayloadReader<'_> {
        std::io::Cursor::new(&[][..])
    }

    fn payload_slices(&self) -> Self::PayloadSlices<'_> {
        std::iter::empty()
    }
}
