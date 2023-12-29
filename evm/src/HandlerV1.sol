// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "solidity-merkle-trees/MerkleMountainRange.sol";
import "solidity-merkle-trees/MerklePatricia.sol";
import "openzeppelin/utils/Context.sol";

import "ismp/IConsensusClient.sol";
import "ismp/IHandler.sol";
import "ismp/IIsmpHost.sol";

contract HandlerV1 is IHandler, Context {
    using Bytes for bytes;

    modifier notFrozen(IIsmpHost host) {
        require(!host.frozen(), "IHandler: frozen");
        _;
    }

    // Storage prefix for request receipts in pallet-ismp
    bytes private constant REQUEST_COMMITMENT_STORAGE_PREFIX =
        hex"103895530afb23bb607661426d55eb8b0484aecefe882c3ce64e6f82507f715a";

    event StateMachineUpdated(uint256 stateMachineId, uint256 height);

    /**
     * @dev Handle incoming consensus messages
     * @param host - Ismp host
     * @param proof - consensus proof
     */
    function handleConsensus(IIsmpHost host, bytes memory proof) external notFrozen(host) {
        require(
            (host.timestamp() - host.consensusUpdateTime()) > host.challengePeriod(),
            "IHandler: still in challenge period"
        );

        // not today, time traveling validators
        require(
            (host.timestamp() - host.consensusUpdateTime()) < host.unStakingPeriod() || _msgSender() == host.admin(),
            "IHandler: still in challenge period"
        );

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

            require(leaf.request.dest.equals(host.host()), "IHandler: Invalid request destination");
            require(
                leaf.request.timeoutTimestamp == 0 || leaf.request.timeoutTimestamp > host.timestamp(),
                "IHandler: Request timed out"
            );

            bytes32 commitment = Message.hash(leaf.request);
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
            require(leaf.response.request.source.equals(host.host()), "IHandler: Invalid response destination");

            bytes32 requestCommitment = Message.hash(leaf.response.request);
            require(host.requestCommitments(requestCommitment), "IHandler: Unknown request");

            bytes32 responseCommitment = Message.hash(leaf.response);
            require(!host.responseCommitments(responseCommitment), "IHandler: Duplicate Post response");

            leaves[i] = MmrLeaf(leaf.kIndex, leaf.index, responseCommitment);
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
    function handlePostTimeouts(IIsmpHost host, PostTimeoutMessage memory message) external notFrozen(host) {
        // fetch the state commitment
        StateCommitment memory state = host.stateMachineCommitment(message.height);
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            PostRequest memory request = message.timeouts[i];
            require(
                request.timeoutTimestamp != 0 && state.timestamp > request.timeoutTimestamp, "Request not timed out"
            );

            bytes32 requestCommitment = Message.hash(request);
            RequestMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown request");

            bytes[] memory keys = new bytes[](1);
            keys[i] = bytes.concat(REQUEST_COMMITMENT_STORAGE_PREFIX, bytes.concat(requestCommitment));

            StorageValue memory entry = MerklePatricia.VerifySubstrateProof(state.stateRoot, message.proof, keys)[0];
            require(entry.value.equals(new bytes(0)), "IHandler: Invalid non-membership proof");

            host.dispatchIncoming(PostTimeout(request), meta, requestCommitment);
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

        StateCommitment memory stateCommitment = host.stateMachineCommitment(message.height);
        bytes32 root = stateCommitment.stateRoot;
        require(root != bytes32(0), "IHandler: Proof height not found!");

        uint256 responsesLength = message.requests.length;
        bytes[] memory proof = message.proof;

        for (uint256 i = 0; i < responsesLength; i++) {
            GetRequest memory request = message.requests[i];
            require(request.source.equals(host.host()), "IHandler: Invalid GET response destination");

            bytes32 requestCommitment = Message.hash(request);
            require(host.requestCommitments(requestCommitment), "IHandler: Unknown GET request");
            require(
                request.timeoutTimestamp == 0 || request.timeoutTimestamp > host.timestamp(),
                "IHandler: GET request timed out"
            );

            StorageValue[] memory values =
                MerklePatricia.ReadChildProofCheck(root, proof, request.keys, bytes.concat(requestCommitment));
            GetResponse memory response = GetResponse({request: request, values: values});
            require(!host.responseCommitments(Message.hash(response)), "IHandler: Duplicate GET response");
            host.dispatchIncoming(response);
        }
    }

    /**
     * @dev dispatch to modules
     * @param host - Ismp host
     * @param message - batch get request timeouts
     */
    function handleGetTimeouts(IIsmpHost host, GetTimeoutMessage memory message) external notFrozen(host) {
        uint256 timeoutsLength = message.timeouts.length;

        for (uint256 i = 0; i < timeoutsLength; i++) {
            GetRequest memory request = message.timeouts[i];
            bytes32 requestCommitment = Message.hash(request);
            RequestMetadata memory meta = host.requestCommitments(requestCommitment);
            require(meta.sender != address(0), "IHandler: Unknown request");

            require(
                request.timeoutTimestamp != 0 && host.timestamp() > request.timeoutTimestamp,
                "IHandler: GET request not timed out"
            );
            host.dispatchIncoming(request, meta, requestCommitment);
        }
    }
}
