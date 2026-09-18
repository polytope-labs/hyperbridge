// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import {Order, TokenInfo} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

/// @dev Derives input quotes from fixture output budgets, rounding takes down to respect
/// the order price. Full-input quotes preserve any output offered above the order amount.
/// Tests for other rates or non-exact integer ratios construct their quotes explicitly.
library IntentQuoteTestUtils {
    function inputs(Order memory order, TokenInfo[] memory outputs) internal pure returns (TokenInfo[] memory quotes) {
        quotes = new TokenInfo[](order.inputs.length);
        for (uint256 i; i < quotes.length; ++i) {
            quotes[i].token = order.inputs[i].token;
            if (i < outputs.length && order.output.assets[i].amount != 0) {
                quotes[i].amount = Math.mulDiv(
                    order.inputs[i].amount,
                    Math.min(outputs[i].amount, order.output.assets[i].amount),
                    order.output.assets[i].amount
                );
            }
        }
    }
}
