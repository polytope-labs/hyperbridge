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

import "./Codec.sol";
import "@polytope-labs/ismp-solidity/StateMachine.sol";
import "@polytope-labs/ismp-solidity/IConsensusClient.sol";

import {MerkleMultiProof} from "@polytope-labs/solidity-merkle-trees/src/MerkleMultiProof.sol";
import {MerkleMountainRange} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import {MerklePatricia} from "@polytope-labs/solidity-merkle-trees/src/MerklePatricia.sol";
import {StorageValue, MmrLeaf} from "@polytope-labs/solidity-merkle-trees/src/Types.sol";
import {ScaleCodec} from "@polytope-labs/solidity-merkle-trees/src/trie/substrate/ScaleCodec.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";

import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

struct Vote {
    // secp256k1 signature from a member of the authority set
    bytes signature;
    // This member's index in the set
    uint256 authorityIndex;
}

// The signed commitment holds a commitment to the latest
// finalized state as well as votes from a supermajority
// of the authority set which confirms this state
struct SignedCommitment {
    // A commitment to the finalized state
    Commitment commitment;
    // The confirming votes
    Vote[] votes;
}

struct RelayChainProof {
    // Signed commitment
    SignedCommitment signedCommitment;
    // Latest leaf added to mmr
    BeefyMmrLeaf latestMmrLeaf;
    // Proof for the latest mmr leaf
    bytes32[] mmrProof;
    // Proof for authorities in current/next session
    Node[][] proof;
}

struct BeefyConsensusProof {
    // The proof items for the relay chain consensus
    RelayChainProof relay;
    // Proof items for parachain headers
    ParachainProof parachain;
}

/**
 * @title The BEEFY Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice This verifies secp256k1 signatures and authority set membership merkle proofs
 * in order to confirm newly finalized states of the Hyperbridge blockchain.
 */
contract BeefyV1 is IConsensusClient, ERC165 {
    using HeaderImpl for Header;

    // The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

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

    // Supermajority not reached
    error SuperMajorityRequired();

    // Provided authorities proof was invalid
    error InvalidAuthoritiesProof();

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusClient).interfaceId || super.supportsInterface(interfaceId);
    }

    function verifyConsensus(
        bytes memory encodedState,
        bytes memory encodedProof
    ) external pure returns (bytes memory, IntermediateState[] memory) {
        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));
        (RelayChainProof memory relay, ParachainProof memory parachain) = abi.decode(
            encodedProof,
            (RelayChainProof, ParachainProof)
        );

        (BeefyConsensusState memory newState, IntermediateState memory intermediate) = verifyConsensus(
            consensusState,
            BeefyConsensusProof(relay, parachain)
        );

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (abi.encode(newState), intermediates);
    }

    // @dev Verify the consensus proof and return the new trusted consensus state and any intermediate states finalized
    // by this consensus proof.
    function verifyConsensus(
        BeefyConsensusState memory trustedState,
        BeefyConsensusProof memory proof
    ) internal pure returns (BeefyConsensusState memory, IntermediateState memory) {
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
        RelayChainProof memory relayProof
    ) internal pure returns (BeefyConsensusState memory, bytes32) {
        uint256 signatures_length = relayProof.signedCommitment.votes.length;
        uint256 latestHeight = relayProof.signedCommitment.commitment.blockNumber;

        if (trustedState.latestHeight >= latestHeight) revert StaleHeight();

        if (
            !checkParticipationThreshold(signatures_length, trustedState.currentAuthoritySet.len) &&
            !checkParticipationThreshold(signatures_length, trustedState.nextAuthoritySet.len)
        ) {
            revert SuperMajorityRequired();
        }

        Commitment memory commitment = relayProof.signedCommitment.commitment;

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
        Node[] memory authorities = new Node[](signatures_length);

        // verify authorities' votes
        for (uint256 i = 0; i < signatures_length; i++) {
            Vote memory vote = relayProof.signedCommitment.votes[i];
            address authority = ECDSA.recover(commitment_hash, vote.signature);
            authorities[i] = Node(vote.authorityIndex, keccak256(abi.encodePacked(authority)));
        }

        bool valid;
        // check authorities proof
        if (is_current_authorities) {
            valid = MerkleMultiProof.VerifyProof(trustedState.currentAuthoritySet.root, relayProof.proof, authorities);
        } else {
            valid = MerkleMultiProof.VerifyProof(trustedState.nextAuthoritySet.root, relayProof.proof, authorities);
        }
        if (!valid) revert InvalidAuthoritiesProof();

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
        RelayChainProof memory relay,
        bytes32 mmrRoot
    ) internal pure {
        bytes32 hash = keccak256(
            Codec.Encode(
                PartialBeefyMmrLeaf({
                    version: relay.latestMmrLeaf.version,
                    parentNumber: relay.latestMmrLeaf.parentNumber,
                    parentHash: relay.latestMmrLeaf.parentHash,
                    nextAuthoritySet: relay.latestMmrLeaf.nextAuthoritySet,
                    extra: relay.latestMmrLeaf.extra
                })
            )
        );
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

        // verify header
        leaves[0] = Node(
            para.index,
            keccak256(bytes.concat(ScaleCodec.encode32(uint32(para.id)), ScaleCodec.encodeBytes(para.header)))
        );

        bool valid = MerkleMultiProof.VerifyProof(headsRoot, proof.proof, leaves);
        if (!valid) revert InvalidMmrProof();
        // extract the state commitment
        StateCommitment memory commitment = header.stateCommitment();
        IntermediateState memory intermediate = IntermediateState({
            stateMachineId: para.id,
            height: header.number,
            commitment: commitment
        });

        return intermediate;
    }

    // @dev Calculates the mmr leaf index for a block whose parent number is given.
    function leafIndex(uint256 activationBlock, uint256 parentNumber) internal pure returns (uint256) {
        if (activationBlock == 0) {
            return parentNumber;
        } else {
            return parentNumber - activationBlock;
        }
    }

    // @dev Check for supermajority participation.
    function checkParticipationThreshold(uint256 len, uint256 total) internal pure returns (bool) {
        return len >= ((2 * total) / 3) + 1;
    }

    // @dev so these structs are included in the abi
    function noOp(BeefyConsensusState memory s, BeefyConsensusProof memory p) external pure {}
}
