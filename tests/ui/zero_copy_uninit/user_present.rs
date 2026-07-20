use up_rust::{
    payload::loan::{LoanUninitPayload, LoanedInitPayload, LoanedUninitPayload},
    StableContainerPayload, StablePayloadInit, UZeroCopyUninitTransport,
    UZeroCopyUninitTransportExt,
};

fn assert_codec<C, T>()
where
    C: LoanUninitPayload<T>,
{
}

fn assert_stable_codec<T>()
where
    T: StablePayloadInit,
    StableContainerPayload<T>: LoanUninitPayload<T>,
{
}

fn assert_transport<T>()
where
    T: UZeroCopyUninitTransport + UZeroCopyUninitTransportExt,
{
}

fn accept_slots<T>(_: LoanedUninitPayload<'_, T>, _: LoanedInitPayload<'_, T>) {}

fn stable_loan_entry<T: StablePayloadInit>(payload: up_rust::LoanedPayloadUninitMut<'_>) {
    let _ = T::init_from_uninit_payload(payload);
}

fn main() {}
