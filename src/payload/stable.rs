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

//! # Stable payloads: explicit layouts initialized in transport storage
//!
//! A stable payload is one fixed-layout value whose type name, size, alignment,
//! fields, and padding are known to both peers. The supported path is:
//!
//! 1. define a `#[repr(C)]` struct and make every padding byte explicit;
//! 2. derive `StablePayload` for layout/type identity;
//! 3. derive `ByteBackedStablePayload` only when every byte pattern is valid;
//! 4. derive `StablePayloadInit` for the generated typestate initializer;
//! 5. initialize the value inside the higher-ranked closure accepted by
//!    `send_uninit_stable_payload`, then return `finish()`.
//!
//! ```text
//! #[repr(C)]
//! #[derive(StablePayload, ByteBackedStablePayload, StablePayloadInit)]
//! #[stable_payload(type_name = "com.example.CanFrameV1")]
//! struct CanFrameV1 {
//!     id: u32,
//!     len: u8,
//!     flags: u8,
//!     data: [u8; 64],
//!     reserved: [u8; 2], // explicit trailing padding
//! }
//!
//! transport.send_uninit_stable_payload::<CanFrameV1>(metadata, |init| {
//!     init.id(0x1ff)
//!         .len(64)
//!         .flags(0)
//!         .data_from_fn(|index| index as u8)
//!         .reserved([0; 2])
//!         .finish()
//! }).await?;
//! ```
//!
//! The snippet is schematic because transport construction and metadata are
//! application choices. The derive and generated setter names are the real
//! shape.
//!
//! ## Where the guarantee lives
//!
//! `InitializedStablePayload` has no public constructor. Generated typestate
//! exposes `finish()` only after every semantic field and generated padding gap
//! has been written. The send helper accepts a `for<'payload>` closure, so a
//! proof tied to one invocation cannot escape that invocation or be substituted
//! into another in safe Rust. The final witness discharge is the implementation
//! of `req~zero-copy-transport-two-phase~1`.
//!
//! Generated field writes route through centralized bounds-checked copy/fill
//! kernels. Unsafe codec implementation, byte-backed layout proof, and typed
//! borrow remain separate expert contracts; deriving these traits is the normal
//! user path. Compile-fail tests in `tests/ui/stable_payload/`, Miri tests, exact
//! byte fixtures, and selected-wire round trips are the executable arbiters.

use std::{
    fmt::Display,
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
};

use mediatype::ReadParams;

#[cfg(feature = "zero-copy-transport")]
use crate::zero_copy::LoanedPayloadUninitMut;
use crate::PayloadEncoding;

#[cfg(feature = "zero-copy-transport")]
use super::loan::{LoanUninitPayload, LoanedUninitPayload};
use super::{
    codec::{PayloadCodec, PayloadLayout},
    loan::{BorrowPayload, LoanPayload},
    UWireError,
};

const STABLE_CONTAINER_ENCODING_ID: &str = "up.stable-container";
const STABLE_CONTAINER_MEDIA_TYPE: &str = "application/vnd.uprotocol.stable-container";

/// Completion proof returned by generated stable payload initializers.
///
/// This token is intentionally not a typed reference into the loan. It proves
/// that a generated typestate builder reached `finish()` after initializing all
/// semantic fields and generated padding gaps.
#[must_use = "return this completion witness to the transmit path or consume it explicitly"]
pub struct InitializedStablePayload<'a, T> {
    _marker: PhantomData<fn(&'a mut T) -> &'a mut T>,
}

impl<'a, T> core::fmt::Debug for InitializedStablePayload<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InitializedStablePayload")
            .finish_non_exhaustive()
    }
}

impl<'a, T> InitializedStablePayload<'a, T> {
    /// Creates a completion proof for a fully initialized internal slot.
    ///
    /// # Safety
    ///
    /// The corresponding slot must be exclusively borrowed for `'a` and contain
    /// one valid initialized `T`, including all implicit padding bytes.
    #[must_use = "return this completion witness to the transmit path or consume it explicitly"]
    #[inline(always)]
    unsafe fn new_unchecked() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// Stable payload type variant used in stable-container metadata.
///
/// `USR-04A` supports one fixed-size value per payload. Runtime-length stable
/// slices are intentionally left for a later API.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StablePayloadVariant {
    /// One fixed-size `T` value with exact payload length `size_of::<T>()`.
    FixedSize,
}

impl StablePayloadVariant {
    /// Returns the stable media-type spelling for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FixedSize => "fixed",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "fixed" => Some(Self::FixedSize),
            _ => None,
        }
    }
}

impl Display for StablePayloadVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Runtime type detail carried by stable-container metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StableTypeDetail<'a> {
    /// Fixed-size stable-container payload variant.
    pub variant: StablePayloadVariant,
    /// Cross-process and cross-language stable type identity.
    pub type_name: &'a str,
    /// Payload value size in bytes.
    pub size: usize,
    /// Required payload value alignment in bytes.
    pub alignment: usize,
}

impl Display for StableTypeDetail<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "type={};variant={};size={};align={}",
            self.type_name, self.variant, self.size, self.alignment
        )
    }
}

/// Hidden recursive field proof used by the `StablePayload` derive.
///
/// Application code should use `#[derive(StablePayload)]` instead of naming this
/// trait directly. Manual implementations are only for externally-audited FFI or
/// codegen payload fields whose layout and ownership are known to be stable.
///
/// # Safety
///
/// Implementors must not contain process-local references, raw pointers,
/// function pointers, heap ownership, or drop glue that would make the value
/// invalid as part of a stable-container payload representation.
#[doc(hidden)]
pub unsafe trait StablePayloadField: Sized + 'static {
    #[doc(hidden)]
    fn __stable_payload_field_check(&self) {}
}

