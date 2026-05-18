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

import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {IConsensusV2, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";

import {MerkleMultiProof} from "@polytope-labs/solidity-merkle-trees/src/MerkleMultiProof.sol";
import {MerkleMountainRange} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import {ScaleCodec} from "@polytope-labs/solidity-merkle-trees/src/trie/polkadot/ScaleCodec.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";

import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

import {Codec} from "./Codec.sol";
import {
    Header,
    HeaderImpl,
    AuthoritySetCommitment,
    Vote,
    RelayChainProof,
    BeefyConsensusProof,
    Commitment,
    BeefyConsensusState,
    PartialBeefyMmrLeaf,
    Parachain,
    ParachainProof
} from "./Types.sol";

/**
 * @title The ECDSA BEEFY Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Verifies BEEFY consensus proofs by checking a 2/3+1 supermajority of secp256k1
 * signatures on-chain, along with merkle multi-proofs of authority set membership. This is
 * the most gas-expensive verifier but requires no off-chain proving infrastructure.
 *
 * @dev The verification flow is:
 *  1. Confirm the commitment's validator set id matches a known authority set.
 *  2. Verify that enough signatures are present to meet the supermajority threshold.
 *  3. Recover signer addresses via ecrecover and verify their membership in the authority
 *     set via a merkle multi-proof against the authority set root.
 *  4. Extract the MMR root from the commitment payload and verify the latest MMR leaf
 *     inclusion via a merkle mountain range proof.
 *  5. Verify parachain header inclusion in the MMR leaf's parachain heads root.
 *  6. Decode each parachain header to extract finalized state commitments.
 *
 * Stale proofs (commitment block number <= trusted latest height) are treated as no-ops.
 */
contract EcdsaBeefy is IConsensusV2, ERC165 {
    using HeaderImpl for Header;

    // The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

    // Provided authority set id was unknown
    error UnknownAuthoritySet();

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
        return interfaceId == type(IConsensusV2).interfaceId
            || super.supportsInterface(interfaceId);
    }

    /// @dev IConsensusV2 entry point. Decodes the proof, verifies consensus, and returns
    /// the updated state along with the latest authority set id.
    function verify(bytes calldata previousState, bytes calldata proof)
        external
        pure
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        BeefyConsensusState memory consensusState = abi.decode(previousState, (BeefyConsensusState));
        (RelayChainProof memory relay, ParachainProof memory parachain) =
            abi.decode(proof, (RelayChainProof, ParachainProof));

        (BeefyConsensusState memory newState, IntermediateState[] memory intermediates) =
            verifyConsensus(consensusState, BeefyConsensusProof(relay, parachain));

        return (abi.encode(newState), intermediates, newState.nextAuthoritySet.id);
    }

    // @dev Verify the consensus proof and return the new trusted consensus state and any intermediate states finalized
    // by this consensus proof.
    function verifyConsensus(BeefyConsensusState memory trustedState, BeefyConsensusProof memory proof)
        internal
        pure
        returns (BeefyConsensusState memory, IntermediateState[] memory)
    {
        // Stale proofs are a no-op: return the previous state with no intermediates so the caller
        // can treat replays as idempotent rather than having to guard against reverts.
        if (trustedState.latestHeight >= proof.relay.signedCommitment.commitment.blockNumber) {
            return (trustedState, new IntermediateState[](0));
        }
        (BeefyConsensusState memory state, bytes32 headsRoot) = verifyMmrUpdateProof(trustedState, proof.relay);
        IntermediateState[] memory intermediate = verifyParachainHeaderProof(headsRoot, proof.parachain);
        return (state, intermediate);
    }

    /**
     * @dev Verifies a new Mmmr root update, the relay chain accumulates its blocks into a merkle mountain range tree
     * which light clients can use as a source for log_2(n) ancestry proofs. This new mmr root hash is signed by
     * the relay chain authority set and we can verify the membership of the authorities who signed this new root
     * using a merkle multi proof and a merkle commitment to the total authorities.
     */
    function verifyMmrUpdateProof(BeefyConsensusState memory trustedState, RelayChainProof memory relayProof)
        internal
        pure
        returns (BeefyConsensusState memory, bytes32)
    {
        uint256 sigLen = relayProof.signedCommitment.votes.length;
        uint256 latestHeight = relayProof.signedCommitment.commitment.blockNumber;
        Commitment memory commitment = relayProof.signedCommitment.commitment;
        if (
            commitment.validatorSetId != trustedState.currentAuthoritySet.id
                && commitment.validatorSetId != trustedState.nextAuthoritySet.id
        ) {
            revert UnknownAuthoritySet();
        }

        bool isCurrentAuthorities = commitment.validatorSetId == trustedState.currentAuthoritySet.id;
        AuthoritySetCommitment memory authoritySet =
            isCurrentAuthorities ? trustedState.currentAuthoritySet : trustedState.nextAuthoritySet;
        if (!checkParticipationThreshold(sigLen, authoritySet.len)) revert SuperMajorityRequired();

        uint256 payloadLength = commitment.payload.length;
        bytes32 mmrRoot;
        for (uint256 i = 0; i < payloadLength; i++) {
            if (commitment.payload[i].id == MMR_ROOT_PAYLOAD_ID && commitment.payload[i].data.length == 32) {
                mmrRoot = Bytes.toBytes32(commitment.payload[i].data);
            }
        }
        if (mmrRoot == bytes32(0)) revert MmrRootHashMissing();

        // verify the commitment
        bytes32 commitmentHash = keccak256(Codec.Encode(commitment));
        MerkleMultiProof.Leaf[] memory authorities = new MerkleMultiProof.Leaf[](sigLen);
        for (uint256 i = 0; i < sigLen; i++) {
            Vote memory vote = relayProof.signedCommitment.votes[i];
            address authority = ECDSA.recover(commitmentHash, vote.signature);
            authorities[i] =
                MerkleMultiProof.Leaf({index: vote.authorityIndex, hash: keccak256(abi.encodePacked(authority))});
        }

        bool valid = MerkleMultiProof.VerifyProof(authoritySet.root, relayProof.proof, authorities, authoritySet.len);
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
    function verifyMmrLeaf(BeefyConsensusState memory trustedState, RelayChainProof memory relay, bytes32 mmrRoot)
        internal
        pure
    {
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
        MerkleMountainRange.Leaf[] memory leaves = new MerkleMountainRange.Leaf[](1);
        leaves[0] = MerkleMountainRange.Leaf({index: relay.latestMmrLeaf.leafIndex, hash: hash});
        bool valid = MerkleMountainRange.VerifyProof(mmrRoot, relay.mmrProof, leaves, leafCount);

        if (!valid) revert InvalidMmrProof();
    }

    // @dev Verifies that some parachain header has been finalized, given the current trusted consensus state.
    function verifyParachainHeaderProof(bytes32 headsRoot, ParachainProof memory proof)
        internal
        pure
        returns (IntermediateState[] memory)
    {
        uint256 len = proof.parachains.length;
        MerkleMultiProof.Leaf[] memory leaves = new MerkleMultiProof.Leaf[](len);
        IntermediateState[] memory intermediates = new IntermediateState[](len);

        for (uint256 i = 0; i < len; i++) {
            Parachain memory para = proof.parachains[i];
            Header memory header = Codec.DecodeHeader(para.header);
            if (header.number == 0) revert IllegalGenesisBlock();

            leaves[i] = MerkleMultiProof.Leaf(
                para.index,
                keccak256(bytes.concat(ScaleCodec.encode32(uint32(para.id)), ScaleCodec.encodeBytes(para.header)))
            );

            StateCommitment memory commitment = header.stateCommitment();
            intermediates[i] =
                IntermediateState({stateMachineId: para.id, height: header.number, commitment: commitment});
        }

        if (len > 0) {
            bool valid = MerkleMultiProof.VerifyProof(headsRoot, proof.proof, leaves, proof.leafCount);
            if (!valid) revert InvalidMmrProof();
        }

        return intermediates;
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
