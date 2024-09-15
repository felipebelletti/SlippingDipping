use std::sync::Arc;

#[allow(unused_imports)]
use crate::common::*;
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::types::TransactionReceipt;

use crate::Dipper;
use colored::Colorize;

pub mod strategies;
pub mod utils;
pub mod simulate;

pub async fn unlock_token_on_dipper<M: Provider>(
    dipper: &Dipper::DipperInstance<alloy::transports::BoxTransport, Arc<M>>,
    target_token_address: Address,
) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
    let method = dipper.removeLock(target_token_address);

    let receipt = method
        .send()
        .await
        .unwrap()
        .with_required_confirmations(1)
        .get_receipt()
        .await
        .unwrap();

    if !receipt.status() {
        return Err(format!("Could not unlock the Dipper contract for Token-Address \"{}\", check the failed transaction on https://etherscan.io/tx/{}", target_token_address, receipt.transaction_hash).into());
    }

    Ok(receipt)
}

pub async fn is_token_locked_on_dipper<M: Provider>(
    dipper: &Dipper::DipperInstance<alloy::transports::BoxTransport, Arc<M>>,
    target_token_address: Address,
) -> bool {
    return dipper.locks(target_token_address).call().await.unwrap()._0;
}
