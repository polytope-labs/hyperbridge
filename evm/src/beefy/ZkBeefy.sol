// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "./Codec.sol";
import "ismp/StateMachine.sol";
import "ismp/IConsensusClient.sol";

import "solidity-merkle-trees/MerkleMultiProof.sol";
import "solidity-merkle-trees/MerkleMountainRange.sol";
import "solidity-merkle-trees/MerklePatricia.sol";
import "solidity-merkle-trees/trie/substrate/ScaleCodec.sol";
import "solidity-merkle-trees/trie/Bytes.sol";
import "openzeppelin/utils/cryptography/ECDSA.sol";
import {IVerifier} from "./verifiers/IVerifier.sol";

struct PlonkConsensusProof {
    /// Commitment message
    Commitment commitment;
    /// Latest leaf added to mmr
    BeefyMmrLeaf latestMmrLeaf;
    /// Proof for the latest mmr leaf
    bytes32[] mmrProof;
    /// Plonk proof for BEEFY consensus
    bytes proof;
}

struct BeefyConsensusProof {
    PlonkConsensusProof relay;
    ParachainProof parachain;
}

contract ZkBeefyV1 is IConsensusClient {
    /// Slot duration in milliseconds
    uint256 public constant SLOT_DURATION = 12000;
    /// The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");
    /// Digest Item ID
    bytes4 public constant ISMP_CONSENSUS_ID = bytes4("ISMP");
    /// ConsensusID for aura
    bytes4 public constant AURA_CONSENSUS_ID = bytes4("aura");

    // Plonk verifier contract
    IVerifier internal _verifier;

    // Authorized paraId.
    uint256 private _paraId;

    constructor(uint256 paraId, IVerifier verifier) {
        _paraId = paraId;
        _verifier = verifier;
    }

    function verifyConsensus(bytes memory encodedState, bytes memory encodedProof)
        external
        returns (bytes memory, IntermediateState memory)
    {
        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));
        (PlonkConsensusProof memory relay, ParachainProof memory parachain) =
            abi.decode(encodedProof, (PlonkConsensusProof, ParachainProof));

        (BeefyConsensusState memory newState, IntermediateState memory intermediate) =
            verifyConsensus(consensusState, BeefyConsensusProof(relay, parachain));

        return (abi.encode(newState), intermediate);
    }

    /// Verify the consensus proof and return the new trusted consensus state and any intermediate states finalized
    /// by this consensus proof.
    function verifyConsensus(BeefyConsensusState memory trustedState, BeefyConsensusProof memory proof)
        internal
        returns (BeefyConsensusState memory, IntermediateState memory)
    {
        // verify mmr root proofs
        (BeefyConsensusState memory state, bytes32 headsRoot) = verifyMmrUpdateProof(trustedState, proof.relay);

        // verify intermediate state commitment proofs
        IntermediateState memory intermediate = verifyParachainHeaderProof(headsRoot, proof.parachain);

        return (state, intermediate);
    }

    /// Verifies a new Mmmr root update, the relay chain accumulates its blocks into a merkle mountain range tree
    /// which light clients can use as a source for log_2(n) ancestry proofs. This new mmr root hash is signed by
    /// the relay chain authority set and we can verify the membership of the authorities who signed this new root
    /// using a merkle multi proof and a merkle commitment to the total authorities.
    function verifyMmrUpdateProof(BeefyConsensusState memory trustedState, PlonkConsensusProof memory relayProof)
        internal
        returns (BeefyConsensusState memory, bytes32)
    {
        uint256 latestHeight = relayProof.commitment.blockNumber;
        require(latestHeight > trustedState.latestHeight, "consensus clients only accept proofs for new headers");

        Commitment memory commitment = relayProof.commitment;
        require(
            commitment.validatorSetId == trustedState.currentAuthoritySet.id
                || commitment.validatorSetId == trustedState.nextAuthoritySet.id,
            "Unknown authority set"
        );

        bool is_current_authorities = commitment.validatorSetId == trustedState.currentAuthoritySet.id;
        uint256 payload_len = commitment.payload.length;
        bytes32 mmrRoot;

        for (uint256 i = 0; i < payload_len; i++) {
            if (commitment.payload[i].id == MMR_ROOT_PAYLOAD_ID && commitment.payload[i].data.length == 32) {
                mmrRoot = Bytes.toBytes32(commitment.payload[i].data);
            }
        }
        require(mmrRoot != bytes32(0), "Mmr root hash not found");

        bytes32 commitment_hash = keccak256(Codec.Encode(commitment));
        bytes32[] memory inputs = new bytes32[](4);

        (bytes32 limb0, bytes32 limb1) = Codec.toFieldElements(commitment_hash);
        inputs[0] = limb0;
        inputs[1] = limb1;

        if (is_current_authorities) {
            (bytes32 limb2, bytes32 limb3) = Codec.toFieldElements(trustedState.currentAuthoritySet.root);
            inputs[2] = limb2;
            inputs[3] = limb3;
        } else {
            (bytes32 limb2, bytes32 limb3) = Codec.toFieldElements(trustedState.nextAuthoritySet.root);
            inputs[2] = limb2;
            inputs[3] = limb3;
        }

        // check BEEFY proof
        require(_verifier.verify(relayProof.proof, inputs), "ZkBEEFY: Invalid plonk proof");

        verifyMmrLeaf(trustedState, relayProof, mmrRoot);

        if (relayProof.latestMmrLeaf.nextAuthoritySet.id > trustedState.nextAuthoritySet.id) {
            trustedState.currentAuthoritySet = trustedState.nextAuthoritySet;
            trustedState.nextAuthoritySet = relayProof.latestMmrLeaf.nextAuthoritySet;
        }

        trustedState.latestHeight = latestHeight;

        return (trustedState, relayProof.latestMmrLeaf.extra);
    }

    /// Stack too deep, sigh solidity
    function verifyMmrLeaf(BeefyConsensusState memory trustedState, PlonkConsensusProof memory relay, bytes32 mmrRoot)
        internal
    {
        bytes32 hash = keccak256(Codec.Encode(relay.latestMmrLeaf));
        uint256 leafCount = leafIndex(trustedState.beefyActivationBlock, relay.latestMmrLeaf.parentNumber) + 1;

        MmrLeaf[] memory leaves = new MmrLeaf[](1);
        leaves[0] = MmrLeaf(relay.latestMmrLeaf.kIndex, relay.latestMmrLeaf.leafIndex, hash);

        require(MerkleMountainRange.VerifyProof(mmrRoot, relay.mmrProof, leaves, leafCount), "Invalid Mmr Proof");
    }

    /// Verifies that some parachain header has been finalized, given the current trusted consensus state.
    function verifyParachainHeaderProof(bytes32 headsRoot, ParachainProof memory proof)
        internal
        view
        returns (IntermediateState memory)
    {
        Node[] memory leaves = new Node[](1);
        Parachain memory para = proof.parachain;
        if (para.id != _paraId) {
            revert("Unknown paraId");
        }

        Header memory header = Codec.DecodeHeader(para.header);
        require(header.number != 0, "Genesis block should not be included");
        // extract verified metadata from header
        bytes32 commitment;
        uint256 timestamp;
        for (uint256 j = 0; j < header.digests.length; j++) {
            if (header.digests[j].isConsensus && header.digests[j].consensus.consensusId == ISMP_CONSENSUS_ID) {
                commitment = Bytes.toBytes32(header.digests[j].consensus.data);
            }

            if (header.digests[j].isPreRuntime && header.digests[j].preruntime.consensusId == AURA_CONSENSUS_ID) {
                uint256 slot = ScaleCodec.decodeUint256(header.digests[j].preruntime.data);
                timestamp = slot * SLOT_DURATION;
            }
        }
        require(timestamp != 0, "timestamp not found!");

        leaves[0] = Node(
            para.index,
            keccak256(bytes.concat(ScaleCodec.encode32(uint32(para.id)), ScaleCodec.encodeBytes(para.header)))
        );
        require(MerkleMultiProof.VerifyProof(headsRoot, proof.proof, leaves), "Invalid parachains heads proof");

        return IntermediateState(para.id, header.number, StateCommitment(timestamp, commitment, header.stateRoot));
    }

    /// Calculates the mmr leaf index for a block whose parent number is given.
    function leafIndex(uint256 activationBlock, uint256 parentNumber) private pure returns (uint256) {
        if (activationBlock == 0) {
            return parentNumber;
        } else {
            return parentNumber - activationBlock;
        }
    }
}