macro_rules! impl_stable_payload_field {
    ($($ty:ty),* $(,)?) => {
        $(
            // SAFETY: Primitive scalar values have no process-local ownership or
            // drop glue, and arrays recurse through this field proof below.
            unsafe impl StablePayloadField for $ty {}
        )*
    };
}

impl_stable_payload_field!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
);

// SAFETY: Arrays are stable fields when their element type is stable by the same
// recursive field proof.
unsafe impl<T, const N: usize> StablePayloadField for [T; N]
where
    T: StablePayloadField,
{
    fn __stable_payload_field_check(&self) {
        for item in self {
            item.__stable_payload_field_check();
        }
    }
}

/// Stable payload identity used by `StableContainerPayload<T>`.
///
/// This phase uses the trait only to build and verify stable-container metadata.
/// Future borrow/no-zero phases consume the same identity when proving that bytes
/// may safely be viewed as `T`.
///
/// # Safety
///
/// Implementors must choose a stable cross-process type name and only implement
/// the trait for types whose size/alignment and initialized byte representation
/// are suitable for the stable-container contract used by the application.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement the stable-payload contract",
    label = "this type cannot be carried as a stable payload",
    note = "add `#[repr(C)]`, derive `StablePayload`, and provide `#[stable_payload(type_name = \"...\")]`",
    note = "derive `ByteBackedStablePayload` only when every field and byte representation satisfies its stronger safety contract"
)]
pub unsafe trait StablePayload: Sized + 'static {
    /// Stable-container variant supported by this type.
    const VARIANT: StablePayloadVariant = StablePayloadVariant::FixedSize;

    /// Stable cross-process type name.
    const TYPE_NAME: &'static str;

    /// Returns the stable cross-process type name.
    #[must_use]
    fn stable_type_name() -> &'static str {
        Self::TYPE_NAME
    }

    /// Returns the stable metadata detail for this type.
    #[must_use]
    fn stable_type_detail() -> StableTypeDetail<'static> {
        StableTypeDetail {
            variant: Self::VARIANT,
            type_name: Self::stable_type_name(),
            size: mem::size_of::<Self>(),
            alignment: mem::align_of::<Self>(),
        }
    }

    /// Hidden recursive field check emitted by the `StablePayload` derive.
    #[doc(hidden)]
    fn __stable_payload_field_check(&self) {}
}

mod byte_backed_stable_field_seal {
    pub trait Sealed {}

    macro_rules! impl_sealed_field {
        ($($ty:ty),* $(,)?) => {
            $(
                impl Sealed for $ty {}
            )*
        };
    }

    impl_sealed_field!(
        (),
        bool,
        char,
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
        f32,
        f64,
    );

    impl<T, const N: usize> Sealed for [T; N] where T: Sealed {}

    impl<T> Sealed for T where T: super::ByteBackedStablePayload {}
}

/// Hidden recursive proof that a field is safe for byte-backed stable payloads.
///
/// This trait is derive-support plumbing, not an application extension point.
/// Use [`ByteBackedStablePayload`] for payload-level public bounds.
#[doc(hidden)]
#[allow(private_bounds)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a byte-backed stable-payload field",
    label = "this field does not satisfy the recursive byte-backed contract",
    note = "use a supported primitive/array field, or derive `ByteBackedStablePayload` for an eligible nested stable type"
)]
pub trait ByteBackedStablePayloadField:
    byte_backed_stable_field_seal::Sealed + Sized + 'static
{
    #[doc(hidden)]
    const SUPPORTS_BYTE_BACKED_STABLE_FIELD: bool;
    /// Views this plain-old-data value as its initialized bytes.
    #[doc(hidden)]
    fn init_bytes(&self) -> &[u8] {
        // SAFETY: this sealed trait is implemented only for primitive fields,
        // recursively byte-backed arrays, and types whose unsafe
        // `ByteBackedStablePayload` contract proves the full live value has no
        // uninitialized padding or process-local state.
        unsafe {
            core::slice::from_raw_parts(
                (self as *const Self).cast::<u8>(),
                core::mem::size_of::<Self>(),
            )
        }
    }
}

macro_rules! impl_byte_backed_stable_field {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ByteBackedStablePayloadField for $ty {
                const SUPPORTS_BYTE_BACKED_STABLE_FIELD: bool = true;
            }
        )*
    };
}

impl_byte_backed_stable_field!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
);

impl<T, const N: usize> ByteBackedStablePayloadField for [T; N]
where
    T: ByteBackedStablePayloadField,
{
    const SUPPORTS_BYTE_BACKED_STABLE_FIELD: bool = T::SUPPORTS_BYTE_BACKED_STABLE_FIELD;
}

impl<T> ByteBackedStablePayloadField for T
where
    T: ByteBackedStablePayload,
{
    const SUPPORTS_BYTE_BACKED_STABLE_FIELD: bool = true;
}

