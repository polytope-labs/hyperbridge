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

import {MerkleMountainRange, MmrLeaf} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import {MerklePatricia, StorageValue} from "@polytope-labs/solidity-merkle-trees/src/MerklePatricia.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";

import {IConsensusClient, IntermediateState, StateMachineHeight, StateCommitment} from "@polytope-labs/ismp-solidity/IConsensusClient.sol";
import {IIsmpHost, FeeMetadata, FrozenStatus} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";
import {IHandler} from "@polytope-labs/ismp-solidity/IHandler.sol";
import {
    Message,
    PostResponse,
    PostRequest,
    GetRequest,
    GetResponse,
    PostRequestMessage,
    PostResponseMessage,
    GetResponseMessage,
    PostRequestTimeoutMessage,
    PostResponseTimeoutMessage,
    GetTimeoutMessage,
    PostRequestLeaf,
    PostResponseLeaf,
    GetResponseLeaf
} from "@polytope-labs/ismp-solidity/Message.sol";
import {Context} from "@openzeppelin/contracts/utils/Context.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

// Storage prefix for request receipts in the pallet-ismp child trie
bytes constant REQUEST_RECEIPTS_STORAGE_PREFIX = hex"526571756573745265636569707473";

// Storage prefix for response receipts in the pallet-ismp child trie
bytes constant RESPONSE_RECEIPTS_STORAGE_PREFIX = hex"526573706f6e73655265636569707473";

/**
 * @title The ISMP Message Handler.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice The Handler is responsible for verifying the cryptographic proofs needed
 * to confirm the validity of incoming requests/responses.
 * Refer to the official ISMP specification. https://docs.hyperbridge.network/protocol/ismp
 */
