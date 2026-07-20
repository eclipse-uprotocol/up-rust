use up_rust::payload::stable::{UnsafeStablePayloadTxSlot, ZeroedStablePayloadTxSlot};

fn main() {
    let _tx_slot: Option<UnsafeStablePayloadTxSlot<'_, ()>> = None;
    let _zeroed_slot: Option<ZeroedStablePayloadTxSlot<'_, ()>> = None;
}
