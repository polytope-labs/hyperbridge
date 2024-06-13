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
pragma solidity 0.8.17;

import {ICallDispatcher, CallDispatcherParams} from "../interfaces/ICallDispatcher.sol";

/// @notice This contract is used to dispatch calls to other contracts.
contract CallDispatcher is ICallDispatcher {
    /// @dev funtion returns `success = false` if the target is not a contract and reverts if the call to the target contract fails.
    function dispatch(CallDispatcherParams memory params) external returns (bytes memory result, bool success) {
        uint32 size;
        address target = params.target;
        assembly {
            size := extcodesize(target)
        }

        if (size > 0) {
            (success, result) = target.call(params.data);
            if (!success) revert(string(result));
            return (result, success);
        }
    }
}
