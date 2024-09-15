use alloy::primitives::Address;
use alloy::sol;
use alloy::{
    eips::BlockId,
    providers::{ProviderBuilder, WalletProvider},
};
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use api::{simulate, strategies};
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

    simulate::simulate(&client).await;
    return;

    let menu_option = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an option")
        .items(&vec!["[ M1 ] Dipper Block Zero"])
        .default(0)
        .interact()
        .unwrap();

    match menu_option {
        0 => {
            strategies::m1::run(client).await;
        }
        _ => unreachable!(),
    }
}

sol! {
    #[allow(missing_docs)]
    // solc v0.8.26; solc contract.sol --via-ir --optimize --abi -o artifacts
    #[sol(rpc)]
    contract Dipper {
        mapping(address => bool) public locks;
        struct DestWallet {
            address addr;
            uint256 amount;
        }
        function m1_dipper(
            uint256 tokensMaxBag,
            uint256 unclogEthAmount,
            uint8 unclog_nloops,
            uint256 minEthLiquidity,
            uint256 bribe_good,
            uint256 bribe_bad,
            uint8 min_successfull_swaps,
            address[] calldata good_validators,
            DestWallet[] calldata destWallets,
            address[] calldata path,
            address pair
        ) external payable {}
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
