mod types;

use self::types::{Wallet, WalletCollection};
use alloy::{primitives::U256, signers::local::PrivateKeySigner};
use anyhow::{anyhow, Error};
use lazy_static::lazy_static;
use std::{fs, str::FromStr};

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
            wallet.signer = ethers_wallet.clone().into();
            wallet.address = ethers_wallet.address();
            wallet.eth_amount_in_wei = U256::from_str(&format!("{:.0}", wallet.eth_amount * 1e18))
                .map_err(|err| anyhow!("Invalid eth_amount in wallets.json: {err}"))?;
        }

        Ok(WalletCollection { wallets })
    }
}
