use up_rust::StablePayloadInit;

struct NotInitialized;

fn needs_stable_init<T: StablePayloadInit>() {}

fn main() {
    needs_stable_init::<NotInitialized>();
}
