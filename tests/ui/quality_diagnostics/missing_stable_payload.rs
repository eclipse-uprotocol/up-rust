use up_rust::StablePayload;

struct NotStable;

fn needs_stable_payload<T: StablePayload>() {}

fn main() {
    needs_stable_payload::<NotStable>();
}
