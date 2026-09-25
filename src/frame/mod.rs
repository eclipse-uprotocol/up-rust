// SPDX-License-Identifier: Apache-2.0
//! # The UFrame data layer
//!
//! Everything about frames-as-data lives here, independent of any transport:
//!
//! - [`metadata`](crate::frame::metadata) — [`UFrameMetadata`](crate::UFrameMetadata), the
//!   family-invariant semantic metadata model, plus
//!   [`PayloadEncoding`](crate::PayloadEncoding) identity and the fallible
//!   projections to and from `UMessage` attributes.
//! - [`codec`](crate::frame::codec) — the canonical metadata field-block profile: how metadata
//!   becomes bytes under a metadata profile.
//! - `envelope` — complete-frame `UPFE` serialization, available with
//!   `owned-frame-transport`, for transports that
//!   carry metadata and payload in one byte envelope. Selected-wire `UPWM`
//!   profile prefixes are owned by the wire adapter, not this module.
//! - [`abi`](crate::frame::abi) — `UFrameMetadataAbiV1`, the fixed-layout `#[repr(C)]` profile
//!   for cross-language shared-memory boundaries. NOTE: its consumers are
//!   C/C++ peers; zero Rust references is its expected steady state.
//!
//! These experimental representation profiles are defined by their codec and
//! the cross-language artifacts in `abi/uframe`. Their UMessage projection
//! follows the uProtocol attribute and payload-presence contracts.

pub mod view;
pub use view::{validate_frame_view_for_transport, UFrameView};
pub mod abi;
pub mod codec;
pub mod metadata;