/// Stronger stable payload proof for byte-backed stable-container paths.
///
/// `StablePayload` proves stable type identity, representation eligibility, and
/// receive-side metadata compatibility. `ByteBackedStablePayload` additionally
/// proves that the type has no implicit inter-field or trailing padding and that
/// every field is recursively byte-backed.
///
/// Prefer `#[derive(StablePayload, ByteBackedStablePayload)]` for hand-written
/// payload types. Manual implementations are reserved for externally audited FFI
/// or code-generated layouts with an equivalent whole-object initialization
/// proof.
///
/// # Safety
///
/// Implementors must guarantee that copying or initializing exactly
/// `size_of::<Self>()` bytes is a sound representation of one valid `Self` for
/// stable-container paths. That requires stable layout, no implicit padding, no
/// drop glue, interior mutability, and recursively byte-backed fields.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement the byte-backed stable-payload contract",
    label = "this type cannot use byte-backed stable-container paths",
    note = "derive `ByteBackedStablePayload` only for an eligible `#[repr(C)]` type with no implicit padding and recursively byte-backed fields"
)]
pub unsafe trait ByteBackedStablePayload: StablePayload {
    /// Whether this type's full object representation can be used by
    /// byte-backed stable-container paths.
    const SUPPORTS_BYTE_BACKED_UNINIT: bool = true;

    #[doc(hidden)]
    const BYTE_BACKED_STABLE_PAYLOAD_CHECK: () = {
        assert!(Self::SUPPORTS_BYTE_BACKED_UNINIT);
        assert!(!mem::needs_drop::<Self>());
    };
}

/// Returns whether `T` can use byte-backed stable-container paths.
#[must_use]
pub const fn stable_payload_supports_byte_backed_uninit<T: ByteBackedStablePayload>() -> bool {
    T::SUPPORTS_BYTE_BACKED_UNINIT && !mem::needs_drop::<T>()
}

/// Compile-time assertion for byte-backed stable-container eligibility.
pub const fn assert_stable_payload_byte_backed_uninit<T>()
where
    T: ByteBackedStablePayload,
{
    assert!(T::SUPPORTS_BYTE_BACKED_UNINIT);
    assert!(!mem::needs_drop::<T>());
    #[allow(path_statements)]
    T::BYTE_BACKED_STABLE_PAYLOAD_CHECK;
}

mod stable_payload_init_complete_value_seal {
    pub trait Sealed {}

    impl<T> Sealed for T where T: super::ByteBackedStablePayload {}

    impl<T, const N: usize> Sealed for [T; N] where T: super::ByteBackedStablePayloadField {}
}

/// Hidden proof that a complete by-value nested initializer can be moved into a field.
///
/// This is derive support, not an application extension point. It is implemented
/// only for byte-backed stable payloads, where moving the complete value into
/// loan storage cannot expose uninitialized implicit padding bytes.
#[doc(hidden)]
#[allow(private_bounds)]
pub trait StablePayloadInitCompleteValue<T>:
    stable_payload_init_complete_value_seal::Sealed + Sized
{
    #[doc(hidden)]
    fn into_complete_value(self) -> T;
}

impl<T> StablePayloadInitCompleteValue<T> for T
where
    T: ByteBackedStablePayload,
{
    fn into_complete_value(self) -> T {
        self
    }
}

impl<T, const N: usize> StablePayloadInitCompleteValue<[T; N]> for [T; N]
where
    T: ByteBackedStablePayloadField,
{
    fn into_complete_value(self) -> [T; N] {
        self
    }
}

/// Hidden typestate marker used by `StablePayloadInit` derive output.
#[doc(hidden)]
#[derive(Debug)]
pub enum StablePayloadInitUnset {}

/// Hidden typestate marker used by `StablePayloadInit` derive output.
#[doc(hidden)]
#[derive(Debug)]
pub enum StablePayloadInitSet {}

/// Generative context for initializing one stable payload slot.
///
/// The callback APIs that provide this context quantify over a fresh invariant
/// slot lifetime. A completion proof can therefore be produced only by
/// finishing the initializer carried by that callback invocation; a proof from
/// separate storage cannot be substituted in safe Rust.
#[must_use = "consume this context to initialize the loaned stable-payload slot"]
pub struct StablePayloadInitContext<'slot, T: StablePayloadInit> {
    init: T::Init<'slot>,
    _invariant: PhantomData<fn(&'slot mut T) -> &'slot mut T>,
}

impl<'slot, T: StablePayloadInit> core::fmt::Debug for StablePayloadInitContext<'slot, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StablePayloadInitContext")
            .finish_non_exhaustive()
    }
}

impl<'slot, T: StablePayloadInit> StablePayloadInitContext<'slot, T> {
    #[inline(always)]
    pub(crate) fn new(init: T::Init<'slot>) -> Self {
        Self {
            init,
            _invariant: PhantomData,
        }
    }

    /// Consumes the context and returns the generated typestate initializer.
    #[must_use]
    #[inline(always)]
    pub fn into_init(self) -> T::Init<'slot> {
        self.init
    }
}

/// Safe generated initialization proof for stable-container payloads.
///
/// Derived implementations initialize one stable payload directly in
/// uninitialized storage. Generated builders expose named typed setters and make
/// `finish()` available only after all required fields are set. Implementations
/// must also initialize any implicit and trailing padding bytes they own without
/// blanket-zeroing the full payload.
///
/// # Safety
///
/// Implementors must guarantee that every successful `finish()` for `Self::Init`
/// returns only after the full transported `size_of::<Self>()` byte range,
/// including implicit padding, contains one valid initialized `Self` in the
/// stable-container representation. Ordinary payload types should use the derive
/// macro; manual implementations are an expert unsafe extension point.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no stable-payload initializer",
    label = "missing `StablePayloadInit` implementation",
    note = "derive `StablePayloadInit` to get the typestate builder used by `send_uninit_stable_payload`"
)]
pub unsafe trait StablePayloadInit: StablePayload {
    /// Generated any-order typestate initializer for this payload type.
    type Init<'a>;

