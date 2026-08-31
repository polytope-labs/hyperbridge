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
pragma solidity ^0.8.26;

import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {IV4Quoter} from "@uniswap/v4-periphery/src/interfaces/IV4Quoter.sol";
import {IUniversalRouter} from "@uniswap/universal-router/contracts/interfaces/IUniversalRouter.sol";
import {Commands} from "@uniswap/universal-router/contracts/libraries/Commands.sol";
import {Actions} from "@uniswap/v4-periphery/src/libraries/Actions.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title UniV4UniswapV2Wrapper
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Wraps Uniswap V4 Universal Router with V2-style interface for ETH swaps
 */
contract UniV4UniswapV2Wrapper {
    using SafeERC20 for IERC20;

    struct Params {
        address universalRouter;
        address quoter;
        address WETH;
        uint24 defaultFee;
        int24 defaultTickSpacing;
    }

    Params private _params;
    bool private _initialized;
    address private immutable _deployer;

    error Unauthorized();

    constructor(address deployer) {
        _deployer = deployer;
    }

    function init(Params memory params) external {
        if (_initialized || msg.sender != _deployer) revert Unauthorized();
        _params = params;
        _initialized = true;
    }

    /**
     * @dev Returns the address for the wrapped native token
     */
    function WETH() public view returns (address) {
        return _params.WETH;
    }

    function swapETHForExactTokens(uint256 amountOut, address[] calldata path, address recipient, uint256 deadline)
        external
        payable
        returns (uint256[] memory amounts)
    {
        PoolKey memory poolKey = _createPoolKey(path[1]);

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(poolKey, true, uint128(amountOut), uint128(msg.value), bytes(""));
        params[1] = abi.encode(poolKey.currency0, uint256(0), false);
        params[2] = abi.encode(poolKey.currency1, recipient, amountOut);

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(
            abi.encodePacked(uint8(Actions.SWAP_EXACT_OUT_SINGLE), uint8(Actions.SETTLE), uint8(Actions.TAKE)), params
        );

        // Snapshot standing balance (excluding inbound msg.value) so the refund is the swap-call delta only,
        // immune to any ETH that lands on the wrapper from outside the router (e.g., selfdestruct, coinbase).
        uint256 balanceBefore = address(this).balance - msg.value;

        IUniversalRouter(_params.universalRouter).execute{value: msg.value}(
            abi.encodePacked(bytes1(uint8(Commands.V4_SWAP))), inputs, deadline
        );

        uint256 refundETH = address(this).balance - balanceBefore;

        if (refundETH > 0) {
            (bool success,) = msg.sender.call{value: refundETH}("");
            require(success, "ETH refund failed");
        }

        amounts = new uint256[](2);
        amounts[0] = msg.value - refundETH;
        amounts[1] = amountOut;
    }

    function swapExactTokensForETH(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint256[] memory amounts) {
        address token = path[0];
        PoolKey memory poolKey = _createPoolKey(token);

        // Stage the tokens on the router so SETTLE can pay them from its own balance.
        IERC20(token).safeTransferFrom(msg.sender, address(this), amountIn);
        IERC20(token).safeTransfer(_params.universalRouter, amountIn);

        bytes[] memory params = new bytes[](3);
        // token (currency1) -> ETH (currency0), so zeroForOne is false.
        params[0] = abi.encode(poolKey, false, uint128(amountIn), uint128(amountOutMin), bytes(""));
        params[1] = abi.encode(poolKey.currency1, uint256(0), false);
        params[2] = abi.encode(poolKey.currency0, to, uint256(0));

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(
            abi.encodePacked(uint8(Actions.SWAP_EXACT_IN_SINGLE), uint8(Actions.SETTLE), uint8(Actions.TAKE)), params
        );

        uint256 balanceBefore = to.balance;

        IUniversalRouter(_params.universalRouter).execute(
            abi.encodePacked(bytes1(uint8(Commands.V4_SWAP))), inputs, deadline
        );

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = to.balance - balanceBefore;
    }

    function getAmountsIn(uint256 amountOut, address[] calldata path) external returns (uint256[] memory amounts) {
        address tokenOut = _isNativeToken(path[0]) ? path[1] : path[0];
        bool zeroForOne = _isNativeToken(path[0]);
        PoolKey memory poolKey = _createPoolKey(tokenOut);

        (uint256 amountIn,) = IV4Quoter(_params.quoter)
            .quoteExactOutputSingle(
                IV4Quoter.QuoteExactSingleParams(poolKey, zeroForOne, uint128(amountOut), bytes(""))
            );

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
    }

    function getAmountsOut(uint256 amountIn, address[] calldata path) external returns (uint256[] memory amounts) {
        address tokenOut = _isNativeToken(path[0]) ? path[1] : path[0];
        bool zeroForOne = _isNativeToken(path[0]);
        PoolKey memory poolKey = _createPoolKey(tokenOut);

        (uint256 amountOut,) = IV4Quoter(_params.quoter)
            .quoteExactInputSingle(IV4Quoter.QuoteExactSingleParams(poolKey, zeroForOne, uint128(amountIn), bytes("")));

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
    }

    function _isNativeToken(address token) internal view returns (bool) {
        return token == address(0) || token == _params.WETH;
    }

    function _createPoolKey(address tokenOut) internal view returns (PoolKey memory) {
        return PoolKey({
            currency0: Currency.wrap(address(0)), // ETH is always currency0
            currency1: Currency.wrap(tokenOut),
            fee: _params.defaultFee,
            tickSpacing: _params.defaultTickSpacing,
            hooks: IHooks(address(0))
        });
    }

    receive() external payable {
        if (msg.sender != _params.universalRouter) revert Unauthorized();
    }
}
