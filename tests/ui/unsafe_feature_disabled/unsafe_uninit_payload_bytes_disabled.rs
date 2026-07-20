use std::mem::MaybeUninit;

use up_rust::payload::loan::LoanedUninitPayload;

#[repr(C)]
struct ManualPayload {
    value: u32,
}

fn main() {
    let mut storage = MaybeUninit::<ManualPayload>::uninit();
    let slot = LoanedUninitPayload::new(&mut storage);

    let _raw = slot.uninit_ptr();
}
