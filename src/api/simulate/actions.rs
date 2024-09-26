use std::fmt::Debug;

use alloy::primitives::utils::parse_ether;
use alloy::sol_types::SolCall;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use hex::FromHex;
// Import necessary crates and modules at the top
use revm::primitives::{Address, Bytes, U256};
use revm::{Database, Evm};

use crate::globals::{V2_FACTORY_ADDRESS, V2_ROUTER_ADDRESS};
use crate::Dipper::calculatePairCall;
use crate::UniswapV2Pair::{transferCall, transferFromCall};
use crate::UniswapV2Router01::addLiquidityETHCall;
use crate::ERC20::{approveCall, balanceOfCall, decimalsCall};

use super::actors::me;
use super::helpers::{bytes_to_address, bytes_to_u256, revm_call};

pub fn add_liquidity<DB: Database>(
    owner_address: Address,
    target_token: Address,
    add_lp_tokens: U256,
    add_lp_eth: U256,
    cache_db: &mut DB,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    <DB as revm::Database>::Error: Debug,
{
    let result = revm_call(
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
        cache_db,
    )?;

    Ok(result)
}

pub fn enable_trading<DB: Database>(
    owner_address: Address,
    target_token: Address,
    maybe_method_id: Option<String>,
    maybe_value: Option<U256>,
    cache_db: &mut DB,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    <DB as revm::Database>::Error: Debug,
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
        cache_db,
    )?;

    Ok(result)
}

pub fn transfer_erc20_tokens<DB: Database>(
    from: Address,
    to: Address,
    amount: U256,
    target_token: Address,
    cache_db: &mut DB,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    <DB as revm::Database>::Error: Debug,
{
    let result = revm_call(
        from,
        target_token,
        Bytes::from(
            transferCall {
                to,
                value: amount,
            }
            .abi_encode(),
        ),
        U256::ZERO,
        cache_db,
    )?;

    Ok(result)
}

pub fn approve<DB: Database>(
    caller: Address,
    spender: Address,
    target_token: Address,
    cache_db: &mut DB,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    <DB as revm::Database>::Error: Debug,
{
    let result = revm_call(
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
        cache_db,
    )?;

    Ok(result)
}

pub fn get_token_balance<DB: Database>(
    account_address: Address,
    target_token: Address,
    cache_db: &mut DB,
) -> Result<U256, Box<dyn std::error::Error>>
where
    <DB as revm::Database>::Error: Debug,
{
    let balance_bytes = revm_call(
        me(),
        target_token,
        Bytes::from(
            balanceOfCall {
                owner: account_address,
            }
            .abi_encode(),
        ),
        U256::ZERO,
        cache_db,
    )?;
    let balance = bytes_to_u256(balance_bytes)?;
    Ok(balance)
}

pub fn calculate_pair_address<DB: Database>(
    token_a: Address,
    token_b: Address,
    dipper_caller: Address,
    dipper_contract_address: Address,
    cache_db: &mut DB,
) -> Result<Address, Box<dyn std::error::Error>>
where
    <DB as revm::Database>::Error: Debug,
{
    let address_bytes = revm_call(
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
        cache_db,
    )?;
    let address = bytes_to_address(address_bytes)?;
    Ok(address)
}
