#![feature(let_chains)]

use alloy::hex::FromHex;
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
use api::mev_builders::builder::{self, Builder};
use api::mev_builders::types::{EndOfBlockBundleParams, SendBundleParams};
use api::utils::erc20::get_approve_raw_tx;
use api::utils::{get_raw_bribe_tx, print_pretty_dashboard};
use api::{mev_builders, sell_stream, simulate, strategies};
use colored::Colorize;
use config::wallet::types::Wallet;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Select};
use revm::primitives::U256;
use revm::{db::AlloyDB, Database, DatabaseRef, Evm};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
#[allow(unused_imports)]
use ERC20::ERC20Instance;

use config::{general::GLOBAL_CONFIG, wallet::GLOBAL_WALLETS};

mod api;
mod common;
mod config;
mod globals;
mod license;

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
    // match license::check_license().await {
    //     Ok(is_valid) => {
    //         if !is_valid {
    //             panic!("Error 309")
    //         }
    //     }
    //     Err(_) => panic!("Error 300"),
    // }

    show_motd();

    let client = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(&GLOBAL_WALLETS.wallets[0].signer)
            .on_builtin(&GLOBAL_CONFIG.provider.rpc_url)
            .await
            .unwrap(),
    );

    // let raw_tx = get_approve_raw_tx(
    //     client.clone(),
    //     GLOBAL_WALLETS.get_wallets()[0].clone(),
    //     Address::from_str("0x0D448711759Ad1Bd147291b9F9f2E90434EfD8c8").unwrap(),
    //     Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap(),
    //     false,
    // ).await.unwrap();
    // mev_builders::broadcast::broadcast_end_of_block_bundle(EndOfBlockBundleParams {
    //     txs: vec![format!("{}", hex::encode(raw_tx))],
    //     // block_number: Some(format!("0x{}", hex::encode((client.get_block_number().await.unwrap() + 2).to_string()))),
    //     block_number: Some("0x1".to_string()),
    //     target_pools: Some(vec!["0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap()]),
    //     ..Default::default()
    // });
    // let builder = &crate::api::mev_builders::RSYNC;
    // loop {
    //     let block_number = client.get_block_number().await.unwrap() + 2;

    //     let raw_tx = get_approve_raw_tx(
    //         client.clone(),
    //         GLOBAL_WALLETS.get_wallets()[0].clone(),
    //         Address::from_str("0x0D448711759Ad1Bd147291b9F9f2E90434EfD8c8").unwrap(),
    //         Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap(),
    //         false,
    //     )
    //     .await
    //     .unwrap();

    //     let bribe_raw_tx = get_raw_bribe_tx(
    //         client.clone(),
    //         GLOBAL_WALLETS.get_wallets()[0].clone(),
    //         0.004,
    //         U256::from(block_number),
    //     )
    //     .await
    //     .unwrap();

    //     println!("{}", block_number);

    //     builder
    //         .send_end_of_block_bundle(EndOfBlockBundleParams {
    //             txs: vec![hex::encode(bribe_raw_tx), hex::encode(raw_tx)],
    //             block_number: Some(format!("0x{:x}", block_number)),
    //             // target_pools: Some(vec!["0xf74B0ab4eFDd5d97F936F4E3475D208b39e8Efc7".parse().unwrap()]),
    //             // txs: vec!["0x02f8b2018201068434b646fb850599a1859f82c350940d448711759ad1bd147291b9f9f2e90434efd8c880b844095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc080a0b04fc1880effc420d89c4a740add63f43b9e5a7787c2813573df898d4ca0acbca06cf457a31bfa739a1937c630486328626793136cc8226939ac9a246bb097470e".to_string()],
    //             ..Default::default()
    //         })
    //         .await
    //         .unwrap();

        // builder.send_bundle(SendBundleParams {
        //     txs: vec![hex::encode(raw_tx)],
        //     block_number: Some(format!("0x{:x}", block_number)),
        //     // target_pools: Some(vec!["0xf74B0ab4eFDd5d97F936F4E3475D208b39e8Efc7".parse().unwrap()]),
        //     // txs: vec!["0x02f8b2018201068434b646fb850599a1859f82c350940d448711759ad1bd147291b9f9f2e90434efd8c880b844095ea7b30000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc080a0b04fc1880effc420d89c4a740add63f43b9e5a7787c2813573df898d4ca0acbca06cf457a31bfa739a1937c630486328626793136cc8226939ac9a246bb097470e".to_string()],
        //     ..Default::default()
        // }).await.unwrap();

        // sleep(Duration::from_secs(6)).await;
    // }
    // return;

    show_pretty_wallet_dashboard(client.clone(), GLOBAL_WALLETS.get_wallets()).await;

    let menu_option = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an option")
        .items(&vec![
            "[ 0 ] M1/M2/M3/M4/M5/M6 Block-Zero Dipper",
            "[ 1 ] Sell-Stream",
        ])
        .default(0)
        .interact()
        .unwrap();

    match menu_option {
        0 => {
            strategies::blockzero_dipper::run(client).await;
        }
        1 => {
            sell_stream::run(client, None, None).await;
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

fn show_motd() {
    println!(
        "{}",
        format!(
            "8888888b.  d8b                   d8b          
888  \"Y88b Y8P                   Y8P          
888    888                                    
888    888 888 88888b.  88888b.  888 88888b.  
888    888 888 888 \"88b 888 \"88b 888 888 \"88b 
888    888 888 888  888 888  888 888 888  888 
888  .d88P 888 888 d88P 888 d88P 888 888  888 
8888888P\"  888 88888P\"  88888P\"  888 888  888 
               888      888                   
               888      888                   
               888      888                   
               "
        )
        .red()
    );
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
        function paybribe_81014001426369(uint256 _targetBlockNumber) external payable {}
        function exploit(
            uint8 maxRounds,
            uint256 expectedLpVariationAfterDip,
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

    #[allow(missing_docs)]
    #[sol(rpc)]
    contract UniswapV2Factory {
        event PairCreated(address indexed token0, address indexed token1, address pair, uint);

        function feeTo() external view returns (address);
        function feeToSetter() external view returns (address);

        function getPair(address tokenA, address tokenB) external view returns (address pair);
        function allPairs(uint) external view returns (address pair);
        function allPairsLength() external view returns (uint);

        function createPair(address tokenA, address tokenB) external returns (address pair);

        function setFeeTo(address) external;
        function setFeeToSetter(address) external;
    }

    #[allow(missing_docs)]
    #[sol(rpc)]
    contract UniswapV2Pair {
        event Approval(address indexed owner, address indexed spender, uint value);
        event Transfer(address indexed from, address indexed to, uint value);

        function name() external pure returns (string memory);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
        function totalSupply() external view returns (uint);
        function balanceOf(address owner) external view returns (uint);
        function allowance(address owner, address spender) external view returns (uint);

        function approve(address spender, uint value) external returns (bool);
        function transfer(address to, uint value) external returns (bool);
        function transferFrom(address from, address to, uint value) external returns (bool);

        function DOMAIN_SEPARATOR() external view returns (bytes32);
        function PERMIT_TYPEHASH() external pure returns (bytes32);
        function nonces(address owner) external view returns (uint);

        function permit(address owner, address spender, uint value, uint deadline, uint8 v, bytes32 r, bytes32 s) external;

        event Mint(address indexed sender, uint amount0, uint amount1);
        event Burn(address indexed sender, uint amount0, uint amount1, address indexed to);
        event Swap(
            address indexed sender,
            uint amount0In,
            uint amount1In,
            uint amount0Out,
            uint amount1Out,
            address indexed to
        );
        event Sync(uint112 reserve0, uint112 reserve1);

        function MINIMUM_LIQUIDITY() external pure returns (uint);
        function factory() external view returns (address);
        function token0() external view returns (address);
        function token1() external view returns (address);
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function price0CumulativeLast() external view returns (uint);
        function price1CumulativeLast() external view returns (uint);
        function kLast() external view returns (uint);

        function mint(address to) external returns (uint liquidity);
        function burn(address to) external returns (uint amount0, uint amount1);
        function swap(uint amount0Out, uint amount1Out, address to, bytes calldata data) external;
        function skim(address to) external;
        function sync() external;

        function initialize(address, address) external;
    }

    #[allow(missing_docs)]
    #[sol(rpc)]
    contract UniswapV2Router01 {
        function factory() external pure returns (address);
        function WETH() external pure returns (address);

        function addLiquidity(
            address tokenA,
            address tokenB,
            uint amountADesired,
            uint amountBDesired,
            uint amountAMin,
            uint amountBMin,
            address to,
            uint deadline
        ) external returns (uint amountA, uint amountB, uint liquidity);
        function addLiquidityETH(
            address token,
            uint amountTokenDesired,
            uint amountTokenMin,
            uint amountETHMin,
            address to,
            uint deadline
        ) external payable returns (uint amountToken, uint amountETH, uint liquidity);
        function removeLiquidity(
            address tokenA,
            address tokenB,
            uint liquidity,
            uint amountAMin,
            uint amountBMin,
            address to,
            uint deadline
        ) external returns (uint amountA, uint amountB);
        function removeLiquidityETH(
            address token,
            uint liquidity,
            uint amountTokenMin,
            uint amountETHMin,
            address to,
            uint deadline
        ) external returns (uint amountToken, uint amountETH);
        function removeLiquidityWithPermit(
            address tokenA,
            address tokenB,
            uint liquidity,
            uint amountAMin,
            uint amountBMin,
            address to,
            uint deadline,
            bool approveMax, uint8 v, bytes32 r, bytes32 s
        ) external returns (uint amountA, uint amountB);
        function removeLiquidityETHWithPermit(
            address token,
            uint liquidity,
            uint amountTokenMin,
            uint amountETHMin,
            address to,
            uint deadline,
            bool approveMax, uint8 v, bytes32 r, bytes32 s
        ) external returns (uint amountToken, uint amountETH);
        function swapExactTokensForTokens(
            uint amountIn,
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);
        function swapTokensForExactTokens(
            uint amountOut,
            uint amountInMax,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
            external
            payable
            returns (uint[] memory amounts);
        function swapTokensForExactETH(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline)
            external
            returns (uint[] memory amounts);
        function swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
            external
            returns (uint[] memory amounts);
        function swapETHForExactTokens(uint amountOut, address[] calldata path, address to, uint deadline)
            external
            payable
            returns (uint[] memory amounts);
        function removeLiquidityETHSupportingFeeOnTransferTokens(
            address token,
            uint liquidity,
            uint amountTokenMin,
            uint amountETHMin,
            address to,
            uint deadline
        ) external returns (uint amountETH);
        function removeLiquidityETHWithPermitSupportingFeeOnTransferTokens(
            address token,
            uint liquidity,
            uint amountTokenMin,
            uint amountETHMin,
            address to,
            uint deadline,
            bool approveMax, uint8 v, bytes32 r, bytes32 s
        ) external returns (uint amountETH);
        function swapExactTokensForTokensSupportingFeeOnTransferTokens(
            uint amountIn,
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external;
        function swapExactETHForTokensSupportingFeeOnTransferTokens(
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external payable;
        function swapExactTokensForETHSupportingFeeOnTransferTokens(
            uint amountIn,
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external;

        function quote(uint amountA, uint reserveA, uint reserveB) external pure returns (uint amountB);
        function getAmountOut(uint amountIn, uint reserveIn, uint reserveOut) external pure returns (uint amountOut);
        function getAmountIn(uint amountOut, uint reserveIn, uint reserveOut) external pure returns (uint amountIn);
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts);
        function getAmountsIn(uint amountOut, address[] calldata path) external view returns (uint[] memory amounts);
    }
}
