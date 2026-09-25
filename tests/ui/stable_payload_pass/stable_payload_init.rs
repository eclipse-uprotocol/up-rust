use core::mem::{size_of, MaybeUninit};
use up_rust::StablePayloadInit as _;

#[repr(C)]
#[derive(up_rust::StablePayload, up_rust::StablePayloadInit)]
#[stable_payload(type_name = "uprotocol.test.TrybuildPass")]
struct Payload {
    tag: u32,
    bytes: [u8; 4],
}

fn main() {
    let mut storage = MaybeUninit::<Payload>::uninit();
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::from_mut(&mut storage).cast::<MaybeUninit<u8>>(),
            size_of::<Payload>(),
        )
    };
    let payload = Payload::init(bytes)
        .unwrap()
        .tag(7)
        .bytes_fill_with(|index| index as u8)
        .finish();
    assert_eq!(payload.tag, 7);
}
