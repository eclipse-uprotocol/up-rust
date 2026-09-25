use core::mem::{size_of, MaybeUninit};
use up_rust::StablePayloadInit as _;

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "uprotocol.test.InitMissingField")]
struct InitMissingField {
    tag: u32,
    count: u32,
}

fn main() {
    let mut storage = MaybeUninit::<InitMissingField>::uninit();
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::from_mut(&mut storage).cast::<MaybeUninit<u8>>(),
            size_of::<InitMissingField>(),
        )
    };
    let init = InitMissingField::init(bytes).unwrap();
    let _ = init.tag(1).finish();
}
