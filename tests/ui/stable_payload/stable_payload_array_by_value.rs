use core::mem::{size_of, MaybeUninit};
use up_rust::StablePayloadInit as _;

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "uprotocol.test.ArrayByValue")]
struct ArrayByValue {
    bytes: [u8; 4],
}

fn main() {
    let mut storage = MaybeUninit::<ArrayByValue>::uninit();
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::from_mut(&mut storage).cast::<MaybeUninit<u8>>(),
            size_of::<ArrayByValue>(),
        )
    };
    let init = ArrayByValue::init(bytes).unwrap();
    let _ = init.bytes(*b"nope");
}
