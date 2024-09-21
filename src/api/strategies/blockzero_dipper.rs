use alloy::{
    contract::CallBuilder,
    dyn_abi::{abi::decode, DynSolType, DynSolValue},
    network::TransactionBuilder,
    primitives::{
        utils::{format_ether, format_units, parse_ether, parse_units},
        Address, Uint,
    },
    rpc::types::TransactionReceipt,
    transports::BoxTransport,
};
use alloy_erc20::LazyToken;
use revm::primitives::{keccak256, Bytes, FixedBytes, U256};
use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::Add,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{join, select, sync::Mutex, time::sleep};

use alloy::providers::Provider;
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input};

use crate::{
    api::{
        is_token_locked_on_dipper,
        mev_builders::{
            self,
            builder::Builder,
            types::{BundleResult, EndOfBlockBundleParams, SendBundleParams},
            BUILDERS,
        },
        sell_stream::{
            self,
            types::{ApedWallet, ExtraCosts},
        },
        unlock_token_on_dipper,
        utils::{
            dipper::extract_dipper_cost_report, get_raw_bribe_tx, print_pretty_dashboard,
            tx_envelope_to_raw_tx,
        },
    },
    config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS},
    globals::{V2_FACTORY_ADDRESS, WETH_ADDRESS},
    license, printlnt,
    Dipper::{self, exploitCall},
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

    let initial_nonce = client
        .get_transaction_count(wallets.get_wallets()[0].address)
        .pending()
        .await
        .unwrap();

    let tx_value = parse_ether(
        &(wallets.get_total_eth_amount() + GLOBAL_CONFIG.sniping.max_eth_spent_on_dipping)
            .to_string(),
    )
    .unwrap();

    let spam_config = SpamConfig {
        client: client.clone(),
        dipper: dipper.clone(),
        dest_wallets: dest_wallets.clone(),
        predicted_pair_address,
        expected_lp_variation_after_dip,
        max_eth_spent_on_dipping,
        min_eth_liquidity,
        tx_value,
        target_token_address,
        max_fee_per_gas_wei_config: parse_units(
            &GLOBAL_CONFIG.tx_builder.max_fee_per_gas.to_string(),
            9,
        )
        .unwrap()
        .to_string()
        .parse::<u128>()
        .unwrap(),
        max_priority_fee_per_gas_wei_config: parse_units(
            &GLOBAL_CONFIG
                .tx_builder
                .max_priority_fee_per_gas
                .to_string(),
            9,
        )
        .unwrap()
        .to_string()
        .parse::<u128>()
        .unwrap(),
        gas_limit: GLOBAL_CONFIG
            .tx_builder
            .dipper_gas_limit
            .parse::<u128>()
            .unwrap(),
    };

    let spam_state = SpamState {
        success_flag: Arc::new(AtomicBool::new(false)),
        total_gas_spent: Arc::new(Mutex::new(0u128)),
        pending_nonce: Arc::new(Mutex::new(initial_nonce)),
    };

    if GLOBAL_CONFIG.sniping.dipper_using_eob {
        start_eob_spamming(spam_config, spam_state).await;
        return;
    }

    start_normal_spamming(spam_config, spam_state).await;
}

struct SpamConfig<M: Provider + 'static> {
    client: Arc<M>,
    dipper: Arc<Dipper::DipperInstance<alloy::transports::BoxTransport, Arc<M>>>,
    dest_wallets: Vec<Dipper::SniperWallet>,
    predicted_pair_address: Address,
    expected_lp_variation_after_dip: U256,
    max_eth_spent_on_dipping: U256,
    min_eth_liquidity: U256,
    tx_value: U256,
    target_token_address: Address,
    max_fee_per_gas_wei_config: u128,
    max_priority_fee_per_gas_wei_config: u128,
    gas_limit: u128,
}

struct SpamState {
    success_flag: Arc<AtomicBool>,
    total_gas_spent: Arc<Mutex<u128>>,
    pending_nonce: Arc<Mutex<u64>>,
}

