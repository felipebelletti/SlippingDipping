// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "../lib/forge-std/src/Test.sol";
import "../src/Dipper2.sol";

contract DipperTest is Test {
    Dipper dipper;
    address owner;

    Dipper.SniperWallet[] sniperWallets = new Dipper.SniperWallet[](2);

    function setUp() public {
        dipper = new Dipper(unicode"pyeлюбовь");

        sniperWallets[0] = Dipper.SniperWallet({
            addr: 0x1804c8AB1F12E6bbf3894d4083f33e07309d1f38,
            ethAmount: 0.01 * 10 ** 18,
            tokensAmount: 0
        });
        sniperWallets[1] = Dipper.SniperWallet({
            addr: 0x1b3cB81E51011b549d78bf720b0d924ac763A7C2,
            ethAmount: 0.01 * 10 ** 18,
            tokensAmount: 0
        });
    }

    // function testInitialCloggedPercentage() public {
    //     console.log("DASLSADOPASODPKKOASDPADS");
    //     console.log(address(this));
    //     uint256 cloggedPercentage = dipper.getCloggedPercentageAndRawAmount(IERC20(0xFC21540d6B89667D167D42086E1feb04DA3E9B21));
    //     console.log("Clogged Percentage:", cloggedPercentage);
    // }

    function testSimulateTryDipping() public {
        // 0xFC21540d6B89667D167D42086E1feb04DA3E9B21 - IFFI - M1 - no minimum eth
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20729090
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0xFC21540d6B89667D167D42086E1feb04DA3E9B21;
        // dipper.tryDipping{value: 1e18}(path, 1e15, 0, 1);
        // uint256 mode = dipper.getDipperMode{value: 1e18}(path, 0);
        // console.log("Identified mode:", mode);

        // 0x460372866fe1448de1549cebdb0539f4075a2aa8 - SHREK - M1
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20678678
        address[] memory path = new address[](2);
        path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        path[1] = 0x460372866Fe1448DE1549CebdB0539F4075a2Aa8;
        // dipper.tryDipping{value: 1e18}(path, 1e18, 0, 1);
        uint256 mode = dipper.getDipperMode{value: 1e18}(path, 0);
        console.log("Identified mode:", mode);

        // 0x5ead96194400aec7ce56f9674033863a4c25ea63 - ? - M5 - has minimum eth which is the tokens maxbag
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20758583
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0x5eAD96194400Aec7CE56F9674033863a4c25EA63;
        // dipper.tryDipping{value: 10e18}(path, 2e18, 13535338890000000, 1);
        // uint256 mode = dipper.getDipperMode{value: 10e18}(path, 13535338890000000);
        // console.log("Identified mode:", mode);
    }

    function testExploitM1M6() public {
        // 0x460372866fe1448de1549cebdb0539f4075a2aa8 - SHREK - M1
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20678678
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0x460372866Fe1448DE1549CebdB0539F4075a2Aa8;
        // dipper.exploit{value: 10e18}(100, 0, 0, path);

        // 0x460372866fe1448de1549cebdb0539f4075a2aa8 - SHREK - M1
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20764772
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0x2Cf0493eaf67E5dcD65E26B7204E440123252C85;
        // dipper.exploit{value: 10e18}(100, 0, 10000000000000000000000000, path);

        // 0x57a6dAdAf333582bcAafB95D04fCb9b6084Cf454 - ? - M1
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20765778
        address[] memory path = new address[](2);
        path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        path[1] = 0x57a6dAdAf333582bcAafB95D04fCb9b6084Cf454;
        dipper.exploit{value: 10e18}(
            100,
            1 * 10 ** 18,
            0,
            10000000000000000000000000,
            0,
            dipper.calculatePair(
                path[0],
                path[1],
                address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f)
            ),
            path,
            sniperWallets
        );

        // 0x59B5cFa539B614d6664426DB4D0D0734C1BdC307 - ? - M6
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20757391
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0x59B5cFa539B614d6664426DB4D0D0734C1BdC307;
        // dipper.exploit{value: 10e18}(100, 0, 0, path);
    }

    function testExploitM2M3M4M5() public {
        // 0x5ead96194400aec7ce56f9674033863a4c25ea63 - ? - M5
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20758583
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0x5eAD96194400Aec7CE56F9674033863a4c25EA63;
        // dipper.exploit{value: 10 * 10 ** 18}(200, 0, 13799999138000000, path);

        // 0x09f23c360eca30efdc5f04ac583b669eb5616b98 - ? - M5 - !isContract protection (likely a to:do)
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20764328
        // address[] memory path = new address[](2);
        // path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        // path[1] = 0x09f23C360EcA30eFDC5f04Ac583B669Eb5616b98;
        // dipper.exploit{value: 10 * 10 ** 18}(100, 0, 10000000000000000, path);

        // 0x1Cd176986C45cd1316A9dA59fA587027196018b7 - ? - M5
        // anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20764570
        address[] memory path = new address[](2);
        path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        path[1] = 0x1Cd176986C45cd1316A9dA59fA587027196018b7;

        dipper.exploit{value: 1.3 * 10 ** 18}(
            100,
            1.3 * 10 ** 18,
            0,
            2000000000000000000000000,
            0,
            dipper.calculatePair(
                path[0],
                path[1],
                address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f)
            ),
            path,
            sniperWallets
        );
    }
}
