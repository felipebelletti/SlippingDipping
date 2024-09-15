use alloy::{
    dyn_abi::abi,
    eips::BlockId,
    primitives::utils::parse_ether,
    providers::Provider,
    rpc::types::state::{AccountOverride, StateOverride},
    signers::k256::U256,
};
use colored::Colorize;
use revm::{
    db::{AlloyDB, EmptyDB},
    primitives::{keccak256, Address},
    Database, DatabaseRef, Evm, InMemoryDB,
};

use crate::globals;

pub async fn simulate<M: Provider + Clone>(client: &M) {
    let latest_block = match client
        .get_block(
            BlockId::latest(),
            alloy::rpc::types::BlockTransactionsKind::Hashes,
        )
        .await
        .unwrap()
    {
        Some(block) => block,
        None => {
            println!(
                "{}",
                format!("Latest block value is None. We cannot proceed.").red()
            );
            return;
        }
    };

    let mut ethersdb = AlloyDB::new(client.clone(), latest_block.header.number.into()).unwrap();
    let mut evm = Evm::builder().with_ref_db(&mut ethersdb).build();

    evm.cfg_mut().limit_contract_code_size = Some(0x100000);
    evm.cfg_mut().disable_block_gas_limit = true;
    evm.cfg_mut().disable_base_fee = true;

    let db = evm.db_mut();
    let b = "0xC9B29bE488b07fFe7f44bB52D0d7234baF6AC8C3"
            .parse::<Address>()
            .unwrap();

    db.basic(
        b,
    )
    .unwrap()
    .unwrap()
    .balance = parse_ether("1000").unwrap();

    db.storage(address, index)

    let c = evm.context.evm.balance(b).unwrap();
    print!("{:?}", c);
}

async fn sim_call_with_unlimited_eth_balance(
    evm: &mut Evm<'_, (), revm::db::InMemoryDB>,
    address: Address,
    amount: f64,
) {
    let mut state = StateOverride::default();
    state.insert(
        *globals::WETH_ADDRESS,
        AccountOverride {
            balance: Some(parse_ether("1000").unwrap()),
            ..Default::default()
        },
    );
}
