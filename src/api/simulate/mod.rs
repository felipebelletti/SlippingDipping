use std::{fmt::Debug, sync::Arc};

use actions::{
    add_liquidity, approve, calculate_pair_address, enable_trading, get_token_balance,
    transfer_erc20_tokens,
};
use actors::{me, weth_addr};
use alloy::{
    dyn_abi::abi,
    eips::{BlockId, BlockNumberOrTag},
    node_bindings::Anvil,
    primitives::{
        utils::{format_ether, parse_ether, parse_units},
        Bytes,
    },
    providers::{Provider, ProviderBuilder},
    rpc::types::state::{AccountOverride, StateOverride},
    sol,
    sol_types::SolCall,
};
use alloy_erc20::arbitrum::WETH;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use helpers::{
    bytes_to_address, bytes_to_u256, insert_mapping_storage_slot, one_ether, revm_call, revm_revert,
};
use hex::FromHex;
use revm::{
    db::{AlloyDB, CacheDB, EmptyDB},
    primitives::{keccak256, Address, U256},
    Database, DatabaseCommit, DatabaseRef, Evm, InMemoryDB,
};

use crate::{
    config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS},
    globals::{self, V2_FACTORY_ADDRESS, V2_ROUTER_ADDRESS, WETH_ADDRESS},
    printlnt,
    Dipper::{calculatePairCall, exploitCall},
    UniswapV2Router01::addLiquidityETHCall,
    ERC20::{approveCall, balanceOfCall},
};

use super::utils::print_pretty_dashboard;

mod actions;
mod actors;
mod helpers;

sol! {
    function owner() public view returns (address);
}

pub async fn simulate<M: Provider + Clone>(client: &M) {
    let dipper_contract_address = GLOBAL_CONFIG.general.dipper_contract;
    let dipper_caller_address = GLOBAL_WALLETS.get_wallets()[0].address;

    let target_token: Address = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Token Address")
        .interact_text()
        .unwrap();

    let mut cache_db: CacheDB<
        AlloyDB<alloy::transports::BoxTransport, alloy::network::Ethereum, M>,
    > = CacheDB::new(AlloyDB::new(client.clone(), BlockId::latest()).unwrap());
    let mut evm = { Evm::builder().with_ref_db(&mut cache_db).build() };
    evm.cfg_mut().disable_balance_check = true;
    evm.cfg_mut().disable_block_gas_limit = true;
    evm.cfg_mut().disable_base_fee = true;
    evm.cfg_mut().limit_contract_code_size = Some(0x100000);
    // let mut db = evm.db_mut();

    // evm.tx().caller = target_token;

    let owner_address = bytes_to_address(
        revm_call(
            &mut evm,
            me(),
            target_token,
            Bytes::from(ownerCall {}.abi_encode()),
            U256::ZERO,
        )
        .unwrap(),
    )
    .unwrap();

    // fund accounts
    evm.db_mut()
        .0
        .load_account(owner_address)
        .unwrap()
        .info
        .balance = U256::MAX;
    evm.db_mut()
        .0
        .load_account(dipper_caller_address)
        .unwrap()
        .info
        .balance = U256::MAX;

    loop {
        let calculated_pair_address = calculate_pair_address(
            &mut evm,
            *WETH_ADDRESS,
            target_token,
            dipper_caller_address,
            dipper_contract_address,
        )
        .unwrap_or_default();

        let owner_token_balance =
            get_token_balance(&mut evm, owner_address, target_token).unwrap_or(U256::ZERO);

        let eth_lp_amount = get_token_balance(&mut evm, calculated_pair_address, *WETH_ADDRESS)
            .unwrap_or(U256::ZERO);

        let tokens_lp_amount = get_token_balance(&mut evm, calculated_pair_address, target_token)
            .unwrap_or(U256::ZERO);

        let clogged_tokens_amount =
            get_token_balance(&mut evm, target_token, target_token).unwrap_or(U256::ZERO);

        let clogged_percentage = if tokens_lp_amount != U256::ZERO {
            clogged_tokens_amount * U256::from(100u64) / tokens_lp_amount
        } else {
            U256::ZERO
        };

        print_pretty_dashboard(
            "Token State",
            vec![
                format!("➤ Pair Address: {}", calculated_pair_address),
                format!("➤ ETH LP Amount: {}", format_ether(eth_lp_amount)),
                format!("➤ Tokens LP Amount: {}", tokens_lp_amount),
                format!(
                    "➤ Clogged Tokens: {} ({}%)",
                    clogged_tokens_amount, clogged_percentage
                ),
                format!("➤ Owner Tokens Balance: {}", owner_token_balance),
            ],
        );

        let menu_option = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an option")
            .items(&vec![
                "[ 0 ] Dipper",
                "[ 1 ] Approve Router + Add Liquidity ETH",
                "[ 2 ] Enable Trading",
                "[ 3 ] Feed the Target Contract with Tokens",
                "[ 4 ] Override Target Contract ETH Balance",
            ])
            .default(0)
            .interact()
            .unwrap();

        match menu_option {
            0 => {}
            1 => {
                approve_and_add_liquidity_eth(&mut evm, owner_address, target_token);
            }
            2 => {
                enable_trading_menu(&mut evm, owner_address, target_token);
            }
            3 => {
                let percentage_input: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the percentage of tokens to transfer (e.g., 80)")
                    .interact_text()
                    .unwrap();

                let mut percentage: u64 = percentage_input.parse().unwrap_or(100);

                if percentage > 100 {
                    println!("Percentage cannot be greater than 100. Using 100%.");
                    percentage = 100;
                }

                let parsed_amount =
                    owner_token_balance * U256::from(percentage) / U256::from(100u64);

                match transfer_erc20_tokens(
                    &mut evm,
                    owner_address,
                    target_token,
                    parsed_amount,
                    target_token,
                ) {
                    Ok(result) => {
                        printlnt!(
                            "{}",
                            format!("Token transfer success | {}", result).bright_green()
                        )
                    }
                    Err(err) => {
                        printlnt!("{}", format!("Token transfer failure | {}", err).red());
                        return;
                    }
                };
            }
            4 => {}
            _ => unreachable!(),
        }
    }
}

