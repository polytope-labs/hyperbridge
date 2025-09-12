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

import {StateMachineHeight} from "./Message.sol";

// The state commiment identifies a commiment to some intermediate state in the state machine.
// This contains some metadata about the state machine like it's own timestamp at the time of this commitment.
struct StateCommitment {
	// This timestamp is useful for handling request timeouts.
	uint256 timestamp;
	// Overlay trie commitment to all ismp requests & response.
	bytes32 overlayRoot;
	// State trie commitment at the given block height
	bytes32 stateRoot;
}

// An intermediate state in the series of state transitions undergone by a given state machine.
struct IntermediateState {
	// the state machine identifier
	uint256 stateMachineId;
	// height of this state machine
	uint256 height;
	// state commitment
	StateCommitment commitment;
}

/**
 * @title The Ismp ConsensusClient
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice The consensus client interface responsible for the verification of consensus datagrams.
 * It's internals are opaque to the ISMP framework allowing it to evolve as needed.
 */
interface IConsensusClient {
	// @dev Given some opaque consensus proof, produce the new consensus state and newly finalized intermediate states.
	function verifyConsensus(
		bytes memory trustedState,
		bytes memory proof
	) external returns (bytes memory, IntermediateState[] memory);
}
