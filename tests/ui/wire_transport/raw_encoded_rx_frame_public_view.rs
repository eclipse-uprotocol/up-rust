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

use std::io::Cursor;

use up_rust::{UEncodedRxFrame, UFrameView, UZeroCopyRxLease};

struct RawEncodedRxFrame;

impl UEncodedRxFrame for RawEncodedRxFrame {
    type PayloadReader<'a>
        = Cursor<&'a [u8]>
    where
        Self: 'a;
    type PayloadSlices<'a>
        = std::iter::Empty<&'a [u8]>
    where
        Self: 'a;

    fn encoded_metadata(&self) -> &[u8] {
        &[]
    }

    fn payload_len(&self) -> usize {
        0
    }

    fn payload_reader(&self) -> Self::PayloadReader<'_> {
        Cursor::new(&[][..])
    }

    fn payload_slices(&self) -> Self::PayloadSlices<'_> {
        std::iter::empty()
    }
}

fn needs_frame_view<T: UFrameView>(_frame: &T) {}

fn needs_zero_copy_rx_lease<T: UZeroCopyRxLease>(_lease: &T) {}

fn main() {
    let raw = RawEncodedRxFrame;

    needs_frame_view(&raw);
    needs_zero_copy_rx_lease(&raw);
}
