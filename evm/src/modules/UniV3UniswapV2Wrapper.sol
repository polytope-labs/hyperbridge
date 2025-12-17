// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.17;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IQuoterV2} from "@uniswap/v3-periphery/contracts/interfaces/IQuoterV2.sol";
import {IV3SwapRouter} from "@uniswap/swap-router-contracts/contracts/interfaces/IV3SwapRouter.sol";
import {IMulticallExtended} from "@uniswap/swap-router-contracts/contracts/interfaces/IMulticallExtended.sol";

import {IWETH} from "../interfaces/IWETH.sol";

/**
 * @title UniV3UniswapV2Wrapper
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev A module that wraps the Uniswap V3 Swap Router02 to provide a compatible interface with the Uniswap V2 Router.
 */
contract UniV3UniswapV2Wrapper {
    using SafeERC20 for IERC20;

    struct Params {
        /// @dev Address of the Wrapped Ether (WETH) token.
        address WETH;
        /// @dev Address of the Uniswap V3 Swap Router02.
        address swapRouter;
        /// @dev Address of the Uniswap V3 quoter contract
        address quoter;
        /// @dev The fees that helps point to the specific pool.
        uint24 maxFee;
    }

    /**
     * @dev Private variable to store the parameters for the UniV3UniswapV2Wrapper module.
     */
    Params private _params;

    /**
     * @dev Private variable to track initialization status.
     */
    bool private _initialized;

    /**
     * @dev The deployer of the contract.
     * The deployer may initialize the contract only once.
     * They also receive all unspent ETH.
     */
    address private _deployer;

    /**
     * @dev Error indicating that a deposit operation has failed.
     */
    error DepositFailed();

    /**
     * @dev Error indicating that a refund operation has failed.
     */
    error RefundFailed();

    /**
     * @dev Error indicating that the caller is not authorized.
     */
    error Unauthorized();

    /**
     * @dev Error indicating that the first token in the path is not WETH.
     */
    error InvalidWethAddress();

    constructor(address deployer) {
        _deployer = deployer;
    }

    /**
     * @notice Initializes the Uniswap V3 to V2 wrapper module
     * @dev Can only be called once
     * @param params Initialization parameters.
     */
    function init(Params memory params) public {
        if (_initialized || msg.sender != _deployer) revert Unauthorized();
        // approve the swap router to spend WETH
        IWETH(params.WETH).approve(params.swapRouter, type(uint256).max);

        _params = params;
        _initialized = true;
    }

    /**
     * @dev Returns the address for the wrapped native token
     */
    function WETH() public view returns (address) {
        return _params.WETH;
    }

    /**
     * @notice Swaps exact amount of ETH for tokens through V3 with deadline protection.
     * @param amountOutMin The minimum amount of tokens to receive
     * @param path Array of token addresses representing the swap path
     * @param recipient Address that will receive the output tokens
     * @param deadline Unix timestamp deadline by which the transaction must confirm
     * @return amounts Array of amounts [ethSpent, tokensReceived]
     */
    function swapExactETHForTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address recipient,
        uint256 deadline
    ) external payable returns (uint256[] memory) {
        address weth = _params.WETH;
        if (path[0] != weth) revert InvalidWethAddress();

        (bool sent, ) = weth.call{value: msg.value}("");
        if (!sent) revert DepositFailed();

        IV3SwapRouter.ExactInputSingleParams memory params = IV3SwapRouter.ExactInputSingleParams({
            tokenIn: weth,
            tokenOut: path[1],
            fee: _params.maxFee,
            recipient: recipient,
            amountIn: msg.value,
            amountOutMinimum: amountOutMin,
            sqrtPriceLimitX96: 0
        });

        bytes memory swapCall = abi.encodeWithSelector(
            IV3SwapRouter.exactInputSingle.selector,
            params
        );

        bytes[] memory data = new bytes[](1);
        data[0] = swapCall;

        bytes[] memory results = IMulticallExtended(_params.swapRouter).multicall(deadline, data);
        uint256 amountReceived = abi.decode(results[0], (uint256));

        uint256[] memory amounts = new uint256[](2);
        amounts[0] = msg.value;
        amounts[1] = amountReceived;

        return amounts;
    }

    /**
     * @notice Swaps ETH for exact amount of tokens through V3 with deadline protection.
     * @param amountOut The exact amount of tokens to receive
     * @param path Array of token addresses representing the swap path
     * @param recipient Address that will receive the output tokens
     * @param deadline Unix timestamp deadline by which the transaction must confirm
     * @return amounts Array of amounts [ethSpent, tokensReceived]
     */
    function swapETHForExactTokens(
        uint256 amountOut,
        address[] calldata path,
        address recipient,
        uint256 deadline
    ) external payable returns (uint256[] memory) {
        address weth = _params.WETH;
        if (path[0] != weth) revert InvalidWethAddress();

        (bool sent, ) = weth.call{value: msg.value}("");
        if (!sent) revert DepositFailed();

        IV3SwapRouter.ExactOutputSingleParams memory params = IV3SwapRouter.ExactOutputSingleParams({
            tokenIn: weth,
            tokenOut: path[1],
            fee: _params.maxFee,
            recipient: recipient,
            amountOut: amountOut,
            amountInMaximum: msg.value,
            sqrtPriceLimitX96: 0
        });

        bytes memory swapCall = abi.encodeWithSelector(
            IV3SwapRouter.exactOutputSingle.selector,
            params
        );

        bytes[] memory data = new bytes[](1);
        data[0] = swapCall;

        bytes[] memory results = IMulticallExtended(_params.swapRouter).multicall(deadline, data);
        uint256 spent = abi.decode(results[0], (uint256));

        if (spent < msg.value) {
            uint256 refund = msg.value - spent;
            IWETH(weth).withdraw(refund);

            (bool success, ) = _deployer.call{value: refund}("");
            if (!success) revert RefundFailed();
        }

        uint256[] memory amounts = new uint256[](2);
        amounts[0] = spent;
        amounts[1] = amountOut;

        return amounts;
    }

    /**
     * @notice Given an output amount of an asset and a path, returns the input amounts required.
     * @param amountOut The amount of the asset you want to receive.
     * @param path An array of token addresses representing the path of the swap.
     * @return amounts An array of input amounts required to obtain the output amount.
     */
    function getAmountsIn(uint amountOut, address[] calldata path) external returns (uint[] memory) {
        IQuoterV2.QuoteExactOutputSingleParams memory params = IQuoterV2.QuoteExactOutputSingleParams({
            tokenIn: path[0],
            tokenOut: path[1],
            amount: amountOut,
            fee: _params.maxFee,
            sqrtPriceLimitX96: 0
        });
        (uint256 amountIn, , , ) = IQuoterV2(_params.quoter).quoteExactOutputSingle(params);
        uint256[] memory amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
        return amounts;
    }

    /**
     * @notice Given an input amount of an asset and a path, returns the output amounts.
     * @param amountIn The amount of the asset you want to swap.
     * @param path An array of token addresses representing the path of the swap.
     * @return amounts An array of output amounts to be received.
     */
    function getAmountsOut(uint amountIn, address[] calldata path) external returns (uint[] memory) {
        IQuoterV2.QuoteExactInputSingleParams memory params = IQuoterV2.QuoteExactInputSingleParams({
            tokenIn: path[0],
            tokenOut: path[1],
            amountIn: amountIn,
            fee: _params.maxFee,
            sqrtPriceLimitX96: 0
        });
        (uint256 amountOut, , , ) = IQuoterV2(_params.quoter).quoteExactInputSingle(params);
        uint256[] memory amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
        return amounts;
    }

    /// @notice Accepts ETH transfers to this contract
    /// @dev Fallback function to receive ETH payments, required for unwrapping WETH
    receive() external payable {}
}