fn approve_and_add_liquidity_eth<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    owner_address: Address,
    target_token: Address,
) where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let percentage_input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter the percentage of tokens to add to LP (e.g., 80)")
        .interact_text()
        .unwrap();

    let mut percentage: u64 = percentage_input.parse().unwrap_or(100);

    if percentage > 100 {
        println!("Percentage cannot be greater than 100. Using 100%.");
        percentage = 100;
    }

    let owner_token_balance = match get_token_balance(evm, owner_address, target_token) {
        Ok(balance) => balance,
        Err(err) => {
            printlnt!(
                "{}",
                format!("Error while retrieving owner's token balance: {}", err).red()
            );
            return;
        }
    };

    let add_lp_tokens = owner_token_balance * U256::from(percentage) / U256::from(100u64);

    let eth_amount_input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter the amount of ETH to add to LP (e.g., 0.1)")
        .default("1".to_string())
        .interact_text()
        .unwrap();

    let add_lp_eth = parse_ether(&eth_amount_input).unwrap_or_else(|err| {
        printlnt!("{}", format!("{}: Invalid ETH amount, using 1 ETH.", err));
        U256::from(10 ^ 18)
    });

    match approve(evm, owner_address, *V2_ROUTER_ADDRESS, target_token) {
        Ok(_) => {
            printlnt!("{}", format!("V2 Router approval success").bright_green())
        }
        Err(err) => {
            printlnt!("{}", format!("V2 Router approval failure | {}", err).red());
            return;
        }
    };

    printlnt!(
        "{}",
        format!(
            "Adding {} ETH x {} Tokens ({}%) to the LP",
            eth_amount_input, add_lp_tokens, percentage
        )
    );

    match add_liquidity(evm, owner_address, target_token, add_lp_tokens, add_lp_eth) {
        Ok(result) => {
            printlnt!(
                "{}",
                format!("Add Liquidity success | {}", result).bright_green()
            );
        }
        Err(err) => {
            printlnt!("{}", format!("Add Liquidity failure | {}", err).red());
        }
    };
}

fn enable_trading_menu<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    owner_address: Address,
    target_token: Address,
) where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let menu_option = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an option")
        .items(&vec![
            "[ 0 ] Normal Enable Trading - enableTrading() - 0x8a8c523c",
            "[ 1 ] Normal Enable Trading - custom method",
            "[ 2 ] LP + Enable Trading Combo | enableTrading() | 0x8a8c523c ",
            "[ 3 ] LP + Enable Trading Combo | custom method ",
        ])
        .default(0)
        .interact()
        .unwrap();

    let (maybe_method_id, maybe_value) = match menu_option {
        0 => (Some("0x8a8c523c".to_string()), Some(U256::ZERO)),
        1 => (None, Some(U256::ZERO)),
        2 => (Some("0x8a8c523c".to_string()), None),
        3 => (None, None),
        _ => unreachable!(),
    };

    match enable_trading(
        evm,
        owner_address,
        target_token,
        maybe_method_id,
        maybe_value,
    ) {
        Ok(response) => {
            printlnt!(
                "{}",
                format!("Enable Trading successfully called | {}", response).bright_green()
            )
        }
        Err(err) => {
            printlnt!("{}", format!("Enable Trading failure | {}", err).red())
        }
    }
}
