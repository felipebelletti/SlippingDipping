// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "forge-std/Script.sol";
import "../src/Dipper.sol";

contract DipperDeploy is Script {
    function run() external {
        vm.startBroadcast();
        
        Dipper dipper = new Dipper();
        
        vm.stopBroadcast();
        
        console.log("Deployed address:", address(dipper));
    }
}
