pub mod types;

use crate::{api::utils::erc20::get_percentage_token_supply, printlnt};

use self::types::{Wallet, WalletCollection};
use alloy::{
    primitives::{utils::parse_units, U256},
    providers::Provider,
    signers::local::PrivateKeySigner,
};
use anyhow::{anyhow, Error};
use colored::Colorize;
use lazy_static::lazy_static;
use revm::primitives::Address;
use std::{fs, str::FromStr, sync::Arc};

use super::general::GLOBAL_CONFIG;

lazy_static! {
    pub static ref GLOBAL_WALLETS: WalletCollection =
        WalletCollection::wallets_from_file("wallets.json").expect("Failed to load wallets.json");
}

impl WalletCollection {
    pub fn new(wallets: Vec<Wallet>) -> WalletCollection {
        WalletCollection { wallets }
    }

    pub fn count(&self) -> usize {
        self.wallets.len()
    }

    pub fn get_wallets(&self) -> &Vec<Wallet> {
        &self.wallets
    }

    pub fn get_total_eth_amount(&self) -> f64 {
        self.wallets.iter().map(|wallet| wallet.eth_amount).sum()
    }

    pub fn filter_by_amount(&self, threshold: f64) -> Vec<&Wallet> {
        self.wallets
            .iter()
            .filter(|w| w.eth_amount > threshold)
            .collect()
    }

    pub fn wallets_from_file(path: &str) -> Result<WalletCollection, Error> {
        let contents = fs::read_to_string(path)?;
        let mut wallets: Vec<Wallet> = serde_json::from_str(&contents)?;

        for wallet in wallets.iter_mut() {
            let ethers_wallet: PrivateKeySigner = wallet
                .private_key
                .parse()
                .map_err(|err| anyhow!("Invalid private key in wallets.json: {err}"))?;
            wallet.signer = Arc::new(ethers_wallet.clone().into());
            wallet.address = ethers_wallet.address();
            wallet.eth_amount_in_wei = U256::from_str(&format!("{:.0}", wallet.eth_amount * 1e18))
                .map_err(|err| anyhow!("Invalid eth_amount in wallets.json: {err}"))?;
        }

        Ok(WalletCollection { wallets })
    }

    pub async fn resolve_tokens_amount<M: Provider>(
        self,
        client: Arc<M>,
        token_address: Address,
        decimals: &u8,
    ) -> WalletCollection {
        let mut wallets = self.wallets.clone();

        for wallet in wallets.iter_mut() {
            if wallet.tokens_amount == "" {
                wallet.tokens_amount_in_wei = U256::from(0);
                continue;
            }

            if wallet.tokens_amount == "<config>" {
                wallet.tokens_amount = GLOBAL_CONFIG.sniping.tokens_amount.clone();
                printlnt!(
                    "{}",
                    format!(
                        "Wallet tokens amount ovewritten with config->tokens_amount = {}",
                        wallet.tokens_amount
                    )
                    .yellow()
                );
            }

            let tokens_amount = if wallet.tokens_amount.contains("%") {
                let percentage = wallet
                    .tokens_amount
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap();
                let tokens = get_percentage_token_supply(&client, token_address, percentage).await;
                printlnt!(
                    "{}",
                    format!(
                        "Wallet {}% tokens resolved to exact {} token on {}",
                        percentage, tokens, token_address
                    )
                    .yellow()
                );
                tokens
            } else {
                let tokens = parse_units(&wallet.tokens_amount.to_string(), *decimals)
                    .expect(&format!(
                        "parse_units({}, {})",
                        &wallet.tokens_amount.to_string(),
                        decimals
                    ))
                    .into();
                printlnt!(
                    "{}",
                    format!(
                        "Tokens amount resolved from {} to {} ({} decimals) on {}",
                        wallet.tokens_amount, tokens, decimals, token_address
                    )
                    .yellow()
                );
                tokens
            };

            wallet.tokens_amount_in_wei = tokens_amount;
        }

        return WalletCollection { wallets };
    }
}
