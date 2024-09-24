use alloy::primitives::Address;
use anyhow::Error;
use lazy_static::lazy_static;
use revm::primitives::U256;
use serde::Deserialize;
use std::{fs, str::FromStr};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub sniping: SnipingConfig,
    pub tx_builder: TransactionBuilder,
    pub provider: ProviderConfig,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub dipper_contract: Address,
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
    pub swap_threshold_tokens_amount: U256,
    pub bribe_amount: f64,
    pub dipper_using_eob: bool,
    pub multi_wallet_mode: MultiWalletMode,
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
    pub static ref GLOBAL_CONFIG: Config =
        Config::from_file("config.toml").expect("Failed to load config.toml");
}

impl Config {
    fn from_file(path: &str) -> Result<Config, Error> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MultiWalletMode {
    MultiTx,
    SingleTx,
}

impl FromStr for MultiWalletMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "multi_tx" => Ok(MultiWalletMode::MultiTx),
            "single_tx" => Ok(MultiWalletMode::SingleTx),
            _ => Err(anyhow::anyhow!(
                "Invalid wallet mode. Choose between \"single_tx\" or \"multi_tx\"."
            )),
        }
    }
}