async fn start_eob_spamming<M: Provider + 'static>(config: SpamConfig<M>, state: SpamState) {
    loop {
        if state.success_flag.load(Ordering::SeqCst) {
            break;
        }

        let client = config.client.clone();
        let dipper = config.dipper.clone();
        let dest_wallets = config.dest_wallets.clone();
        let success_flag = state.success_flag.clone();
        let target_token_address = config.target_token_address;
        let predicted_pair_address = config.predicted_pair_address;
        let expected_lp_variation_after_dip = config.expected_lp_variation_after_dip;
        let max_eth_spent_on_dipping = config.max_eth_spent_on_dipping;
        let min_eth_liquidity = config.min_eth_liquidity;
        let tx_value = config.tx_value;
        let gas_limit = config.gas_limit;

        let caller_wallet = GLOBAL_WALLETS.get_wallets()[0].clone();

        tokio::spawn(async move {
            let target_block_number = client.get_block_number().await.unwrap() + 2;
            let nonce = client
                .get_transaction_count(caller_wallet.address)
                .await
                .unwrap();

            let (max_fee_per_gas, max_priority_fee_per_gas) = match client
                .estimate_eip1559_fees(None)
                .await
            {
                Ok(fees) => (fees.max_fee_per_gas, fees.max_priority_fee_per_gas),
                Err(err) => {
                    printlnt!("{}", format!("The estimate_eip1559_fees request failed. Therefore the transaction wasn't generated. Set `gas_oracle` under config.toml if that keeps happening a lot. Also remember to manually set the gas fees. {}", err));
                    return;
                }
            };

            let (encoded_dipper_tx, dipper_tx_hash) = {
                let dipper_tx = build_dipper_transaction(
                    &dipper,
                    nonce,
                    gas_limit,
                    tx_value,
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                    expected_lp_variation_after_dip,
                    max_eth_spent_on_dipping,
                    min_eth_liquidity,
                    predicted_pair_address,
                    target_token_address,
                    dest_wallets.clone(),
                );

                let signed_tx = dipper_tx
                    .into_transaction_request()
                    .build(&caller_wallet.signer)
                    .await
                    .unwrap();
                let tx_hash = signed_tx.tx_hash().to_owned();

                (tx_envelope_to_raw_tx(signed_tx), tx_hash)
            };

            let encoded_bribe_tx = get_raw_bribe_tx(
                client.clone(),
                caller_wallet.clone(),
                nonce + 1,
                GLOBAL_CONFIG.sniping.bribe_amount,
                U256::from(target_block_number),
            )
            .await
            .unwrap();

            let pseudo_eob_task = mev_builders::broadcast::broadcast_bundle(
                SendBundleParams {
                    txs: vec![hex::encode(encoded_dipper_tx.clone())],
                    block_number: Some(format!("0x{:x}", target_block_number)),
                    reverting_tx_hashes: Some(vec![dipper_tx_hash]),
                    builders: Some(vec![
                        "flashbots".to_string(),
                        "f1b.io".to_string(),
                        "rsync".to_string(),
                        // "beaverbuild.org".to_string(), // removed on purpose, we dont want beaver
                        "builder0x69".to_string(),
                        // "Titan".to_string(), // removed on purpose, Titan covers eob
                        "EigenPhi".to_string(),
                        "boba-builder".to_string(),
                        "Gambit Labs".to_string(),
                        "payload".to_string(),
                        "Loki".to_string(),
                        "BuildAI".to_string(),
                        "JetBuilder".to_string(),
                        "tbuilder".to_string(),
                        "penguinbuild".to_string(),
                        "bobthebuilder".to_string(),
                        "BTCS".to_string(),
                        "bloXroute".to_string(),
                    ]),
                    ..Default::default()
                },
                mev_builders::PSEUDO_EOB_BUILDERS.to_vec(),
            );

            let eob_task = mev_builders::broadcast::broadcast_end_of_block_bundle(
                EndOfBlockBundleParams {
                    txs: vec![
                        hex::encode(encoded_dipper_tx),
                        hex::encode(encoded_bribe_tx),
                    ],
                    block_number: Some(format!("0x{:x}", target_block_number)),
                    target_pools: Some(vec![predicted_pair_address]),
                    reverting_tx_hashes: None,
                },
                BUILDERS.to_vec(),
            );

            let (result_pseudo_eob_task, result_eob_task) = join!(pseudo_eob_task, eob_task);

            handle_task_result(result_pseudo_eob_task, &dipper_tx_hash, "Pseudo EoB Bundle");
            handle_task_result(result_eob_task, &dipper_tx_hash, "Normal EoB Bundle");

            let receipt = match client.get_transaction_receipt(dipper_tx_hash).await {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    printlnt!(
                        "{}",
                        format!("Bundle not landed | {}", &dipper_tx_hash).red()
                    );
                    return;
                }
                Err(err) => {
                    printlnt!("Error getting tx receipt: {err}");
                    return;
                }
            };

            let gas_used = receipt.gas_used;
            let gas_price = receipt.effective_gas_price;
            let gas_cost_in_wei = gas_used * gas_price;
            let gas_cost_in_eth: f64 = format_ether(gas_cost_in_wei).parse::<f64>().unwrap();

            if receipt.status() {
                printlnt!(
                    "{}",
                    format!(
                        "DIPPER SUCCESS | {} | Gas-Spent: {} ({:.4} ETH) | Total Spent on Gas: {:.4} ETH",
                        receipt.transaction_hash, gas_used, gas_cost_in_eth, gas_cost_in_eth
                    )
                    .green()
                );
                success_flag.store(true, Ordering::SeqCst);

                license::send_telemetry_message(format!(
                    "BlockZero Dipper successful tx: {}",
                    receipt.transaction_hash
                ));

                handle_successful_snipe(
                    client.clone(),
                    target_token_address,
                    receipt,
                    dipper.address().clone(),
                    0.0, // @TODO
                         // should be 0 for now, otherwise it'd duplicate the gas cost as we're calculating it using the wallet balance before dip block - after the dip block
                         // btw, that should be handled with the pre and post tx state, instead of (block+1) - (block-1) state. likely a to:do
                )
                .await;
                return;
            }
        });

        sleep(Duration::from_secs_f64(
            GLOBAL_CONFIG.sniping.spammer_secs_delay,
        ))
        .await;
    }
}

