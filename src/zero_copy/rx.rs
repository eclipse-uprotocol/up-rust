/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::io::Cursor;

use crate::frame::UFrameView;
use crate::payload::codec::PayloadCodec;
use crate::payload::loan::BorrowPayload;
use crate::payload::stable::{StableContainerPayload, StablePayload};
use crate::zero_copy::loan::{LoanedPayload, PayloadLoanProvenance};
use crate::NativeProfileAgreement;
use crate::{validate_frame_view_for_transport, UFrameMetadata, UStatus, UWireError};

/// Public receive lease for zero-copy transport families.
///
/// A delivered lease retains the storage and native owner objects needed by its
/// views until the lease is dropped. Unregistering a listener or dropping the
/// transport must not revoke outstanding leases. Bindings defer destruction of
/// native subscriptions/mappings as necessary; callers need not copy or drain
/// retained leases merely to stop delivery.
///
/// This base trait specifies storage lifetime and frame access, not a measured
/// end-to-end no-copy guarantee. A binding may adapt owned receive storage to
/// this API. Contiguous loan access and its provenance are separate capabilities
/// exposed by [`ULoanedContiguousZeroCopyRxFrame`]; physical copy behavior must
/// also be documented by the binding.
pub trait UZeroCopyRxLease: UFrameView {}

/// Receive frame that exposes one contiguous loan-backed payload.
pub trait ULoanedContiguousZeroCopyRxFrame: UZeroCopyRxLease {
    /// Returns one contiguous loan-backed payload.
    ///
    /// # Errors
    ///
    /// Returns an error when contiguous loan-backed storage is unavailable.
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError>;

    /// Returns the payload loan provenance.
    ///
    /// # Errors
    ///
    /// Returns an error when a payload loan is unavailable.
    fn payload_loan_provenance(&self) -> Result<PayloadLoanProvenance, UWireError> {
        self.loaned_contiguous_payload()
            .map(LoanedPayload::provenance)
    }

    /// Safely borrows a stable payload from the receive loan.
    ///
    /// # Errors
    ///
    /// Returns an error for missing, incompatible or invalid payload bytes.
    fn borrow_stable_payload<T>(&self, profile: &NativeProfileAgreement) -> Result<&T, UWireError>
    where
        T: StablePayload,
    {
        if let Some(retained) = self.native_profile() {
            if retained.profile() != profile.profile() {
                return Err(crate::NativeProfileError::ProfileMismatch.into());
            }
        }
        StableContainerPayload::<T>::verify_metadata(self.metadata(), Some(profile))?;
        let payload = self.loaned_contiguous_payload()?;
        StableContainerPayload::<T>::borrow_payload(payload.bytes())
    }
}

/// In-memory receive lease used by adapters and tests.
#[derive(Clone, Debug)]
pub struct UVecRxLease {
    metadata: UFrameMetadata,
    payload: Option<Vec<u8>>,
}

impl UVecRxLease {
    /// Creates and validates an in-memory receive lease.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata and payload presence are inconsistent.
    pub fn new(metadata: UFrameMetadata, payload: Option<Vec<u8>>) -> Result<Self, UStatus> {
        let lease = Self { metadata, payload };
        validate_frame_view_for_transport(&lease)?;
        Ok(lease)
    }
}

impl UFrameView for UVecRxLease {
    type PayloadReader<'a> = Cursor<&'a [u8]>;
    type PayloadSlices<'a> = std::option::IntoIter<&'a [u8]>;

    fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    fn payload_len(&self) -> usize {
        self.payload.as_deref().map_or(0, <[u8]>::len)
    }

    fn has_payload(&self) -> bool {
        self.payload.is_some()
    }

    fn payload_reader(&self) -> Self::PayloadReader<'_> {
        Cursor::new(self.payload.as_deref().unwrap_or_default())
    }

    fn payload_slices(&self) -> Self::PayloadSlices<'_> {
        self.payload.as_deref().into_iter()
    }

    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }
}

impl UZeroCopyRxLease for UVecRxLease {}

impl ULoanedContiguousZeroCopyRxFrame for UVecRxLease {
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError> {
        let bytes = self.payload.as_deref().ok_or(UWireError::MissingPayload)?;
        // SAFETY: The bytes are owned by this receive lease for the returned
        // borrow lifetime; no copy or coalescing occurs here.
        Ok(
            unsafe {
                LoanedPayload::new_unchecked(bytes, PayloadLoanProvenance::OwnedReceiveLease)
            },
        )
    }
}
