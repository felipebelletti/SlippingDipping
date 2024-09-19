use alloy::primitives::Address;
use revm::primitives::U256;
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
    pub expected_lp_variation_after_dip: f64,
    pub max_eth_spent_on_dipping: f64,
    pub max_failed_user_swaps: u8,
    pub max_dipper_rounds: u8,
    pub min_eth_liquidity: f64,
    pub spammer_secs_delay: f64,
    pub swap_threshold_tokens_amount: U256
}

#[derive(Debug, Deserialize)]
pub struct TransactionBuilder {
    pub dipper_gas_limit: String,
    pub snipe_gas_limit: String,
    pub sell_gas_limit: String,
    pub approve_gas_limit: String,
    pub max_fee_per_gas: f64,
    pub max_priority_fee_per_gas: f64,
    pub sell_slippage_percent: f64,
    pub gas_oracle: bool,
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
}
