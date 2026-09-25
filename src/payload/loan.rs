/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use core::mem;

use crate::payload::codec::{DecodePayload, PayloadCodec, PayloadIdentity};
use crate::payload::stable::{StableContainerPayload, StablePayload};
use crate::{NativeProfileAgreement, NativeProfileError, UWireError};

/// Typed borrowing contract for a payload codec.
///
/// This is the application-selected byte-level operation. Frame/lease helpers
/// must first use [`PayloadCodec::verify_metadata`] with their scoped context;
/// this byte-only method cannot establish the identity of an enclosing frame.
///
/// # Safety
///
/// Implementations must not return a reference until size, alignment and every
/// bit-validity requirement of `T` have been established.
pub unsafe trait BorrowPayload<T>: PayloadCodec {
    /// Safely borrows `T` from arbitrary payload bytes after complete validation.
    ///
    /// # Errors
    ///
    /// Returns an error for incompatible length, alignment or bit patterns.
    fn borrow_payload(src: &[u8]) -> Result<&T, UWireError>;

    /// Borrows `T` through the codec's unchecked lane.
    ///
    /// The default remains checked. A codec may override only with an explicit
    /// safety argument proving every omitted validation obligation.
    ///
    /// # Safety
    ///
    /// The caller must satisfy every additional invariant documented by an
    /// overriding implementation. The inherited implementation is always safe.
    unsafe fn borrow_payload_unchecked(src: &[u8]) -> Result<&T, UWireError> {
        Self::borrow_payload(src)
    }
}

impl<T> PayloadCodec for StableContainerPayload<T>
where
    T: StablePayload,
{
    fn codec_name() -> &'static str {
        T::TYPE_NAME
    }

    fn payload_identity(
        profile: Option<&NativeProfileAgreement>,
    ) -> Result<PayloadIdentity, UWireError> {
        let profile = profile.ok_or(NativeProfileError::InvalidProfile(
            "native operation requires a scoped agreement",
        ))?;
        Self::identity(profile).map(PayloadIdentity::Native)
    }
}

// SAFETY: This implementation validates exact layout, address alignment and all
// field bit patterns before its only byte-to-reference cast.
unsafe impl<T> BorrowPayload<T> for StableContainerPayload<T>
where
    T: StablePayload,
{
    fn borrow_payload(src: &[u8]) -> Result<&T, UWireError> {
        StableContainerPayload::<T>::validate_representation()?;
        if src.len() != mem::size_of::<T>() {
            return Err(UWireError::invalid_payload_length(
                mem::size_of::<T>(),
                src.len(),
            ));
        }
        if !(src.as_ptr() as usize).is_multiple_of(mem::align_of::<T>()) {
            return Err(UWireError::invalid_payload(format!(
                "payload address is not aligned to {} bytes",
                mem::align_of::<T>()
            )));
        }
        if !T::validate_field_bytes(src) {
            return Err(UWireError::invalid_payload(format!(
                "payload bytes are not valid for {}",
                T::TYPE_NAME
            )));
        }
        // SAFETY: Exact size and alignment were checked above, and the stable
        // validator proved every field's bit validity before this cast.
        Ok(unsafe { cast_validated_payload(src) })
    }
}

impl<'a, T> DecodePayload<'a, &'a T> for StableContainerPayload<T>
where
    T: StablePayload,
{
    fn decode_payload(src: &'a [u8]) -> Result<&'a T, UWireError> {
        Self::borrow_payload(src)
    }
}

unsafe fn cast_validated_payload<T>(src: &[u8]) -> &T {
    // SAFETY: The caller has proved exact size/alignment and T bit validity.
    unsafe { &*src.as_ptr().cast::<T>() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.Borrowed")]
    struct Borrowed {
        flag: bool,
        padding: [u8; 3],
        letter: char,
    }

    #[test]
    fn arbitrary_slice_borrow_checks_bits_and_alignment() {
        let value = Borrowed {
            flag: true,
            padding: [0; 3],
            letter: 'A',
        };
        let bytes = unsafe {
            core::slice::from_raw_parts(
                core::ptr::from_ref(&value).cast::<u8>(),
                mem::size_of::<Borrowed>(),
            )
        };
        assert_eq!(
            StableContainerPayload::<Borrowed>::borrow_payload(bytes).unwrap(),
            &value
        );

        let mut invalid = bytes.to_vec();
        *invalid.first_mut().expect("non-empty stable payload") = 2;
        assert!(StableContainerPayload::<Borrowed>::borrow_payload(&invalid).is_err());
        assert!(StableContainerPayload::<Borrowed>::borrow_payload(
            bytes.get(..1).expect("one-byte prefix")
        )
        .is_err());
    }
}
