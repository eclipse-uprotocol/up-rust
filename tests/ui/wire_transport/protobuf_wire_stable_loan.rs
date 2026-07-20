/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use protobuf::well_known_types::wrappers::StringValue;
use up_rust::{LoanPayload, ProtobufWire, UWirePayload};

fn needs_typed_loan<W, T>()
where
    W: UWirePayload<T>,
    <W as UWirePayload<T>>::Codec: LoanPayload<T>,
{
}

fn main() {
    needs_typed_loan::<ProtobufWire, StringValue>();
}
