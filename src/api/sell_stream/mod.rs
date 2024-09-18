use alloy::primitives::aliases::U112;
use alloy::primitives::utils::{format_ether, format_units};
use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use alloy_erc20::{Erc20ProviderExt, LazyToken};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input};
use methods::{sell_from_single_wallet, sell_percentage_from_all_wallets};
use std::sync::Arc;
use tokio::sync::broadcast;
use types::{ApedWallet, ExtraCosts};

use crate::config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS};
use crate::globals::{V2_FACTORY_ADDRESS, V2_ROUTER_ADDRESS, WETH_ADDRESS};
use crate::{printlnt, UniswapV2Factory, UniswapV2Pair, UniswapV2Router01, ERC20};

use super::utils::erc20::check_and_approve;

mod methods;
pub mod types;

pub async fn run<M: Provider + 'static>(
    client: Arc<M>,
    maybe_token_address: Option<Address>,
    maybe_extra_costs: Option<ExtraCosts>,
) {
    let token_address: Address = {
        if maybe_token_address.is_none() {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the Token Address to monitor")
                .interact_text()
                .unwrap()
        } else {
            maybe_token_address.unwrap()
        }
    };

    let (tx, mut rx) = broadcast::channel::<String>(16);

    let client_clone = client.clone();
    let maybe_extra_costs = maybe_extra_costs.clone();
    let display_task = tokio::spawn({
        let client = client.clone();
        let token_address = token_address.clone();
        async move {
            display_sell_stream(client, token_address, maybe_extra_costs).await;
        }
    });

    let input_task = tokio::spawn({
        let tx = tx.clone();
        async move {
            loop {
                let sell_command = tokio::task::spawn_blocking(|| {
                    Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter sell command (e.g., qq, qw, q1)")
                        .interact_text()
                        .unwrap()
                })
                .await
                .unwrap();
                tx.send(sell_command).unwrap();
            }
        }
    });

    loop {
        tokio::select! {
            Ok(cmd) = rx.recv() => {
                handle_sell_command(client_clone.clone(), token_address.clone(), cmd).await;
            }
            else => break,
        }
    }

    let _ = display_task.await;
    let _ = input_task.await;
}

