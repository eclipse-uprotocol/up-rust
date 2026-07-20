/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Compatibility coverage for the feature-gated role import modules.

use up_rust::selected_wire_user_api::{ProtobufWire, UNativePrefixWireTransport};
use up_rust::transport_implementer_api::PreparedTxLoanSpec;
use up_rust::wire_implementer_api::{NativePrefixFrameMetadataCodec, UProtocolNativeWire};

#[test]
fn role_modules_preserve_compatibility_imports() {
    let _ = std::any::TypeId::of::<ProtobufWire>();
    let _ = std::any::TypeId::of::<UNativePrefixWireTransport<(), UProtocolNativeWire>>();
    let _ = std::any::TypeId::of::<PreparedTxLoanSpec>();
    let _ = std::any::TypeId::of::<NativePrefixFrameMetadataCodec>();
}
