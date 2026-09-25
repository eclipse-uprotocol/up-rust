/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

mod loan;
mod rx;
mod transport;
mod tx;

pub use loan::{LoanedPayload, PayloadAlignment, PayloadLoanProvenance};
pub use rx::{ULoanedContiguousZeroCopyRxFrame, UVecRxLease, UZeroCopyRxLease};
pub use transport::{
    UZeroCopyListener, UZeroCopyTransport, UZeroCopyTransportExt, UZeroCopyTransportImpl,
    UZeroCopyUninitTransportImpl,
};
pub use tx::{
    UTxBuffer, UTxLoanSpec, UTxPayloadSpec, UUninitTxBuffer, UVecTxBuffer, UVecUninitTxBuffer,
};

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of, MaybeUninit};

    use crate::payload::stable::{StablePayloadField as _, StablePayloadInit as _};
    use crate::{
        NativeProfile, NativeProfileAgreement, NativeProfileMode, NativeProfileTable, PayloadCodec,
        PayloadEncoding, StableContainerPayload, StablePayload, UFrameMetadata, UMessageBuilder,
        UTxBuffer, UUninitTxBuffer, UUri,
    };
    use std::sync::Arc;

    use super::*;

    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload, crate::StablePayloadInit)]
    #[stable_payload(type_name = "uprotocol.test.ZeroCopy")]
    struct ZeroCopyValue {
        bytes: [u8; 4],
    }

    fn metadata() -> UFrameMetadata {
        let topic = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();
        UFrameMetadata::publish(topic)
            .with_native_payload_identity(
                StableContainerPayload::<ZeroCopyValue>::identity(&profile()).unwrap(),
            )
            .build()
            .unwrap()
    }

    fn profile() -> NativeProfileAgreement {
        let table = NativeProfileTable::new([(
            PayloadEncoding::from_id(0xF101).unwrap(),
            ZeroCopyValue::native_representation(),
        )])
        .unwrap();
        let profile = NativeProfile::new("test", 1, NativeProfileMode::Table(table)).unwrap();
        NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap()
    }

    #[test]
    fn aligned_initialized_and_uninitialized_buffers_preserve_layout() {
        let mut buffer = UVecTxBuffer::with_alignment(
            metadata(),
            size_of::<ZeroCopyValue>(),
            align_of::<ZeroCopyValue>(),
        )
        .unwrap();
        assert_eq!(
            (buffer.payload().as_ptr() as usize) % align_of::<ZeroCopyValue>(),
            0
        );
        buffer.payload_mut().copy_from_slice(b"wire");

        let mut uninit = UVecUninitTxBuffer::with_alignment(
            metadata(),
            size_of::<ZeroCopyValue>(),
            align_of::<ZeroCopyValue>(),
        )
        .unwrap();
        let initialized = ZeroCopyValue::init(uninit.payload_uninit_mut())
            .unwrap()
            .bytes_from_slice(b"test")
            .unwrap()
            .finish();
        assert!(ZeroCopyValue::validate_field_bytes(initialized.as_bytes()));
        let _ = initialized;
        let initialized_buffer = unsafe { uninit.assume_payload_initialized() };
        assert_eq!(initialized_buffer.payload(), b"test");
    }

    #[test_case::test_case(false; "private table agreement")]
    #[test_case::test_case(true; "explicit contract defined agreement")]
    fn receive_lease_borrows_valid_stable_payload(contract_defined: bool) {
        let agreement = if contract_defined {
            let contract =
                crate::NativeContract::new("test.event", ZeroCopyValue::native_representation())
                    .unwrap();
            let profile =
                NativeProfile::new("test", 1, NativeProfileMode::ContractDefined(contract))
                    .unwrap();
            NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap()
        } else {
            profile()
        };
        let identity = StableContainerPayload::<ZeroCopyValue>::identity(&agreement).unwrap();
        let metadata = metadata()
            .without_payload_encoding()
            .with_native_payload_identity(identity)
            .unwrap();
        let lease = UVecRxLease::new(metadata, Some(b"wire".to_vec())).unwrap();
        assert_eq!(
            lease
                .borrow_stable_payload::<ZeroCopyValue>(&agreement)
                .unwrap(),
            &ZeroCopyValue { bytes: *b"wire" }
        );
        assert_eq!(
            lease.payload_loan_provenance().unwrap(),
            PayloadLoanProvenance::OwnedReceiveLease
        );
    }

    #[test]
    fn loan_spec_rejects_metadata_payload_mismatch() {
        let topic = UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).unwrap();
        let message = UMessageBuilder::publish(topic).build().unwrap();
        let no_payload = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            None,
        )
        .unwrap();
        assert!(UTxLoanSpec::new(
            no_payload,
            UTxPayloadSpec::Present {
                len: 1,
                alignment: PayloadAlignment::new(1).unwrap(),
            },
        )
        .is_err());
    }

    #[test]
    fn native_codec_rejects_missing_context() {
        assert!(StableContainerPayload::<ZeroCopyValue>::payload_identity(None).is_err());
    }

    #[test]
    fn fixed_decoder_does_not_ignore_native_identity() {
        use crate::UFrameView;
        let metadata = metadata()
            .without_payload_encoding()
            .with_payload_encoding(PayloadEncoding::RAW)
            .unwrap()
            .with_native_type_token(crate::NativeTypeToken::from_u32(0))
            .unwrap();
        let lease = UVecRxLease::new(metadata, Some(b"wire".to_vec())).unwrap();
        assert!(lease
            .decode_payload_from_reader_as::<crate::payload::codec::RawBytes, Vec<u8>>(
                crate::PayloadDecodeLimit::new(4)
            )
            .is_err());
    }

    #[test_case::test_case(4; "scalar alignment")]
    #[test_case::test_case(64; "cache line alignment")]
    #[test_case::test_case(4096; "page alignment")]
    fn cloning_transmit_buffer_preserves_alignment_and_bytes(alignment: usize) {
        let mut original = UVecTxBuffer::with_alignment(metadata(), 4, alignment).unwrap();
        original.payload_mut().copy_from_slice(b"wire");
        let cloned = original.clone();
        assert_eq!((cloned.payload().as_ptr() as usize) % alignment, 0);
        assert_eq!(cloned.payload(), b"wire");
        original.payload_mut().fill(0);
        assert_eq!(cloned.payload(), b"wire");
    }

    #[test_case::test_case(None; "missing carried token")]
    #[test_case::test_case(Some(0); "wrong carried token")]
    fn receive_lease_rejects_identity_before_borrow(token: Option<u32>) {
        let encoding = PayloadEncoding::from_id(0xF101).unwrap();
        let mut metadata = metadata()
            .without_payload_encoding()
            .with_payload_encoding(encoding)
            .unwrap();
        if let Some(token) = token {
            metadata = metadata
                .with_native_type_token(crate::NativeTypeToken::from_u32(token))
                .unwrap();
        }
        let lease = UVecRxLease::new(metadata, Some(b"wire".to_vec())).unwrap();
        assert!(lease
            .borrow_stable_payload::<ZeroCopyValue>(&profile())
            .is_err());
    }

    #[test]
    fn maybe_uninit_byte_layout_matches_bytes() {
        assert_eq!(size_of::<MaybeUninit<u8>>(), 1);
    }
}
