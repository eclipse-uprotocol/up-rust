use up_rust::{
    verify_uninit_tx_buffer_payload_layout, LoanedPayloadUninitMut, UUninitTxBuffer,
    UZeroCopyUninitTransportCore, UZeroCopyUninitTransportImpl,
};

fn assert_uninit_buffer<T: UUninitTxBuffer>() {}
fn assert_uninit_transport_impl<T: UZeroCopyUninitTransportImpl>() {}
fn assert_uninit_transport_core<T: UZeroCopyUninitTransportCore>() {}

fn verify_layout<T: UUninitTxBuffer>(buffer: &mut T) {
    let _ = verify_uninit_tx_buffer_payload_layout(buffer, 0, 1);
}

fn accept_loan(_: LoanedPayloadUninitMut<'_>) {}

fn main() {}
