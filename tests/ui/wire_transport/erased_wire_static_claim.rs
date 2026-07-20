/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use up_rust::UWire;

fn needs_static_wire<W: UWire + ?Sized>() {}

fn main() {
    needs_static_wire::<dyn UWire>();
}
