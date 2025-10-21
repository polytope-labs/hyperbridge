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

/**
 * @title StateCommitment
 * @notice Represents a commitment to an intermediate state in the state machine
 * @dev Contains metadata about the state machine including timestamp and merkle roots
 */
struct StateCommitment {
    /// @notice Unix timestamp of the state machine at the time of this commitment
    /// @dev Used for calculating request timeouts and enforcing time-based logic
    uint256 timestamp;
    /// @notice Merkle root of the overlay trie containing all ISMP requests and responses
    /// @dev Used to verify inclusion proofs for cross-chain messages
    bytes32 overlayRoot;
    /// @notice Merkle root of the state trie at the given block height
    /// @dev Represents the complete state of the state machine at this height
    bytes32 stateRoot;
}

/**
 * @title StateMachineHeight
 * @notice Uniquely identifies a specific height in a state machine
 * @dev Consensus clients may track multiple concurrent state machines, hence the need for an identifier
 */
struct StateMachineHeight {
    /// @notice Unique identifier for the state machine (e.g., parachain ID, chain ID)
    /// @dev Each blockchain or parachain in the network has a unique identifier
    uint256 stateMachineId;
    /// @notice Block height or number in the state machine
    /// @dev Represents the sequential position in the blockchain
    uint256 height;
}

/**
 * @title IntermediateState
 * @notice Represents an intermediate state in the state transition sequence of a state machine
 * @dev Used to track finalized states that have been verified through consensus
 */
struct IntermediateState {
    /// @notice Unique identifier for the state machine
    /// @dev Same as StateMachineHeight.stateMachineId
    uint256 stateMachineId;
    /// @notice Block height of this intermediate state
    /// @dev The specific height at which this state was committed
    uint256 height;
    /// @notice The state commitment at this height
    /// @dev Contains the timestamp and merkle roots for this state
    StateCommitment commitment;
}

/**
 * @title IConsensus
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Interface for consensus verification in the Hyperbridge protocol
 * @dev Consensus clients implement this interface to verify state transitions from different chains.
 * The internals are intentionally opaque to the ISMP framework, allowing consensus mechanisms
 * to evolve independently (e.g., GRANDPA for Polkadot, Sync Committee for Ethereum).
 * Different consensus mechanisms can be plugged in as long as they conform to this interface.
 */
interface IConsensus {
    /**
     * @notice Verifies a consensus proof and returns the updated consensus state
     * @dev This function is called by the Handler to verify incoming consensus updates.
     * The implementation details vary based on the consensus mechanism being verified.
     * @param trustedState The current trusted consensus state (encoded based on consensus type)
     * @param proof The consensus proof to be verified (e.g., validator signatures, merkle proofs)
     * @return The new consensus state after verification (encoded)
     * @return Array of newly finalized intermediate states that can be trusted
     */
    function verifyConsensus(bytes memory trustedState, bytes memory proof)
        external
        returns (bytes memory, IntermediateState[] memory);
}
