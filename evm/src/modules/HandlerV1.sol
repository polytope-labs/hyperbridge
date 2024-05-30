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

import {MerkleMountainRange, MmrLeaf} from "solidity-merkle-trees/MerkleMountainRange.sol";
import {MerklePatricia, StorageValue} from "solidity-merkle-trees/MerklePatricia.sol";
import {Context} from "openzeppelin/utils/Context.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {IConsensusClient, IntermediateState, StateMachineHeight, StateCommitment} from "ismp/IConsensusClient.sol";
import {IHandler} from "ismp/IHandler.sol";
import {IIsmpHost, PostResponse, PostRequest, GetRequest, GetResponse, FeeMetadata} from "ismp/IIsmpHost.sol";
import {
    Message,
    PostRequestMessage,
    PostResponseMessage,
    GetResponseMessage,
    PostRequestTimeoutMessage,
    PostResponseTimeoutMessage,
    GetTimeoutMessage,
    PostRequestLeaf,
    PostResponseLeaf
} from "ismp/Message.sol";

// Storage prefix for request receipts in the pallet-ismp child trie
bytes constant REQUEST_RECEIPTS_STORAGE_PREFIX = hex"526571756573745265636569707473";

// Storage prefix for request receipts in the pallet-ismp child trie
bytes constant RESPONSE_RECEIPTS_STORAGE_PREFIX = hex"526573706f6e73655265636569707473";

