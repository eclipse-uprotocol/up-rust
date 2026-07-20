/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use protobuf::well_known_types::wrappers::StringValue;
use up_rust::{EncodePayload, StableContainerWireFormat};

fn needs_protobuf_encode<W, T>()
where
    W: EncodePayload<T>,
{
}

fn main() {
    needs_protobuf_encode::<StableContainerWireFormat, StringValue>();
}
