use alloy::{
    eips::BlockId,
    network::{Ethereum, TransactionBuilder},
    primitives::{utils::parse_units, Address, Bytes, U256},
    providers::{Provider, RootProvider},
    rpc::types::TransactionRequest,
    sol_types::{SolCall, SolValue},
    transports::http::{Client, Http},
};

use anyhow::{anyhow, Result};
use revm::{
    db::{AlloyDB, CacheDB},
    primitives::{ExecutionResult, Output, TransactTo},
    DatabaseRef, Evm,
};
use revm::{
    primitives::{keccak256, AccountInfo, Bytecode},
    Database,
};
use std::time::Duration;
use std::{fmt::Debug, sync::Arc};
use tokio::time::Instant;

use crate::ERC20::balanceOfCall;

use super::actors::me;

pub fn measure_start(label: &str) -> (String, Instant) {
    (label.to_string(), Instant::now())
}

pub fn measure_end(start: (String, Instant)) -> Duration {
    let elapsed = start.1.elapsed();
    println!("Elapsed: {:.2?} for '{}'", elapsed, start.0);
    elapsed
}

pub fn one_ether() -> U256 {
    parse_units("1.0", "ether").unwrap().into()
}

pub fn volumes(from: U256, to: U256, count: usize) -> Vec<U256> {
    let start = U256::ZERO;
    let mut volumes = Vec::new();
    let distance = to - from;
    let step = distance / U256::from(count);

    for i in 1..(count + 1) {
        let current = start + step * U256::from(i);
        volumes.push(current);
    }

    volumes.reverse();
    volumes
}

pub fn build_tx(to: Address, from: Address, calldata: Bytes, base_fee: u128) -> TransactionRequest {
    TransactionRequest::default()
        .to(to)
        .from(from)
        .with_input(calldata)
        .nonce(0)
        .gas_limit(1000000)
        .max_fee_per_gas(base_fee)
        .max_priority_fee_per_gas(0)
        .build_unsigned()
        .unwrap()
        .into()
}

pub fn revm_call<DB: Database>(
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
    cache_db: &mut DB,
) -> Result<Bytes>
where
    <DB as revm::Database>::Error: Debug,
{
    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.caller = from;
            tx.transact_to = TransactTo::Call(to);
            tx.data = calldata;
            tx.value = value;
        })
        .build();

    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            return Err(anyhow!("execution failed: {result:?}"));
        }
    };

    Ok(value)
}

pub fn revm_revert<DB: Database>(
    from: Address,
    to: Address,
    calldata: Bytes,
    cache_db: &mut DB,
) -> Result<Bytes>
where
    <DB as revm::Database>::Error: Debug,
{
    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.caller = from;
            tx.transact_to = TransactTo::Call(to);
            tx.data = calldata;
            tx.value = U256::ZERO;
        })
        .build();
    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Revert { output: value, .. } => value,
        _ => {
            panic!("It should never happen!");
        }
    };

    Ok(value)
}

pub async fn init_account_with_bytecode(
    address: Address,
    bytecode: Bytecode,
    cache_db: &mut AlloyCacheDB,
) -> Result<()> {
    let code_hash = bytecode.hash_slow();
    let acc_info = AccountInfo {
        balance: U256::ZERO,
        nonce: 0_u64,
        code: Some(bytecode),
        code_hash,
    };

    cache_db.insert_account_info(address, acc_info);
    Ok(())
}

pub async fn insert_mapping_storage_slot<Backend>(
    contract: Address,
    slot: U256,
    slot_address: Address,
    value: U256,
    cache_db: &mut CacheDB<Backend>,
) -> Result<()>
where
    Backend: DatabaseRef,
    Backend::Error: Debug,
    // <Backend as DatabaseRef>::Error: std::error::Error + Send,
{
    let hashed_balance_slot = keccak256((slot_address, slot).abi_encode());

    cache_db
        .insert_account_storage(contract, hashed_balance_slot.into(), value)
        .unwrap();
    Ok(())
}

pub fn bytes_to_address(bytes: Bytes) -> Result<Address, Box<dyn std::error::Error>> {
    if bytes.len() != 32 {
        return Err("Invalid response length".into());
    }

    Ok(Address::from_slice(&bytes[12..]))
}

pub fn bytes_to_bool(bytes: Bytes) -> Result<bool, Box<dyn std::error::Error>> {
    if bytes.len() != 32 {
        return Err("Invalid response length".into());
    }

    match bytes[31] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err("Invalid boolean value".into()),
    }
}

pub fn bytes_to_u256(bytes: Bytes) -> Result<U256, Box<dyn std::error::Error>> {
    if bytes.len() != 32 {
        return Err("Invalid response length".into());
    }

    Ok(U256::from_be_slice(&bytes))
}

fn cache_dir() -> String {
    ".evm_cache".to_string()
}
