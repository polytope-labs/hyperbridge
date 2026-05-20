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

import {EvmHost, HostParams} from "./EvmHost.sol";

/**
 * @title The TestnetHost
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice An EvmHost variant for testnet deployments that re-introduces a
 * privileged admin path on `updateHostParams`. The base `EvmHost` accepts
 * host-params updates only via cross-chain governance (`hostManager`); on a
 * testnet that's impractical, so this contract additionally allows the
 * configured `admin` to update params and resets per-state-machine heights
 * for the supplied state machines.
 *
 * Uses the same `constructor(address admin)` signature as `EvmHost`, so
 * CREATE2 deployments under a shared salt produce a deterministic address
 * per host type (mainnet hosts share one address; testnet hosts share
 * another).
 */
contract TestnetHost is EvmHost {
    constructor(address _admin) EvmHost(_admin) {}

    /**
     * @dev Updates the HostParams. Callable by either `hostManager`
     * (cross-chain governance) or the configured `admin`. When invoked by
     * the admin, resets `_latestStateMachineHeight` for each state machine
     * in `params.stateMachines` before applying the new params, mirroring
     * the prior testnet behavior that lived inside `EvmHost`.
     */
    function updateHostParams(HostParams memory params) external override {
        address caller = _msgSender();
        if (caller != _hostParams.hostManager && caller != _hostParams.admin) {
            revert UnauthorizedAction();
        }

        if (caller == _hostParams.admin) {
            uint256 whitelistLength = params.stateMachines.length;
            for (uint256 i = 0; i < whitelistLength; ++i) {
                delete _latestStateMachineHeight[params.stateMachines[i]];
            }
        }

        updateHostParamsInternal(params);
    }

    /**
     * @dev Permit the admin to (re)initialize the consensus state at any
     * time, since testnets often need to be reset.
     */
    function _canReinitConsensus() internal pure override returns (bool) {
        return true;
    }
}