contract HandlerV1 is IHandler, ERC165, Context {
    using Bytes for bytes;
    using Message for PostResponse;
    using Message for PostRequest;
    using Message for GetRequest;
    using Message for GetResponse;

    // The cosensus client has now expired to mitigate
    // long fork attacks, this is unrecoverable.
    error ConsensusClientExpired();

    // The IsmpHost has been frozen by the admin
    error HostFrozen();

    // Challenge period has not yet elapsed
    error ChallengePeriodNotElapsed();

    // The requested state commitment does not exist
    error StateCommitmentNotFound();

    // The message destination is not intended for this host
    error InvalidMessageDestination();

    // The provided message has now timed-out
    error MessageTimedOut();

    // The provided message has not timed-out
    error MessageNotTimedOut();

    // The message has been previously processed
    error DuplicateMessage();

    // The provided message is unknown to the host
    error UnknownMessage();

    // The provided proof is invalid
    error InvalidProof();

    /**
     * @dev Checks if the host permits incoming datagrams
     */
    modifier notFrozen(IIsmpHost host) {
        FrozenStatus state = host.frozen();
        if (state == FrozenStatus.Incoming || state == FrozenStatus.All) revert HostFrozen();
        _;
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IHandler).interfaceId || super.supportsInterface(interfaceId);
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

        if (delay >= host.unStakingPeriod()) revert ConsensusClientExpired();

        (bytes memory verifiedState, IntermediateState[] memory intermediates) = IConsensusClient(
            host.consensusClient()
        ).verifyConsensus(host.consensusState(), proof);
        host.storeConsensusState(verifiedState);

        uint256 intermediatesLen = intermediates.length;
        for (uint256 i = 0; i < intermediatesLen; i++) {
            IntermediateState memory intermediate = intermediates[i];
            // check that we know this state machine and it's a new update
            uint256 latestHeight = host.latestStateMachineHeight(intermediate.stateMachineId);
            if (latestHeight != 0 && intermediate.height > latestHeight) {
                StateMachineHeight memory stateMachineHeight = StateMachineHeight({
                    stateMachineId: intermediate.stateMachineId,
                    height: intermediate.height
                });
                host.storeStateMachineCommitment(stateMachineHeight, intermediate.commitment);
            }
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
        if (challengePeriod != 0 && challengePeriod > delay) revert ChallengePeriodNotElapsed();

        uint256 requestsLen = request.requests.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](requestsLen);

        for (uint256 i = 0; i < requestsLen; ++i) {
            PostRequestLeaf memory leaf = request.requests[i];
            // check destination
            if (!leaf.request.dest.equals(host.host())) revert InvalidMessageDestination();
            // check time-out
            if (timestamp >= leaf.request.timeout()) revert MessageTimedOut();
            // duplicate request?
            bytes32 commitment = leaf.request.hash();
            if (host.requestReceipts(commitment) != address(0)) revert DuplicateMessage();

            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, commitment);
        }

        bytes32 root = host.stateMachineCommitment(request.proof.height).overlayRoot;
        if (root == bytes32(0)) revert StateCommitmentNotFound();
        bool valid = MerkleMountainRange.VerifyProof(root, request.proof.multiproof, leaves, request.proof.leafCount);
        if (!valid) revert InvalidProof();

        for (uint256 i = 0; i < requestsLen; ++i) {
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
        if (challengePeriod != 0 && challengePeriod > delay) revert ChallengePeriodNotElapsed();

        uint256 responsesLength = response.responses.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](responsesLength);

        for (uint256 i = 0; i < responsesLength; ++i) {
            PostResponseLeaf memory leaf = response.responses[i];
            // check time-out
            if (timestamp >= leaf.response.timeout()) revert MessageTimedOut();

            // known request? also serves as a source check
            bytes32 requestCommitment = leaf.response.request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            if (meta.sender == address(0)) revert InvalidProof();

            // duplicate response?
            if (host.responseReceipts(leaf.response.hash()).relayer != address(0)) revert DuplicateMessage();
            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, leaf.response.hash());
        }

        bytes32 root = host.stateMachineCommitment(response.proof.height).overlayRoot;
        if (root == bytes32(0)) revert StateCommitmentNotFound();
        bool valid = MerkleMountainRange.VerifyProof(root, response.proof.multiproof, leaves, response.proof.leafCount);
        if (!valid) revert InvalidProof();

        for (uint256 i = 0; i < responsesLength; ++i) {
            PostResponseLeaf memory leaf = response.responses[i];
            host.dispatchIncoming(leaf.response, _msgSender());
        }
    }

    /**
     * @dev check response proofs, message delay and timeouts, then dispatch get responses to modules
     * @param host - Ismp host
     * @param message - batch get responses
     */
    function handleGetResponses(IIsmpHost host, GetResponseMessage calldata message) external notFrozen(host) {
        uint256 timestamp = block.timestamp;
        uint256 delay = timestamp - host.stateMachineCommitmentUpdateTime(message.proof.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) revert ChallengePeriodNotElapsed();

        uint256 responsesLength = message.responses.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](responsesLength);

        for (uint256 i = 0; i < responsesLength; ++i) {
            GetResponseLeaf memory leaf = message.responses[i];
            // don't check for timeouts because it's checked on Hyperbridge

            // known request? also serves as source check
            bytes32 requestCommitment = leaf.response.request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            if (meta.sender == address(0)) revert UnknownMessage();

            // duplicate response?
            if (host.responseReceipts(requestCommitment).relayer != address(0)) revert DuplicateMessage();
            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, leaf.response.hash());
        }

        bytes32 root = host.stateMachineCommitment(message.proof.height).overlayRoot;
        if (root == bytes32(0)) revert StateCommitmentNotFound();
        bool valid = MerkleMountainRange.VerifyProof(root, message.proof.multiproof, leaves, message.proof.leafCount);
        if (!valid) revert InvalidProof();

        for (uint256 i = 0; i < responsesLength; ++i) {
            GetResponseLeaf memory leaf = message.responses[i];
            host.dispatchIncoming(leaf.response, _msgSender());
        }
    }

    /**
     * @dev Checks the provided timed-out requests and their proofs, before dispatching them to their relevant destination modules
     * @param host - IsmpHost
     * @param message - batch post request timeouts
     */
    function handlePostRequestTimeouts(
        IIsmpHost host,
        PostRequestTimeoutMessage calldata message
    ) external notFrozen(host) {
        uint256 delay = block.timestamp - host.stateMachineCommitmentUpdateTime(message.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) revert ChallengePeriodNotElapsed();

        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        if (state.stateRoot == bytes32(0)) revert StateCommitmentNotFound();
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; ++i) {
            PostRequest memory request = message.timeouts[i];
            // timed-out?
            if (request.timeout() > state.timestamp) revert MessageNotTimedOut();

            // known request? also serves as source check
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            if (meta.sender == address(0)) revert UnknownMessage();

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(REQUEST_RECEIPTS_STORAGE_PREFIX, bytes.concat(requestCommitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            if (entry.value.length != 0) revert InvalidProof();

            host.dispatchTimeOut(request, meta, requestCommitment);
        }
    }

    /**
     * @dev Check the provided timeouts and their proofs before dispatching them to their relevant modules
     * @param host - Ismp host
     * @param message - batch post response timeouts
     */
    function handlePostResponseTimeouts(
        IIsmpHost host,
        PostResponseTimeoutMessage calldata message
    ) external notFrozen(host) {
        uint256 delay = block.timestamp - host.stateMachineCommitmentUpdateTime(message.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) revert ChallengePeriodNotElapsed();

        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        if (state.stateRoot == bytes32(0)) revert StateCommitmentNotFound();
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; ++i) {
            PostResponse memory response = message.timeouts[i];
            // timed-out?
            if (response.timeout() > state.timestamp) revert MessageNotTimedOut();

            // known response? also serves as source check
            bytes32 responseCommitment = response.hash();
            FeeMetadata memory meta = host.responseCommitments(responseCommitment);
            if (meta.sender == address(0)) revert UnknownMessage();

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(RESPONSE_RECEIPTS_STORAGE_PREFIX, bytes.concat(responseCommitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            if (entry.value.length != 0) revert InvalidProof();

            host.dispatchTimeOut(response, meta, responseCommitment);
        }
    }

    /**
     * @dev Check the provided Get request timeouts, then dispatch to modules
     * @param host - Ismp host
     * @param message - batch get request timeouts
     */
    function handleGetRequestTimeouts(IIsmpHost host, GetTimeoutMessage calldata message) external notFrozen(host) {
        uint256 delay = block.timestamp - host.stateMachineCommitmentUpdateTime(message.height);
        uint256 challengePeriod = host.challengePeriod();
        if (challengePeriod != 0 && challengePeriod > delay) revert ChallengePeriodNotElapsed();

        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        if (state.stateRoot == bytes32(0)) revert StateCommitmentNotFound();
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; ++i) {
            GetRequest memory request = message.timeouts[i];
            // timed-out?
            if (request.timeout() > state.timestamp) revert MessageNotTimedOut();

            bytes32 commitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(commitment);
            if (meta.sender == address(0)) revert UnknownMessage();

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(REQUEST_RECEIPTS_STORAGE_PREFIX, bytes.concat(commitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            if (entry.value.length != 0) revert InvalidProof();

            host.dispatchTimeOut(request, meta, commitment);
        }
    }
}
