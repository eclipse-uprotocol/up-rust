use std::mem::MaybeUninit;

use up_rust::payload::loan::LoanedUninitPayload;

#[repr(C)]
struct ManualPayload {
    value: u32,
}

fn main() {
    let mut storage = MaybeUninit::<ManualPayload>::uninit();
    let mut slot = LoanedUninitPayload::new(&mut storage);

    let _raw = unsafe { slot.as_mut_ptr() };
}
