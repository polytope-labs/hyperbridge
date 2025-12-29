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

import {TokenInfo} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {IDispatcher, PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IIntentPriceOracle} from "@hyperbridge/core/apps/IntentPriceOracle.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";

/**
 * @title VWAPOracle
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Gas-efficient oracle for tracking cumulative VWAP spreads for same-token swaps
 * @dev Only supports same-token swaps
 * @dev Tracks spreads per (source chain, token address)
 */
contract VWAPOracle is IIntentPriceOracle, HyperApp {
    /**
     * @dev Enum representing the different kinds of incoming requests
     */
    enum RequestKind {
        /// @dev Identifies a request for updating token decimals
        UpdateTokenDecimals
    }

    /**
     * @dev Struct for a single token decimal configuration
     */
    struct TokenDecimal {
        /// @dev Token address
        address token;
        /// @dev Number of decimals
        uint8 decimals;
    }

    /**
     * @dev Struct for batch updating token decimals via governance
     */
    struct TokenDecimalsUpdate {
        /// @dev The source chain identifier
        bytes sourceChain;
        /// @dev Array of token decimal configurations
        TokenDecimal[] tokens;
    }

    /**
     * @dev Basis points denominator
     */
    uint256 private constant BPS_DENOMINATOR = 10_000;

    /**
     * @dev Address of the IsmpHost contract
     */
    address private _host;

    /**
     * @dev Address that can update decimals (only for initialization)
     */
    address private _admin;

    /**
     * @dev Mapping from (sourceChainHash, token) => decimals for remote source chain tokens
     * @dev Destination chain token decimals are read directly from IERC20Metadata.decimals()
     */
    mapping(bytes32 => mapping(address => uint8)) private _tokenDecimals;

    /**
     * @dev Mapping from (sourceChainHash, token) => cumulative spread data
     */
    mapping(bytes32 => mapping(address => CumulativeSpreadData)) private _tokenSpreads;

    /// @notice Thrown when an unauthorized action is attempted
    error Unauthorized();

    /// @notice Thrown when invalid input is provided
    error InvalidInput();

    constructor(address admin) {
        _admin = admin;
    }

    /**
     * @inheritdoc HyperApp
     */
    function host() public view override returns (address) {
        return _host;
    }

    /**
     * @inheritdoc IIntentPriceOracle
     */
    function decimals(bytes memory sourceChain, address token) external view returns (uint8) {
        bytes32 chainHash = keccak256(sourceChain);
        return _tokenDecimals[chainHash][token];
    }

    /**
     * @inheritdoc IIntentPriceOracle
     */
    function spread(bytes memory sourceChain, address token) external view returns (CumulativeSpreadData memory data) {
        bytes32 chainHash = keccak256(sourceChain);
        return _tokenSpreads[chainHash][token];
    }

    /**
     * @notice Initializes the oracle with host and initial token decimals
     * @param hostAddr The IsmpHost contract address
     * @param updates Array of token decimal configurations for different chains
     * @dev Can only be called once by admin, then admin is reset to address(0)
     */
    function init(address hostAddr, TokenDecimalsUpdate[] memory updates) external {
        if (msg.sender != _admin) revert Unauthorized();

        _host = hostAddr;

        // Process all token decimal updates
        _processTokenDecimalsUpdates(updates);

        // Reset admin after initialization
        _admin = address(0);
    }

    /**
     * @inheritdoc IIntentPriceOracle
     */
    function recordSpread(
        bytes32 commitment,
        bytes memory sourceChain,
        TokenInfo[] calldata inputs,
        TokenInfo[] calldata outputs
    ) external {
        // Validate inputs and outputs have the same length
        if (inputs.length != outputs.length || inputs.length == 0) {
            return;
        }

        bytes32 sourceChainHash = keccak256(sourceChain);
        uint256 tokensLen = inputs.length;
        for (uint256 i = 0; i < tokensLen; i++) {
            address inputToken = address(uint160(uint256(inputs[i].token)));
            address outputToken = address(uint160(uint256(outputs[i].token)));

            // Get decimals for input token from storage (remote chain)
            // Native tokens (address(0)) use 18 decimals
            uint8 inputDecimals = inputToken == address(0) ? 18 : _tokenDecimals[sourceChainHash][inputToken];
            if (inputDecimals == 0) continue; // Skip if decimals not configured

            // Get decimals for output token directly from contract (local chain)
            // Native tokens (address(0)) use 18 decimals
            uint8 outputDecimals = outputToken == address(0) ? 18 : IERC20Metadata(outputToken).decimals();

            // Normalize both amounts to 18 decimals for comparison
            uint256 inputAmountNormalized = _normalizeAmount(inputs[i].amount, inputDecimals);
            uint256 outputAmountNormalized = _normalizeAmount(outputs[i].amount, outputDecimals);

            // Calculate spread for this token: (output - input) / input * 10000
            // Positive spread = filler provided more tokens (good for user)
            // Negative spread = filler provided fewer tokens (filler captured spread)
            int256 spreadBps = 0;
            if (inputAmountNormalized > 0) {
                int256 amountDiff = int256(outputAmountNormalized) - int256(inputAmountNormalized);
                spreadBps = (amountDiff * int256(BPS_DENOMINATOR)) / int256(inputAmountNormalized);
            }

            // Update cumulative spread data for this token (weighted by volume)
            int256 weightedSpread = spreadBps * int256(inputAmountNormalized);
            _updateCumulativeSpread(_tokenSpreads[sourceChainHash][inputToken], weightedSpread, inputAmountNormalized);

            // Emit event for each token
            emit SpreadRecorded(commitment, outputToken, spreadBps);
        }
    }

    /**
     * @notice Handles incoming cross-chain governance requests
     * @param incoming The incoming post request
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        // Only hyperbridge is permitted to perform these actions
        if (keccak256(incoming.request.source) != keccak256(IDispatcher(host()).hyperbridge())) revert Unauthorized();

        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));

        if (kind == RequestKind.UpdateTokenDecimals) {
            TokenDecimalsUpdate[] memory updates = abi.decode(incoming.request.body[1:], (TokenDecimalsUpdate[]));
            _processTokenDecimalsUpdates(updates);
        }
    }

    /**
     * @notice Normalizes token amount to 18 decimals
     * @param amount The amount to normalize
     * @param _decimals The current decimals of the token
     * @return normalized The normalized amount (18 decimals)
     */
    function _normalizeAmount(uint256 amount, uint8 _decimals) private pure returns (uint256 normalized) {
        if (_decimals == 18) {
            return amount;
        } else if (_decimals < 18) {
            return amount * (10 ** (18 - _decimals));
        } else {
            return amount / (10 ** (_decimals - 18));
        }
    }

    /**
     * @notice Processes token decimals updates for multiple chains
     * @param updates Array of token decimal configurations for different chains
     */
    function _processTokenDecimalsUpdates(TokenDecimalsUpdate[] memory updates) internal {
        uint256 updatesLen = updates.length;
        for (uint256 j = 0; j < updatesLen; j++) {
            bytes32 chainHash = keccak256(updates[j].sourceChain);
            uint256 tokensLen = updates[j].tokens.length;
            for (uint256 i = 0; i < tokensLen; i++) {
                if (updates[j].tokens[i].decimals == 0) revert InvalidInput();

                _tokenDecimals[chainHash][updates[j].tokens[i].token] = updates[j].tokens[i].decimals;
                emit TokenDecimalsUpdated(
                    updates[j].sourceChain, updates[j].tokens[i].token, updates[j].tokens[i].decimals
                );
            }
        }
    }

    /**
     * @notice Updates cumulative spread data
     * @param data Storage reference to the cumulative spread data
     * @param weightedSpread The weighted spread (spread * volume)
     * @param volume The volume for this fill
     */
    function _updateCumulativeSpread(CumulativeSpreadData storage data, int256 weightedSpread, uint256 volume) private {
        data.weightedSpreadSum += weightedSpread;
        data.totalVolume += volume;
        data.fillCount += 1;
        data.lastUpdate = block.timestamp;
    }
}
