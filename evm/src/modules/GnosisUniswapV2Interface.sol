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

contract GnosisUniswapV2Interface {
    using SafeERC20 for IERC20;

    error DepositFailed();
    error MsgValueLessThanExactAmount();

    /**
     * @dev Returns the address for the wrapped native token on Gnosis mainnet
     */
    function WETH() public pure returns (address) {
        return address(0xe91D153E0b41518A2Ce8Dd3D7944Fa863463a97d);
    }

    /**
     * @dev The native token for Gnosis itself is DAI so this method simply wraps
     * the native token and returns it to the caller.
     */
    function swapETHForExactTokens(
        uint amountOut,
        address[] calldata,
        address,
        uint
    ) external payable returns (uint[] memory) {
        if (amountOut > msg.value) revert MsgValueLessThanExactAmount();

        (bool sent, ) = WETH().call{value: msg.value}("");
        if (!sent) revert DepositFailed();

        IERC20(WETH()).safeTransfer(msg.sender, msg.value);

        uint256[] memory out = new uint256[](1);
        out[0] = msg.value;
        return out;
    }

    /**
     * @dev Returns the quoted amount for the dispatch.
     */
    function getAmountsIn(uint amountOut, address[] calldata) external pure returns (uint[] memory) {
        uint256[] memory out = new uint256[](1);
        out[0] = amountOut;
        return out;
    }
}