    /// Creates a generated initializer from an exact uninitialized payload range.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte range does not match this stable payload's
    /// size and alignment.
    fn init_from_uninit_bytes<'a>(
        payload: &'a mut [mem::MaybeUninit<u8>],
    ) -> Result<Self::Init<'a>, UWireError> {
        let slot = StablePayloadInitSlot::<Self>::from_uninit_bytes(payload)?;
        Self::__init_from_slot(slot)
    }

    /// Creates a generated initializer from a transport loan's visible payload range.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload range does not match this stable payload's
    /// size and alignment.
    #[cfg(feature = "zero-copy-transport")]
    fn init_from_uninit_payload<'a>(
        payload: LoanedPayloadUninitMut<'a>,
    ) -> Result<Self::Init<'a>, UWireError> {
        Self::init_from_uninit_bytes(payload.into_uninit_bytes_mut_internal())
    }

    /// Creates a generated initializer from a nested stable payload slot.
    #[doc(hidden)]
    fn __init_from_slot<'a>(
        slot: StablePayloadInitSlot<'a, Self>,
    ) -> Result<Self::Init<'a>, UWireError>;

    /// Creates a generative context from a nested slot.
    #[doc(hidden)]
    #[inline(always)]
    fn __init_context_from_slot<'a>(
        slot: StablePayloadInitSlot<'a, Self>,
    ) -> Result<StablePayloadInitContext<'a, Self>, UWireError> {
        Self::__init_from_slot(slot).map(StablePayloadInitContext::new)
    }
}

/// Hidden typed view over uninitialized storage used by generated initializers.
///
/// The type is public only so derive output in downstream crates can name it.
/// Application code should use `#[derive(StablePayloadInit)]` and the generated
/// setters instead of constructing or manipulating slots directly.
#[doc(hidden)]
pub struct StablePayloadInitSlot<'a, T> {
    // Safe-by-construction: the slot IS the exclusive borrow of the target
    // region. All writers below are bounds-checked safe code; the only
    // `unsafe` in the initialization path is the final witness discharge in
    // `assume_init` and the POD byte view in
    // `ByteBackedStablePayloadField::init_bytes`.
    bytes: &'a mut [mem::MaybeUninit<u8>],
    _marker: PhantomData<&'a mut mem::MaybeUninit<T>>,
}

impl<'a, T> core::fmt::Debug for StablePayloadInitSlot<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StablePayloadInitSlot")
            .finish_non_exhaustive()
    }
}

