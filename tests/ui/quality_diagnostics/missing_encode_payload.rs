use up_rust::{payload::codec::RawBytes, EncodePayload};

fn needs_encoder<C: EncodePayload<u32>>() {}

fn main() {
    needs_encoder::<RawBytes>();
}
