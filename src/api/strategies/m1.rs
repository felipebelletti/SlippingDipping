use alloy::{
    network::TransactionBuilder,
    primitives::{
        utils::{parse_ether, parse_units},
        Address,
    },
    rpc::types::TransactionRequest,
};
use alloy_erc20::LazyToken;
use revm::primitives::{Bytes, U256};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, Mutex},
    time::sleep,
};

use alloy::providers::Provider;
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input};

use crate::{
    api::{
        is_token_locked_on_dipper, unlock_token_on_dipper,
        utils::erc20::get_percentage_token_supply,
    }, config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS}, globals::{V2_FACTORY_ADDRESS, WETH_ADDRESS}, printlnt, Dipper
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

    if is_token_locked_on_dipper(&dipper, target_token_address).await {
        match unlock_token_on_dipper(&dipper, target_token_address).await {
            Ok(_) => {}
            Err(err) => {
                printlnt!("{}", err.to_string().red());
                return;
            }
        };
    };

    // M1 REQUIRED ARGS

    let predicted_pair_address = dipper
        .calculatePair(*WETH_ADDRESS, target_token_address, *V2_FACTORY_ADDRESS)
        .call()
        .await
        .unwrap()
        .pair;

    let dest_wallets = GLOBAL_WALLETS
        .get_wallets()
        .iter()
        .map(|wallet| Dipper::DestWallet {
            addr: wallet.address,
            amount: wallet.eth_amount_in_wei,
        })
        .collect::<Vec<_>>();

    let amount_buy_tokens: alloy::primitives::Uint<256, 4> = {
        if GLOBAL_CONFIG.sniping.tokens_amount.contains("%") {
            let percentage = GLOBAL_CONFIG
                .sniping
                .tokens_amount
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap();
            get_percentage_token_supply(&client, target_token_address, percentage).await
        } else {
            parse_units(
                &GLOBAL_CONFIG.sniping.tokens_amount.to_string(),
                *target_token_decimals,
            )
            .expect(&format!(
                "parse_units({}, {})",
                &GLOBAL_CONFIG.sniping.tokens_amount.to_string(),
                target_token_decimals
            ))
            .into()
        }
    };

    let unclog_eth_amount =
        parse_ether(&GLOBAL_CONFIG.sniping.eth_amount_for_unclogging.to_string()).unwrap();
    let min_eth_liquidity =
        parse_ether(&GLOBAL_CONFIG.sniping.min_eth_liquidity.to_string()).unwrap();
    let bribe_good =
        parse_ether(&GLOBAL_CONFIG.sniping.bribe_eth_good_validators.to_string()).unwrap();
    let bribe_bad =
        parse_ether(&GLOBAL_CONFIG.sniping.bribe_eth_bad_validators.to_string()).unwrap();

    // TRANSACTION FIELDS

    let initial_nonce = client
        .get_transaction_count(GLOBAL_WALLETS.wallets[0].address)
        .pending()
        .await
        .unwrap();

    let pending_nonce = Arc::new(Mutex::new(initial_nonce));

    let tx_value = parse_ether(
        &(GLOBAL_WALLETS.get_total_eth_amount()
            + GLOBAL_CONFIG.sniping.eth_amount_for_unclogging
            + GLOBAL_CONFIG.get_greatest_bribe_eth())
        .to_string(),
    )
    .unwrap();

    let max_fee_per_gas = parse_units(&GLOBAL_CONFIG.tx_builder.max_fee_per_gas.to_string(), 9)
        .unwrap()
        .to_string()
        .parse::<u128>()
        .map_err(|err| format!("Error converting tx_builder->max_fee_per_gas into u128: {err}"))
        .unwrap();

    let max_priority_fee_per_gas = parse_units(
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
        .snipe_gas_limit
        .parse::<u128>()
        .unwrap();

    // SPAM LOOP

    let success_flag = Arc::new(AtomicBool::new(false));
    loop {
        if success_flag.load(Ordering::SeqCst) {
            break;
        }

        let dipper = dipper.clone();
        let dest_wallets = dest_wallets.clone();
        let success_flag = success_flag.clone();
        let pending_nonce = pending_nonce.clone();

        tokio::spawn(async move {
            let mut nonce = pending_nonce.lock().await;

            let built_tx = dipper
                .m1_dipper(
                    amount_buy_tokens,
                    unclog_eth_amount,
                    GLOBAL_CONFIG.sniping.unclog_nloops,
                    min_eth_liquidity,
                    bribe_good,
                    bribe_bad,
                    GLOBAL_CONFIG.sniping.min_successfull_swaps,
                    GLOBAL_CONFIG.sniping.good_validators.clone(),
                    dest_wallets,
                    vec![*WETH_ADDRESS, target_token_address].into(),
                    predicted_pair_address,
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

            if receipt.status() {
                printlnt!(
                    "{}",
                    format!(
                        "SUCCESSFUL DIPPER TRANSACTION | {}",
                        receipt.transaction_hash
                    )
                    .green()
                );
                success_flag.store(true, Ordering::SeqCst);
            }
        });

        sleep(Duration::from_secs_f64(
            GLOBAL_CONFIG.sniping.spammer_secs_delay,
        ))
        .await;
    }
}

fn get_dipper_tx(
    from: Address,
    to: Address,
    calldata: Bytes,
    nonce: u64,
    value: U256,
    gas_limit: u128,
    max_fee_per_gas: u128,
    max_priority_fee_per_gas: u128,
) -> TransactionRequest {
    TransactionRequest::default()
        .with_from(from)
        .with_to(to)
        .with_input(calldata.clone())
        .with_nonce(nonce)
        .with_chain_id(1)
        .with_value(value)
        .with_gas_limit(gas_limit) // TODO get from config
        .with_max_fee_per_gas(max_fee_per_gas)
        .with_max_priority_fee_per_gas(max_priority_fee_per_gas)
}