impl<'a, T> StablePayloadInitSlot<'a, T> {
    /// Creates a slot from uninitialized payload bytes after exact stable layout validation.
    #[inline(always)]
    fn from_uninit_bytes(bytes: &'a mut [mem::MaybeUninit<u8>]) -> Result<Self, UWireError>
    where
        T: StablePayload,
    {
        StableContainerPayload::<T>::check_uninit_layout(bytes)?;
        Ok(Self {
            bytes,
            _marker: PhantomData,
        })
    }

    /// Copies initialized bytes into an uninitialized destination region.
    ///
    /// This is one of exactly two `unsafe` kernels in the initialization
    /// write path; every safe writer routes through it so that field and
    /// array writes compile to `memcpy`, never to per-byte loops — a hard
    /// requirement for zero-copy transports moving large fixed-size
    /// payloads (64 KiB stream chunks, radar detection lists).
    #[inline(always)]
    fn copy_to_uninit(dst: &mut [mem::MaybeUninit<u8>], src: &[u8]) {
        debug_assert_eq!(dst.len(), src.len());
        // SAFETY: `dst` and `src` have equal length (checked above and by
        // every bounds-checked caller); the regions cannot overlap because
        // `dst` is an exclusive borrow of transport-loaned storage while
        // `src` borrows caller memory; writing `u8` bytes through
        // `MaybeUninit<u8>` is always valid.
        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr().cast::<u8>(), src.len());
        }
    }

    /// Fills an uninitialized destination region with one byte value.
    ///
    /// The second of the two write-path `unsafe` kernels; compiles to
    /// `memset`.
    #[inline(always)]
    fn fill_uninit(dst: &mut [mem::MaybeUninit<u8>], value: u8) {
        // SAFETY: writing `u8` bytes through `MaybeUninit<u8>` within the
        // exclusive borrow is always valid.
        unsafe {
            ptr::write_bytes(dst.as_mut_ptr().cast::<u8>(), value, dst.len());
        }
    }

    #[inline(always)]
    fn checked_range(&mut self, offset: usize, len: usize) -> &mut [mem::MaybeUninit<u8>] {
        let end = offset
            .checked_add(len)
            .expect("stable payload field range overflow: layout and derive offsets disagree");
        self.bytes
            .get_mut(offset..end)
            .expect("stable payload field write out of bounds: layout and derive offsets disagree")
    }

    /// Zero-fills `len` bytes at `offset` (padding regions).
    #[doc(hidden)]
    #[inline(always)]
    pub fn write_padding(&mut self, offset: usize, len: usize) {
        Self::fill_uninit(self.checked_range(offset, len), 0);
    }

    /// Writes one plain-old-data field value at `offset`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn write_field<U: ByteBackedStablePayloadField>(&mut self, offset: usize, value: U) {
        let src = value.init_bytes();
        Self::copy_to_uninit(self.checked_range(offset, src.len()), src);
    }

    /// Copies raw bytes at `offset`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn write_bytes(&mut self, offset: usize, src: &[u8]) {
        Self::copy_to_uninit(self.checked_range(offset, src.len()), src);
    }

    /// Fills `len` bytes at `offset` with `value`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn fill_bytes(&mut self, offset: usize, len: usize, value: u8) {
        Self::fill_uninit(self.checked_range(offset, len), value);
    }

    /// Fills `len` bytes at `offset` from a byte-producing function.
    #[doc(hidden)]
    #[inline(always)]
    pub fn fill_bytes_with(
        &mut self,
        offset: usize,
        len: usize,
        mut value: impl FnMut(usize) -> u8,
    ) {
        for (index, dst) in self.checked_range(offset, len).iter_mut().enumerate() {
            *dst = mem::MaybeUninit::new(value(index));
        }
    }

    /// Copies a `[U; N]`-shaped region from a slice of POD elements.
    #[doc(hidden)]
    #[inline(always)]
    pub fn copy_array_from_slice<U: ByteBackedStablePayloadField>(
        &mut self,
        offset: usize,
        src: &[U],
        expected: usize,
    ) -> Result<(), UWireError> {
        if src.len() != expected {
            return Err(UWireError::invalid_payload_length(expected, src.len()));
        }
        // POD contract: a `[U]` of byte-backed fields is itself a contiguous
        // initialized byte region, so the whole array is ONE memcpy.
        let byte_len = mem::size_of_val(src);
        // SAFETY: `U: ByteBackedStablePayloadField` guarantees `U` (and
        // therefore `[U]`) is plain-old-data with no padding surprises; the
        // slice's bytes are fully initialized.
        let src_bytes = unsafe { core::slice::from_raw_parts(src.as_ptr().cast::<u8>(), byte_len) };
        Self::copy_to_uninit(self.checked_range(offset, byte_len), src_bytes);
        Ok(())
    }

    /// Fills a `[U; N]`-shaped region with copies of one POD element.
    #[doc(hidden)]
    #[inline(always)]
    pub fn fill_array<U: ByteBackedStablePayloadField>(
        &mut self,
        offset: usize,
        len: usize,
        value: U,
    ) {
        let elem = mem::size_of::<U>();
        let byte_len = elem
            .checked_mul(len)
            .expect("stable payload array fill length overflow");
        if elem == 0 {
            return;
        }
        let bytes = value.init_bytes();
        for element in self.checked_range(offset, byte_len).chunks_exact_mut(elem) {
            Self::copy_to_uninit(element, bytes);
        }
    }

    /// Re-borrows the sub-region for one field as a typed slot.
    #[doc(hidden)]
    #[inline(always)]
    pub fn field_slot<U>(&mut self, offset: usize) -> StablePayloadInitSlot<'_, U> {
        let len = mem::size_of::<U>();
        let end = offset
            .checked_add(len)
            .expect("stable payload field slot offset overflow");
        let bytes = self
            .bytes
            .get_mut(offset..end)
            .expect("stable payload field slot out of bounds");
        assert_eq!(
            bytes.as_mut_ptr().align_offset(mem::align_of::<U>()),
            0,
            "stable payload field slot is misaligned"
        );
        StablePayloadInitSlot {
            bytes,
            _marker: PhantomData,
        }
    }

    /// Re-borrows the sub-region for one array element as a typed slot.
    #[doc(hidden)]
    #[inline(always)]
    pub fn array_element_slot<U>(
        &mut self,
        offset: usize,
        index: usize,
    ) -> StablePayloadInitSlot<'_, U> {
        let element_offset = index
            .checked_mul(mem::size_of::<U>())
            .and_then(|relative| offset.checked_add(relative))
            .expect("stable payload array element offset overflow");
        self.field_slot::<U>(element_offset)
    }

    /// Discharges the initialization witness.
    ///
    /// This is available only to generated all-set typestate builders. Safe
    /// code cannot construct a slot directly.
    #[doc(hidden)]
    #[must_use = "the discharged witness must be returned to or consumed by the completion path"]
    #[inline(always)]
    pub fn assume_init(self) -> InitializedStablePayload<'a, T> {
        // SAFETY: slots can originate only in `StablePayloadInit` entry points,
        // and generated code exposes this method only on its all-set typestate.
        unsafe { InitializedStablePayload::new_unchecked() }
    }
}

/// Type-agnostic stable-container metadata parsed from a payload encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableContainerPayloadInfo {
    /// Cross-process and cross-language stable type identity.
    pub type_name: String,
    /// Stable-container payload variant.
    pub variant: StablePayloadVariant,
    /// Exact payload size in bytes.
    pub size: usize,
    /// Advertised payload alignment in bytes.
    pub alignment: usize,
}

impl StableContainerPayloadInfo {
    /// Native custom encoding id for stable-container payloads.
    pub const ENCODING_ID: &'static str = STABLE_CONTAINER_ENCODING_ID;

    /// Parses stable-container metadata from a payload encoding.
    ///
    /// # Errors
    ///
    /// Returns an error if the encoding is not stable-container metadata or if
    /// the stable-container content type is malformed.
    pub fn parse(encoding: &PayloadEncoding) -> Result<Self, UWireError> {
        let expected = PayloadEncoding::custom(Self::ENCODING_ID, STABLE_CONTAINER_MEDIA_TYPE)
            .expect("stable-container media type is valid");
        let Some((id, content_type)) = encoding.custom_identity() else {
            return Err(UWireError::UnsupportedEncoding {
                expected: Box::new(expected),
                actual: Box::new(encoding.clone()),
            });
        };
        if id != Self::ENCODING_ID {
            return Err(UWireError::UnsupportedEncoding {
                expected: Box::new(expected),
                actual: Box::new(encoding.clone()),
            });
        }
        Self::parse_content_type(content_type).map_err(UWireError::invalid_payload)
    }

