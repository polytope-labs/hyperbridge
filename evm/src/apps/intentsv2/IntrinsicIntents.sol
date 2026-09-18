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
pragma solidity ^0.8.24;

import {IntentsBase} from "./IntentsBase.sol";
import {TokenInfo, Order, Params, FillOptions} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title IntrinsicIntents
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Same-chain intent logic: partial fills, same-chain cancel, and escrow release.
 */
abstract contract IntrinsicIntents is IntentsBase {
    using SafeERC20 for IERC20;

    /**
     * @dev Pays each leg and releases the escrow it earns, all on this chain.
     */
    function _fillOrder(Order calldata order, FillOptions calldata options, bytes32 commitment)
        internal
        returns (FillResult memory result)
    {
        result = _fillLegs(order, options, commitment);
        _withdraw(
            Withdrawal({
                commitment: commitment,
                beneficiary: bytes32(uint256(uint160(msg.sender))),
                tokens: result.releasedInputs,
                isRefund: false,
                finalize: result.fullyFilled
            })
        );
    }

    /**
     * @dev Refunds the remaining escrow to the creator. Only the creator may cancel until the
     * deadline.
     */
    function _cancelSameChain(Order calldata order, bytes32 commitment) internal {
        if (order.user != bytes32(uint256(uint160(msg.sender))) && _blockNumber() <= order.deadline) {
            revert Unauthorized();
        }

        uint256 inputsLen = order.inputs.length;
        TokenInfo[] memory remainingTokens = new TokenInfo[](inputsLen);
        bool hasEscrow = false;
        for (uint256 i; i < inputsLen;) {
            uint256 escrowed = _orders[commitment][i];
            if (escrowed > 0) hasEscrow = true;
            remainingTokens[i] = TokenInfo({token: order.inputs[i].token, amount: escrowed});
            unchecked {
                ++i;
            }
        }
        if (!hasEscrow) revert UnknownOrder();

        _withdraw(
            Withdrawal({
                commitment: commitment,
                beneficiary: order.user,
                tokens: remainingTokens,
                isRefund: true,
                finalize: true
            })
        );
    }
}
