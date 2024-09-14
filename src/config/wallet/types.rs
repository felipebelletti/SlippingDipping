use alloy::{network::EthereumWallet, primitives::{Address, U256}};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Wallet {
    pub private_key: String,
    pub eth_amount: f64,
    #[serde(skip_deserializing)]
    pub signer: EthereumWallet,
    #[serde(skip_deserializing)]
    pub address: Address,
    #[serde(skip_deserializing)]
    pub eth_amount_in_wei: U256,
    #[serde(skip_deserializing)]
    pub nonce: U256,
}

pub struct WalletCollection {
    pub wallets: Vec<Wallet>,
}
