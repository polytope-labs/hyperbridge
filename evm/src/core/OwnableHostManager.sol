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

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {HostParams, IHostManager} from "./EvmHost.sol";
import {HostManager, HostManagerParams} from "./HostManager.sol";

/**
 * @title The Ownable ISMP HostManager.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice A HostManager whose owner can update the host params without a governance request.
 * The owner is set separately from `HostManagerParams.admin`, the relayer that delivers
 * governance. Cross-chain governance from Hyperbridge is inherited and works as before.
 */
contract OwnableHostManager is HostManager, Ownable {
    constructor(address admin, HostManagerParams memory managerParams) HostManager(managerParams) Ownable(admin) {}

    /**
     * @dev Replaces the host params without a governance request from Hyperbridge.
     */
    function updateHostParams(HostParams memory params) external onlyOwner {
        IHostManager(host()).updateHostParams(params);
    }
}
