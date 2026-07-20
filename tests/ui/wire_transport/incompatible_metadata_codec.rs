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

use up_rust::{
    UFrameMetadata, UProtocolNativeWire, UWireMetadataCodec, UWireMetadataContext,
    UWireMetadataError, UWireTransport,
};

#[derive(Default)]
struct Core;

struct IncompatibleMetadataCodec;

impl UWireMetadataCodec for IncompatibleMetadataCodec {
    fn encode_frame_metadata(
        &self,
        _context: UWireMetadataContext,
        _metadata: &UFrameMetadata,
    ) -> Result<Vec<u8>, UWireMetadataError> {
        Err(UWireMetadataError::WrongMagic)
    }

    fn decode_frame_metadata(
        &self,
        _context: UWireMetadataContext,
        _src: &[u8],
    ) -> Result<UFrameMetadata, UWireMetadataError> {
        Err(UWireMetadataError::WrongMagic)
    }
}

fn main() {
    let _transport = UWireTransport::new(
        Core,
        UProtocolNativeWire,
        IncompatibleMetadataCodec,
    );
}
