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

/// Payload presence and layout requested for a transmit loan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UTxPayloadSpec {
    /// The frame carries no payload and no payload encoding metadata.
    Absent,
    /// The frame carries a payload, including a present empty payload.
    Present {
        /// Payload length in bytes.
        len: usize,
        /// Required payload alignment.
        alignment: PayloadAlignment,
    },
}

/// Validated visible application payload alignment in bytes.
///
/// The value is always nonzero and a power of two. Runtime values from config,
/// metadata, or transport boundaries must be constructed with [`Self::new`] or
/// [`TryFrom<usize>`] so those boundaries keep their explicit validation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PayloadAlignment(usize);

impl PayloadAlignment {
    /// Alignment for absent payloads and byte-oriented payloads.
    pub const ONE: Self = Self(1);

    /// Creates a payload alignment after validating the nonzero power-of-two invariant.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is zero or not a power of two.
    pub fn new(value: usize) -> Result<Self, UStatus> {
        if value == 0 {
            return Err(invalid_argument(
                "payload alignment must be greater than zero",
            ));
        }
        if !value.is_power_of_two() {
            return Err(invalid_argument("payload alignment must be a power of two"));
        }
        Ok(Self(value))
    }

    /// Returns the raw alignment value in bytes.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

impl TryFrom<usize> for PayloadAlignment {
    type Error = UStatus;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl UTxPayloadSpec {
    /// Creates a present-payload spec from length and alignment.
    ///
    /// # Errors
    ///
    /// Returns an error if `alignment` is zero or not a power of two.
    pub fn present(len: usize, alignment: usize) -> Result<Self, UStatus> {
        let alignment = PayloadAlignment::new(alignment)?;
        Ok(Self::present_with_alignment(len, alignment))
    }

    /// Creates a present-payload spec from length and a validated alignment.
    #[must_use]
    pub fn present_with_alignment(len: usize, alignment: PayloadAlignment) -> Self {
        Self::Present { len, alignment }
    }

    /// Creates a present-empty-payload spec.
    #[must_use]
    pub fn present_empty() -> Self {
        Self::Present {
            len: 0,
            alignment: PayloadAlignment::ONE,
        }
    }

    /// Returns whether this spec represents a present payload.
    #[must_use]
    pub fn is_present(self) -> bool {
        matches!(self, Self::Present { .. })
    }

    /// Returns the visible application payload length.
    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::Absent => 0,
            Self::Present { len, .. } => len,
        }
    }

    /// Returns true if the visible application payload length is zero.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Returns the requested visible application payload alignment.
    #[must_use]
    pub fn alignment(self) -> usize {
        self.payload_alignment().as_usize()
    }

    /// Returns the validated payload alignment proof for this spec.
    #[must_use]
    pub fn payload_alignment(self) -> PayloadAlignment {
        match self {
            Self::Absent => PayloadAlignment::ONE,
            Self::Present { alignment, .. } => alignment,
        }
    }
}

/// Validated transport-independent transmit loan specification.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a transmit loan specification has no effect until passed to a loan method"]
pub struct UTxLoanSpec<S = crate::Validated> {
    metadata: UFrameMetadata,
    payload: UTxPayloadSpec,
    _state: core::marker::PhantomData<S>,
}

impl UTxLoanSpec<crate::Validated> {
    /// Creates a validated transmit loan spec.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata is invalid or if payload presence and payload
    /// encoding metadata disagree.
    pub(crate) fn new(metadata: UFrameMetadata, payload: UTxPayloadSpec) -> Result<Self, UStatus> {
        UTxLoanSpec::new_unchecked(metadata, payload).validate()
    }

    /// Creates a no-payload transmit loan spec.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata is invalid or still carries payload encoding.
    pub fn no_payload(metadata: UFrameMetadata) -> Result<Self, UStatus> {
        Self::new(metadata, UTxPayloadSpec::Absent)
    }

