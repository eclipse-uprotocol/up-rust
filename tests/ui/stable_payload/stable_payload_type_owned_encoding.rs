use up_rust::StablePayload;

#[repr(C)]
#[derive(StablePayload)]
#[stable_payload(type_name = "test.TypeOwnedId", payload_encoding_id = 0xF000)]
struct TypeOwnedId {
    word: u32,
}

fn main() {}
