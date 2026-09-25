/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::{
    PayloadCodecIdentity, PayloadEncoding, UWire, UWirePayload, WireIdentity,
    UFRAME_FIELDS_METADATA_LAYOUT_ID,
};

pub(crate) struct FakePayload;

pub(crate) struct FakeCodec;

impl PayloadCodecIdentity for FakeCodec {
    fn name() -> &'static str {
        "fake-external-wire-codec"
    }

    fn encoding() -> PayloadEncoding {
        PayloadEncoding::from_id(0xFFED).expect("valid private-use fixture id")
    }
}

pub(crate) struct FakeExternalWire;

impl UWire for FakeExternalWire {
    const WIRE_ID: WireIdentity = WireIdentity::new("test.fake.external-wire", 0x80ed);
    const PAYLOAD_FAMILY_ID: WireIdentity = WireIdentity::new("test.fake.payload", 0x80ee);
    const METADATA_LAYOUT_ID: WireIdentity = UFRAME_FIELDS_METADATA_LAYOUT_ID;
    const FORMAT_VERSION: u16 = 1;
}

impl UWirePayload<FakePayload> for FakeExternalWire {
    type Codec = FakeCodec;
}