    /// Creates a present-payload transmit loan spec.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata is invalid, has no payload encoding, or if
    /// the requested payload alignment is invalid.
    pub fn payload(
        metadata: UFrameMetadata,
        payload_len: usize,
        alignment: usize,
    ) -> Result<Self, UStatus> {
        Self::new(metadata, UTxPayloadSpec::present(payload_len, alignment)?)
    }

    /// Creates a present-empty-payload transmit loan spec.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata is invalid or has no payload encoding.
    pub fn present_empty_payload(metadata: UFrameMetadata) -> Result<Self, UStatus> {
        Self::new(metadata, UTxPayloadSpec::present_empty())
    }
}

impl UTxLoanSpec<crate::Unvalidated> {
    /// Creates a transmit loan spec without validation.
    #[must_use = "validate the specification before loaning"]
    pub fn new_unchecked(metadata: UFrameMetadata, payload: UTxPayloadSpec) -> Self {
        Self {
            metadata,
            payload,
            _state: core::marker::PhantomData,
        }
    }

    /// Validates the spec, transitioning it to the [`Validated`](crate::Validated) state.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata is invalid or if payload presence and
    /// payload encoding metadata disagree.
    pub fn validate(self) -> Result<UTxLoanSpec<crate::Validated>, UStatus> {
        let spec = UTxLoanSpec {
            metadata: self.metadata,
            payload: self.payload,
            _state: core::marker::PhantomData,
        };
        validate_tx_loan_spec(&spec)?;
        Ok(spec)
    }
}

impl<S> UTxLoanSpec<S> {
    /// Returns the immutable frame metadata associated with this loan.
    #[must_use]
    pub fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    /// Consumes the spec and returns its metadata.
    #[must_use]
    pub fn into_metadata(self) -> UFrameMetadata {
        self.metadata
    }

    /// Returns the payload presence and layout spec.
    #[must_use]
    pub fn payload_spec(&self) -> UTxPayloadSpec {
        self.payload
    }

    /// Returns whether the transmit frame carries a payload.
    #[must_use]
    pub fn has_payload(&self) -> bool {
        self.payload.is_present()
    }

    /// Returns the visible application payload length.
    #[must_use]
    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }

    /// Returns the requested visible application payload alignment.
    #[must_use]
    pub fn payload_alignment(&self) -> usize {
        self.payload.alignment()
    }

    /// Returns the validated payload alignment proof.
    #[must_use]
    pub fn payload_alignment_proof(&self) -> PayloadAlignment {
        self.payload.payload_alignment()
    }
}

/// *Role: transmit storage a zero-copy transport lends; write the payload in place, then commit — see the [trait map](crate::guide::trait_map).*
///
/// Mutable transmit storage reserved from a zero-copy transport.
pub trait UTxBuffer {
    /// Returns the immutable frame metadata associated with this transmit loan.
    fn metadata(&self) -> &UFrameMetadata;

    /// Returns the current payload bytes in the transmit loan.
    fn payload(&self) -> &[u8];

    /// Returns mutable payload storage for direct serialization into the loan.
    fn payload_mut(&mut self) -> &mut [u8];
}

/// *Role: an uninitialized transmit loan; filled safely via the typed init API — see the [trait map](crate::guide::trait_map).*
///
/// Mutable transmit storage whose application payload bytes are not yet initialized.
pub trait UUninitTxBuffer {
    /// Initialized transmit loan type produced after payload initialization.
    type Initialized: UTxBuffer;

    /// Returns the immutable frame metadata associated with this transmit loan.
    fn metadata(&self) -> &UFrameMetadata;

    /// Returns the visible application payload length.
    fn payload_len(&self) -> usize;

    /// Returns mutable uninitialized application payload storage.
    fn payload_uninit_mut(&mut self) -> &mut [MaybeUninit<u8>];

    /// Converts this uninitialized loan into its initialized TX buffer form.
    ///
    /// # Safety
    ///
    /// The caller must guarantee every visible application payload byte has been
    /// initialized before conversion.
    unsafe fn assume_payload_init(self) -> Self::Initialized;
}

