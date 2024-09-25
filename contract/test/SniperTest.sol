// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "../lib/forge-std/src/Test.sol";
import "../src/Sniper.sol";

contract DipperTest is Test {
    Sniper sniper;
    address owner;

    Sniper.SniperWallet[] sniperWallets = new Sniper.SniperWallet[](2);

    function setUp() public {
        sniper = new Sniper(unicode"pyeлюбовь");

        sniperWallets[0] = Sniper.SniperWallet({
            addr: 0x1804c8AB1F12E6bbf3894d4083f33e07309d1f38,
            ethAmount: 3 * 10 ** 18,
            tokensAmount: 2000000000000000000000000
        });
        sniperWallets[1] = Sniper.SniperWallet({
            addr: 0x1b3cB81E51011b549d78bf720b0d924ac763A7C2,
            ethAmount: 3 * 10 ** 18,
            tokensAmount: 2000000000000000000000000
        });
    }

    function testSnipe() public {
        // 0x1Cd176986C45cd1316A9dA59fA587027196018b7 - ? - M5
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20764570
        address[] memory path = new address[](2);
        path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        path[1] = 0x1Cd176986C45cd1316A9dA59fA587027196018b7;

        sniper.snipe{value: 10 * 10 ** 18}(
            sniper.calculatePair(
                path[0],
                path[1],
                address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f)
            ),
            path,
            sniperWallets,
            1 * 10 ** 18, // max_bribe_eth
            2 * 10 ** 18, // min_eth_liquidity
            0 // max_failed_swaps
        );
    }
}