    /// Returns whether this metadata is compatible with local stable payload `T`.
    #[must_use]
    pub fn is_compatible_with<T>(&self) -> bool
    where
        T: StablePayload,
    {
        let expected = T::stable_type_detail();
        self.type_name == expected.type_name
            && self.variant == expected.variant
            && self.size == expected.size
            && self.alignment >= expected.alignment
    }

    fn parse_content_type(content_type: &str) -> Result<Self, String> {
        let media_type = mediatype::MediaType::parse(content_type)
            .map_err(|error| format!("invalid media type: {error}"))?;
        let expected_media_type = mediatype::MediaType::parse(STABLE_CONTAINER_MEDIA_TYPE)
            .expect("stable-container media type is valid");
        if media_type.essence() != expected_media_type {
            return Err(format!(
                "media type must be {}",
                expected_media_type.essence()
            ));
        }

        let type_name = required_stable_parameter(&media_type, "type")?;
        if type_name.is_empty() {
            return Err("type parameter must not be empty".to_string());
        }
        let variant = required_stable_parameter(&media_type, "variant")?;
        let variant = StablePayloadVariant::parse(&variant).ok_or_else(|| {
            format!(
                "variant parameter must be {}",
                StablePayloadVariant::FixedSize
            )
        })?;
        let size = required_stable_parameter(&media_type, "size")?
            .parse::<usize>()
            .map_err(|error| format!("invalid size parameter: {error}"))?;
        let alignment = required_stable_parameter(&media_type, "align")?
            .parse::<usize>()
            .map_err(|error| format!("invalid align parameter: {error}"))?;
        PayloadLayout::new(size, alignment).map_err(|error| error.to_string())?;

        Ok(Self {
            type_name,
            variant,
            size,
            alignment,
        })
    }
}

fn required_stable_parameter(
    media_type: &mediatype::MediaType<'_>,
    name: &'static str,
) -> Result<String, String> {
    media_type
        .get_param(mediatype::Name::new_unchecked(name))
        .map(|value| value.unquoted_str().into_owned())
        .ok_or_else(|| format!("missing {name} parameter"))
}

/// Transport-independent stable-container payload identity.
///
/// `USR-04A` uses this type for metadata and owned-byte preservation. Typed
/// zero-copy borrows are intentionally deferred to the stable borrow proof API.
pub struct StableContainerPayload<T>(PhantomData<T>);

impl<T> core::fmt::Debug for StableContainerPayload<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StableContainerPayload")
            .finish_non_exhaustive()
    }
}

impl<T: StablePayload> StableContainerPayload<T> {
    /// Native custom encoding id for stable-container payloads.
    pub const ENCODING_ID: &'static str = STABLE_CONTAINER_ENCODING_ID;

    const MEDIA_TYPE: &'static str = STABLE_CONTAINER_MEDIA_TYPE;

    /// Returns the stable-container payload encoding for `T`.
    #[must_use]
    pub fn encoding() -> PayloadEncoding {
        <Self as PayloadCodec>::payload_encoding()
    }

    fn stable_content_type() -> String {
        let detail = T::stable_type_detail();
        format!(
            "{};type={};variant={};size={};align={}",
            Self::MEDIA_TYPE,
            Self::quote_parameter(detail.type_name),
            detail.variant,
            detail.size,
            detail.alignment
        )
    }

    fn quote_parameter(value: &str) -> String {
        let quoted = mediatype::Value::quote(value);
        if quoted.starts_with('"') {
            quoted.into_owned()
        } else {
            format!("\"{quoted}\"")
        }
    }

    fn verify_content_type(content_type: &str) -> Result<(), String> {
        let info = StableContainerPayloadInfo::parse_content_type(content_type)?;
        let expected = T::stable_type_detail();
        if info.type_name != expected.type_name {
            return Err(format!("type parameter must be {}", expected.type_name));
        }
        if info.variant != expected.variant {
            return Err(format!(
                "variant parameter must be {}",
                expected.variant.as_str()
            ));
        }
        if info.size != expected.size {
            return Err(format!("size parameter must be {}", expected.size));
        }
        if info.alignment < expected.alignment {
            return Err(format!(
                "align parameter must be at least {}",
                expected.alignment
            ));
        }
        Ok(())
    }

    pub(crate) fn borrow_checked_payload(src: &[u8]) -> Result<&T, UWireError> {
        let expected_len = mem::size_of::<T>();
        if src.len() != expected_len {
            return Err(UWireError::invalid_payload(format!(
                "payload length must be {expected_len}, got {}",
                src.len()
            )));
        }

        let alignment = mem::align_of::<T>();
        let address = src.as_ptr() as usize;
        if !address.is_multiple_of(alignment) {
            return Err(UWireError::invalid_payload(format!(
                "payload address {address} is not aligned to {alignment}"
            )));
        }

        // SAFETY: length and alignment were verified above. Reaching this helper
        // requires the loan-backed receive proof, and `T: StablePayload` is the
        // unsafe contract that the bytes represent one initialized `T`.
        Ok(unsafe { &*src.as_ptr().cast::<T>() })
    }

