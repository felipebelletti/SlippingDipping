use std::sync::Arc;

use alloy::consensus::Receipt;
use alloy::eips::eip2718::Encodable2718;
use alloy::rpc::types::TransactionReceipt;
use alloy::{
    consensus::SignableTransaction,
    network::{EthereumWallet, NetworkWallet, TransactionBuilder},
    providers::{Provider, WalletProvider},
};
use colored::Colorize;
use revm::primitives::{Address, U256};

use crate::config::general::GLOBAL_CONFIG;
use crate::{
    api::utils::tx_envelope_to_raw_tx,
    config::wallet::GLOBAL_WALLETS,
    globals::{V2_ROUTER_ADDRESS, WETH_ADDRESS},
    printlnt, UniswapV2Router01,
    ERC20::{self, balanceOfReturn},
};

pub async fn sell_percentage_from_all_wallets<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
    percentage: f64,
) {
    let wallets = GLOBAL_WALLETS.get_wallets();

    for (index, wallet) in wallets.iter().enumerate() {
        match sell_percentage_from_wallet(
            client.clone(),
            token_address,
            wallet.address,
            wallet.signer.clone(),
            percentage,
            index,
            false,
        )
        .await
        {
            Ok(receipt) => printlnt!(
                "{}",
                format!(
                    "Successfully sold {}% wallet | Address: {} | Hash: {}",
                    percentage, wallet.address, receipt.transaction_hash
                )
                .bright_green()
            ),
            Err(err) => {
                printlnt!("{}", format!("Error while selling wallet: {err}").red());
            }
        };
    }
}

pub async fn sell_from_single_wallet<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
    wallet_index: usize,
) {
    let wallets = GLOBAL_WALLETS.get_wallets();

    if wallet_index < wallets.len() {
        let wallet = &wallets[wallet_index];
        match sell_percentage_from_wallet(
            client.clone(),
            token_address,
            wallet.address,
            wallet.signer.clone(),
            100.0,
            wallet_index,
            false,
        )
        .await
        {
            Ok(receipt) => printlnt!(
                "{}",
                format!(
                    "Successfully sold 100% wallet | Address: {} | Hash: {}",
                    wallet.address, receipt.transaction_hash
                )
                .bright_green()
            ),
            Err(err) => {
                printlnt!("{}", format!("Error while selling wallet: {err}").red());
            }
        };
    } else {
        printlnt!("Wallet index out of range.");
    }
}

pub async fn sell_percentage_from_wallet<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
    wallet_address: Address,
    wallet_signer: Arc<EthereumWallet>,
    percentage: f64,
    wallet_index: usize,
    estimate_gas: bool,
) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
    let token = ERC20::new(token_address, client.clone());

    let balance = token
        .balanceOf(wallet_address)
        .call()
        .await
        .unwrap_or(balanceOfReturn { _0: U256::ZERO })
        ._0;

    if balance.is_zero() {
        return Err("No token balance for selling".into());
    }

    let amount_to_sell = (balance * U256::from((percentage * 100.0) as u64)) / U256::from(10000u64);

    if amount_to_sell.is_zero() {
        return Err("No token balance for selling".into());
    }

    let uniswap_router = UniswapV2Router01::new(*V2_ROUTER_ADDRESS, client.clone());
    let path = vec![token_address, *WETH_ADDRESS];

    let amounts_out = uniswap_router
        .getAmountsOut(amount_to_sell, path.clone())
        .call()
        .await
        .map_err(|err| format!("Error getting amounts out: {}", err))?;

    let expected_eth_amount = *amounts_out.amounts.last().unwrap();

    if expected_eth_amount.is_zero() {
        return Err("Expected ETH amount is zero, cannot proceed with swap".into());
    }

    let slippage_multiplier = U256::from(100u64 - GLOBAL_CONFIG.tx_builder.sell_slippage_percent as u64);
    let amount_out_min = (expected_eth_amount * slippage_multiplier) / U256::from(100u64);

    let nonce = client
        .get_transaction_count(wallet_address)
        .await
        .map_err(|err| format!("Expected nonce: {err}"))?;

    let estimate_eip1559_fees = client
        .estimate_eip1559_fees(None)
        .await
        .map_err(|err| format!("estimate_eip1559_fees err: {err}"))?;

    let signed_swap_raw_tx = {
        let mut swap_tx = uniswap_router
            .swapExactTokensForETHSupportingFeeOnTransferTokens(
                amount_to_sell,
                amount_out_min,
                path,
                wallet_address,
                U256::MAX,
            )
            .from(wallet_address)
            .nonce(nonce)
            .max_priority_fee_per_gas(estimate_eip1559_fees.max_priority_fee_per_gas)
            .max_fee_per_gas(estimate_eip1559_fees.max_fee_per_gas);

        if estimate_gas {
            let estimated_gas = client
                .estimate_gas(&swap_tx.clone().into_transaction_request())
                .await
                .map_err(|err| format!("approve estimate_gas err: {err}"))?;

            swap_tx = swap_tx.gas(estimated_gas);
        } else {
            swap_tx = swap_tx.gas(
                GLOBAL_CONFIG
                    .tx_builder
                    .sell_gas_limit
                    .parse::<u128>()
                    .unwrap(),
            );
        }

        tx_envelope_to_raw_tx(
            swap_tx
                .into_transaction_request()
                .build(&wallet_signer)
                .await
                .unwrap(),
        )
    };

    let tx = client
        .send_raw_transaction(&signed_swap_raw_tx)
        .await
        .map_err(|err| format!("Error sending raw swap transaction: {err}"))?;

    let receipt = tx
        .get_receipt()
        .await
        .map_err(|err| format!("Error getting swap tx receipt: {err}"))?;

    Ok(receipt)
}
