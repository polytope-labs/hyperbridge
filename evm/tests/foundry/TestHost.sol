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

import {EvmHost, HostParams} from "../../src/core/EvmHost.sol";
import {TestnetHost} from "../../src/core/TestnetHost.sol";

/// @dev Test wrapper preserving the legacy single-shot `(HostParams)`
/// constructor used by the existing Foundry tests. Extends `TestnetHost`
/// so that tests can call `updateHostParams` from the admin and re-init
/// the consensus state. For mainnet-behavior tests, use `MainnetTestHost`.
contract TestHost is TestnetHost {
    constructor(HostParams memory params) TestnetHost(params.admin) {
        updateHostParamsInternal(params);
    }
}

/// @dev Test wrapper around the strict-mainnet `EvmHost`, used by tests
/// that need to assert mainnet behavior (no admin updateHostParams,
/// consensus state only initializable once).
contract MainnetTestHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params.admin) {
        updateHostParamsInternal(params);
    }
}
