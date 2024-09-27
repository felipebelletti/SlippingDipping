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
use core::str;
use rand::{thread_rng, Rng};
use revm::{
    db::{AlloyDB, CacheDB},
    primitives::{ExecutionResult, Output, TransactTo},
    DatabaseCommit, DatabaseRef, Evm,
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

pub fn revm_call<'a, DB>(
    evm: &mut Evm<'a, (), revm::db::WrapDatabaseRef<&'a mut DB>>,
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
) -> Result<Bytes, Box<dyn std::error::Error>>
where
    DB: Database + revm::DatabaseRef,
    <DB as Database>::Error: std::fmt::Debug,
    <DB as DatabaseRef>::Error: Debug,
    DB: DatabaseCommit,
{
    evm.tx_mut().clear();
    evm.tx_mut().caller = from;
    evm.tx_mut().transact_to = TransactTo::Call(to);
    evm.tx_mut().data = calldata;
    evm.tx_mut().value = value;

    // let ref_tx = evm.transact().unwrap();
    // let result = ref_tx.result;
    let result = evm.transact_commit().unwrap();

    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            return Err(anyhow!("execution failed: {result:?}").into());
        }
    };

    Ok(value)
}

pub fn init_account_with_bytecode<ExtDB>(
    cache_db: &mut CacheDB<ExtDB>,
    address: Address,
    balance: U256,
    bytecode: Bytecode,
) where
    ExtDB: Database + DatabaseRef,
    <ExtDB as Database>::Error: Debug,
    <ExtDB as DatabaseRef>::Error: Debug,
{
    let code_hash = bytecode.hash_slow();
    let acc_info = AccountInfo {
        balance,
        nonce: 0_u64,
        code: Some(bytecode),
        code_hash,
    };

    cache_db.insert_account_info(address, acc_info);
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

pub fn bytes_to_u8(bytes: Bytes) -> Result<u8, Box<dyn std::error::Error>> {
    if bytes.len() < 1 {
        return Err("Byte array is too short to convert to u8".into());
    }

    Ok(bytes[0])
}

pub fn parse_revert_message(hex_data: &str) -> Result<String, Box<dyn std::error::Error>> {
    let hex_data = if hex_data.starts_with("0x") {
        &hex_data[2..]
    } else {
        hex_data
    };

    let bytes = hex::decode(hex_data)?;

    if bytes.len() < 4 + 32 + 32 {
        return Err("Input is too short to contain a valid revert message.".into());
    }

    let offset = 4 + 32; // 4 bytes for selector, 32 bytes for offset
    let length = usize::from_be_bytes(bytes[offset..offset + 32].try_into()?);

    let start = offset + 32;
    let end = start + length;

    let message_bytes = &bytes[start..end];
    let message = str::from_utf8(message_bytes)?;

    Ok(message.to_string())
}

pub fn generate_random_buyer_address() -> Address {
    let mut rng = thread_rng();
    let mut random_bytes = [0u8; 20];
    rng.fill(&mut random_bytes); // Fill with random bytes
    Address::from_slice(&random_bytes) // Return as Ethereum address
}

fn cache_dir() -> String {
    ".evm_cache".to_string()
}
