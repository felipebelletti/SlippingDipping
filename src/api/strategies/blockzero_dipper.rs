use alloy::{
    dyn_abi::{abi::decode, DynSolType, DynSolValue},
    primitives::{
        utils::{format_ether, format_units, parse_ether, parse_units},
        Address, Uint,
    },
    rpc::types::TransactionReceipt,
};
use alloy_erc20::LazyToken;
use revm::primitives::{keccak256, Bytes, U256};
use std::{
    ops::Add,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::Mutex, time::sleep};

use alloy::providers::Provider;
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input};

use crate::{
    api::{
        is_token_locked_on_dipper,
        sell_stream::{self, types::{ApedWallet, ExtraCosts}},
        unlock_token_on_dipper,
        utils::{dipper::extract_dipper_cost_report, print_pretty_dashboard},
    },
    config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS},
    globals::{V2_FACTORY_ADDRESS, WETH_ADDRESS},
    license, printlnt, Dipper,
};

pub async fn run<M: Provider + 'static>(client: Arc<M>) {
    let dipper: Arc<Dipper::DipperInstance<alloy::transports::BoxTransport, Arc<M>>> =
        Arc::new(Dipper::new(
            GLOBAL_CONFIG.general.dipper_contract.clone(),
            client.clone(),
        ));

    let target_token_address: Address = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Token Address")
        .interact_text()
        .unwrap();
    let target_token = LazyToken::new(target_token_address, &client);
    let target_token_decimals = target_token.decimals().await.unwrap();

    license::send_telemetry_message(format!(
        "Running BlockZero Dipper targeting: {}",
        target_token_address
    ));

    if is_token_locked_on_dipper(&dipper, target_token_address).await {
        match unlock_token_on_dipper(&dipper, target_token_address).await {
            Ok(_) => {}
            Err(err) => {
                printlnt!("{}", err.to_string().red());
                return;
            }
        };
    };

    let wallets = GLOBAL_WALLETS
        .clone()
        .resolve_tokens_amount(client.clone(), target_token_address, target_token_decimals)
        .await;

    // BLOCKZERO REQUIRED ARGS

    let predicted_pair_address = dipper
        .calculatePair(*WETH_ADDRESS, target_token_address, *V2_FACTORY_ADDRESS)
        .call()
        .await
        .unwrap()
        .pair;

    let dest_wallets = wallets
        .get_wallets()
        .iter()
        .map(|wallet| Dipper::SniperWallet {
            addr: wallet.address,
            ethAmount: wallet.eth_amount_in_wei,
            tokensAmount: wallet.tokens_amount_in_wei,
        })
        .collect::<Vec<_>>();

    let expected_lp_variation_after_dip_f64 = GLOBAL_CONFIG.sniping.expected_lp_variation_after_dip;

    if expected_lp_variation_after_dip_f64 < 0.0 || expected_lp_variation_after_dip_f64 > 100.0 {
        panic!("Expected LP variation after dip must be between 0.0 and 100.0");
    }

    let expected_lp_variation_after_dip = U256::from(expected_lp_variation_after_dip_f64 * 100.0);
    let max_eth_spent_on_dipping =
        parse_ether(&GLOBAL_CONFIG.sniping.max_eth_spent_on_dipping.to_string()).unwrap();
    let min_eth_liquidity =
        parse_ether(&GLOBAL_CONFIG.sniping.min_eth_liquidity.to_string()).unwrap();

    // TRANSACTION FIELDS

    let initial_nonce = client
        .get_transaction_count(wallets.get_wallets()[0].address)
        .pending()
        .await
        .unwrap();

    let pending_nonce = Arc::new(Mutex::new(initial_nonce));

    let tx_value = parse_ether(
        &(wallets.get_total_eth_amount() + GLOBAL_CONFIG.sniping.max_eth_spent_on_dipping)
            .to_string(),
    )
    .unwrap();

    let max_fee_per_gas_wei_config =
        parse_units(&GLOBAL_CONFIG.tx_builder.max_fee_per_gas.to_string(), 9)
            .unwrap()
            .to_string()
            .parse::<u128>()
            .map_err(|err| format!("Error converting tx_builder->max_fee_per_gas into u128: {err}"))
            .unwrap();

    let max_priority_fee_per_gas_wei_config = parse_units(
        &GLOBAL_CONFIG
            .tx_builder
            .max_priority_fee_per_gas
            .to_string(),
        9,
    )
    .unwrap()
    .to_string()
    .parse::<u128>()
    .unwrap();

    let gas_limit = GLOBAL_CONFIG
        .tx_builder
        .dipper_gas_limit
        .parse::<u128>()
        .unwrap();

    // SPAM LOOP

    let total_gas_spent = Arc::new(Mutex::new(0u128));
    let success_flag = Arc::new(AtomicBool::new(false));

    loop {
        if success_flag.load(Ordering::SeqCst) {
            break;
        }

        let client = client.clone();
        let dipper = dipper.clone();
        let dest_wallets = dest_wallets.clone();
        let success_flag = success_flag.clone();
        let total_gas_spent = total_gas_spent.clone();
        let pending_nonce = pending_nonce.clone();

        tokio::spawn(async move {
            let mut nonce = pending_nonce.lock().await;

            let (max_fee_per_gas, max_priority_fee_per_gas) = {
                if GLOBAL_CONFIG.tx_builder.gas_oracle {
                    let fees = match client.estimate_eip1559_fees(None).await {
                        Ok(fees) => (fees.max_fee_per_gas, fees.max_priority_fee_per_gas),
                        Err(err) => {
                            printlnt!("{}", format!("The estimate_eip1559_fees request failed. Therefore the transaction wasn't generated. Set `gas_oracle` under config.toml if that keeps happening a lot. Also remember to manually set the gas fees. {}", err));
                            return;
                        }
                    };

                    fees
                } else {
                    (
                        max_fee_per_gas_wei_config,
                        max_priority_fee_per_gas_wei_config,
                    )
                }
            };

            let formatted_max_fee_per_gas =
                format_units(max_fee_per_gas, 9).unwrap_or("unknown".to_string());
            let formatted_max_priority_fee_per_gas =
                format_units(max_priority_fee_per_gas, 9).unwrap_or("unknown".to_string());

            if GLOBAL_CONFIG.tx_builder.gas_oracle {
                printlnt!("{}", format!("🌊🌊 Gas Oracle prices applied! MaxFeePerGas = {formatted_max_fee_per_gas}, MaxPriorityFee = {formatted_max_priority_fee_per_gas}").bright_cyan())
            }

            let built_tx = dipper
                .exploit(
                    GLOBAL_CONFIG.sniping.max_dipper_rounds,
                    expected_lp_variation_after_dip,
                    max_eth_spent_on_dipping,
                    min_eth_liquidity,
                    GLOBAL_CONFIG.sniping.swap_threshold_tokens_amount,
                    GLOBAL_CONFIG.sniping.max_failed_user_swaps,
                    predicted_pair_address,
                    vec![*WETH_ADDRESS, target_token_address].into(),
                    dest_wallets,
                )
                .gas(gas_limit)
                .nonce(*nonce)
                .value(tx_value)
                .max_fee_per_gas(max_fee_per_gas)
                .max_priority_fee_per_gas(max_priority_fee_per_gas);

            let mut tx = match built_tx.send().await {
                Ok(tx) => tx,
                Err(err) => {
                    printlnt!("Error broadcasting tx: {err}");
                    return;
                }
            };
            *nonce += 1;

            printlnt!(
                "{}",
                format!("Dipper Transaction Broadcasted | {}", tx.tx_hash()).cyan()
            );

            tx.set_required_confirmations(1);
            let receipt = match tx.get_receipt().await {
                Ok(receipt) => receipt,
                Err(err) => {
                    printlnt!("Error getting tx receipt: {err}");
                    return;
                }
            };

            let gas_used = receipt.gas_used;
            let gas_price = receipt.effective_gas_price;
            let gas_cost_in_wei = gas_used * gas_price;
            let mut total_gas_spent_lock = total_gas_spent.lock().await;
            *total_gas_spent_lock += gas_cost_in_wei;

            let gas_cost_in_eth: f64 = format_ether(gas_cost_in_wei).parse::<f64>().unwrap();
            let total_gas_spent_eth = format_ether(*total_gas_spent_lock).parse::<f64>().unwrap();

            if receipt.status() {
                printlnt!(
                    "{}",
                    format!(
                        "DIPPER SUCCESS | {} | Gas-Spent: {} ({:.4} ETH) | Total Spent on Gas: {:.4} ETH | FeePerGas: {} | FeePriority: {}",
                        receipt.transaction_hash, gas_used, gas_cost_in_eth, total_gas_spent_eth, formatted_max_fee_per_gas, formatted_max_priority_fee_per_gas
                    )
                    .green()
                );
                success_flag.store(true, Ordering::SeqCst);

                license::send_telemetry_message(format!(
                    "BlockZero Dipper successful tx: {}",
                    receipt.transaction_hash
                ));

                client
                    .get_transaction_by_hash(receipt.transaction_hash)
                    .await
                    .unwrap()
                    .unwrap();

                handle_successful_snipe(
                    client,
                    target_token_address,
                    receipt,
                    dipper.address().clone(),
                    total_gas_spent_eth,
                )
                .await;
                return;
            }

            println!(
                "{}",
                format!(
                    "Failed | {} | Gas-Spent: {} ({} ETH) | Total Spent on Gas: {} ETH | FeePerGas: {} | FeePriority: {}",
                    receipt.transaction_hash, gas_used, gas_cost_in_eth, total_gas_spent_eth, formatted_max_fee_per_gas, formatted_max_priority_fee_per_gas
                )
                .red()
            );
        });

        sleep(Duration::from_secs_f64(
            GLOBAL_CONFIG.sniping.spammer_secs_delay,
        ))
        .await;
    }
}

