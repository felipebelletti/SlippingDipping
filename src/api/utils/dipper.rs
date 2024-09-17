use alloy::rpc::types::TransactionReceipt;
use revm::primitives::{keccak256, Address, Bytes, U256};
use alloy::sol_types::{sol_data::*, SolValue};

pub fn extract_dipper_cost_report(
    receipt: TransactionReceipt,
    dipper_address: Address,
) -> Option<U256> {
    let event_signature = keccak256(b"dipperCostReport(uint256)");

    receipt.inner.logs()
        .iter()
        .find_map(|log| {
            if log.address() == dipper_address && log.topic0() == Some(&event_signature) {
                decode_dipper_cost_report(&log.data().data)
            } else {
                None
            }
        })
}

fn decode_dipper_cost_report(encoded_data: &Bytes) -> Option<U256> {
    type EventData = (U256,);
    EventData::abi_decode(encoded_data, true).ok().map(|data| data.0)
}
