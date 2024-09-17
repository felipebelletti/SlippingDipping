use alloy::{network::EthereumWallet, primitives::{Address, U256}};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Wallet {
    pub private_key: String,
    pub eth_amount: f64,
    pub tokens_amount: String,
    #[serde(skip_deserializing)]
    pub signer: EthereumWallet,
    #[serde(skip_deserializing)]
    pub address: Address,
    #[serde(skip_deserializing)]
    pub eth_amount_in_wei: U256,
    #[serde(skip_deserializing)]
    pub tokens_amount_in_wei: U256,
    #[serde(skip_deserializing)]
    pub nonce: U256,
}

#[derive(Clone)]
pub struct WalletCollection {
    pub wallets: Vec<Wallet>,
}
