use std::io::Cursor;

use up_rust::{
    LoanedPayload, NativePrefixFrameMetadataCodec, StableContainerWireFormat,
    UEncodedLoanedRxFrame, UEncodedRxFrame, UWireRx,
};

#[repr(C)]
#[derive(up_rust::StablePayload)]
#[stable_payload(type_name = "uprotocol.test.UnsafePass")]
struct Payload {
    value: bool,
}

struct Raw;

impl UEncodedRxFrame for Raw {
    type PayloadReader<'a> = Cursor<&'a [u8]>;
    type PayloadSlices<'a> = core::iter::Once<&'a [u8]>;

    fn encoded_metadata(&self) -> &[u8] { &[] }
    fn payload_len(&self) -> usize { 0 }
    fn payload_reader(&self) -> Self::PayloadReader<'_> { Cursor::new(&[]) }
    fn payload_slices(&self) -> Self::PayloadSlices<'_> { core::iter::once(&[]) }
}

impl UEncodedLoanedRxFrame for Raw {
    fn loaned_contiguous_payload(&self) -> Result<LoanedPayload<'_>, up_rust::UWireError> {
        unreachable!()
    }
}

fn borrow(frame: &UWireRx<Raw, StableContainerWireFormat, NativePrefixFrameMetadataCodec>) {
    let _ = unsafe { frame.borrow_payload_unchecked::<Payload>() };
}

fn main() {}