async fn display_sell_stream<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
    maybe_extra_costs: Option<ExtraCosts>,
) {
    let token = ERC20::new(token_address, client.clone());

    let decimals = token.decimals().call().await.unwrap()._0;

    let wallets = GLOBAL_WALLETS.get_wallets();

    let mut latest_block = client.get_block_number().await.unwrap_or_default();

    check_and_approve(
        client.clone(),
        &wallets,
        token_address,
        *V2_ROUTER_ADDRESS,
        false,
    )
    .await;

    loop {
        let current_block = client.get_block_number().await.unwrap_or_default();

        if true {
            latest_block = current_block;

            let mut total_token_balance_wei = U256::ZERO;
            let token_price_in_eth =
                match get_token_price_in_eth(client.clone(), token_address).await {
                    Ok(price_in_eth) => price_in_eth,
                    Err(err) => {
                        println!("{}", format!("Err get_token_price_in_eth: {err}").red());
                        continue;
                    }
                };

            println!(
                "{}",
                "─────────────────────────────────────────────────────────".green()
            );
            println!(
                "{}",
                format!("Block Number: {}", latest_block.to_string().cyan())
            );

            let mut total_eth_out = 0.0;
            let mut total_cost_eth = 0.0;

            for (index, wallet) in wallets.iter().enumerate() {
                let token_balance_wei = token.balanceOf(wallet.address).call().await.unwrap()._0;
                total_token_balance_wei += token_balance_wei;

                let token_balance_parsed = format_units(token_balance_wei, decimals)
                    .unwrap()
                    .parse::<f64>()
                    .unwrap();

                let eth_value = token_balance_parsed * token_price_in_eth;

                if let Some(ref extra_costs) = maybe_extra_costs
                    && let Some(ref aped_wallets) = extra_costs.aped_wallets
                {
                    let aped_wallet = match aped_wallets
                        .iter()
                        .find(|el| el.wallet.address == wallet.address)
                    {
                        Some(aped_wallet) => aped_wallet,
                        None => {
                            printlnt!("{}", format!("Mentioned `aped_wallet` entry with address \"{}\" isn't present within Wallets", wallet.address).red());
                            continue;
                        }
                    };

                    let weth_aped = aped_wallet.aped_weth;
                    let profit_percentage = ((eth_value / weth_aped) * 100.0) - 100.0;
                    total_eth_out += eth_value;
                    total_cost_eth += weth_aped;

                    println!(
                        "{}",
                        format!(
                            "[{}] Wallet: {} | {:.4} tokens => {:.4} ETH | Profit: {:.2}%",
                            index,
                            wallet.address.to_string().yellow(),
                            token_balance_parsed,
                            eth_value,
                            profit_percentage
                        )
                    );

                    continue;
                }

                println!(
                    "{}",
                    format!(
                        "➤ [{}] Wallet: {} | Balance: {:.4} | ETH Value: {:.4} ETH",
                        index,
                        wallet.address.to_string().yellow(),
                        token_balance_parsed,
                        eth_value
                    )
                );
            }

            if let Some(ref extra_costs) = maybe_extra_costs {
                if let Some(dipper_cost) = extra_costs.dipper_cost_eth {
                    println!("➤ Dipper Cost: {:.4} ETH", dipper_cost);
                    total_cost_eth += dipper_cost;
                }
                if let Some(gas_cost) = extra_costs.gas_cost_eth {
                    println!("➤ Gas Cost: {:.4} ETH", gas_cost);
                    total_cost_eth += gas_cost;
                }
            }

        let total_profit_percentage = ((total_eth_out / total_cost_eth) * 100.0) - 100.0;

            println!(
                "➤ Total Cost: {:.4} ETH => {:.4} ETH | Total Profit: {:.2}%",
                total_cost_eth, total_eth_out, total_profit_percentage
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

async fn get_token_price_in_eth<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
) -> Result<f64, Box<dyn std::error::Error>> {
    let factory = UniswapV2Factory::new(*V2_FACTORY_ADDRESS, client.clone());

    let pair_address = factory
        .getPair(token_address, *WETH_ADDRESS)
        .call()
        .await
        .unwrap()
        .pair;

    if pair_address == Address::ZERO {
        return Err("Pair address not found".into());
    }

    let pair = UniswapV2Pair::new(pair_address, client.clone());

    let (_, reserve0, reserve1) = {
        let response = pair.getReserves().call().await.unwrap();
        (
            response.blockTimestampLast,
            response.reserve0,
            response.reserve1,
        )
    };

    let token0 = LazyToken::new(pair.token0().call().await.unwrap()._0, &client);
    let token1 = LazyToken::new(pair.token1().call().await.unwrap()._0, &client);

    let decimals0 = token0
        .decimals()
        .await
        .map_err(|err| format!("Error getting token0->decimals(): {err}"))?;
    let decimals1 = token1
        .decimals()
        .await
        .map_err(|err| format!("Error getting token1->decimals(): {err}"))?;

    let reserve0_eth: f64 = format_units(U256::from(reserve0), *decimals0)
        .unwrap()
        .parse::<f64>()
        .map_err(|err| {
            format!("Error formatting reserve0 ({reserve0}, decimals={decimals0}) units: {err}")
        })?;
    let reserve1_eth: f64 = format_units(U256::from(reserve1), *decimals1)
        .unwrap()
        .parse::<f64>()
        .map_err(|err| {
            format!(
                "Error getting formatting reserve1 ({reserve0}, decimals={decimals0}) units: {err}"
            )
        })?;

    let price_in_eth = if token0.address() == &token_address {
        reserve1_eth / reserve0_eth
    } else {
        reserve0_eth / reserve1_eth
    };

    Ok(price_in_eth)
}

async fn handle_sell_command<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
    sell_command: String,
) {
    match sell_command.as_str() {
        "qq" => {
            // Sell 100% from every wallet
            sell_percentage_from_all_wallets(client.clone(), token_address, 100.0).await;
        }
        "qw" => {
            // Sell 50% from every wallet
            sell_percentage_from_all_wallets(client.clone(), token_address, 50.0).await;
        }
        cmd if cmd.starts_with('q') => {
            // Sell 100% from a single wallet by index
            if let Ok(index) = cmd[1..].parse::<usize>() {
                sell_from_single_wallet(client.clone(), token_address, index).await;
            } else {
                printlnt!("Invalid wallet index.");
            }
        }
        _ => {
            printlnt!("Invalid sell command.");
        }
    }
}
