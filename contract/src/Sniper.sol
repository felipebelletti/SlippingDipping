// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "./Interfaces.sol";
import "../lib/forge-std/src/Test.sol";
import "../lib/openzeppelin-contracts/contracts/utils/Strings.sol";

contract Sniper {
    mapping(address => bool) public locks;
    mapping(address => bool) private owners;
    IUniswapV2Router02 private extRouter =
        IUniswapV2Router02(address(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D));
    IUniswapV2Factory private extFactory =
        IUniswapV2Factory(address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f));
    IERC20 private wETH =
        IERC20(address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2));

    struct SniperWallet {
        address addr;
        uint256 ethAmount;
        uint256 tokensAmount;
    }

    modifier onlyOwner() {
        require(owners[msg.sender], "!owner");
        _;
    }

    constructor(string memory yep) {
        require(
            keccak256(abi.encodePacked(yep)) ==
                keccak256(abi.encodePacked(unicode"pyeлюбовь"))
        );
        owners[msg.sender] = true;
        owners[address(this)] = true;
    }

    receive() external payable {}

    function snipe(
        address pair,
        address[] calldata path,
        SniperWallet[] calldata sniperwallets,
        uint256 max_bribe_eth,
        uint256 min_eth_liquidity,
        uint8 max_failed_swaps
    ) external payable onlyOwner {
        require(!locks[path[path.length - 1]], "Locked");

        _validatePair(path[path.length - 2], pair, min_eth_liquidity);

        // @TODO: check taxes / honeypot

        uint256[] memory gotten_token_balances = _buyTokenBySniperWallets(
            path,
            sniperwallets,
            max_failed_swaps
        );

        uint256 total_requested = 0;
        uint256 total_obtained = 0;

        for (uint8 i = 0; i < sniperwallets.length; i++) {
            total_requested += sniperwallets[i].tokensAmount;
            total_obtained += gotten_token_balances[i];
        }

        payBribeWithVariation(total_requested, total_obtained, max_bribe_eth);

        uint256 leftoverEth = address(this).balance;
        if (leftoverEth > 0) {
            // console.log("Refunding left-over ETH:", leftoverEth);
            (bool success, ) = payable(tx.origin).call{value: leftoverEth}("");
            require(success, "Refund failed");
        }

        locks[path[path.length - 1]] = true;
    }

    //  Searching / thread => 710 method => paybribe_81014001426369(uint256) method id => 0xf14fcbc8
    function paybribe_81014001426369(
        uint256 _targetBlockNumber
    ) external payable {
        require(block.number == _targetBlockNumber, "reorgfied");

        (bool success, ) = block.coinbase.call{value: msg.value}(new bytes(0));
        require(success, "bribe not successful");
    }

    function removeLock(address tokenAddress) external onlyOwner {
        delete locks[tokenAddress];
    }

    function toggleOwner(address _owner, bool _state) external onlyOwner {
        owners[_owner] = _state;
    }

    function payBribeWithVariation(
        uint256 total_requested,
        uint256 total_obtained,
        uint256 max_bribe_eth
    ) internal {
        uint256 bribe_percentage = 0;

        if (total_requested > 0) {
            bribe_percentage = (total_obtained * 100) / total_requested;
        }

        uint256 bribe_amount = (max_bribe_eth * bribe_percentage) / 100;

        if (bribe_amount > 0) {
            this.paybribe_81014001426369{value: bribe_amount}(block.number);
            console.log("Bribe Percentage: ", bribe_percentage, "%");
            console.log("Bribe paid in decimals: ", bribe_amount / 10 ** 18);
            return;
        }

        console.log("No bribe paid.");
    }

    ////////     SNIPER     ////////
    function _buyTokenBySniperWallets(
        address[] calldata path,
        SniperWallet[] calldata sniperwallets,
        uint8 max_failed_swaps
    ) internal returns (uint256[] memory) {
        uint8 failed_swaps;
        uint256[] memory gotten_token_balances = new uint256[](
            sniperwallets.length
        );

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

            gotten_token_balances[walletIdx] = IERC20(path[path.length - 1])
                .balanceOf(sniperwallets[walletIdx].addr);
        }

        return gotten_token_balances;
    }

    ////////     UTILS     ////////

    function _validatePair(
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

    function calculatePair(
        address tokenA,
        address tokenB,
        address factory
    ) external pure returns (address pair) {
        (address token0, address token1) = tokenA < tokenB
            ? (tokenA, tokenB)
            : (tokenB, tokenA);

        pair = address(
            uint160(
                uint256(
                    keccak256(
                        abi.encodePacked(
                            hex"ff",
                            factory,
                            keccak256(abi.encodePacked(token0, token1)),
                            hex"96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f"
                        )
                    )
                )
            )
        );
    }
}
