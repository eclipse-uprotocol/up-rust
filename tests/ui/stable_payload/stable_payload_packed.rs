use up_rust::StablePayload;

#[repr(C, packed)]
#[derive(StablePayload)]
#[stable_payload(type_name = "test.Packed")]
struct Packed {
    byte: u8,
    word: u32,
}

fn main() {}
