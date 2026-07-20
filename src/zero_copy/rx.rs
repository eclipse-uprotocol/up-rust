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

use super::*;

pub use crate::frame::view::{validate_frame_view_for_transport, UFrameView};

/// *Role: a received payload read in place; dropping it returns the storage — see the [trait map](crate::guide::trait_map).*
///
/// Receive-side zero-copy frame lease returned by a transport.
pub trait UZeroCopyRxLease: UFrameView {}

/// *Role: a receive lease over contiguous loan-backed storage; lets typed payloads be borrowed without copying — see the [trait map](crate::guide::trait_map).*
///
/// Receive lease that can expose a contiguous payload from loan-backed storage.
pub trait ULoanedContiguousZeroCopyRxFrame: UZeroCopyRxLease {
    /// Returns one contiguous loan-backed application payload view.
    ///
    /// Implementations must not allocate, copy, or coalesce payload bytes to
    /// satisfy this method.
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, UWireError>;

    /// Returns diagnostic provenance for successful loaned payload borrows.
    fn payload_loan_provenance(&self) -> Result<PayloadLoanProvenance, UWireError> {
        Ok(self.loaned_contiguous_payload()?.provenance())
    }

    /// Returns only loan-backed contiguous payload bytes.
    fn try_loaned_contiguous_payload(&self) -> Result<&[u8], UWireError> {
        Ok(self.loaned_contiguous_payload()?.as_bytes())
    }

    /// Borrows one stable-container value from loan-backed contiguous storage.
    fn borrow_stable_payload<T>(&self) -> Result<&T, UWireError>
    where
        T: StablePayload,
    {
        self.borrow_payload_as::<StableContainerPayload<T>, T>()
    }

    /// Borrows one typed value from loan-backed contiguous storage using codec `C`.
    ///
    /// This is the low-level codec-selected receive form. Selected-wire receive
    /// wrappers expose a wire-selected `borrow_payload` helper so ordinary callers
    /// do not need to name `C`.
    fn borrow_payload_as<C, T>(&self) -> Result<&T, UWireError>
    where
        C: BorrowPayload<T>,
    {
        C::verify_encoding(self.metadata().payload_encoding())?;
        if !self.has_payload() {
            return Err(UWireError::MissingPayload);
        }
        let payload = self.loaned_contiguous_payload()?;
        C::borrow_payload(payload.as_bytes())
    }
}

/// *Role: implemented by applications to receive zero-copy leases — see the [trait map](crate::guide::trait_map).*
///
/// A handler for processing zero-copy receive leases.
#[async_trait]
pub trait UZeroCopyListener<Rx>: Send + Sync
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    /// Handles one received zero-copy frame lease.
    async fn on_receive_zero_copy(&self, frame: Rx);
}