/// Mutable uninitialized payload bytes with explicit transport-loan provenance.
pub struct LoanedPayloadUninitMut<'a> {
    bytes: &'a mut [MaybeUninit<u8>],
    provenance: PayloadLoanProvenance,
}

impl<'a> core::fmt::Debug for LoanedPayloadUninitMut<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoanedPayloadUninitMut")
            .finish_non_exhaustive()
    }
}

impl<'a> LoanedPayloadUninitMut<'a> {
    /// Creates a mutable uninitialized loaned payload view from transport-owned storage.
    ///
    /// # Safety
    ///
    /// `bytes` must be a valid mutable uninitialized byte slice for `'a`, with no
    /// other active access path, and must be the exact visible application
    /// payload range for the loan described by `provenance`.
    #[must_use]
    pub unsafe fn new_unchecked(
        bytes: &'a mut [MaybeUninit<u8>],
        provenance: PayloadLoanProvenance,
    ) -> Self {
        Self { bytes, provenance }
    }

    #[must_use]
    /// Returns where this loan's storage came from.
    pub fn provenance(&self) -> PayloadLoanProvenance {
        self.provenance
    }

    #[must_use]
    /// Returns the payload capacity of this loan in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    /// Returns `true` if the loan has zero payload capacity.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    #[cfg(feature = "zero-copy-transport")]
    pub(crate) fn as_uninit_bytes_mut_internal(&mut self) -> &mut [MaybeUninit<u8>] {
        self.bytes
    }

    #[cfg(feature = "zero-copy-transport")]
    pub(crate) fn into_uninit_bytes_mut_internal(self) -> &'a mut [MaybeUninit<u8>] {
        self.bytes
    }
}

/// Verifies the visible transmit payload layout exposed by a zero-copy loan.
///
/// # Errors
///
/// Returns an error if the requested layout is invalid or the exposed payload
/// range does not match it.
pub fn verify_tx_buffer_payload_layout(
    buffer: &mut impl UTxBuffer,
    payload_len: usize,
    alignment: usize,
) -> Result<(), UStatus> {
    validate_payload_layout(payload_len, alignment)?;
    let payload = buffer.payload_mut();
    verify_payload_slice_layout(payload, payload_len, alignment)
}

/// Verifies the visible uninitialized transmit payload layout exposed by a loan.
///
/// # Errors
///
/// Returns an error if the requested layout is invalid or the exposed payload
/// range does not match it.
pub fn verify_uninit_tx_buffer_payload_layout(
    buffer: &mut impl UUninitTxBuffer,
    payload_len: usize,
    alignment: usize,
) -> Result<(), UStatus> {
    validate_payload_layout(payload_len, alignment)?;
    let payload = buffer.payload_uninit_mut();
    if payload.len() != payload_len {
        return Err(internal_zero_copy_error(format!(
            "transport returned uninitialized TX payload length {} but {payload_len} was requested",
            payload.len()
        )));
    }
    if !payload.is_empty() && !(payload.as_ptr() as usize).is_multiple_of(alignment) {
        return Err(internal_zero_copy_error(format!(
            "transport returned uninitialized TX payload alignment {} but {alignment} was requested",
            payload.as_ptr() as usize
        )));
    }
    Ok(())
}
/// Validates a transmit buffer before committing it through a public zero-copy boundary.
///
/// # Errors
///
/// Returns an error if metadata is invalid or initialized payload bytes disagree
/// with payload encoding metadata.
pub fn validate_tx_buffer_for_transport(
    buffer: &mut (impl UTxBuffer + ?Sized),
) -> Result<(), UStatus> {
    validate_metadata(buffer.metadata())?;
    if !buffer.payload().is_empty() && buffer.metadata().payload_encoding().is_none() {
        return Err(invalid_argument(
            "TX buffer carries payload bytes without payload encoding",
        ));
    }
    Ok(())
}
