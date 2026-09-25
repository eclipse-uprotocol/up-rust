use up_rust::StablePayloadField;

struct Manual(u8);

unsafe impl StablePayloadField for Manual {
    fn field_representation() -> up_rust::NativeRepresentation {
        up_rust::NativeRepresentation::new("manual", 0, 1, 1, up_rust::NativeByteOrder::native(), vec![]).unwrap()
    }

    fn validate_field_bytes(bytes: &[u8]) -> bool {
        bytes == [0]
    }
}

fn main() {}