    pub(crate) fn check_uninit_layout(src: &[mem::MaybeUninit<u8>]) -> Result<(), UWireError> {
        let expected_len = mem::size_of::<T>();
        if src.len() != expected_len {
            return Err(UWireError::invalid_payload_length(expected_len, src.len()));
        }

        let alignment = mem::align_of::<T>();
        let address = src.as_ptr() as usize;
        if !address.is_multiple_of(alignment) {
            return Err(UWireError::invalid_payload(format!(
                "payload address {address} is not aligned to {alignment}"
            )));
        }
        Ok(())
    }

    fn check_initialized_layout(src: &[u8]) -> Result<(), UWireError> {
        let expected_len = mem::size_of::<T>();
        if src.len() != expected_len {
            return Err(UWireError::invalid_payload_length(expected_len, src.len()));
        }

        if expected_len == 0 {
            return Ok(());
        }

        let alignment = mem::align_of::<T>();
        let address = src.as_ptr() as usize;
        if !address.is_multiple_of(alignment) {
            return Err(UWireError::invalid_payload(format!(
                "payload address {address} is not aligned to {alignment}"
            )));
        }
        Ok(())
    }
}

impl<T> PayloadCodec for StableContainerPayload<T>
where
    T: StablePayload,
{
    fn codec_name() -> &'static str {
        "stable-container"
    }

    fn payload_encoding() -> PayloadEncoding {
        PayloadEncoding::custom(Self::ENCODING_ID, Self::stable_content_type())
            .expect("stable-container payload encoding is valid")
    }

    fn verify_encoding(actual: Option<&PayloadEncoding>) -> Result<(), UWireError> {
        let expected = Self::payload_encoding();
        let actual = actual.ok_or(UWireError::MissingEncoding)?;
        let Some((id, content_type)) = actual.custom_identity() else {
            return Err(UWireError::UnsupportedEncoding {
                expected: Box::new(expected),
                actual: Box::new(actual.clone()),
            });
        };
        if id != Self::ENCODING_ID {
            return Err(UWireError::UnsupportedEncoding {
                expected: Box::new(expected),
                actual: Box::new(actual.clone()),
            });
        }
        Self::verify_content_type(content_type).map_err(|reason| {
            UWireError::invalid_payload(format!(
                "incompatible stable payload: expected {}; actual {} ({reason})",
                T::stable_type_detail(),
                content_type
            ))
        })
    }
}

// SAFETY:
// - `loan_payload` verifies exact length and alignment before casting the loaned
//   destination to `*mut T`.
// - `T: ByteBackedStablePayload + Default` provides a no-padding stable payload
//   proof and a safe initialized value before `&mut T` is exposed to callers.
unsafe impl<T> LoanPayload<T> for StableContainerPayload<T>
where
    T: ByteBackedStablePayload + Default,
{
    fn loan_layout() -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(mem::size_of::<T>(), mem::align_of::<T>())
    }

    fn loan_payload(dst: &mut [u8]) -> Result<&mut T, UWireError> {
        Self::check_initialized_layout(dst)?;
        dst.fill(0);
        let ptr = if mem::size_of::<T>() == 0 {
            NonNull::<T>::dangling().as_ptr()
        } else {
            dst.as_mut_ptr().cast::<T>()
        };
        // SAFETY: the checked byte range has the exact `T` layout, or `T` is a
        // zero-sized byte-backed type using an aligned dangling pointer. Writing
        // `T::default()` creates one valid initialized `T` before returning the
        // unique mutable reference tied to `dst`'s borrow.
        unsafe { ptr.write(T::default()) };
        // SAFETY: `ptr` now points to one initialized `T`; `dst` is exclusively
        // borrowed for the returned lifetime and no other reference to the value
        // is created by this helper.
        Ok(unsafe { &mut *ptr })
    }
}

// SAFETY:
// - `loan_uninit_payload` checks exact length and alignment before constructing a
//   typed uninit slot.
// - `T: ByteBackedStablePayload` proves safe no-zero TX cannot expose
//   uninitialized implicit padding when the full `size_of::<T>()` byte range is
//   committed after the returned initialized marker is produced.
#[cfg(feature = "zero-copy-transport")]
unsafe impl<T> LoanUninitPayload<T> for StableContainerPayload<T>
where
    T: ByteBackedStablePayload,
{
    fn loan_uninit_layout() -> Result<PayloadLayout, UWireError> {
        PayloadLayout::new(mem::size_of::<T>(), mem::align_of::<T>())
    }

    fn loan_uninit_payload<'a>(
        mut dst: LoanedPayloadUninitMut<'a>,
    ) -> Result<LoanedUninitPayload<'a, T>, UWireError> {
        let bytes = dst.as_uninit_bytes_mut_internal();
        Self::check_uninit_layout(bytes)?;
        // This is the ONE byte-region -> typed-slot entry point of the safe
        // loan path: layout is verified (exact `T` size, alignment) and the
        // exclusive byte borrow is reinterpreted as an exclusive borrow of a
        // single uninitialized `T`.
        // SAFETY: `check_uninit_layout` verified exact `T` length and
        // alignment; `MaybeUninit<T>` has no validity requirements; the
        // returned reference inherits the exclusive `'a` borrow.
        let slot = unsafe { &mut *bytes.as_mut_ptr().cast::<mem::MaybeUninit<T>>() };
        Ok(LoanedUninitPayload::new(slot))
    }
}

