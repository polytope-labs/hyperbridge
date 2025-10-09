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

import {ISwapRouter} from "@uniswap/v3-periphery/contracts/interfaces/ISwapRouter.sol";
import {IQuoter} from "@uniswap/v3-periphery/contracts/interfaces/IQuoter.sol";

import {IWETH} from "../interfaces/IWETH.sol";

/**
 * @title UniV3UniswapV2Wrapper
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev A module that wraps the Uniswap V3 Swap Router to provide a compatible interface with the Uniswap V2 Router.
 */
contract UniV3UniswapV2Wrapper {
    using SafeERC20 for IERC20;

    struct Params {
        /// @dev Address of the Wrapped Ether (WETH) token.
        address WETH;
        /// @dev Address of the Uniswap V3 Swap Router.
        address swapRouter;
        /// @dev Address of the Uniswap V3 quoter contract
        address quoter;
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
     * @dev The maximum allowable fees for the UniV3UniswapV2Wrapper module.
     * This constant represents a fee of 0.05%, which is equivalent to 500 basis points.
     */
    uint24 constant MAX_FEES = 500; // 0.05%

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
 * @notice Swaps exact amount of ETH for tokens through V3.
 * @param amountOutMin The minimum amount of tokens to receive
 * @param path Array of token addresses representing the swap path
 * @param recipient Address that will receive the output tokens
 * @param deadline Unix timestamp deadline by which the transaction must confirm
 * @return amounts Array of amounts [ethSpent, tokensReceived]
 */
function swapExactETHForTokens(
    uint256 amountOutMin,  // Changed from amountOut
    address[] calldata path,
    address recipient,
    uint256 deadline
) external payable returns (uint256[] memory) {
    address weth = _params.WETH;
    if (path[0] != weth) revert InvalidWethAddress();


    (bool sent, ) = weth.call{value: msg.value}("");
    if (!sent) revert DepositFailed();

   
    ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
        tokenIn: weth,
        tokenOut: path[1],
        fee: MAX_FEES,
        recipient: recipient,
        deadline: deadline,
        amountIn: msg.value, 
        amountOutMinimum: amountOutMin, 
        sqrtPriceLimitX96: 0
    });
    
    uint256 amountReceived = ISwapRouter(_params.swapRouter).exactInputSingle(params);
    

    uint256[] memory amounts = new uint256[](2);
    amounts[0] = msg.value; 
    amounts[1] = amountReceived; 

    return amounts;
}

/**
 * @notice Swaps ETH for exact amount of tokens through V3.
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

    ISwapRouter.ExactOutputSingleParams memory params = ISwapRouter.ExactOutputSingleParams({
        tokenIn: weth,
        tokenOut: path[1],
        fee: MAX_FEES,
        recipient: recipient,
        deadline: deadline,
        amountOut: amountOut,
        amountInMaximum: msg.value,
        sqrtPriceLimitX96: 0
    });
    
    uint256 spent = ISwapRouter(_params.swapRouter).exactOutputSingle(params);
    
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
    uint256 quote = IQuoter(_params.quoter).quoteExactOutputSingle(path[0], path[1], MAX_FEES, amountOut, 0);
    uint256[] memory amounts = new uint256[](2);
    amounts[0] = quote;        
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
    uint256 quote = IQuoter(_params.quoter).quoteExactInputSingle(path[0], path[1], MAX_FEES, amountIn, 0);
    uint256[] memory amounts = new uint256[](2);
    amounts[0] = amountIn;     
    amounts[1] = quote;        
    return amounts;
}

    /// @notice Accepts ETH transfers to this contract
    /// @dev Fallback function to receive ETH payments, required for unwrapping WETH
    receive() external payable {}
}
