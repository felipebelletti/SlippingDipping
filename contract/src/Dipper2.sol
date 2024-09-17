// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "./Interfaces.sol";
import "../lib/forge-std/src/Test.sol";

contract Dipper {
    mapping(address => bool) public locks;
    mapping(address => bool) private owners;
    IUniswapV2Router02 private extRouter =
        IUniswapV2Router02(address(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D));
    IUniswapV2Factory private extFactory =
        IUniswapV2Factory(address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f));

    struct SniperWallet {
        address addr;
        uint256 ethAmount;
        uint256 tokensAmount;
    }

    modifier onlyOwner() {
        require(owners[msg.sender], "!owner");
        _;
    }

    constructor() {
        owners[msg.sender] = true;
        owners[address(this)] = true;
    }

    receive() external payable {}

    event dipperCostReport(uint256 dipperCost);

    function validatePair(
        address src,
        address pairAddress,
        uint256 minEthLiquidity
    ) internal view {
        IUniswapV2Pair pair = IUniswapV2Pair(pairAddress);

        try pair.getReserves() returns (
            uint112 reserve0,
            uint112 reserve1,
            uint32
        ) {
            uint112 ethLiquidity = pair.token0() == src ? reserve0 : reserve1;
            require(ethLiquidity > minEthLiquidity, "Insufficient Liquidity");
        } catch {
            revert("!Pair");
        }
    }

    function exploit(
        uint8 maxRounds,
        uint256 maxEthSpentOnExploit,
        uint256 minEthLiquidity,
        uint256 swapThresholdTokens,
        uint8 sniper_max_failed_swaps,
        // address pair,
        address[] calldata path,
        SniperWallet[] calldata sniperWallets
    ) public payable onlyOwner {
        require(!locks[path[path.length - 1]], "Locked");

        // validatePair(path[path.length - 2], pair, minEthLiquidity);

        uint8 mode = getDipperMode(path, swapThresholdTokens);
        require(mode != 0, "Could not identify");

        if (mode == 1) {
            _exploit_m1_m6(path, 0, maxRounds);
        }

        if (mode == 2) {
            _exploit_m2_m3_m4_m5(
                path,
                0,
                swapThresholdTokens,
                maxEthSpentOnExploit,
                maxRounds
            );
        }

        _buyTokenBySniperWallets(path, sniperWallets, sniper_max_failed_swaps);

        uint256 leftoverEth = address(this).balance;
        if (leftoverEth > 0) {
            console.log("Refunding left-over ETH:", leftoverEth);
            (bool success, ) = payable(tx.origin).call{value: leftoverEth}("");
            require(success, "Refund failed");
        }

        locks[path[path.length - 1]] = true;
    }

    function getDipperMode(
        address[] calldata path,
        uint256 swapThreshold
    ) public payable onlyOwner returns (uint8 mode) {
        // m1, m6 - tryDipping(0.001, <not required / not used / zero>, 1)
        console.log("M1 / M6 Tests");
        if (_simulateDipping(path, 1e15, 0, 1) == 1) {
            return 1;
        }

        // m2, m3, m5 - tryDipping(<a reasonable amount for buying a maxbag per round>, maxBag, 1)
        console.log("M2, M3, M5 Tests");
        if (_simulateDipping(path, msg.value, swapThreshold, 1) == 1) {
            return 2;
        }
    }

    function getCloggedPercentageAndRawAmount(
        IERC20 token
    ) public view onlyOwner returns (uint256, uint256) {
        uint256 clogged = token.balanceOf(address(token));
        uint256 supply = token.totalSupply();

        console.log("CloggedAmount: ", clogged);
        console.log("SupplyAmount:  ", supply);

        require(supply > 0, "Total supply must be greater than zero");

        // Supports up to 2 decimal places.
        // ParsedCloggedPercentage = getCloggedPercentageAndRawAmount() / 100
        return ((clogged * 10000) / supply, clogged);
    }

    /*
        m1 = tryDipping(0.001, <not required / not used / zero>, 1)
        m2, m3, m5 = tryDipping(<a reasonable amount for buying a maxbag per round>, maxBag, 1)
        m6 = tryDipping(<a reasonable amount for buying a maxbag per round>, fullCloggedAmount, 1)
        This function should always revert, thus recreating a simulation scenario where nothing really happens
    */
    function tryDipping(
        address[] calldata path,
        uint256 weth_per_round,
        uint256 tokens_per_round,
        uint8 rounds
    ) external payable onlyOwner {
        IERC20 token = IERC20(path[path.length - 1]);

        // calculates the clogged %
        (uint256 initialClogged, ) = getCloggedPercentageAndRawAmount(token);
        console.log("initialCloggedPercentage: ", initialClogged);

        // approve
        token.approve(address(extRouter), type(uint256).max);

        for (uint8 i = 0; i < rounds; i++) {
            // swap buy
            if (tokens_per_round == 0) {
                extRouter.swapExactETHForTokensSupportingFeeOnTransferTokens{
                    value: weth_per_round
                }(0, path, address(this), block.timestamp + 120);
            } else {
                extRouter.swapETHForExactTokens{value: weth_per_round}(
                    tokens_per_round,
                    path,
                    address(this),
                    block.timestamp + 120
                );
            }

            address[] memory sellPath = _invertPath(path);
            // swap sell
            extRouter.swapExactTokensForETHSupportingFeeOnTransferTokens(
                token.balanceOf(address(this)),
                0,
                sellPath,
                address(this),
                block.timestamp + 120
            );
        }

        // calculates the clogged % variation
        (uint256 cloggedAfterUnclog, ) = getCloggedPercentageAndRawAmount(
            token
        );
        console.log("cloggedAfterUnclogPercentage: ", cloggedAfterUnclog);
        uint256 cloggedVariation = initialClogged >= cloggedAfterUnclog
            ? initialClogged - cloggedAfterUnclog
            : 0;
        console.log("cloggedVariation: ", cloggedVariation);

        // If clogged variation is below our threshold (no variation at all), revert with "nope"
        if (cloggedVariation == 0) {
            revert("nope");
        }

        revert("ok");
    }

    ////////     Exploitation Methods     ////////
    function _exploit_m1_m6(
        address[] calldata path,
        uint256 target_clogged_percentage,
        uint8 max_sell_swaps
    ) internal {
        IERC20 token = IERC20(path[path.length - 1]);

        token.approve(address(extRouter), type(uint256).max);

        (uint256 initialClogged, ) = getCloggedPercentageAndRawAmount(token);
        console.log("initialClogged: ", initialClogged);

        uint256 initialEthBalance = address(this).balance;

        try
            extRouter.swapExactETHForTokensSupportingFeeOnTransferTokens{
                value: 0.01 * 10 ** 18
            }(0, path, address(this), block.timestamp + 120)
        {} catch Error(string memory reason) {
            revert(string(abi.encodePacked("!xpl1_6 buy swap: ", reason)));
        }

        uint256 tokensPerSellRound = token.balanceOf(address(this)) /
            max_sell_swaps;
        address[] memory sellPath = _invertPath(path);

        for (uint8 i = 0; i < max_sell_swaps; i++) {
            (
                uint256 cloggedPercentageBefore,

            ) = getCloggedPercentageAndRawAmount(token);

            // swap sell
            try
                extRouter.swapExactTokensForETHSupportingFeeOnTransferTokens(
                    tokensPerSellRound,
                    0,
                    sellPath,
                    address(this),
                    block.timestamp + 120
                )
            {} catch Error(string memory reason) {
                revert(string(abi.encodePacked("!xpl1_6 sell swap: ", reason)));
            }

            (
                uint256 cloggedPercentageAfter,

            ) = getCloggedPercentageAndRawAmount(token);
            uint256 roundUncloggedPercentage = cloggedPercentageBefore >=
                cloggedPercentageAfter
                ? cloggedPercentageBefore - cloggedPercentageAfter
                : 0;

            console.log("Unclog Round", i);
            console.log("Before Clogged:", cloggedPercentageBefore);
            console.log("After Clogged:", cloggedPercentageAfter);
            console.log("Round Unclogged:", roundUncloggedPercentage);

            if (cloggedPercentageAfter <= target_clogged_percentage) {
                console.log(
                    "Clogged Percentage is lower than the target clogged percentage. We're done."
                );
                emit dipperCostReport(initialEthBalance - address(this).balance);
                return;
            }

            if (roundUncloggedPercentage == 0) {
                console.log(
                    "Round Unclogged Percentage was",
                    roundUncloggedPercentage,
                    ". Which means the unclogging is not being effective anymore. We're done."
                );
                emit dipperCostReport(initialEthBalance - address(this).balance);
                return;
            }
        }

        revert("xpl1_6: Sorry, we could not, but at least we tried.");
    }

    function _exploit_m2_m3_m4_m5(
        address[] calldata path,
        uint256 target_clogged_percentage,
        uint256 minSwapThreshold,
        uint256 maxEthSpent,
        uint8 max_rounds
    ) internal {
        require(
            address(this).balance >= maxEthSpent,
            "maxEthSpent is lower than contract's balance."
        );

        IERC20 token = IERC20(path[path.length - 1]);
        token.approve(address(extRouter), type(uint256).max);

        address[] memory sellPath = _invertPath(path);
        uint256 initialEthBalance = address(this).balance;

        for (uint8 i = 0; i < max_rounds; i++) {
            uint256 ethSpent = initialEthBalance - address(this).balance;
            console.log("Spent ", ethSpent, " ETH so far - round:", i);

            require(
                ethSpent < maxEthSpent,
                string(
                    abi.encodePacked(
                        "ETH Consumption is above our threshold. counter=",
                        i
                    )
                )
            );

            (
                uint256 cloggedPercentageBefore,

            ) = getCloggedPercentageAndRawAmount(token);

            try
                extRouter.swapETHForExactTokens{value: address(this).balance}(
                    minSwapThreshold,
                    path,
                    address(this),
                    block.timestamp + 120
                )
            {} catch Error(string memory reason) {
                revert(
                    string(abi.encodePacked("xpl_2_3_4_5: Buy error: ", reason))
                );
            }

            try
                extRouter.swapExactTokensForETHSupportingFeeOnTransferTokens(
                    token.balanceOf(address(this)),
                    0,
                    sellPath,
                    address(this),
                    block.timestamp + 120
                )
            {} catch Error(string memory reason) {
                revert(
                    string(abi.encodePacked("xpl2_3_4_5: Sell error: ", reason))
                );
            }

            (
                uint256 cloggedPercentageAfter,

            ) = getCloggedPercentageAndRawAmount(token);
            uint256 roundUncloggedPercentage = cloggedPercentageBefore >=
                cloggedPercentageAfter
                ? cloggedPercentageBefore - cloggedPercentageAfter
                : 0;

            console.log("Unclog Round #", i);
            console.log("Before Clogged  %:", cloggedPercentageBefore);
            console.log("After Clogged   %:", cloggedPercentageAfter);
            console.log("Round Unclogged %:", roundUncloggedPercentage);

            if (cloggedPercentageAfter <= target_clogged_percentage) {
                console.log(
                    "Clogged Percentage is lower than the target clogged percentage. We're done."
                );
                emit dipperCostReport(initialEthBalance - address(this).balance);
                return;
            }

            if (roundUncloggedPercentage == 0) {
                console.log(
                    "Round Unclogged Percentage was",
                    roundUncloggedPercentage,
                    ". Which means the unclogging is not being effective anymore. We're done."
                );
                emit dipperCostReport(initialEthBalance - address(this).balance);
                return;
            }
        }

        revert("xpl2_3_4_5: Sorry, we could not, but at least we tried.");
    }

    function removeLock(address tokenAddress) external onlyOwner {
        delete locks[tokenAddress];
    }

    function toggleOwner(address _owner, bool _state) external onlyOwner {
        owners[_owner] = _state;
    }

    ////////     SNIPER     ////////
    function _buyTokenBySniperWallets(
        address[] calldata path,
        SniperWallet[] calldata sniperwallets,
        uint8 max_failed_swaps
    ) internal {
        uint8 failed_swaps;

        for (
            uint8 walletIdx = 0;
            walletIdx < sniperwallets.length;
            walletIdx++
        ) {
            if (sniperwallets[walletIdx].tokensAmount == 0) {
                // exactEth
                uint[] memory amounts = extRouter.getAmountsOut(
                    sniperwallets[walletIdx].ethAmount,
                    path
                );
                uint256 minTokensOut = (amounts[1] * 30) / 100; // 70% slippage

                try
                    extRouter
                        .swapExactETHForTokensSupportingFeeOnTransferTokens{
                        value: sniperwallets[walletIdx].ethAmount
                    }(
                        minTokensOut,
                        path,
                        sniperwallets[walletIdx].addr,
                        block.timestamp + 120
                    )
                {} catch Error(string memory) {
                    failed_swaps += 1;
                }
            } else {
                // usually maxbag
                try
                    extRouter.swapETHForExactTokens{
                        value: sniperwallets[walletIdx].ethAmount
                    }(
                        sniperwallets[walletIdx].tokensAmount,
                        path,
                        sniperwallets[walletIdx].addr,
                        block.timestamp + 120
                    )
                {} catch Error(string memory) {
                    failed_swaps += 1;
                }
            }
            if (failed_swaps > max_failed_swaps) {
                revert("too many failed swaps");
            }
        }
    }

    ////////     UTILS     ////////

    function _simulateDipping(
        address[] calldata path,
        uint256 weth_per_round,
        uint256 tokens_per_round,
        uint8 rounds
    ) internal returns (uint8) {
        try this.tryDipping(path, weth_per_round, tokens_per_round, rounds) {
            // just because of lint - TryDipping will always revert.
            return 0;
        } catch Error(string memory reason) {
            if (
                keccak256(abi.encodePacked(reason)) ==
                keccak256(abi.encodePacked("ok"))
            ) {
                return 1;
            } else {
                return 2;
            }
        }
    }

    function _invertPath(
        address[] calldata path
    ) internal pure returns (address[] memory) {
        uint256 length = path.length;
        address[] memory invertedPath = new address[](length);
        for (uint256 i = 0; i < length; i++) {
            invertedPath[i] = path[length - 1 - i];
        }
        return invertedPath;
    }
}