// SAFETY:
// - `borrow_payload` verifies exact length and alignment before casting.
// - `T: StablePayload` is the stable-container safety contract that the visible
//   payload bytes represent one initialized `T` for receive-side borrowing.
unsafe impl<T> BorrowPayload<T> for StableContainerPayload<T>
where
    T: StablePayload,
{
    fn borrow_payload(src: &[u8]) -> Result<&T, UWireError> {
        Self::borrow_checked_payload(src)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "owned-frame-transport")]
    use bytes::Bytes;

    #[cfg(feature = "owned-frame-transport")]
    use crate::{UMessageBuilder, UOwnedFrame, UUri};

    use super::*;

    #[cfg(feature = "owned-frame-transport")]
    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("topic")
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct VehiclePose {
        x: i32,
        y: i32,
    }

    // SAFETY: upholds the POD layout contract: repr(C), declared padding
    // only, every byte of a live value initialized.
    unsafe impl StablePayload for VehiclePose {
        const TYPE_NAME: &'static str = "example.vehicle.VehiclePose";
    }

    fn stable_container_encoding(
        type_name: &str,
        variant: &str,
        size: usize,
        align: usize,
    ) -> PayloadEncoding {
        PayloadEncoding::custom(
            StableContainerPayload::<VehiclePose>::ENCODING_ID,
            format!(
                "application/vnd.uprotocol.stable-container;type=\"{type_name}\";variant={variant};size={size};align={align}"
            ),
        )
        .expect("valid custom encoding")
    }

    #[test]
    fn stable_payload_type_detail_is_used_in_encoding() {
        let detail = VehiclePose::stable_type_detail();

        assert_eq!(detail.variant, StablePayloadVariant::FixedSize);
        assert_eq!(detail.type_name, "example.vehicle.VehiclePose");
        assert_eq!(detail.size, mem::size_of::<VehiclePose>());
        assert_eq!(detail.alignment, mem::align_of::<VehiclePose>());

        let encoding = StableContainerPayload::<VehiclePose>::encoding();
        let (id, content_type) = encoding.custom_identity().expect("custom encoding");
        assert_eq!(id, StableContainerPayload::<VehiclePose>::ENCODING_ID);
        assert_eq!(
            content_type,
            "application/vnd.uprotocol.stable-container;type=\"example.vehicle.VehiclePose\";variant=fixed;size=8;align=4"
        );
    }

    #[test]
    fn stable_container_payload_info_parses_type_agnostic_metadata() {
        let info =
            StableContainerPayloadInfo::parse(&StableContainerPayload::<VehiclePose>::encoding())
                .expect("parse stable metadata");

        assert_eq!(info.type_name, "example.vehicle.VehiclePose");
        assert_eq!(info.variant, StablePayloadVariant::FixedSize);
        assert_eq!(info.size, mem::size_of::<VehiclePose>());
        assert_eq!(info.alignment, mem::align_of::<VehiclePose>());
        assert!(info.is_compatible_with::<VehiclePose>());
    }

    #[test]
    fn stable_container_payload_info_rejects_malformed_metadata() {
        let encoding = PayloadEncoding::custom(
            StableContainerPayloadInfo::ENCODING_ID,
            "application/vnd.uprotocol.stable-container;variant=fixed;size=8;align=4",
        )
        .expect("custom encoding");

        let error = StableContainerPayloadInfo::parse(&encoding).unwrap_err();

        assert!(
            matches!(error, UWireError::InvalidPayload(message) if message.contains("missing type"))
        );
    }

    #[test]
    fn stable_container_verify_accepts_larger_advertised_alignment() {
        let encoding = stable_container_encoding(
            "example.vehicle.VehiclePose",
            "fixed",
            mem::size_of::<VehiclePose>(),
            mem::align_of::<VehiclePose>() * 2,
        );

        StableContainerPayload::<VehiclePose>::verify_encoding(Some(&encoding))
            .expect("larger alignment is compatible");
    }

    #[test]
    fn stable_container_verify_rejects_incompatible_metadata() {
        let wrong_type = stable_container_encoding(
            "example.vehicle.OtherPose",
            "fixed",
            mem::size_of::<VehiclePose>(),
            mem::align_of::<VehiclePose>(),
        );
        let wrong_size = stable_container_encoding(
            "example.vehicle.VehiclePose",
            "fixed",
            mem::size_of::<VehiclePose>() + 1,
            mem::align_of::<VehiclePose>(),
        );
        let insufficient_alignment = stable_container_encoding(
            "example.vehicle.VehiclePose",
            "fixed",
            mem::size_of::<VehiclePose>(),
            mem::align_of::<VehiclePose>() / 2,
        );

        for encoding in [wrong_type, wrong_size, insufficient_alignment] {
            assert!(matches!(
                StableContainerPayload::<VehiclePose>::verify_encoding(Some(&encoding)),
                Err(UWireError::InvalidPayload(_))
            ));
        }
    }

    #[cfg(feature = "owned-frame-transport")]
    #[test]
    fn stable_container_owned_frame_preserves_bytes_and_custom_metadata() {
        let payload = Bytes::from_static(b"\x0a\x00\x00\x00\x14\x00\x00\x00");
        assert_eq!(payload.len(), mem::size_of::<VehiclePose>());
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(StableContainerPayload::<VehiclePose>::encoding()),
        )
        .expect("metadata");

        let frame = UOwnedFrame::with_payload(metadata, payload.clone()).expect("owned frame");

        assert_eq!(
            frame.metadata().payload_encoding(),
            Some(&StableContainerPayload::<VehiclePose>::encoding())
        );
        assert_eq!(frame.payload(), Some(&payload));
        assert_eq!(frame.payload_bytes(), payload.as_ref());
    }
}
