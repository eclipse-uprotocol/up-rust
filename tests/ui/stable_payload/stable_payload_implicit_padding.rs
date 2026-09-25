use up_rust::{StablePayload, StablePayloadField};

#[repr(C)]
#[derive(StablePayload)]
#[stable_payload(type_name = "test.Padding")]
struct Padding {
    byte: u8,
    word: u32,
}

fn main() {
    let _ = Padding::field_representation();
}
