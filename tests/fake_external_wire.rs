/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! External wire implementer compile fixture.

mod support;

use support::fake_external_wire_crate::{FakeCodec, FakeExternalWire, FakePayload};
use up_rust::{PayloadCodec, UWirePayload};

#[test]
fn fake_external_wire_uses_an_explicit_private_use_payload_id() {
    fn assert_mapping<W>()
    where
        W: UWirePayload<FakePayload, Codec = FakeCodec>,
    {
    }

    assert_mapping::<FakeExternalWire>();
    assert_eq!(FakeCodec::payload_encoding(None).unwrap().id(), 0xFFED);
}
