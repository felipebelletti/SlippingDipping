use crate::config::wallet::types::Wallet;

#[derive(Debug, Clone)]
pub struct ExtraCosts {
    pub aped_wallets: Option<Vec<ApedWallet>>,
    pub dipper_cost_eth: Option<f64>,
    pub gas_cost_eth: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ApedWallet {
    pub wallet: Wallet,
    pub aped_weth: f64,
}
