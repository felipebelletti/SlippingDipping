use alloy::primitives::Address;
use serde::Deserialize;
use std::fs;
use lazy_static::lazy_static;
use anyhow::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub sniping: SnipingConfig,
    pub tx_builder: TransactionBuilder,
    pub provider: ProviderConfig,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub dipper_contract: Address
}

#[derive(Debug, Deserialize)]
pub struct SnipingConfig {
    pub tokens_amount: String,
    pub eth_amount_for_unclogging: f64,
    pub unclog_nloops: u8,
    pub min_eth_liquidity: f64,
    pub bribe_eth_good_validators: f64,
    pub bribe_eth_bad_validators: f64,
    pub good_validators: Vec<Address>,
    pub spammer_secs_delay: f64,
    pub min_successfull_swaps: u8,
}

#[derive(Debug, Deserialize)]
pub struct TransactionBuilder {
    pub snipe_gas_limit: String,
    pub max_fee_per_gas: f64,
    pub max_priority_fee_per_gas: f64,
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub rpc_url: String,
}

lazy_static! {
    pub static ref GLOBAL_CONFIG: Config = Config::from_file("config.toml")
        .expect("Failed to load config.toml");
}

impl Config {
    fn from_file(path: &str) -> Result<Config, Error> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn get_greatest_bribe_eth(&self) -> f64 {
        self.sniping.bribe_eth_good_validators
            .max(self.sniping.bribe_eth_bad_validators)
    }
}
