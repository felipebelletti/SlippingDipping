use std::fmt::Debug;

use alloy::primitives::utils::parse_ether;
use alloy::sol_types::SolCall;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use hex::FromHex;
// Import necessary crates and modules at the top
use revm::primitives::{Address, Bytes, U256};
use revm::{Database, DatabaseCommit, DatabaseRef, Evm};

use crate::config::general::GLOBAL_CONFIG;
use crate::globals::{V2_FACTORY_ADDRESS, V2_ROUTER_ADDRESS, WETH_ADDRESS};
use crate::Dipper::{calculatePairCall, exploitCall};
use crate::UniswapV2Pair::{transferCall, transferFromCall};
use crate::UniswapV2Router01::{addLiquidityETHCall, swapETHForExactTokensCall};
use crate::ERC20::{approveCall, balanceOfCall, decimalsCall, totalSupplyCall};

use super::actors::me;
use super::helpers::{bytes_to_address, bytes_to_u256, bytes_to_u8, revm_call};

pub fn dipper_exploit<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    dipper_address: Address,
    caller: Address,
    pair: Address,
    path: Vec<Address>,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let call = exploitCall {
        pair,
        path: path,
        maxEthSpentOnExploit: parse_ether("1000").unwrap(),
        maxRounds: u8::MAX,
        minEthLiquidity: U256::ZERO,
        expectedLpVariationAfterDip: U256::ZERO,
        swapThresholdTokens: GLOBAL_CONFIG.sniping.swap_threshold_tokens_amount,
        sniperWallets: vec![],
        sniper_max_failed_swaps: 0,
    };

    let result = revm_call(
        evm,
        caller,
        dipper_address,
        Bytes::from(call.abi_encode()),
        parse_ether("1000").unwrap(),
    )?;

    Ok(result)
}

pub fn add_liquidity<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    owner_address: Address,
    target_token: Address,
    add_lp_tokens: U256,
    add_lp_eth: U256,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let result = revm_call(
        evm,
        owner_address,
        *V2_ROUTER_ADDRESS,
        Bytes::from(
            addLiquidityETHCall {
                amountTokenDesired: add_lp_tokens,
                amountTokenMin: add_lp_tokens,
                to: owner_address,
                token: target_token,
                amountETHMin: U256::ZERO,
                deadline: U256::MAX,
            }
            .abi_encode(),
        ),
        U256::from(add_lp_eth),
    )?;

    Ok(result)
}

pub fn enable_trading<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    owner_address: Address,
    target_token: Address,
    maybe_method_id: Option<String>,
    maybe_value: Option<U256>,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let enable_trading_method_id = maybe_method_id.unwrap_or_else(|| {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enable Trading Method Id (e.g: 0x8a8c523c)")
            .interact_text()
            .unwrap()
    });

    let enable_trading_calldata = enable_trading_method_id
        .starts_with("0x")
        .then(|| Vec::from_hex(&enable_trading_method_id[2..]).unwrap())
        .unwrap_or_else(|| Vec::from_hex(&enable_trading_method_id).unwrap());

    let result = revm_call(
        evm,
        owner_address,
        target_token,
        Bytes::from(enable_trading_calldata),
        maybe_value.unwrap_or_else(|| {
            let eth_lp_f64: f64 = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("ETH Liquidity Amount to be added (e.g: 0.1)")
                .interact_text()
                .unwrap();
            parse_ether(eth_lp_f64.to_string().as_str()).unwrap()
        }),
    )?;

    Ok(result)
}

pub fn transfer_erc20_tokens<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    from: Address,
    to: Address,
    amount: U256,
    target_token: Address,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let result = revm_call(
        evm,
        from,
        target_token,
        Bytes::from(transferCall { to, value: amount }.abi_encode()),
        U256::ZERO,
    )?;

    Ok(result)
}

pub fn approve<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    caller: Address,
    spender: Address,
    target_token: Address,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let result = revm_call(
        evm,
        caller,
        target_token,
        Bytes::from(
            approveCall {
                amount: U256::MAX,
                spender,
            }
            .abi_encode(),
        ),
        U256::ZERO,
    )?;

    Ok(result)
}

pub fn get_token_balance<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    account_address: Address,
    target_token: Address,
) -> Result<U256, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let balance_bytes = revm_call(
        evm,
        me(),
        target_token,
        Bytes::from(
            balanceOfCall {
                owner: account_address,
            }
            .abi_encode(),
        ),
        U256::ZERO,
    )?;
    let balance = bytes_to_u256(balance_bytes)?;
    Ok(balance)
}

pub fn calculate_pair_address<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    token_a: Address,
    token_b: Address,
    dipper_caller: Address,
    dipper_contract_address: Address,
) -> Result<Address, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let address_bytes = revm_call(
        evm,
        dipper_caller,
        dipper_contract_address,
        Bytes::from(
            calculatePairCall {
                factory: *V2_FACTORY_ADDRESS,
                tokenA: token_a,
                tokenB: token_b,
            }
            .abi_encode(),
        ),
        U256::ZERO,
    )?;
    let address = bytes_to_address(address_bytes)?;
    Ok(address)
}

pub fn get_decimals<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    target_token: Address,
) -> Result<u8, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let decimals_bytes = revm_call(
        evm,
        me(),
        target_token,
        Bytes::from(decimalsCall {}.abi_encode()),
        U256::ZERO,
    )?;
    let decimals = bytes_to_u8(decimals_bytes)?;
    Ok(decimals)
}

pub fn get_total_supply<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    target_token: Address,
) -> Result<U256, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    let total_supply_bytes = revm_call(
        evm,
        me(),
        target_token,
        Bytes::from(totalSupplyCall {}.abi_encode()),
        U256::ZERO,
    )?;
    let total_supply = bytes_to_u256(total_supply_bytes)?;
    Ok(total_supply)
}

pub fn swap_eth_for_exact_tokens<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    swapper: Address,
    target_token: Address,
    tokens: U256,
    max_spend_eth: U256,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    Ok(revm_call(
        evm,
        swapper,
        *V2_ROUTER_ADDRESS,
        Bytes::from(
            swapETHForExactTokensCall {
                amountOut: tokens,
                deadline: U256::MAX,
                path: vec![*WETH_ADDRESS, target_token],
                to: swapper,
            }
            .abi_encode(),
        ),
        max_spend_eth,
    )?)
}
