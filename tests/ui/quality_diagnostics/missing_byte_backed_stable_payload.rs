use up_rust::{ByteBackedStablePayload, StablePayload};

#[repr(C)]
struct StableOnly(u8);

unsafe impl StablePayload for StableOnly {
    const TYPE_NAME: &'static str = "com.example.StableOnlyV1";
}

fn needs_byte_backed<T: ByteBackedStablePayload>() {}

fn main() {
    needs_byte_backed::<StableOnly>();
}