async fn handle_successful_snipe<M: Provider + 'static>(
    client: Arc<M>,
    token_address: Address,
    receipt: TransactionReceipt,
    dipper_address: Address,
    total_gas_spent_eth: f64,
) {
    let dipper_block = receipt.block_number.unwrap();

    let mut aped_wallets: Vec<ApedWallet> = vec![];
    for wallet in GLOBAL_WALLETS.get_wallets() {
        let balance_before_dipper_block = client
            .get_balance(wallet.address)
            .number(dipper_block - 1)
            .await
            .unwrap();
        let balance_after_dipper_block = client
            .get_balance(wallet.address)
            .number(dipper_block)
            .await
            .unwrap();

        aped_wallets.push(ApedWallet {
            wallet: wallet.clone(),
            aped_weth: format_ether(balance_before_dipper_block - balance_after_dipper_block)
                .parse::<f64>()
                .unwrap(),
        })
    }

    let maybe_dipper_cost_wei = extract_dipper_cost_report(receipt, dipper_address);
    let dipper_cost_eth_str: String = {
        if let Some(dipper_cost_wei) = maybe_dipper_cost_wei {
            format_ether(dipper_cost_wei)
        } else {
            "[unknown]".to_string()
        }
    };
    let dipper_cost_eth_f64 = dipper_cost_eth_str.parse::<f64>().unwrap_or(0.0);

    let total_operation_cost_eth =
        total_gas_spent_eth + dipper_cost_eth_f64;

    print_pretty_dashboard(
        "Dipper Cost Report",
        vec![
            format!(
                "➤ Total Gas Cost: {:.4} ETH",
                total_gas_spent_eth.to_string().yellow()
            ),
            format!(
                "➤ Dipping Cost (Buying and Selling bags): {:.4} ETH",
                dipper_cost_eth_str.yellow()
            ),
            format!(
                "➤ Total Cost (Dipping + Gas): {:.4} ETH",
                total_operation_cost_eth.to_string().yellow()
            ),
        ],
    );

    sell_stream::run(client, Some(token_address), Some(ExtraCosts {
        aped_wallets: Some(aped_wallets),
        dipper_cost_eth: Some(dipper_cost_eth_f64),
        gas_cost_eth: Some(total_gas_spent_eth)
    })).await;
}
