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

import {TokenInfo} from "./IntentGatewayV2.sol";

/**
 * @title IIntentPriceOracle
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for tracking cumulative weighted average spreads for same-token swaps
 * @dev Only supports same-token swaps across chains
 */
interface IIntentPriceOracle {
    /**
     * @notice Event emitted when spread is recorded for a filled order
     * @param commitment The order commitment hash
     * @param destinationToken The destination token address
     * @param spreadBps The spread in basis points
     */
    event SpreadRecorded(bytes32 indexed commitment, address indexed destinationToken, int256 spreadBps);

    /**
     * @notice Event emitted when token decimals are updated
     * @param sourceChain The source chain state machine ID
     * @param token The token address
     * @param decimals The number of decimals
     */
    event TokenDecimalsUpdated(bytes sourceChain, address indexed token, uint8 decimals);

    /**
     * @notice Records the spread for a filled order and computes weighted average
     * @param commitment The order commitment hash
     * @param sourceChain The source chain identifier (bytes format)
     * @param inputs The input tokens that were escrowed
     * @param outputs The output tokens provided by the filler (actual amounts)
     */
    function recordSpread(
        bytes32 commitment,
        bytes memory sourceChain,
        TokenInfo[] calldata inputs,
        TokenInfo[] calldata outputs
    ) external;

    /**
     * @notice Gets the decimals for a token on a specific source chain
     * @param sourceChain The source chain identifier
     * @param token The token address
     * @return decimals The number of decimals (defaults to 18 if not set)
     */
    function decimals(bytes memory sourceChain, address token) external view returns (uint8 decimals);

    /**
     * @notice Gets the cumulative weighted average spread data for a token on a source chain
     * @param sourceChain The source chain identifier
     * @param token The token address
     * @return data The cumulative spread data
     */
    function spread(bytes memory sourceChain, address token) external view returns (int256 data);
}
