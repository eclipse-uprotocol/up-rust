use core::mem::{size_of, MaybeUninit};
use up_rust::StablePayloadInit as _;

#[repr(C)]
#[derive(Clone, up_rust::StablePayload)]
#[stable_payload(type_name = "uprotocol.test.Leaf")]
struct Leaf {
    value: u32,
}

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "uprotocol.test.Parent")]
struct Parent {
    leaves: [Leaf; 2],
}

fn main() {
    let mut storage = MaybeUninit::<Parent>::uninit();
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::from_mut(&mut storage).cast::<MaybeUninit<u8>>(),
            size_of::<Parent>(),
        )
    };
    let init = Parent::init(bytes).unwrap();
    let _ = init.leaves_value([Leaf { value: 1 }, Leaf { value: 2 }]);
}
