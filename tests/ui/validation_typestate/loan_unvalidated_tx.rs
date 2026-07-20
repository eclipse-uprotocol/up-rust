use up_rust::{Unvalidated, UTxLoanSpec, UZeroCopyTransport};

async fn loan_unvalidated<T>(transport: &T, spec: UTxLoanSpec<Unvalidated>)
where
    T: UZeroCopyTransport + ?Sized,
{
    transport.loan_tx(spec).await.unwrap();
}

fn main() {}