fn handle_task_result(
    task_results: HashMap<String, Result<BundleResult, String>>,
    tx_hash: &FixedBytes<32>,
    bundle_type: &str,
) {
    for (builder_name, result) in task_results {
        match result {
            Ok(task_ok) => {
                printlnt!(
                    "{}",
                    format!(
                        "{} Sent | TxHash: {} | Bundle Hash: {} | Builder: {}",
                        bundle_type, tx_hash, task_ok.bundle_hash, builder_name
                    )
                    .yellow()
                )
            }
            Err(err) => {
                printlnt!(
                    "{}",
                    format!(
                        "{} Error | Reason: {} | Builder: {}",
                        bundle_type, err, builder_name
                    )
                    .red()
                )
            }
        }
    }
}

async fn start_normal_spamming<M: Provider + 'static>(config: SpamConfig<M>, state: SpamState) {
    loop {
        if state.success_flag.load(Ordering::SeqCst) {
            break;
        }

        let client = config.client.clone();
        let dipper = config.dipper.clone();
        let dest_wallets = config.dest_wallets.clone();
        let success_flag = state.success_flag.clone();
        let total_gas_spent = state.total_gas_spent.clone();
        let pending_nonce = state.pending_nonce.clone();
        let target_token_address = config.target_token_address;
        let predicted_pair_address = config.predicted_pair_address;
        let expected_lp_variation_after_dip = config.expected_lp_variation_after_dip;
        let max_eth_spent_on_dipping = config.max_eth_spent_on_dipping;
        let min_eth_liquidity = config.min_eth_liquidity;
        let tx_value = config.tx_value;
        let gas_limit = config.gas_limit;
        let max_fee_per_gas_wei_config = config.max_fee_per_gas_wei_config;
        let max_priority_fee_per_gas_wei_config = config.max_priority_fee_per_gas_wei_config;

        tokio::spawn(async move {
            let mut nonce = pending_nonce.lock().await;

            let (max_fee_per_gas, max_priority_fee_per_gas) = {
                if GLOBAL_CONFIG.tx_builder.gas_oracle {
                    match client.estimate_eip1559_fees(None).await {
                        Ok(fees) => (fees.max_fee_per_gas, fees.max_priority_fee_per_gas),
                        Err(err) => {
                            printlnt!("{}", format!("The estimate_eip1559_fees request failed. Therefore the transaction wasn't generated. Set `gas_oracle` under config.toml if that keeps happening a lot. Also remember to manually set the gas fees. {}", err));
                            return;
                        }
                    }
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

            let built_tx = build_dipper_transaction(
                &dipper,
                *nonce,
                gas_limit,
                tx_value,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                expected_lp_variation_after_dip,
                max_eth_spent_on_dipping,
                min_eth_liquidity,
                predicted_pair_address,
                target_token_address,
                dest_wallets.clone(),
            );

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

                handle_successful_snipe(
                    client.clone(),
                    target_token_address,
                    receipt,
                    dipper.address().clone(),
                    total_gas_spent_eth,
                )
                .await;
                return;
            }

            printlnt!(
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

fn build_dipper_transaction<M: Provider + 'static>(
    dipper: &Arc<Dipper::DipperInstance<alloy::transports::BoxTransport, Arc<M>>>,
    nonce: u64,
    gas_limit: u128,
    tx_value: U256,
    max_fee_per_gas: u128,
    max_priority_fee_per_gas: u128,
    expected_lp_variation_after_dip: U256,
    max_eth_spent_on_dipping: U256,
    min_eth_liquidity: U256,
    predicted_pair_address: Address,
    target_token_address: Address,
    dest_wallets: Vec<Dipper::SniperWallet>,
) -> CallBuilder<BoxTransport, &Arc<M>, PhantomData<exploitCall>> {
    dipper
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
        .nonce(nonce)
        .value(tx_value)
        .max_fee_per_gas(max_fee_per_gas)
        .max_priority_fee_per_gas(max_priority_fee_per_gas)
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
            // @todo-1234: following the previous to:do market we've just set, aped_weth will include **every** cost
            // so it should be handled as aped_weth + gas + dip cost
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

    let total_operation_cost_eth = total_gas_spent_eth + dipper_cost_eth_f64;

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

    sell_stream::run(
        client,
        Some(token_address),
        Some(ExtraCosts {
            aped_wallets: Some(aped_wallets),
            dipper_cost_eth: Some(dipper_cost_eth_f64),
            gas_cost_eth: Some(total_gas_spent_eth),
        }),
    )
    .await;
}
