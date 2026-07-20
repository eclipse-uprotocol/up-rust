/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::{
    transport_implementer_api, wire, wire_implementer_api, wire_transport, UEncodedRxFrame, UWire,
    UWirePayload, UWireTransport,
};

fn main() {
    let _ = (
        transport_implementer_api,
        wire,
        wire_implementer_api,
        wire_transport,
    );
}
