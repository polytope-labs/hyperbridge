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
import {TokenInfo, Order, Params, WithdrawalRequest, FillOptions} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
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
     * @dev Delivers output and releases the corresponding local escrow for each leg.
     * Partial fills reopen the order for another solver. A completing fill also executes
     * the beneficiary's calldata; `_fillLegs` rejects incomplete fills that carry calldata.
     * @param order The order being filled.
     * @param options The solver's output payments, required input quotes and fee parameters.
     * @param commitment The keccak256 hash of the ABI-encoded order.
     */
    function _fillSameChain(Order calldata order, FillOptions calldata options, bytes32 commitment) internal {
        _filled[commitment] = msg.sender;
        FillResult memory result = _fillLegs(order, options, commitment);
        WithdrawalRequest memory body = WithdrawalRequest({
            commitment: commitment, tokens: result.releasedInputs, beneficiary: bytes32(uint256(uint160(msg.sender)))
        });
        _withdraw(body, false, result.fullyFilled);

        if (result.fullyFilled) {
            _execute(order, order.output.assets.length);
            emit OrderFilled(commitment, msg.sender, result.creditedOutputs, result.releasedInputs);
        } else {
            delete _filled[commitment];
            emit PartialFill(commitment, msg.sender, result.creditedOutputs, result.releasedInputs);
        }

        if (result.nativeRemaining > 0) _sendValue(msg.sender, result.nativeRemaining);
    }

    /**
     * @dev Cancels a same-chain order and refunds the remaining escrowed tokens to the user.
     *
     * The original order creator (order.user) may cancel through the deadline. Cancellation is
     * permissionless strictly after the deadline, using `_blockNumber()` so Arbitrum deployments
     * use their L2 block number. Collects all remaining escrow balances (which may be reduced by
     * prior partial fills) and refunds them to the original user via `_withdraw`. `cancelOrder` is
     * the only caller and has already established that this chain is the order's source.
     *
     * `cancelOrder` has already emitted `OrderCancelled`; the `EscrowRefunded` of this refund
     * follows it in the same transaction.
     *
     * @param order The order to cancel.
     * @param commitment The keccak256 hash of the ABI-encoded order.
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

        WithdrawalRequest memory body =
            WithdrawalRequest({commitment: commitment, tokens: remainingTokens, beneficiary: order.user});

        _withdraw(body, true, true);
    }
}
