//! Cross-crate benchmark and contract-test fixtures (plumbing).
//!
//! These types exist to be shared by the transport crates' benches and
//! payload-contract suites; they are not part of the crate's designed
//! API surface. Item-level docs and Debug impls are deliberately not
//! required here.
#![allow(missing_docs, missing_debug_implementations)]
/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Shared benchmark fixture contracts.

#[cfg(feature = "payload-contract-fixtures")]
pub mod payload_contract;
