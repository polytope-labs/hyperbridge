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

import {IHandler} from "./IHandler.sol";

/**
 * @title IHandlerV2
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Extended handler interface that supports batching multiple handler calls into a single transaction.
 * @dev Relayers can ABI-encode individual handler calls (handleConsensus, handlePostRequests, etc.)
 * and submit them as a single batchCall, reducing gas overhead and simplifying relayer logic.
 */
interface IHandlerV2 is IHandler {
    /**
     * @dev Process a batch of encoded handler calls in a single transaction.
     * Each element in `calls` is an ABI-encoded call to one of the handler functions.
     * The handler decodes and executes them sequentially. If any call fails, the entire batch reverts.
     * @param calls Array of ABI-encoded function calls
     */
    function batchCall(bytes[] memory calls) external;
}
