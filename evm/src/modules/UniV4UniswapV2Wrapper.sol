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

/**
 * @title UniV4UniswapV2Wrapper
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Wraps Uniswap V4 Universal Router with V2-style interface for ETH swaps
 */
contract UniV4UniswapV2Wrapper {
    struct Params {
        address universalRouter;
        address quoter;
        address WETH;
        uint24 defaultFee;
        int24 defaultTickSpacing;
    }

    Params private _params;
    bool private _initialized;
    address private _deployer;

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

    function swapExactETHForTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address recipient,
        uint256 deadline
    ) external payable returns (uint256[] memory amounts) {

        PoolKey memory poolKey = _createPoolKey(path[1]);

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(poolKey, true, uint128(msg.value), uint128(amountOutMin), bytes(""));
        params[1] = abi.encode(poolKey.currency0, msg.value, false);
        params[2] = abi.encode(poolKey.currency1, recipient, uint256(0)); // Using 0 to take full output balance

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(
            abi.encodePacked(uint8(Actions.SWAP_EXACT_IN_SINGLE), uint8(Actions.SETTLE), uint8(Actions.TAKE)),
            params
        );

        uint256 balanceBefore = IERC20(path[1]).balanceOf(recipient);

        IUniversalRouter(_params.universalRouter).execute{value: msg.value}(
            abi.encodePacked(bytes1(uint8(Commands.V4_SWAP))),
            inputs,
            deadline
        );

        uint256 balanceAfter = IERC20(path[1]).balanceOf(recipient);

        amounts = new uint256[](2);
        amounts[0] = msg.value;
        amounts[1] = balanceAfter - balanceBefore;
    }

        function swapETHForExactTokens(
        uint256 amountOut,
        address[] calldata path,
        address recipient,
        uint256 deadline
    ) external payable returns (uint256[] memory amounts) {

        PoolKey memory poolKey = _createPoolKey(path[1]);

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(poolKey, true, uint128(amountOut), uint128(msg.value), bytes(""));
        params[1] = abi.encode(poolKey.currency0, uint256(0), false);
        params[2] = abi.encode(poolKey.currency1, recipient, amountOut);

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(
            abi.encodePacked(
                uint8(Actions.SWAP_EXACT_OUT_SINGLE),
                uint8(Actions.SETTLE),
                uint8(Actions.TAKE)
            ),
            params
        );

        IUniversalRouter(_params.universalRouter).execute{value: msg.value}(
            abi.encodePacked(bytes1(uint8(Commands.V4_SWAP))),
            inputs,
            deadline
        );


        uint256 refundETH = address(this).balance;


        if (refundETH > 0) {
            (bool success, ) = _deployer.call{value: refundETH}("");
            require(success, "ETH refund failed");
        }

        amounts = new uint256[](2);
        amounts[0] = msg.value - refundETH;
        amounts[1] = amountOut;
    }

    function getAmountsIn(uint256 amountOut, address[] calldata path)
        external
        returns (uint256[] memory amounts)
    {

        (uint256 amountIn, ) = IV4Quoter(_params.quoter).quoteExactOutputSingle(
            IV4Quoter.QuoteExactSingleParams(_createPoolKey(path[1]), true, uint128(amountOut), bytes(""))
        );

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
    }

    function getAmountsOut(uint256 amountIn, address[] calldata path)
        external
        returns (uint256[] memory amounts)
    {

        (uint256 amountOut, ) = IV4Quoter(_params.quoter).quoteExactInputSingle(
            IV4Quoter.QuoteExactSingleParams(_createPoolKey(path[1]), true, uint128(amountIn), bytes(""))
        );

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
    }

    function _createPoolKey(address tokenOut)
        internal
        view
        returns (PoolKey memory)
    {
        return PoolKey({
            currency0: Currency.wrap(address(0)),  // ETH is always currency0
            currency1: Currency.wrap(tokenOut),
            fee: _params.defaultFee,
            tickSpacing: _params.defaultTickSpacing,
            hooks: IHooks(address(0))
        });
    }

    receive() external payable {}
}
