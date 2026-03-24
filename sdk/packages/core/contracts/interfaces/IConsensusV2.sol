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

import {IntermediateState} from "./IConsensus.sol";

/**
 * @title IConsensusV2
 * @author Polytope Labs (hello@polytope.technology)
 * @notice V2 consensus verification interface that additionally returns epoch/authority-set information.
 * @dev Consensus clients implement this interface to verify state transitions and report authority set changes.
 */
interface IConsensusV2 {
    /**
     * @dev Verifies a consensus proof and returns the updated state, newly finalized intermediate states,
     * and the new authority set ID if an epoch transition occurred.
     * @param previousState The current trusted consensus state (encoded based on consensus type)
     * @param proof The consensus proof to be verified
     * @return The new consensus state after verification (encoded)
     * @return Array of newly finalized intermediate states that can be trusted
     * @return The new authority set ID, or 0 if no epoch change occurred
     */
    function verify(bytes memory previousState, bytes memory proof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256);
}
