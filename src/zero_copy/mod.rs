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

//! Zero-copy loans, receive leases, and transport contracts.

use std::{
    any::Any,
    collections::HashMap,
    io::Cursor,
    mem::MaybeUninit,
    ops::Deref,
    sync::{Arc, LazyLock, Mutex},
};

use async_trait::async_trait;
#[cfg(any(test, feature = "test-util"))]
use std::collections::VecDeque;
use tracing::warn;

#[cfg(feature = "zero-copy-transport")]
use crate::payload::{
    loan::{LoanUninitPayload, LoanedInitPayload, LoanedUninitPayload},
    stable::{InitializedStablePayload, StablePayloadInit, StablePayloadInitContext},
};
#[cfg(feature = "selected-wire-transport-adapter")]
use crate::wire::UWirePayload;
#[cfg(feature = "selected-wire-transport-adapter")]
use crate::wire_transport::UHasWire;
use crate::{
    payload::{
        codec::PayloadCodec,
        loan::{BorrowPayload, LoanPayload},
        stable::{StableContainerPayload, StablePayload},
        UWireError,
    },
    utransport::verify_filter_criteria,
    UCode, UFrameMetadata, UStatus, UUri,
};

mod rx;
mod tests;
mod transport;
mod tx;

pub use rx::*;
#[cfg(feature = "test-util")]
pub use tests::InMemoryZeroCopyTransport;
pub use tests::{UVecRxLease, UVecTxBuffer, UVecUninitTxBuffer};
pub use transport::UZeroCopyUninitTransportImpl;
#[cfg(feature = "perf-diagnostics")]
#[doc(hidden)]
pub use transport::UninitStableSendPhases;
pub use transport::{UZeroCopyTransport, UZeroCopyTransportExt, UZeroCopyTransportImpl};
#[cfg(feature = "zero-copy-transport")]
pub use transport::{UZeroCopyUninitTransport, UZeroCopyUninitTransportExt};
pub use tx::*;

/// Diagnostic provenance for loan-backed payload storage.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PayloadLoanProvenance {
    /// Payload bytes are backed by a transport loan whose domain is opaque to up-rust.
    OpaqueTransportLoan,
}

/// Immutable payload bytes with explicit transport-loan provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoanedPayload<'a> {
    bytes: &'a [u8],
    provenance: PayloadLoanProvenance,
}

impl<'a> LoanedPayload<'a> {
    /// Creates a loaned payload view from transport-owned storage.
    ///
    /// # Safety
    ///
    /// Callers must guarantee `bytes` stays valid and immutable for `'a`, and
    /// that the storage was not allocated or coalesced solely to manufacture a
    /// zero-copy receive proof.
    #[must_use]
    pub unsafe fn new_unchecked(bytes: &'a [u8], provenance: PayloadLoanProvenance) -> Self {
        Self { bytes, provenance }
    }

    #[must_use]
    /// Returns where this payload's storage came from.
    pub fn provenance(self) -> PayloadLoanProvenance {
        self.provenance
    }

    #[must_use]
    /// Returns the payload as one contiguous byte slice.
    pub fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }

    #[must_use]
    /// Returns the payload length in bytes.
    pub fn len(self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    /// Returns `true` if the payload is empty.
    pub fn is_empty(self) -> bool {
        self.bytes.is_empty()
    }
}

impl AsRef<[u8]> for LoanedPayload<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl Deref for LoanedPayload<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.bytes
    }
}

fn validate_tx_loan_spec(spec: &UTxLoanSpec) -> Result<(), UStatus> {
    validate_metadata(spec.metadata())?;
    validate_payload_presence(
        spec.has_payload(),
        spec.metadata().payload_encoding().is_some(),
        "TX loan spec",
    )?;
    validate_payload_layout(spec.payload_len(), spec.payload_alignment())
}

pub(crate) use crate::frame::view::{
    frame_metadata_error, validate_metadata, validate_payload_presence,
};

fn validate_payload_layout(payload_len: usize, alignment: usize) -> Result<(), UStatus> {
    PayloadAlignment::new(alignment)?;
    let _ = payload_len;
    Ok(())
}

fn verify_payload_slice_layout(
    payload: &[u8],
    payload_len: usize,
    alignment: usize,
) -> Result<(), UStatus> {
    if payload.len() != payload_len {
        return Err(internal_zero_copy_error(format!(
            "transport returned TX payload length {} but {payload_len} was requested",
            payload.len()
        )));
    }
    if !payload.is_empty() && !(payload.as_ptr() as usize).is_multiple_of(alignment) {
        return Err(internal_zero_copy_error(format!(
            "transport returned TX payload alignment {} but {alignment} was requested",
            payload.as_ptr() as usize
        )));
    }
    Ok(())
}

fn aligned_offset(address: usize, alignment: usize) -> usize {
    if alignment == 1 {
        0
    } else {
        (alignment - (address % alignment)) % alignment
    }
}

fn invalid_argument(message: impl Into<String>) -> UStatus {
    UStatus::fail_with_code(UCode::InvalidArgument, message.into())
}

fn internal_zero_copy_error(message: impl Into<String>) -> UStatus {
    UStatus::fail_with_code(UCode::Internal, message.into())
}
