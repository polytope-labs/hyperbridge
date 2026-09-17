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

import {IntrinsicIntents} from "./IntrinsicIntents.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {Order, FillOptions, TokenInfo} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title IntrinsicModule
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Delegatecall module for same-chain fills and cancels. `IntentGatewayV2` validates the
 * call and delegatecalls here, so this runs in the proxy's storage with `msg.sender` and
 * `msg.value` intact. Shares the gateway's storage layout and declares no storage of its own.
 */
contract IntrinsicModule is IntrinsicIntents {
    constructor() EIP712("IntentGateway", "2") {}

    /**
     * @dev Rejects calls made directly to the module.
     */
    modifier onlyDelegated() {
        if (address(this) == __self) revert Unauthorized();
        _;
    }

    /**
     * @dev Same-chain fill, validated by `IntentGatewayV2.fillOrder`.
     * @param order The order to fill.
     * @param options The solver's output amounts.
     * @param commitment The order commitment hash.
     */
    function fillSameChain(
        Order calldata order,
        FillOptions calldata options,
        bytes32 commitment,
        TokenInfo[] calldata inputs
    ) external payable onlyDelegated {
        _fillSameChain(order, options, commitment, inputs);
    }

    /**
     * @dev Same-chain cancel, validated by `IntentGatewayV2.cancelOrder`.
     * @param order The order to cancel.
     * @param commitment The order commitment hash.
     */
    function cancelSameChain(Order calldata order, bytes32 commitment) external payable onlyDelegated {
        _cancelSameChain(order, commitment);
    }
}
