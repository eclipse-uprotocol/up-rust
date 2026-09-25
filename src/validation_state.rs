// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// SPDX-License-Identifier: Apache-2.0

//! Validation typestate markers shared by builder-validated types
//! ([`UOwnedFrame`](crate::UOwnedFrame), [`UTxLoanSpec`](crate::UTxLoanSpec)).

/// Typestate marker: not yet validated.
///
/// Values in this state come from decode paths or `new_unchecked`
/// constructors; the only way forward is the type's `validate` method,
/// which transitions to [`Validated`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unvalidated {}

/// Typestate marker: validation has passed.
///
/// This is the default state parameter — the type written plain means a
/// validated value, which is what every transport API accepts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Validated {}
