/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use core::{
    marker::PhantomData,
    mem,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use crate::{
    NativeByteOrder, NativeField, NativePayloadIdentity, NativeProfileAgreement,
    NativeRepresentation, NativeTypeToken, PayloadEncoding, UWireError,
};

/// Field-level bit-validity contract used by stable payload validation.
///
/// # Safety
///
/// Implementations must return `true` only when `bytes` has exactly the layout
/// of `Self` and every represented bit pattern is valid for `Self`.
/// Every valid value must have fully initialized bytes, with no implicit padding,
/// pointers, borrowed state or drop glue. Its structural descriptor must be exact.
pub unsafe trait StablePayloadField: Sized {
    /// Whether every bit pattern of exactly `size_of::<Self>()` bytes is valid.
    const FIELD_BITS_ALWAYS_VALID: bool;

    /// Describes the complete field representation, including nested structure.
    fn field_representation() -> NativeRepresentation;

    /// Validates one field representation without constructing `Self`.
    fn validate_field_bytes(bytes: &[u8]) -> bool;
}

/// Stable in-memory payload contract for a deployment-agreed ABI domain.
///
/// # Safety
///
/// Implementors must have a fixed layout, fully initialized object
/// representation, and a validator that rejects every invalid bit pattern.
pub unsafe trait StablePayload: StablePayloadField + Sized + 'static {
    /// Cross-language stable type name.
    const TYPE_NAME: &'static str;
    /// Layout variant.
    const VARIANT: u32 = 0;
    /// Returns this type's structural representation, independent of allocation.
    fn native_representation() -> NativeRepresentation {
        Self::field_representation()
    }

    /// Returns the structural token; this is never a payload encoding ID.
    fn native_type_token() -> NativeTypeToken {
        Self::native_representation().token()
    }
}

/// Typestate marker for an uninitialized stable field.
#[derive(Debug)]
pub struct StablePayloadInitUnset;

/// Typestate marker for an initialized stable field.
#[derive(Debug)]
pub struct StablePayloadInitSet;

/// Derive-generated safe initialization entry point for a stable payload.
pub trait StablePayloadInit: StablePayload {
    /// Typestate initializer generated for this payload.
    type Initializer<'a>
    where
        Self: 'a;

    /// Starts initializing `Self` in caller-provided byte storage.
    ///
    /// # Errors
    ///
    /// Returns an error unless storage has exact size and alignment.
    fn init(storage: &mut [MaybeUninit<u8>]) -> Result<Self::Initializer<'_>, UWireError>;
}

/// Witness that every field of a stable payload has been initialized.
#[derive(Debug)]
pub struct InitializedStablePayload<'a, T>
where
    T: StablePayload,
{
    value: &'a mut T,
}

impl<T> Deref for InitializedStablePayload<'_, T>
where
    T: StablePayload,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> DerefMut for InitializedStablePayload<'_, T>
where
    T: StablePayload,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<T> InitializedStablePayload<'_, T>
where
    T: StablePayload,
{
    /// Returns the fully initialized object representation.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: The derive-generated typestate reaches this witness only after
        // every field was written; the initializer zeroes all padding first.
        unsafe {
            core::slice::from_raw_parts(
                core::ptr::from_ref::<T>(self.value).cast::<u8>(),
                mem::size_of::<T>(),
            )
        }
    }
}

/// Prepares exact aligned storage for a derive-generated initializer.
#[doc(hidden)]
pub fn prepare_stable_payload_storage<T>(
    storage: &mut [MaybeUninit<u8>],
) -> Result<*mut T, UWireError>
where
    T: StablePayload,
{
    StableContainerPayload::<T>::validate_representation()?;
    if storage.len() != mem::size_of::<T>() {
        return Err(UWireError::invalid_payload_length(
            mem::size_of::<T>(),
            storage.len(),
        ));
    }
    if !(storage.as_ptr() as usize).is_multiple_of(mem::align_of::<T>()) {
        return Err(UWireError::invalid_payload(format!(
            "stable initialization storage is not aligned to {} bytes",
            mem::align_of::<T>()
        )));
    }
    storage.fill(MaybeUninit::new(0));
    Ok(storage.as_mut_ptr().cast::<T>())
}