/// Entry point for the hyperbridge. Implementation of the ISMP handler protocol
contract HandlerV1 is IHandler, Context {
    using Bytes for bytes;
    using Message for PostResponse;
    using Message for PostRequest;
    using Message for GetRequest;
    //    using Message for GetResponse;

    modifier notFrozen(IIsmpHost host) {
        require(!host.frozen(), "IHandler: frozen");
        _;
    }

    /**
     * @dev Handle incoming consensus messages. These message are accompanied with some cryptographic proof.
     * If the Host's internal consensus client verifies this proof successfully,
     * The `StateCommitment` enters the preconfigured challenge period.
     * @param host - `IsmpHost`
     * @param proof - consensus proof
     */
    function handleConsensus(IIsmpHost host, bytes calldata proof) external notFrozen(host) {
        uint256 delay = block.timestamp - host.consensusUpdateTime();

        if (delay >= host.unStakingPeriod()) {
            revert("IHandler: consensus client is now expired");
        }

        (bytes memory verifiedState, IntermediateState memory intermediate) =
            IConsensusClient(host.consensusClient()).verifyConsensus(host.consensusState(), proof);
        host.storeConsensusState(verifiedState);

        // check that we know this state machine and it's a new update
        uint256 latestHeight = host.latestStateMachineHeight(intermediate.stateMachineId);
        if (latestHeight != 0 && intermediate.height > latestHeight) {
            StateMachineHeight memory stateMachineHeight =
                StateMachineHeight({stateMachineId: intermediate.stateMachineId, height: intermediate.height});
            host.storeStateMachineCommitment(stateMachineHeight, intermediate.commitment);
        }
    }

    /**
     * @dev Checks the provided requests and their proofs, before dispatching them to their relevant destination modules
     * @param host - `IsmpHost`
     * @param request - batch post requests
     */
    function handlePostRequests(IIsmpHost host, PostRequestMessage calldata request) external notFrozen(host) {
        uint256 timestamp = block.timestamp;
        uint256 delay = timestamp - host.stateMachineCommitmentUpdateTime(request.proof.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) {
            revert("IHandler: still in challenge period");
        }

        uint256 requestsLen = request.requests.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](requestsLen);

        for (uint256 i = 0; i < requestsLen; i++) {
            PostRequestLeaf memory leaf = request.requests[i];
            // check destination
            if (!leaf.request.dest.equals(host.host())) {
                revert("IHandler: Invalid request destination");
            }
            // check time-out
            if (timestamp > leaf.request.timeout()) {
                revert("IHandler: Request timed out");
            }
            // duplicate request?
            bytes32 commitment = leaf.request.hash();
            if (host.requestReceipts(commitment) != address(0)) {
                revert("IHandler: Duplicate request");
            }

            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, commitment);
        }

        bytes32 root = host.stateMachineCommitment(request.proof.height).overlayRoot;
        if (root == bytes32(0)) {
            revert("IHandler: Proof height not found!");
        }
        if (!MerkleMountainRange.VerifyProof(root, request.proof.multiproof, leaves, request.proof.leafCount)) {
            revert("IHandler: Invalid request proofs");
        }

        for (uint256 i = 0; i < requestsLen; i++) {
            PostRequestLeaf memory leaf = request.requests[i];
            host.dispatchIncoming(leaf.request, _msgSender());
        }
    }

    /**
     * @dev Checks the provided responses and their proofs, before dispatching them to their relevant destination modules
     * @param host - `IsmpHost`
     * @param response - batch post responses
     */
    function handlePostResponses(IIsmpHost host, PostResponseMessage calldata response) external notFrozen(host) {
        uint256 timestamp = block.timestamp;
        uint256 delay = timestamp - host.stateMachineCommitmentUpdateTime(response.proof.height);
        uint256 challengePeriod = host.challengePeriod();

        if (challengePeriod != 0 && challengePeriod > delay) {
            revert("IHandler: still in challenge period");
        }

        uint256 responsesLength = response.responses.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](responsesLength);

        for (uint256 i = 0; i < responsesLength; i++) {
            PostResponseLeaf memory leaf = response.responses[i];
            // check time-out
            if (timestamp > leaf.response.timeout()) {
                revert("IHandler: Response timed out");
            }
            // known request? also serves as a source check
            bytes32 requestCommitment = leaf.response.request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            if (meta.sender == address(0)) {
                revert("IHandler: Unknown request");
            }

            // duplicate response?
            if (host.responseReceipts(leaf.response.hash()).relayer != address(0)) {
                revert("IHandler: Duplicate Post response");
            }
            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, leaf.response.hash());
        }

        bytes32 root = host.stateMachineCommitment(response.proof.height).overlayRoot;
        if (root == bytes32(0)) {
            revert("IHandler: Proof height not found!");
        }
        if (!MerkleMountainRange.VerifyProof(root, response.proof.multiproof, leaves, response.proof.leafCount)) {
            revert("IHandler: Invalid response proofs");
        }

        for (uint256 i = 0; i < responsesLength; i++) {
            PostResponseLeaf memory leaf = response.responses[i];
            host.dispatchIncoming(leaf.response, _msgSender());
        }
    }

    /**
     * @dev Checks the provided timed-out requests and their proofs, before dispatching them to their relevant destination modules
     * @param host - IsmpHost
     * @param message - batch post request timeouts
     */
    function handlePostRequestTimeouts(IIsmpHost host, PostRequestTimeoutMessage calldata message)
        external
        notFrozen(host)
    {
        uint256 delay = block.timestamp - host.stateMachineCommitmentUpdateTime(message.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) {
            revert("IHandler: still in challenge period");
        }

        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        if (state.stateRoot == bytes32(0)) {
            revert("IHandler: State Commitment doesn't exist");
        }
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            PostRequest memory request = message.timeouts[i];
            // timed-out?
            if (request.timeout() > state.timestamp) {
                revert("Request not timed out");
            }

            // known request? also serves as source check
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown request");

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(REQUEST_RECEIPTS_STORAGE_PREFIX, bytes.concat(requestCommitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            if (entry.value.length != 0) {
                revert("IHandler: Invalid non-membership proof");
            }

            host.dispatchIncoming(request, meta, requestCommitment);
        }
    }

    /**
     * @dev Check the provided timeouts and their proofs before dispatching them to their relevant modules
     * @param host - Ismp host
     * @param message - batch post response timeouts
     */
    function handlePostResponseTimeouts(IIsmpHost host, PostResponseTimeoutMessage calldata message)
        external
        notFrozen(host)
    {
        uint256 delay = block.timestamp - host.stateMachineCommitmentUpdateTime(message.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) {
            revert("IHandler: still in challenge period");
        }

        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        if (state.stateRoot == bytes32(0)) {
            revert("IHandler: State Commitment doesn't exist");
        }
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            PostResponse memory response = message.timeouts[i];
            // timed-out?
            if (response.timeout() > state.timestamp) {
                revert("IHandler: Response not timed out");
            }

            // known response? also serves as source check
            bytes32 responseCommitment = response.hash();
            FeeMetadata memory meta = host.responseCommitments(responseCommitment);
            if (meta.sender == address(0)) {
                revert("IHandler: Unknown response");
            }

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(RESPONSE_RECEIPTS_STORAGE_PREFIX, bytes.concat(responseCommitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            if (entry.value.length != 0) {
                revert("IHandler: Invalid non-membership proof");
            }

            host.dispatchIncoming(response, meta, responseCommitment);
        }
    }

    /**
     * @dev check response proofs, message delay and timeouts, then dispatch get responses to modules
     * @param host - Ismp host
     * @param message - batch get responses
     */
    function handleGetResponses(IIsmpHost host, GetResponseMessage calldata message) external notFrozen(host) {
        uint256 timestamp = block.timestamp;
        uint256 delay = timestamp - host.stateMachineCommitmentUpdateTime(message.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) {
            revert("IHandler: still in challenge period");
        }

        bytes32 root = host.stateMachineCommitment(message.height).stateRoot;
        if (root == bytes32(0)) {
            revert("IHandler: Proof height not found!");
        }

        uint256 responsesLength = message.requests.length;
        bytes[] memory proof = message.proof;

        for (uint256 i = 0; i < responsesLength; i++) {
            GetRequest memory request = message.requests[i];
            // timed-out?
            if (timestamp > request.timeout()) {
                revert("IHandler: GET request timed out");
            }

            // known request? also serves as source check
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            if (meta.sender == address(0)) {
                revert("IHandler: Unknown GET request");
            }

            // duplicate response?
            if (host.responseReceipts(requestCommitment).relayer != address(0)) {
                revert("IHandler: Duplicate GET response");
            }
            StorageValue[] memory values =
                MerklePatricia.ReadChildProofCheck(root, proof, request.keys, bytes.concat(requestCommitment));
            GetResponse memory response = GetResponse({request: request, values: values});

            host.dispatchIncoming(response, _msgSender());
        }
    }

    /**
     * @dev Check the provided Get request timeouts, then dispatch to modules
     * @param host - Ismp host
     * @param message - batch get request timeouts
     */
    function handleGetRequestTimeouts(IIsmpHost host, GetTimeoutMessage calldata message) external notFrozen(host) {
        uint256 timeoutsLength = message.timeouts.length;
        uint256 timestamp = block.timestamp;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            GetRequest memory request = message.timeouts[i];
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            if (meta.sender == address(0)) {
                revert("IHandler: Unknown request");
            }

            if (request.timeout() > timestamp) {
                revert("IHandler: GET request not timed out");
            }
            host.dispatchIncoming(request, meta, requestCommitment);
        }
    }
}
