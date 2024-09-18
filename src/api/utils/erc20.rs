use std::sync::Arc;

use alloy::{
    network::{EthereumWallet, NetworkWallet, TransactionBuilder},
    primitives::{Address, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use alloy_erc20::LazyToken;
use colored::Colorize;
use tokio::task::JoinHandle;

use crate::{
    config::{general::GLOBAL_CONFIG, wallet::types::Wallet},
    globals::V2_ROUTER_ADDRESS,
    printlnt, ERC20,
};

use super::tx_envelope_to_raw_tx;

pub async fn get_percentage_token_supply<M: Provider>(
    provider: &M,
    token_address: Address,
    percentage: f64,
) -> alloy::primitives::Uint<256, 4> {
    let supported_precision = 8; // Support up to 8 decimal places
    let base: u64 = 10_u64.pow(supported_precision as u32);

    let token = LazyToken::new(token_address, provider);
    let total_supply = token.total_supply().await.unwrap();

    let percentage_scaled = (percentage * base as f64).round() as u64;

    let scaled_total_supply = total_supply * U256::from(percentage_scaled);
    let scaled_base = U256::from(base as u64 * 100); // Multiply base by 100 to handle 1.00% as 100

    let tokens_amount = scaled_total_supply / scaled_base;

    tokens_amount
}

pub async fn approve_token<M: Provider>(
    client: Arc<M>,
    wallet: Wallet,
    token_address: Address,
    spender: Address,
    estimate_gas: bool,
) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
    let token = ERC20::new(token_address, &client);
    let wallet_address: Address = wallet.address;
    let wallet_signer = wallet.signer;

    let nonce = client
        .get_transaction_count(wallet_address)
        .await
        .map_err(|err| format!("Expected nonce: {err}"))?;

    let estimate_eip1559_fees = client
        .estimate_eip1559_fees(None)
        .await
        .map_err(|err| format!("estimate_eip1559_fees err: {err}"))?;

    let encoded_tx = {
        let mut tx = token
            .approve(spender, U256::MAX)
            .from(wallet_address)
            .nonce(nonce)
            .max_priority_fee_per_gas(estimate_eip1559_fees.max_priority_fee_per_gas)
            .max_fee_per_gas(estimate_eip1559_fees.max_fee_per_gas);

        if estimate_gas {
            let estimated_gas = client
                .estimate_gas(&tx.clone().into_transaction_request())
                .await
                .map_err(|err| format!("approve estimate_gas err: {err}"))?;

            tx = tx.gas(estimated_gas);
        } else {
            tx = tx.gas(
                GLOBAL_CONFIG
                    .tx_builder
                    .approve_gas_limit
                    .parse::<u128>()
                    .unwrap(),
            )
        }

        tx_envelope_to_raw_tx(
            tx.into_transaction_request()
                .build(&wallet_signer)
                .await
                .unwrap(),
        )
    };

    let tx = client
        .send_raw_transaction(&encoded_tx)
        .await
        .map_err(|err| format!("Error creating the approve rawTx: {err}"))?;

    let receipt = tx
        .get_receipt()
        .await
        .map_err(|err| format!("Error getting approve tx receipt: {err}"))?;

    Ok(receipt)
}

pub async fn check_and_approve<M: Provider + 'static>(
    client: Arc<M>,
    wallets: &Vec<Wallet>,
    token_address: Address,
    spender: Address,
    wait_tasks: bool,
) {
    let mut tasks: Vec<JoinHandle<()>> = vec![];

    for wallet in wallets.clone() {
        let client = client.clone();

        let task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let token = Arc::new(ERC20::new(token_address, &client));
            let wallet_address = wallet.address;

            let allowance = match token.allowance(wallet_address, spender).call().await {
                Ok(allowance) => allowance._0,
                Err(err) => {
                    printlnt!("{}", format!("Could not get the allowance for owner \"{}\" and spender \"{spender}\": {err}", wallet.address).red());
                    return;
                }
            };

            if allowance == U256::MAX {
                return;
            }

            match approve_token(client, wallet, token_address, spender, true).await {
                Ok(receipt) => {
                    printlnt!(
                        "{}",
                        format!(
                            "Token Approved | Wallet: {} | Hash: {}",
                            wallet_address, receipt.transaction_hash
                        )
                        .bright_green()
                    );
                }
                Err(err) => {
                    printlnt!(
                        "{}",
                        format!(
                            "Token Approval Error | Wallet: {} | Err: {err}",
                            wallet_address
                        )
                        .red()
                    )
                }
            };
        });

        tasks.push(task);
    }

    if wait_tasks {
        futures::future::join_all(tasks).await;
    }
}
