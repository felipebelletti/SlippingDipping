// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.26;

interface IUniswapV2Router02 {
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

    function swapETHForExactTokens(
        uint amountOut,
        address[] calldata path,
        address to,
        uint deadline
    ) external payable returns (uint[] memory amounts);
}

interface IUniswapV2Factory {
    function getPair(
        address tokenA,
        address tokenB
    ) external view returns (address pair);
}

interface IUniswapV2Pair {
    function token0() external view returns (address);

    function token1() external view returns (address);

    function getReserves()
        external
        view
        returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
}

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);

    function approve(address spender, uint256 amount) external returns (bool);
}

contract Dipper {
    mapping(address => bool) public locks;
    mapping(address => bool) private owners;
    IUniswapV2Router02 private extRouter =
        IUniswapV2Router02(address(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D));
    IUniswapV2Factory private extFactory =
        IUniswapV2Factory(address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f));

    struct DestWallet {
        address addr;
        uint256 amount;
    }

    modifier onlyOwner() {
        require(owners[msg.sender], "!owner");
        _;
    }

    constructor() {
        owners[msg.sender] = true;
    }

    receive() external payable {}

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
            require(ethLiquidity > minEthLiquidity, "!mlq");
        } catch {
            revert("!pair");
        }
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
    ) external payable onlyOwner {
        require(!locks[path[path.length - 1]], "locked");

        validatePair(path[path.length - 2], pair, minEthLiquidity);
        unclog_by_nloops(unclogEthAmount, unclog_nloops, path);

        uint8 max_failed_swaps = uint8(destWallets.length) -
            min_successfull_swaps;
        uint8 failed_swaps;

        for (uint8 walletIdx = 0; walletIdx < destWallets.length; walletIdx++) {
            try
                extRouter.swapETHForExactTokens{
                    value: destWallets[walletIdx].amount
                }(
                    tokensMaxBag,
                    path,
                    destWallets[walletIdx].addr,
                    block.timestamp + 120
                )
            {} catch Error(string memory) {
                failed_swaps += 1;
            }
            if (failed_swaps > max_failed_swaps) {
                revert("not enough successfull swaps");
            }
        }

        uint256 bribe = isAddressInArray(good_validators, block.coinbase)
            ? bribe_good
            : bribe_bad;
        if (bribe > 0) {
            (bool success, ) = payable(block.coinbase).call{value: bribe}("");
            require(success, "!bribe");
        }

        // Refund any leftover ETH to the msg.sender
        uint256 leftoverEth = address(this).balance;
        if (leftoverEth > 0) {
            (bool success, ) = payable(msg.sender).call{value: leftoverEth}("");
            require(success, "Refund failed");
        }

        locks[path[path.length - 1]] = true;
    }

    function unclog_by_nloops(
        uint256 ethAmount,
        uint8 nloops,
        address[] calldata path
    ) internal {
        if (nloops == 0) return;

        try
            extRouter.swapExactETHForTokensSupportingFeeOnTransferTokens{
                value: ethAmount
            }(0, path, address(this), block.timestamp + 120)
        {} catch Error(string memory reason) {
            revert(string(abi.encodePacked("!unc #1: ", reason)));
        }

        IERC20 destToken = IERC20(path[path.length - 1]);
        uint256 tokensBalance = destToken.balanceOf(address(this));
        uint256 sellTokensPerLoop = tokensBalance / nloops;
        address[] memory sellPath = invertPath(path);

        destToken.approve(
            address(extRouter),
            0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        );

        for (uint8 x = 1; x <= nloops; x++) {
            try
                extRouter.swapExactTokensForETHSupportingFeeOnTransferTokens(
                    sellTokensPerLoop,
                    0,
                    sellPath,
                    tx.origin,
                    block.timestamp + 120
                )
            {} catch Error(string memory reason) {
                revert(string(abi.encodePacked("!unc #loop: ", reason)));
            }

            // if we end up with less tokens than the expected, recalculate sellTokensPerLoop
            uint256 leftBalance = destToken.balanceOf(address(this));
            if (sellTokensPerLoop > leftBalance && x < nloops) {
                sellTokensPerLoop = leftBalance / (nloops - x);
            }
        }
    }

    function invertPath(
        address[] calldata path
    ) internal pure returns (address[] memory) {
        uint256 length = path.length;
        address[] memory invertedPath = new address[](length);
        for (uint256 i = 0; i < length; i++) {
            invertedPath[i] = path[length - 1 - i];
        }
        return invertedPath;
    }

    function isAddressInArray(
        address[] memory addresses,
        address target
    ) internal pure returns (bool) {
        uint length = addresses.length;
        for (uint i = 0; i < length; i++) {
            if (addresses[i] == target) {
                return true;
            }
        }
        return false;
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

    function removeLock(address tokenAddress) external onlyOwner {
        delete locks[tokenAddress];
    }

    function toggleOwner(address _owner, bool _state) external onlyOwner {
        owners[_owner] = _state;
    }
}
