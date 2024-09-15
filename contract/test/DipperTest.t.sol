// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "../lib/forge-std/src/Test.sol";
import "../src/Dipper2.sol";

contract DipperTest is Test {
    Dipper dipper;
    address owner;

    function setUp() public {
        dipper = new Dipper();
    }

    // function testInitialCloggedPercentage() public {
    //     console.log("DASLSADOPASODPKKOASDPADS");
    //     console.log(address(this));
    //     uint256 cloggedPercentage = dipper.getCloggedPercentage(IERC20(0xFC21540d6B89667D167D42086E1feb04DA3E9B21));
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
        address[] memory path = new address[](2);
        path[0] = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
        path[1] = 0x460372866Fe1448DE1549CebdB0539F4075a2Aa8;
        dipper.exploit{value: 10e18}(100, 0, 0, path);
    }
}
