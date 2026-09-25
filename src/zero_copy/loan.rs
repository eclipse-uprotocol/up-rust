/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use crate::UWireError;

/// Validated power-of-two payload alignment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PayloadAlignment(pub(crate) usize);

impl PayloadAlignment {
    /// Validates a payload alignment.
    ///
    /// # Errors
    ///
    /// Returns an error for zero or non-power-of-two values.
    pub fn new(alignment: usize) -> Result<Self, UWireError> {
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(UWireError::invalid_payload(format!(
                "payload alignment {alignment} must be a non-zero power of two"
            )));
        }
        Ok(Self(alignment))
    }

    /// Returns the alignment as `usize`.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Provenance class for one contiguous receive loan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadLoanProvenance {
    /// Storage is owned by a public receive lease.
    OwnedReceiveLease,
    /// Storage is loaned by an opaque physical transport.
    OpaqueTransportLoan,
}

/// Contiguous payload bytes carrying explicit loan provenance.
#[derive(Clone, Copy, Debug)]
pub struct LoanedPayload<'a> {
    bytes: &'a [u8],
    provenance: PayloadLoanProvenance,
}

impl<'a> LoanedPayload<'a> {
    /// Creates a loan after the caller has established its provenance.
    ///
    /// # Safety
    ///
    /// `bytes` must remain backed by the represented receive loan for `'a` and
    /// must not have been allocated, copied or coalesced for this constructor.
    #[must_use]
    pub const unsafe fn new_unchecked(bytes: &'a [u8], provenance: PayloadLoanProvenance) -> Self {
        Self { bytes, provenance }
    }

    /// Returns the contiguous payload bytes.
    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// Returns the loan provenance.
    #[must_use]
    pub const fn provenance(self) -> PayloadLoanProvenance {
        self.provenance
    }
}
