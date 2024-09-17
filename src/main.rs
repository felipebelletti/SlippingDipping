use alloy::primitives::utils::format_ether;
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::sol;
use alloy::{
    eips::BlockId,
    providers::{ProviderBuilder, WalletProvider},
};
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use api::utils::print_pretty_dashboard;
use api::{simulate, strategies};
use colored::Colorize;
use config::wallet::types::Wallet;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Select};
use revm::{db::AlloyDB, Database, DatabaseRef, Evm};
use std::sync::Arc;
#[allow(unused_imports)]
use ERC20::ERC20Instance;

use config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS};

mod api;
mod common;
mod config;
mod globals;

#[macro_export]
macro_rules! printlnt {
    ($($arg:tt)*) => {{
        use chrono::Local;
        let current_time = Local::now().format("%H:%M:%S");
        print!("[ {} ] ", current_time);
        std::println!($($arg)*);
    }};
}

#[tokio::main]
async fn main() {
    let client = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(&GLOBAL_WALLETS.wallets[0].signer)
            .on_builtin(&GLOBAL_CONFIG.provider.rpc_url)
            .await
            .unwrap(),
    );

    show_pretty_wallet_dashboard(client.clone(), GLOBAL_WALLETS.get_wallets()).await;

    let menu_option = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an option")
        .items(&vec!["[ 0 ] M1/M2/M3/M4/M5/M6 Block-Zero Dipper"])
        .default(0)
        .interact()
        .unwrap();

    match menu_option {
        0 => {
            strategies::blockzero_dipper::run(client).await;
        }
        _ => unreachable!(),
    }
}

async fn show_pretty_wallet_dashboard<M: Provider>(client: Arc<M>, wallets: &Vec<Wallet>) {
    let header = "╭──────────────────────── Wallets ────────────────────────╮";
    let footer = "╰─────────────────────────────────────────────────────────╯";

    println!("{}", header.bold().green());

    for wallet in wallets {
        let balance: String = format_ether(client.get_balance(wallet.address).await.unwrap());
        let balance_rounded = format!("{:.4}", balance); // Limita a 4 casas decimais
        println!(
            "{}",
            format!(
                "{} {} {} {} {} {}",
                "│".green(),
                "➤".bright_blue(),
                wallet.address.to_string().yellow(),
                "│".white(),
                format!("{} ETH", balance_rounded.purple()),
                "│".green(),
            )
        );
    }

    println!("{}", footer.bold().green());
}

sol! {
    #[allow(missing_docs)]
    // solc v0.8.26; solc contract.sol --via-ir --optimize --abi -o artifacts
    #[sol(rpc)]
    contract Dipper {
        mapping(address => bool) public locks;
        struct SniperWallet {
            address addr;
            uint256 ethAmount;
            uint256 tokensAmount;
        }
        function exploit(
            uint8 maxRounds,
            uint256 maxEthSpentOnExploit,
            uint256 minEthLiquidity,
            uint256 swapThresholdTokens,
            uint8 sniper_max_failed_swaps,
            address pair,
            address[] calldata path,
            SniperWallet[] calldata sniperWallets
        ) external payable onlyOwner {}
        function removeLock(address tokenAddress) external {}
        function calculatePair(
            address tokenA,
            address tokenB,
            address factory
        ) external pure returns (address pair) {}
    }

    #[allow(missing_docs)]
    #[sol(rpc)]
    contract ERC20 {
        function name() external pure returns (string memory);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
        function totalSupply() external view returns (uint);
        function balanceOf(address owner) external view returns (uint);
        function allowance(address owner, address spender) external view returns (uint);
        function approve(address spender, uint256 amount) external returns (bool);
    }
}
