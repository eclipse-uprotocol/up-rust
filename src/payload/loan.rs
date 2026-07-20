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

#[cfg(feature = "zero-copy-transport")]
use std::{mem, ptr::NonNull};

#[cfg(feature = "zero-copy-transport")]
use crate::zero_copy::LoanedPayloadUninitMut;

use super::{codec::PayloadCodec, codec::PayloadLayout, UWireError};

/// Borrows a typed payload directly from contiguous receive storage.
///
/// This is the receive-side true zero-copy codec hook. Implementors must not
/// allocate, copy, or deserialize into owned storage before returning `&T`.
///
/// # Safety
///
/// Implementors must validate that `src` has the exact length and alignment for
/// one initialized `T`, that the bytes are a valid representation for `T` under
/// this codec, and that returning `&T` cannot observe uninitialized padding or
/// process-local state.
pub unsafe trait BorrowPayload<T>: PayloadCodec {
    /// Borrows `T` from contiguous payload bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload bytes cannot be borrowed as one valid `T`.
    fn borrow_payload(src: &[u8]) -> Result<&T, UWireError>;
}

/// Initializes and borrows a typed value directly in initialized transmit storage.
///
/// # Safety
///
/// Implementors must guarantee that [`LoanPayload::loan_payload`] returns
/// `&mut T` only when the destination byte range is uniquely borrowed, has the
/// exact layout returned by [`LoanPayload::loan_layout`], and contains one valid
/// initialized `T` for the returned lifetime.
pub unsafe trait LoanPayload<T>: PayloadCodec {
    /// Returns the exact layout required for a typed initialized transmit loan.
    ///
    /// # Errors
    ///
    /// Returns an error if this codec cannot loan `T` into initialized TX storage.
    fn loan_layout() -> Result<PayloadLayout, UWireError>;

    /// Initializes `dst` and returns a typed mutable view over the loaned payload.
    ///
    /// # Errors
    ///
    /// Returns an error if `dst` does not have the required length or alignment.
    fn loan_payload(dst: &mut [u8]) -> Result<&mut T, UWireError>;
}

/// Initializes a typed value directly in uninitialized transmit storage.
///
/// # Safety
///
/// Implementors must guarantee that [`LoanUninitPayload::loan_uninit_payload`]
/// validates the destination range and only returns a slot when the range is one
/// uniquely borrowed allocation, has the exact layout returned by
/// [`LoanUninitPayload::loan_uninit_layout`], and is valid for writes of one
/// `MaybeUninit<T>`.
#[cfg(feature = "zero-copy-transport")]
pub unsafe trait LoanUninitPayload<T>: PayloadCodec {
    /// Returns the exact layout required for a typed uninitialized transmit loan.
    ///
    /// # Errors
    ///
    /// Returns an error if this codec cannot loan `T` into uninitialized TX storage.
    fn loan_uninit_layout() -> Result<PayloadLayout, UWireError>;

    /// Validates `dst` and returns an uninitialized typed payload slot.
    ///
    /// # Errors
    ///
    /// Returns an error if `dst` does not have the required length or alignment.
    fn loan_uninit_payload<'a>(
        dst: LoanedPayloadUninitMut<'a>,
    ) -> Result<LoanedUninitPayload<'a, T>, UWireError>;
}

/// Uninitialized typed payload slot borrowed from a transmit loan.
#[cfg(feature = "zero-copy-transport")]
pub struct LoanedUninitPayload<'a, T> {
    // Safe-by-construction: an exclusive borrow of the uninitialized target.
    slot: &'a mut mem::MaybeUninit<T>,
}

#[cfg(feature = "zero-copy-transport")]
impl<'a, T> core::fmt::Debug for LoanedUninitPayload<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoanedUninitPayload")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<'a, T> LoanedUninitPayload<'a, T> {
    /// Wraps an exclusive borrow of the uninitialized target value.
    #[must_use]
    pub fn new(slot: &'a mut mem::MaybeUninit<T>) -> Self {
        Self { slot }
    }

    /// Initializes the target by move and returns the initialized handle.
    ///
    /// Fully safe: `MaybeUninit::write` performs the move and hands back the
    /// initialized reference; no copy occurs beyond the value move itself.
    #[must_use]
    pub fn write(self, value: T) -> LoanedInitPayload<'a, T> {
        LoanedInitPayload {
            value: self.slot.write(value),
        }
    }

    pub(crate) fn uninit_ptr(&self) -> NonNull<mem::MaybeUninit<T>> {
        NonNull::from(&*self.slot)
    }
}

/// An initialized, exclusively borrowed payload value inside loaned storage.
#[cfg(feature = "zero-copy-transport")]
pub struct LoanedInitPayload<'a, T> {
    value: &'a mut T,
}

#[cfg(feature = "zero-copy-transport")]
impl<'a, T> core::fmt::Debug for LoanedInitPayload<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoanedInitPayload").finish_non_exhaustive()
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<'a, T> LoanedInitPayload<'a, T> {
    pub(crate) fn initialized_ptr(&self) -> NonNull<T> {
        NonNull::from(&*self.value)
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<T> AsMut<T> for LoanedInitPayload<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.value
    }
}

#[cfg(feature = "zero-copy-transport")]
impl<T> AsRef<T> for LoanedInitPayload<'_, T> {
    fn as_ref(&self) -> &T {
        self.value
    }
}