/// Writes one derive-validated field into prepared storage.
#[doc(hidden)]
pub unsafe fn write_stable_payload_field<T, F>(payload: *mut T, offset: usize, value: F) {
    // SAFETY: The derive computes an in-bounds field offset for F and permits
    // this function only once for that field through typestate.
    unsafe { payload.cast::<u8>().add(offset).cast::<F>().write(value) };
}

/// Writes one array field element directly without constructing the array.
#[doc(hidden)]
pub unsafe fn write_stable_payload_array<T, E, const N: usize, F>(
    payload: *mut T,
    offset: usize,
    mut element: F,
) where
    F: FnMut(usize) -> E,
{
    let array = unsafe { payload.cast::<u8>().add(offset).cast::<E>() };
    let mut index = 0;
    while index < N {
        // SAFETY: The derive proves this field is [E; N], and each index is
        // visited exactly once while it is strictly less than N.
        unsafe { array.add(index).write(element(index)) };
        index += 1;
    }
}

/// Creates the final witness after derive-generated typestate reaches all-set.
#[doc(hidden)]
pub unsafe fn finish_stable_payload_init<'a, T>(payload: *mut T) -> InitializedStablePayload<'a, T>
where
    T: StablePayload,
{
    // SAFETY: All fields were initialized exactly once and padding was zeroed.
    InitializedStablePayload {
        value: unsafe { &mut *payload },
    }
}

/// Stable-container payload codec marker.
#[derive(Debug)]
pub struct StableContainerPayload<T>(PhantomData<fn() -> T>);

impl<T> StableContainerPayload<T>
where
    T: StablePayload,
{
    /// Resolves this complete type under an explicit deployment agreement.
    ///
    /// # Errors
    ///
    /// Rejects a wrong descriptor or a type absent from the agreed profile.
    pub fn identity(profile: &NativeProfileAgreement) -> Result<NativePayloadIdentity, UWireError> {
        let representation = Self::validate_representation()?;
        profile
            .identity_for(&representation)
            .map_err(UWireError::from)
    }

    /// Returns the profile's encoding allocation for this complete type.
    ///
    /// # Errors
    ///
    /// Returns the same representation/membership errors as [`Self::identity`].
    pub fn encoding(profile: &NativeProfileAgreement) -> Result<PayloadEncoding, UWireError> {
        Self::identity(profile).map(NativePayloadIdentity::encoding)
    }

    /// Checks manual descriptors against the actual Rust layout and target.
    ///
    /// # Errors
    ///
    /// Rejects inconsistent type name, revision, size, alignment or byte order.
    pub fn validate_representation() -> Result<NativeRepresentation, UWireError> {
        let representation = T::native_representation();
        if representation.name() != T::TYPE_NAME
            || representation.revision() != T::VARIANT
            || representation.size() != mem::size_of::<T>() as u64
            || representation.alignment() != mem::align_of::<T>() as u64
            || representation.byte_order() != NativeByteOrder::native()
        {
            return Err(UWireError::invalid_payload(
                "stable representation does not match its Rust type and target",
            ));
        }
        Ok(representation)
    }
}

fn scalar_representation<T>(name: &str) -> NativeRepresentation {
    NativeRepresentation::new(
        name,
        0,
        mem::size_of::<T>() as u64,
        mem::align_of::<T>() as u64,
        NativeByteOrder::native(),
        vec![],
    )
    .expect("Rust scalar layout")
}

macro_rules! always_valid_field {
    ($($ty:ty),+ $(,)?) => {
        $(
            unsafe impl StablePayloadField for $ty {
                const FIELD_BITS_ALWAYS_VALID: bool = true;

                fn field_representation() -> NativeRepresentation {
                    scalar_representation::<Self>(stringify!($ty))
                }

                fn validate_field_bytes(bytes: &[u8]) -> bool {
                    bytes.len() == mem::size_of::<Self>()
                }
            }
        )+
    };
}

always_valid_field!(
    (),
    u8,
    i8,
    u16,
    i16,
    u32,
    i32,
    u64,
    i64,
    u128,
    i128,
    usize,
    isize,
    f32,
    f64
);

unsafe impl StablePayloadField for bool {
    const FIELD_BITS_ALWAYS_VALID: bool = false;

    fn field_representation() -> NativeRepresentation {
        scalar_representation::<Self>("bool")
    }

    fn validate_field_bytes(bytes: &[u8]) -> bool {
        matches!(bytes, [0] | [1])
    }
}

