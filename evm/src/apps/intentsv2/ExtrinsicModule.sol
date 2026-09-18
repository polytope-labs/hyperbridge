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

import {ExtrinsicIntents} from "./ExtrinsicIntents.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {Order, FillOptions, CancelOptions} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title ExtrinsicModule
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Delegatecall module for cross-chain fills and cancels, and for the `onAccept` and
 * `onGetResponse` handlers inherited from `ExtrinsicIntents`. `IntentGatewayV2` delegatecalls
 * here, so this runs in the proxy's storage with `msg.sender` and `msg.value` intact. Shares the
 * gateway's storage layout and declares no storage of its own.
 */
contract ExtrinsicModule is ExtrinsicIntents {
    constructor() EIP712("IntentGateway", "2") {}

    /**
     * @dev Rejects calls made directly to the module.
     */
    modifier onlyDelegated() {
        if (address(this) == __self) revert Unauthorized();
        _;
    }

    /**
     * @dev Cross-chain fill on the destination chain, validated by `IntentGatewayV2.fillOrder`.
     * @param order The order to fill.
     * @param options The solver's per-leg quotes and dispatch fees.
     * @param commitment The order commitment hash.
     */
    function fillCrossChain(Order calldata order, FillOptions calldata options, bytes32 commitment)
        external
        payable
        onlyDelegated
    {
        _fillCrossChain(order, options, commitment);
    }

    /**
     * @dev Cancel from the source chain, validated by `IntentGatewayV2.cancelOrder`.
     * @param order The order to cancel.
     * @param options The proof height and relayer fee.
     * @param commitment The order commitment hash.
     */
    function cancelFromSource(Order calldata order, CancelOptions calldata options, bytes32 commitment)
        external
        payable
        onlyDelegated
    {
        _cancelFromSource(order, options, commitment);
    }

    /**
     * @dev Cancel from the destination chain, validated by `IntentGatewayV2.cancelOrder`.
     * @param order The order to cancel.
     * @param options The relayer fee.
     * @param commitment The order commitment hash.
     */
    function cancelFromDest(Order calldata order, CancelOptions calldata options, bytes32 commitment)
        external
        payable
        onlyDelegated
    {
        _cancelFromDest(order, options, commitment);
    }
}
