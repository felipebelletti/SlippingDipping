use std::{fmt::Debug, sync::Arc};

use actions::{
    add_liquidity, approve, calculate_pair_address, dipper_exploit, enable_trading, get_decimals,
    get_token_balance, get_total_supply, swap_eth_for_exact_tokens, transfer_erc20_tokens,
};
use actors::{me, weth_addr};
use alloy::{
    dyn_abi::abi,
    eips::{BlockId, BlockNumberOrTag},
    node_bindings::Anvil,
    primitives::{
        utils::{format_ether, format_units, parse_ether, parse_units},
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
    bytes_to_address, bytes_to_u256, generate_random_buyer_address, init_account_with_bytecode,
    insert_mapping_storage_slot, one_ether, revm_call,
};
use hex::FromHex;
use revm::{
    db::{AlloyDB, CacheDB, EmptyDB},
    primitives::{keccak256, Address, Bytecode, U256},
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

    let decimals = get_decimals(&mut evm, target_token).unwrap();

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

    let mut simulated_swappers_in: Vec<Address> = vec![];

    'simulator_loop: loop {
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

        let token_total_supply = get_total_supply(&mut evm, target_token).unwrap();

        let clogged_tokens_amount =
            get_token_balance(&mut evm, target_token, target_token).unwrap_or(U256::ZERO);

        let clogged_percentage = if tokens_lp_amount != U256::ZERO {
            clogged_tokens_amount * U256::from(100u64) / token_total_supply
        } else {
            U256::ZERO
        };

        let mut dashboard = vec![
            format!("➤ Token Address: {}", target_token),
            format!("➤ Pair Address: {}", calculated_pair_address),
            format!("➤ ETH LP Amount: {}", format_ether(eth_lp_amount)),
            format!(
                "➤ Tokens LP Amount: {}",
                format_units(tokens_lp_amount, decimals).unwrap()
            ),
            format!(
                "➤ Clogged Tokens: {} ({}%)",
                format_units(clogged_tokens_amount, decimals).unwrap(),
                clogged_percentage
            ),
            format!("➤ Owner Tokens Balance: {}", owner_token_balance),
        ];
        dashboard.extend(
            simulated_swappers_in
                .iter()
                .enumerate()
                .map(
                    |(i, &swapper)| match get_token_balance(&mut evm, swapper, target_token) {
                        Ok(balance) => format!(
                            "💳 Swapper #{}: {}, Balance: {}",
                            i,
                            swapper,
                            format_units(balance, decimals).unwrap()
                        ),
                        Err(err) => format!(
                            "💳 Swapper #{}: {}, Error fetching balance: {}",
                            i, swapper, err
                        ),
                    },
                ),
        );

        print_pretty_dashboard("Token State", dashboard);

        let menu_option = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an option")
            .items(&vec![
                "[ 0 ] Dipper",
                "[ 1 ] Approve Router + Add Liquidity ETH",
                "[ 2 ] Enable Trading",
                "[ 3 ] Feed the Target Contract with Tokens",
                "[ 4 ] Override Target Contract ETH Balance",
                "[ 5 ] Swaps in",
            ])
            .default(0)
            .interact()
            .unwrap();

        match menu_option {
            0 => {
                let dipper_caller_balance_before = evm
                    .db_mut()
                    .0
                    .load_account(dipper_caller_address)
                    .unwrap()
                    .info
                    .balance;

                match dipper_exploit(
                    &mut evm,
                    dipper_contract_address,
                    dipper_caller_address,
                    calculated_pair_address,
                    vec![*WETH_ADDRESS, target_token],
                ) {
                    Ok(result) => {
                        printlnt!(
                            "{}",
                            format!("Dipper Exploit Success | {}", result).bright_green()
                        );
                    }
                    Err(err) => {
                        printlnt!("{}", format!("Dipper Exploit Failure | {}", err).red());
                        continue;
                    }
                }

                let dipper_caller_balance_after = evm
                    .db_mut()
                    .0
                    .load_account(dipper_caller_address)
                    .unwrap()
                    .info
                    .balance;

                let dipping_operation_cost =
                    dipper_caller_balance_before - dipper_caller_balance_after;

                let eth_lp_amount_after =
                    get_token_balance(&mut evm, calculated_pair_address, *WETH_ADDRESS)
                        .unwrap_or(U256::ZERO);

                let eth_lp_absolute_difference = if eth_lp_amount > eth_lp_amount_after {
                    eth_lp_amount - eth_lp_amount_after
                } else {
                    eth_lp_amount_after - eth_lp_amount
                };

                let eth_percentage_difference = (format_ether(eth_lp_absolute_difference)
                    .parse::<f64>()
                    .unwrap()
                    / format_ether(eth_lp_amount).parse::<f64>().unwrap())
                    * 100.0;

                let clogged_tokens_amount_after =
                    get_token_balance(&mut evm, target_token, target_token).unwrap_or(U256::ZERO);

                let clogged_absolute_difference =
                    if clogged_tokens_amount > clogged_tokens_amount_after {
                        clogged_tokens_amount - clogged_tokens_amount_after
                    } else {
                        clogged_tokens_amount_after - clogged_tokens_amount
                    };

                let clogged_percentage_difference =
                    (format_units(clogged_absolute_difference, decimals)
                        .unwrap()
                        .parse::<f64>()
                        .unwrap()
                        / format_units(clogged_tokens_amount, decimals)
                            .unwrap()
                            .parse::<f64>()
                            .unwrap())
                        * 100.0;

                print_pretty_dashboard(
                    &"💥 State After Dipping 💥".bold().underline().bright_cyan(),
                    vec![
                        format!(
                            "💧 ETH LP   : {:.4} ⟶ {:.4} [{:.4}%]",
                            format_ether(eth_lp_amount)
                                .bright_white()
                                .on_bright_blue()
                                .bold(),
                            format_ether(eth_lp_amount_after)
                                .bright_white()
                                .on_bright_blue()
                                .bold(),
                            format!("{:.4}%", eth_percentage_difference).yellow().bold()
                        ),
                        format!(
                            "🤡 CLOGGED  : {} ⟶ {} [{:.4}%]",
                            format_units(clogged_tokens_amount, decimals)
                                .unwrap()
                                .to_string()
                                .bright_red()
                                .bold(),
                            format_units(clogged_tokens_amount_after, decimals)
                                .unwrap()
                                .to_string()
                                .bright_red()
                                .bold(),
                            format!("{:.4}%", clogged_percentage_difference)
                                .green()
                                .bold()
                        ),
                        format!(
                            "💰 Operational Cost: {} ETH",
                            format_ether(dipping_operation_cost)
                                .bright_magenta()
                                .bold()
                                .underline()
                        ),
                    ],
                );
            }
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
            4 => {
                let new_balance_str: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the new Account ETH balance")
                    .interact_text()
                    .unwrap();

                printlnt!(
                    "{}",
                    format!(
                        "Loading {} as the new ETH Balance of Account {}",
                        new_balance_str, target_token
                    )
                    .yellow()
                );

                evm.db_mut()
                    .0
                    .load_account(target_token)
                    .unwrap()
                    .info
                    .balance = parse_ether(&new_balance_str).unwrap();
            }
            5 => {
                let swaps_count: usize = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("How many swaps")
                    .interact_text()
                    .unwrap();
                let tokens: U256 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Amount of tokens for each swap (no decimal wrapped)")
                    .interact_text()
                    .unwrap();

                for idx in 0..swaps_count {
                    let swapper = generate_random_buyer_address();
                    init_account_with_bytecode(
                        evm.db_mut().0,
                        swapper,
                        U256::MAX,
                        Bytecode::default(),
                    );

                    printlnt!(
                        "{}",
                        format!("Account Initialized #{idx}: {swapper}").yellow()
                    );

                    match swap_eth_for_exact_tokens(
                        &mut evm,
                        swapper,
                        target_token,
                        tokens,
                        U256::MAX.wrapping_div(U256::from(4)),
                    ) {
                        Ok(_) => {
                            printlnt!(
                                "{}",
                                format!("Account: {swapper} #{idx} | Status: Swap Successfull")
                                    .bright_green()
                            );
                            simulated_swappers_in.push(swapper);
                        }
                        Err(err) => {
                            printlnt!(
                                "{}",
                                format!(
                                    "Account: {swapper} #{idx} | Status: Swap Failed | Reason: {}",
                                    err
                                )
                                .red()
                            )
                        }
                    }
                }
            }
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
