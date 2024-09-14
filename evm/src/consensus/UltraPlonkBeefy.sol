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

import "@polytope-labs/ismp-solidity/StateMachine.sol";
import "@polytope-labs/ismp-solidity/IConsensusClient.sol";

import "@polytope-labs/solidity-merkle-trees/src/MerkleMultiProof.sol";
import "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import "@polytope-labs/solidity-merkle-trees/src/trie/substrate/ScaleCodec.sol";
import "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import "./verifiers/IVerifier.sol";
import "./Codec.sol";

import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

struct UltraPlonkConsensusProof {
    // Commitment message
    Commitment commitment;
    // Latest leaf added to mmr
    BeefyMmrLeaf latestMmrLeaf;
    // Proof for the latest mmr leaf
    bytes32[] mmrProof;
    // UltraPlonk proof for BEEFY consensus
    bytes proof;
}

struct BeefyConsensusProof {
    // The proof items for the relay chain consensus
    UltraPlonkConsensusProof relay;
    // Proof items for parachain headers
    ParachainProof parachain;
}

/**
 * @title The UltraPlonk BEEFY Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Similar to the BeefyV1 client but delegates secp256k1 signature verification
 * and authority set membership proof checks to an ultraplonk circuit.
 */
contract UltraPlonkBeefy is IConsensusClient, ERC165 {
    using HeaderImpl for Header;

    // Slot duration in milliseconds
    uint256 public constant SLOT_DURATION = 12_000;

    // The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

    // Digest Item ID
    bytes4 public constant ISMP_CONSENSUS_ID = bytes4("ISMP");

    // ConsensusID for aura
    bytes4 public constant AURA_CONSENSUS_ID = bytes4("aura");

    // Plonk verifier contract
    IVerifier internal _verifier;

    // Provided authority set id was unknown
    error UnknownAuthoritySet();

    // Provided consensus proof height is stale
    error StaleHeight();

    // Provided ultra plonk proof was invalid
    error InvalidUltraPlonkProof();

    // Mmr root hash was not found in header digests
    error MmrRootHashMissing();

    // Provided Mmr Proof was invalid
    error InvalidMmrProof();

    // Genesis block should not be provided
    error IllegalGenesisBlock();

    constructor(IVerifier verifier) {
        _verifier = verifier;
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusClient).interfaceId || super.supportsInterface(interfaceId);
    }

    function verifyConsensus(
        bytes memory encodedState,
        bytes memory encodedProof
    ) external view returns (bytes memory, IntermediateState memory) {
        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));
        (UltraPlonkConsensusProof memory relay, ParachainProof memory parachain) = abi.decode(
            encodedProof,
            (UltraPlonkConsensusProof, ParachainProof)
        );

        (BeefyConsensusState memory newState, IntermediateState memory intermediate) = verifyConsensus(
            consensusState,
            BeefyConsensusProof(relay, parachain)
        );

        return (abi.encode(newState), intermediate);
    }

    // @dev Verify the consensus proof and return the new trusted consensus state and any intermediate states finalized
    // by this consensus proof.
    function verifyConsensus(
        BeefyConsensusState memory trustedState,
        BeefyConsensusProof memory proof
    ) internal view returns (BeefyConsensusState memory, IntermediateState memory) {
        // verify mmr root proofs
        (BeefyConsensusState memory state, bytes32 headsRoot) = verifyMmrUpdateProof(trustedState, proof.relay);

        // verify intermediate state commitment proofs
        IntermediateState memory intermediate = verifyParachainHeaderProof(headsRoot, proof.parachain);

        return (state, intermediate);
    }

    /** @dev Verifies a new Mmmr root update, the relay chain accumulates its blocks into a merkle mountain range tree
     * which light clients can use as a source for log_2(n) ancestry proofs. This new mmr root hash is signed by
     * the relay chain authority set and we can verify the membership of the authorities who signed this new root
     * using a merkle multi proof and a merkle commitment to the total authorities.
     */
    function verifyMmrUpdateProof(
        BeefyConsensusState memory trustedState,
        UltraPlonkConsensusProof memory relayProof
    ) internal view returns (BeefyConsensusState memory, bytes32) {
        uint256 latestHeight = relayProof.commitment.blockNumber;

        if (trustedState.latestHeight >= latestHeight) revert StaleHeight();

        Commitment memory commitment = relayProof.commitment;

        if (
            commitment.validatorSetId != trustedState.currentAuthoritySet.id &&
            commitment.validatorSetId != trustedState.nextAuthoritySet.id
        ) {
            revert UnknownAuthoritySet();
        }

        bool is_current_authorities = commitment.validatorSetId == trustedState.currentAuthoritySet.id;
        uint256 payload_len = commitment.payload.length;
        bytes32 mmrRoot;

        for (uint256 i = 0; i < payload_len; i++) {
            if (commitment.payload[i].id == MMR_ROOT_PAYLOAD_ID && commitment.payload[i].data.length == 32) {
                mmrRoot = Bytes.toBytes32(commitment.payload[i].data);
            }
        }
        if (mmrRoot == bytes32(0)) revert MmrRootHashMissing();

        bytes32 commitment_hash = keccak256(Codec.Encode(commitment));
        bytes32[] memory inputs = new bytes32[](4);

        (inputs[0], inputs[1]) = Codec.toFieldElements(commitment_hash);
        if (is_current_authorities) {
            (inputs[2], inputs[3]) = Codec.toFieldElements(trustedState.currentAuthoritySet.root);
        } else {
            (inputs[2], inputs[3]) = Codec.toFieldElements(trustedState.nextAuthoritySet.root);
        }

        // check ultraplonk proof
        if (!_verifier.verify(relayProof.proof, inputs)) revert InvalidUltraPlonkProof();

        verifyMmrLeaf(trustedState, relayProof, mmrRoot);

        if (relayProof.latestMmrLeaf.nextAuthoritySet.id > trustedState.nextAuthoritySet.id) {
            trustedState.currentAuthoritySet = trustedState.nextAuthoritySet;
            trustedState.nextAuthoritySet = relayProof.latestMmrLeaf.nextAuthoritySet;
        }

        trustedState.latestHeight = latestHeight;

        return (trustedState, relayProof.latestMmrLeaf.extra);
    }

    // @dev Stack too deep, sigh solidity
    function verifyMmrLeaf(
        BeefyConsensusState memory trustedState,
        UltraPlonkConsensusProof memory relay,
        bytes32 mmrRoot
    ) internal pure {
        bytes32 hash = keccak256(Codec.Encode(relay.latestMmrLeaf));
        uint256 leafCount = leafIndex(trustedState.beefyActivationBlock, relay.latestMmrLeaf.parentNumber) + 1;

        MmrLeaf[] memory leaves = new MmrLeaf[](1);
        leaves[0] = MmrLeaf(relay.latestMmrLeaf.kIndex, relay.latestMmrLeaf.leafIndex, hash);
        bool valid = MerkleMountainRange.VerifyProof(mmrRoot, relay.mmrProof, leaves, leafCount);

        if (!valid) revert InvalidMmrProof();
    }

    // @dev Verifies that some parachain header has been finalized, given the current trusted consensus state.
    function verifyParachainHeaderProof(
        bytes32 headsRoot,
        ParachainProof memory proof
    ) internal pure returns (IntermediateState memory) {
        Node[] memory leaves = new Node[](1);
        Parachain memory para = proof.parachain;

        Header memory header = Codec.DecodeHeader(para.header);

        if (header.number == 0) revert IllegalGenesisBlock();

        leaves[0] = Node(
            para.index,
            keccak256(bytes.concat(ScaleCodec.encode32(uint32(para.id)), ScaleCodec.encodeBytes(para.header)))
        );

        bool valid = MerkleMultiProof.VerifyProof(headsRoot, proof.proof, leaves);
        if (!valid) revert InvalidMmrProof();

        StateCommitment memory commitment = header.stateCommitment();

        return IntermediateState({stateMachineId: para.id, height: header.number, commitment: commitment});
    }

    // @dev Calculates the mmr leaf index for a block whose parent number is given.
    function leafIndex(uint256 activationBlock, uint256 parentNumber) internal pure returns (uint256) {
        if (activationBlock == 0) {
            return parentNumber;
        } else {
            return parentNumber - activationBlock;
        }
    }
}
