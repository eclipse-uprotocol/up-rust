/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
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

//! Family-neutral frame views.
//!
//! [`UFrameView`] is the shared vocabulary for reading a validated frame's
//! metadata and ordered payload bytes. Both experimental transport families
//! (owned frames and zero-copy leases) expose their received frames through
//! it, and [`validate_frame_view_for_transport`] is the shared boundary check
//! a transport runs before handing a frame to the caller.

use std::io::Read;

use crate::frame::metadata::{UFrameMetadata, UFrameMetadataError};
use crate::payload::codec::{PayloadCodec, ReadDecodePayload};
use crate::payload::UWireError;
use crate::{UCode, UStatus};

fn invalid_argument(message: impl Into<String>) -> UStatus {
    UStatus::fail_with_code(UCode::InvalidArgument, message)
}

fn internal_view_error(message: impl Into<String>) -> UStatus {
    UStatus::fail_with_code(UCode::Internal, message)
}

pub(crate) fn validate_metadata(metadata: &UFrameMetadata) -> Result<(), UStatus> {
    metadata.validate().map_err(frame_metadata_error)
}

pub(crate) fn frame_metadata_error(error: UFrameMetadataError) -> UStatus {
    invalid_argument(format!("invalid frame metadata: {error}"))
}

pub(crate) fn validate_payload_presence(
    has_payload: bool,
    has_encoding: bool,
    context: &str,
) -> Result<(), UStatus> {
    match (has_payload, has_encoding) {
        (true, true) | (false, false) => Ok(()),
        (true, false) => Err(invalid_argument(format!(
            "{context} carries payload bytes without payload encoding"
        ))),
        (false, true) => Err(invalid_argument(format!(
            "{context} carries payload encoding without payload bytes"
        ))),
    }
}

/// *Role: the neutral read vocabulary every received frame speaks, whatever the family; implemented by frames and leases, consumed by decoding code — see the trait map.*
///
/// Neutral view of frame metadata plus ordered payload bytes.
pub trait UFrameView {
    /// Reader over the payload bytes in order.
    type PayloadReader<'a>: Read + 'a
    where
        Self: 'a;
    /// Iterator over the payload's storage segments in order.
    type PayloadSlices<'a>: Iterator<Item = &'a [u8]> + 'a
    where
        Self: 'a;

    /// Returns the native frame metadata.
    fn metadata(&self) -> &UFrameMetadata;

    /// Returns the number of application payload bytes visible through this view.
    fn payload_len(&self) -> usize;

    /// Returns whether this view carries a payload, including a present empty payload.
    fn has_payload(&self) -> bool {
        self.payload_len() > 0 || self.metadata().payload_encoding().is_some()
    }

    /// Returns an ordered reader over the application payload bytes.
    fn payload_reader(&self) -> Self::PayloadReader<'_>;

    /// Returns ordered borrowed payload slices.
    fn payload_slices(&self) -> Self::PayloadSlices<'_>;

    /// Returns a contiguous borrowed payload view when this view can provide one without copying.
    fn try_contiguous_payload(&self) -> Option<&[u8]> {
        None
    }

    /// Decodes this frame view from its ordered payload reader with codec `C`.
    ///
    /// This path works for both contiguous and segmented receive storage without
    /// forcing a coalescing copy before the selected codec sees the bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame has no payload, has missing or incompatible
    /// encoding metadata, or if the codec cannot decode the payload bytes.
    fn decode_payload_from_reader_as<C, T>(&self) -> Result<T, UWireError>
    where
        C: PayloadCodec + ReadDecodePayload<T>,
    {
        C::verify_encoding(self.metadata().payload_encoding())?;
        if !self.has_payload() {
            return Err(UWireError::MissingPayload);
        }
        C::decode_payload_from_reader(self.payload_reader(), self.payload_len())
    }
}

/// Validates a frame view before returning it from a public zero-copy boundary.
///
/// # Errors
///
/// Returns an error if metadata is invalid, payload presence disagrees with
/// payload encoding, or ordered slices do not match `payload_len`.
pub fn validate_frame_view_for_transport(
    frame: &(impl UFrameView + ?Sized),
) -> Result<(), UStatus> {
    validate_metadata(frame.metadata())?;
    validate_payload_presence(
        frame.has_payload(),
        frame.metadata().payload_encoding().is_some(),
        "frame view",
    )?;
    let mut observed = 0_usize;
    for slice in frame.payload_slices() {
        observed = observed
            .checked_add(slice.len())
            .ok_or_else(|| internal_view_error("frame view payload slices overflow usize"))?;
    }
    if observed != frame.payload_len() {
        return Err(internal_view_error(format!(
            "frame view payload slices yielded {observed} bytes but payload_len returned {}",
            frame.payload_len()
        )));
    }
    Ok(())
}
