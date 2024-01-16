// SPDX-License-Identifier: UNLICENSED
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
} from "ismp/IIsmp.sol";

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

    // Storage prefix for request receipts in pallet-ismp
    bytes private constant REQUEST_RECEIPTS_STORAGE_PREFIX =
        hex"103895530afb23bb607661426d55eb8b0484aecefe882c3ce64e6f82507f715a";

    // Storage prefix for request receipts in pallet-ismp
    bytes private constant RESPONSE_RECEIPTS_STORAGE_PREFIX =
        hex"103895530afb23bb607661426d55eb8b554b72b7162725f9457d35ecafb8b02f";

    event StateMachineUpdated(uint256 stateMachineId, uint256 height);

    /**
     * @dev Handle incoming consensus messages
     * @param host - Ismp host
     * @param proof - consensus proof
     */
    function handleConsensus(IIsmpHost host, bytes memory proof) external notFrozen(host) {
        uint256 delay = host.timestamp() - host.consensusUpdateTime();
        require(delay > host.challengePeriod(), "IHandler: still in challenge period");

        // not today, time traveling validators
        require(delay < host.unStakingPeriod() || _msgSender() == host.admin(), "IHandler: still in challenge period");

        (bytes memory verifiedState, IntermediateState memory intermediate) =
            IConsensusClient(host.consensusClient()).verifyConsensus(host.consensusState(), proof);
        host.storeConsensusState(verifiedState);
        host.storeConsensusUpdateTime(host.timestamp());

        if (intermediate.height > host.latestStateMachineHeight()) {
            StateMachineHeight memory stateMachineHeight =
                StateMachineHeight({stateMachineId: intermediate.stateMachineId, height: intermediate.height});
            host.storeStateMachineCommitment(stateMachineHeight, intermediate.commitment);
            host.storeStateMachineCommitmentUpdateTime(stateMachineHeight, host.timestamp());
            host.storeLatestStateMachineHeight(stateMachineHeight.height);

            emit StateMachineUpdated({
                stateMachineId: stateMachineHeight.stateMachineId,
                height: stateMachineHeight.height
            });
        }
    }

    /**
     * @dev check request proofs, message delay and timeouts, then dispatch post requests to modules
     * @param host - Ismp host
     * @param request - batch post requests
     */
    function handlePostRequests(IIsmpHost host, PostRequestMessage memory request) external notFrozen(host) {
        uint256 delay = host.timestamp() - host.stateMachineCommitmentUpdateTime(request.proof.height);
        require(delay > host.challengePeriod(), "IHandler: still in challenge period");

        uint256 requestsLen = request.requests.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](requestsLen);

        for (uint256 i = 0; i < requestsLen; i++) {
            PostRequestLeaf memory leaf = request.requests[i];
            // check destination
            require(leaf.request.dest.equals(host.host()), "IHandler: Invalid request destination");
            // check time-out
            require(leaf.request.timeout() > host.timestamp(), "IHandler: Request timed out");
            // duplicate request?
            bytes32 commitment = leaf.request.hash();
            require(!host.requestReceipts(commitment), "IHandler: Duplicate request");

            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, commitment);
        }

        bytes32 root = host.stateMachineCommitment(request.proof.height).overlayRoot;
        require(root != bytes32(0), "IHandler: Proof height not found!");
        require(
            MerkleMountainRange.VerifyProof(root, request.proof.multiproof, leaves, request.proof.leafCount),
            "IHandler: Invalid request proofs"
        );

        for (uint256 i = 0; i < requestsLen; i++) {
            PostRequestLeaf memory leaf = request.requests[i];
            host.dispatchIncoming(leaf.request);
        }
    }

    /**
     * @dev check response proofs, message delay and timeouts, then dispatch post responses to modules
     * @param host - Ismp host
     * @param response - batch post responses
     */
    function handlePostResponses(IIsmpHost host, PostResponseMessage memory response) external notFrozen(host) {
        uint256 delay = host.timestamp() - host.stateMachineCommitmentUpdateTime(response.proof.height);
        require(delay > host.challengePeriod(), "IHandler: still in challenge period");

        uint256 responsesLength = response.responses.length;
        MmrLeaf[] memory leaves = new MmrLeaf[](responsesLength);

        for (uint256 i = 0; i < responsesLength; i++) {
            PostResponseLeaf memory leaf = response.responses[i];
            // check time-out
            require(leaf.response.timeout() > host.timestamp(), "IHandler: Response timed out");
            // known request? also serves as a source check
            bytes32 requestCommitment = leaf.response.request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown request");

            // duplicate response?
            require(!host.responseReceipts(requestCommitment), "IHandler: Duplicate Post response");
            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, leaf.response.hash());
        }

        bytes32 root = host.stateMachineCommitment(response.proof.height).overlayRoot;
        require(root != bytes32(0), "IHandler: Proof height not found!");
        require(
            MerkleMountainRange.VerifyProof(root, response.proof.multiproof, leaves, response.proof.leafCount),
            "IHandler: Invalid response proofs"
        );

        for (uint256 i = 0; i < responsesLength; i++) {
            PostResponseLeaf memory leaf = response.responses[i];
            host.dispatchIncoming(leaf.response);
        }
    }

    /**
     * @dev check timeout proofs then dispatch to modules
     * @param host - Ismp host
     * @param message - batch post request timeouts
     */
    function handlePostRequestTimeouts(IIsmpHost host, PostRequestTimeoutMessage memory message)
        external
        notFrozen(host)
    {
        uint256 delay = host.timestamp() - host.stateMachineCommitmentUpdateTime(message.height);
        require(delay > host.challengePeriod(), "IHandler: still in challenge period");

        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        require(state.stateRoot != bytes32(0), "IHandler: State Commitment doesn't exist");
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            PostRequest memory request = message.timeouts[i];
            // timed-out?
            require(state.timestamp > request.timeout(), "Request not timed out");

            // known request? also serves as source check
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown request");

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(REQUEST_RECEIPTS_STORAGE_PREFIX, bytes.concat(requestCommitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            require(entry.value.equals(new bytes(0)), "IHandler: Invalid non-membership proof");

            host.dispatchIncoming(request, meta, requestCommitment);
        }
    }

    /**
     * @dev check timeout proofs then dispatch to modules
     * @param host - Ismp host
     * @param message - batch post response timeouts
     */
    function handlePostResponseTimeouts(IIsmpHost host, PostResponseTimeoutMessage memory message)
        external
        notFrozen(host)
    {
        uint256 delay = host.timestamp() - host.stateMachineCommitmentUpdateTime(message.height);
        require(delay > host.challengePeriod(), "IHandler: still in challenge period");
        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        require(state.stateRoot != bytes32(0), "IHandler: State Commitment doesn't exist");
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            PostResponse memory response = message.timeouts[i];
            // timed-out?
            require(state.timestamp > response.timeout(), "IHandler: Response not timed out");

            // known response? also serves as source check
            bytes32 responseCommitment = response.hash();
            FeeMetadata memory meta = host.responseCommitments(responseCommitment);
            require(meta.sender != address(0), "IHandler: Unknown response");

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(RESPONSE_RECEIPTS_STORAGE_PREFIX, bytes.concat(responseCommitment));

            // verify state trie non-membership proofs
            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            require(entry.value.equals(new bytes(0)), "IHandler: Invalid non-membership proof");

            host.dispatchIncoming(response, meta, responseCommitment);
        }
    }

    /**
     * @dev check response proofs, message delay and timeouts, then dispatch get responses to modules
     * @param host - Ismp host
     * @param message - batch get responses
     */
    function handleGetResponses(IIsmpHost host, GetResponseMessage memory message) external notFrozen(host) {
        uint256 delay = host.timestamp() - host.stateMachineCommitmentUpdateTime(message.height);
        require(delay > host.challengePeriod(), "IHandler: still in challenge period");

        bytes32 root = host.stateMachineCommitment(message.height).stateRoot;
        require(root != bytes32(0), "IHandler: Proof height not found!");

        uint256 responsesLength = message.requests.length;
        bytes[] memory proof = message.proof;

        for (uint256 i = 0; i < responsesLength; i++) {
            GetRequest memory request = message.requests[i];
            // timed-out?
            require(request.timeout() > host.timestamp(), "IHandler: GET request timed out");

            // known request? also serves as source check
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown GET request");

            // duplicate response?
            require(!host.responseReceipts(requestCommitment), "IHandler: Duplicate GET response");
            StorageValue[] memory values =
                MerklePatricia.ReadChildProofCheck(root, proof, request.keys, bytes.concat(requestCommitment));
            GetResponse memory response = GetResponse({request: request, values: values});

            host.dispatchIncoming(response);
        }
    }

    /**
     * @dev dispatch to modules
     * @param host - Ismp host
     * @param message - batch get request timeouts
     */
    function handleGetRequestTimeouts(IIsmpHost host, GetTimeoutMessage memory message) external notFrozen(host) {
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            GetRequest memory request = message.timeouts[i];
            bytes32 requestCommitment = request.hash();
            FeeMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown request");

            require(host.timestamp() > request.timeout(), "IHandler: GET request not timed out");
            host.dispatchIncoming(request, meta, requestCommitment);
        }
    }
}