unsafe impl StablePayloadField for char {
    const FIELD_BITS_ALWAYS_VALID: bool = false;

    fn field_representation() -> NativeRepresentation {
        scalar_representation::<Self>("char")
    }

    fn validate_field_bytes(bytes: &[u8]) -> bool {
        let [a, b, c, d] = bytes else {
            return false;
        };
        char::from_u32(u32::from_ne_bytes([*a, *b, *c, *d])).is_some()
    }
}

unsafe impl<T, const N: usize> StablePayloadField for [T; N]
where
    T: StablePayloadField,
{
    const FIELD_BITS_ALWAYS_VALID: bool = T::FIELD_BITS_ALWAYS_VALID;

    fn field_representation() -> NativeRepresentation {
        NativeRepresentation::new(
            "array",
            0,
            mem::size_of::<Self>() as u64,
            mem::align_of::<Self>() as u64,
            NativeByteOrder::native(),
            vec![NativeField::repeated(
                "element",
                0,
                N as u64,
                T::field_representation(),
            )],
        )
        .expect("Rust stable array layout")
    }

    fn validate_field_bytes(bytes: &[u8]) -> bool {
        if bytes.len() != mem::size_of::<Self>() {
            return false;
        }
        if N == 0 {
            return true;
        }
        let field_size = mem::size_of::<T>();
        if field_size == 0 {
            return T::validate_field_bytes(&[]);
        }
        let mut rest = bytes;
        while let Some((field, tail)) = rest.split_at_checked(field_size) {
            if !T::validate_field_bytes(field) {
                return false;
            }
            rest = tail;
            if rest.is_empty() {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, crate::StablePayload, crate::StablePayloadInit)]
    #[stable_payload(type_name = "uprotocol.test.Initialized")]
    struct Initialized {
        tag: u32,
        bytes: [u8; 4],
        valid: bool,
        padding: [u8; 3],
    }

    fn storage_for<T>(value: &mut MaybeUninit<T>) -> &mut [MaybeUninit<u8>] {
        // SAFETY: MaybeUninit<T> is writable storage of exactly size_of::<T>()
        // bytes and retains T's alignment.
        unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::from_mut(value).cast::<MaybeUninit<u8>>(),
                mem::size_of::<T>(),
            )
        }
    }

    #[repr(C)]
    #[derive(crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.SameLayout")]
    struct Unsigned {
        value: u32,
    }

    #[repr(C)]
    #[derive(crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.SameLayout")]
    struct Signed {
        value: i32,
    }

    #[repr(C)]
    #[derive(crate::StablePayload)]
    #[stable_payload(type_name = "uprotocol.test.SameLayout", variant = 1)]
    struct Revised {
        value: u32,
    }

    #[test_case::test_case(Signed::native_representation(); "same name size and alignment different field semantics")]
    #[test_case::test_case(Revised::native_representation(); "layout revision changes token")]
    fn derived_identity_includes_structure_and_revision(other: NativeRepresentation) {
        let baseline = Unsigned::native_representation();
        assert_eq!(baseline.size(), other.size());
        assert_eq!(baseline.alignment(), other.alignment());
        assert_ne!(baseline.token(), other.token());
    }

    #[test]
    fn constrained_fields_fail_closed() {
        assert!(bool::validate_field_bytes(&[0]));
        assert!(bool::validate_field_bytes(&[1]));
        assert!(!bool::validate_field_bytes(&[2]));
        assert!(char::validate_field_bytes(&('x' as u32).to_ne_bytes()));
        assert!(!char::validate_field_bytes(&0x0011_0000_u32.to_ne_bytes()));
        assert!(!<[bool; 2]>::validate_field_bytes(&[0, 2]));
    }

    #[test]
    fn derive_initializer_writes_each_field_without_whole_array_values() {
        let mut storage = MaybeUninit::<Initialized>::uninit();
        let initialized = Initialized::init(storage_for(&mut storage))
            .unwrap()
            .tag(7)
            .bytes_from_slice(b"test")
            .unwrap()
            .valid(true)
            .padding_fill(0)
            .finish();

        assert_eq!(initialized.tag, 7);
        assert_eq!(initialized.bytes, *b"test");
        assert!(initialized.valid);
        assert!(Initialized::validate_field_bytes(initialized.as_bytes()));
    }
}
