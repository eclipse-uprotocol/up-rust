use up_rust::{Unvalidated, UOwnedFrame, UOwnedTransport};

async fn send_unvalidated<T>(transport: &T, frame: UOwnedFrame<Unvalidated>)
where
    T: UOwnedTransport + ?Sized,
{
    transport.send_owned(frame).await.unwrap();
}

fn main() {}
