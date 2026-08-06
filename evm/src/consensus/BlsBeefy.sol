// SPDX-License-Identifier: Apache-2.0
// Copyright (C) Polytope Labs Ltd.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.30;

import {IConsensusV2, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {MerkleMultiProof} from "@polytope-labs/solidity-merkle-trees/src/MerkleMultiProof.sol";
import {MerkleMountainRange} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import {ScaleCodec} from "@polytope-labs/solidity-merkle-trees/src/trie/polkadot/ScaleCodec.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

import {Codec} from "./Codec.sol";
import {
    Header,
    HeaderImpl,
    AuthoritySetCommitment,
    BlsBeefyConsensusProof,
    BlsRelayChainProof,
    BlsSigner,
    Commitment,
    BeefyConsensusState,
    PartialBeefyMmrLeaf,
    Parachain,
    ParachainProof
} from "./Types.sol";
import {BlsAggregate} from "./BlsAggregate.sol";

/**
 * @title The aggregate BLS12-381 BEEFY consensus client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Verifies BEEFY finality by checking one aggregate BLS signature rather than recovering
 * each authority's ECDSA signature. The signature check is then flat in the size of the validator
 * set, where [`EcdsaBeefy`] pays one `ecrecover` per signature. Everything else, MMR leaf
 * inclusion and parachain header proofs, is the same work as the ECDSA client does.
 *
 * @dev The verification flow is:
 *  1. Confirm the commitment's validator set id matches a known authority set.
 *  2. Confirm enough validators signed to meet the supermajority threshold.
 *  3. Confirm the signers' public keys are in the authority set via a merkle multi-proof, and that
 *     no signer is claimed twice.
 *  4. Verify the aggregate signature in one pairing check.
 *  5. Extract the MMR root from the commitment payload and verify the latest MMR leaf inclusion.
 *  6. Verify parachain header inclusion in the leaf's parachain heads root and decode the
 *     finalized state commitments.
 *
 * Stale proofs are a no-op, matching the ECDSA client, so replays are idempotent.
 *
 * Two encoding notes that are easy to get wrong:
 *  - Public keys are taken **uncompressed**, because EIP-2537 has no decompression precompile.
 *    The keyset commitment is still over the compressed encoding, as the runtime builds it; the
 *    contract compresses each key to compute its merkle leaf, which is cheap in that direction.
 *  - BEEFY puts signatures in G1 and public keys in G2, the opposite of the Ethereum convention,
 *    so an eth2 BLS library does not transfer.
 *
 * Requires an EVM with EIP-2537, so Prague or later.
 */
contract BlsBeefy is IConsensusV2, ERC165 {
    using HeaderImpl for Header;

    /// @dev The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

    /// @dev Provided authority set id was unknown.
    error UnknownAuthoritySet();
    /// @dev Mmr root hash was not found in the commitment payload.
    error MmrRootHashMissing();
    /// @dev Provided Mmr proof was invalid.
    error InvalidMmrProof();
    /// @dev Genesis block should not be provided.
    error IllegalGenesisBlock();
    /// @dev Supermajority not reached.
    error SuperMajorityRequired();
    /// @dev Provided authorities proof was invalid.
    error InvalidAuthoritiesProof();
    /// @dev Signer indices must strictly ascend and lie inside the authority set.
    error InvalidSignerOrdering();
    /// @dev The aggregate pairing check rejected the signature.
    error InvalidAggregateSignature();

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusV2).interfaceId || super.supportsInterface(interfaceId);
    }

    /// @dev IConsensusV2 entry point. Decodes the proof, verifies consensus, and returns the
    /// updated state along with the latest authority set id.
    function verify(bytes calldata previousState, bytes calldata proof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        BeefyConsensusState memory consensusState = abi.decode(previousState, (BeefyConsensusState));
        (BlsRelayChainProof memory relay, ParachainProof memory parachain) =
            abi.decode(proof, (BlsRelayChainProof, ParachainProof));

        // Stale proofs are a no-op: return the previous state with no intermediates so the caller
        // can treat replays as idempotent rather than having to guard against reverts.
        if (consensusState.latestHeight >= relay.commitment.blockNumber) {
            return (abi.encode(consensusState), new IntermediateState[](0), consensusState.nextAuthoritySet.id);
        }

        (BeefyConsensusState memory newState, bytes32 headsRoot) = verifyMmrUpdateProof(consensusState, relay);
        IntermediateState[] memory intermediates = verifyParachainHeaderProof(headsRoot, parachain);

        return (abi.encode(newState), intermediates, newState.nextAuthoritySet.id);
    }

    /**
     * @dev Verifies a new mmr root update. The relay chain accumulates its blocks into a merkle
     * mountain range, and the new root is signed by the authority set. Here that signature is a
     * single aggregate rather than one per validator, so establishing which validators signed is
     * a merkle multi-proof of their public keys plus one pairing check.
     */
    function verifyMmrUpdateProof(BeefyConsensusState memory trustedState, BlsRelayChainProof memory relayProof)
        internal
        view
        returns (BeefyConsensusState memory, bytes32)
    {
        Commitment memory commitment = relayProof.commitment;
        if (
            commitment.validatorSetId != trustedState.currentAuthoritySet.id
                && commitment.validatorSetId != trustedState.nextAuthoritySet.id
        ) {
            revert UnknownAuthoritySet();
        }

        bool isCurrentAuthorities = commitment.validatorSetId == trustedState.currentAuthoritySet.id;
        AuthoritySetCommitment memory authoritySet =
            isCurrentAuthorities ? trustedState.currentAuthoritySet : trustedState.nextAuthoritySet;

        verifyAuthorities(
            Codec.Encode(commitment),
            relayProof.signers,
            relayProof.aggregateSignature,
            authoritySet.root,
            relayProof.blsCommitment,
            relayProof.keysetProof,
            relayProof.proof,
            authoritySet.len
        );

        uint256 payloadLength = commitment.payload.length;
        bytes32 mmrRoot;
        for (uint256 i = 0; i < payloadLength; i++) {
            if (commitment.payload[i].id == MMR_ROOT_PAYLOAD_ID && commitment.payload[i].data.length == 32) {
                mmrRoot = Bytes.toBytes32(commitment.payload[i].data);
            }
        }
        if (mmrRoot == bytes32(0)) revert MmrRootHashMissing();

        verifyMmrLeaf(trustedState, relayProof, mmrRoot);

        if (relayProof.latestMmrLeaf.nextAuthoritySet.id > trustedState.nextAuthoritySet.id) {
            trustedState.currentAuthoritySet = trustedState.nextAuthoritySet;
            trustedState.nextAuthoritySet = relayProof.latestMmrLeaf.nextAuthoritySet;
        }
        trustedState.latestHeight = commitment.blockNumber;

        return (trustedState, relayProof.latestMmrLeaf.extra);
    }

    /**
     * @notice Establish that a supermajority of the committed authority set signed `commitment`.
     * @param commitment the SCALE-encoded BEEFY commitment that was signed
     * @param signers the validators claiming to have signed, in strictly ascending index order
     * @param aggregateSignature the summed G1 signature, 128 bytes uncompressed
     * @param keysetRoot the authority set's keyset commitment
     * @param authorityProof merkle multi-proof of the signers' keys against `keysetRoot`
     * @param authorityCount total validators in the set
     */
    function verifyAuthorities(
        bytes memory commitment,
        BlsSigner[] memory signers,
        bytes memory aggregateSignature,
        bytes32 keysetRoot,
        bytes32 blsCommitment,
        bytes32[] memory keysetProof,
        bytes32[] memory authorityProof,
        uint256 authorityCount
    ) public view {
        uint256 count = signers.length;
        if (!checkParticipationThreshold(count, authorityCount)) revert SuperMajorityRequired();

        // Strictly ascending and inside the set. Without this a prover could repeat one signer to
        // clear the threshold, and the aggregate would happily verify the repeated key against the
        // repeated signature.
        for (uint256 i = 0; i < count; ++i) {
            if (signers[i].authorityIndex >= authorityCount) revert InvalidSignerOrdering();
            if (i > 0 && signers[i].authorityIndex <= signers[i - 1].authorityIndex) {
                revert InvalidSignerOrdering();
            }
        }

        // The pairing check only proves the holders of these keys signed. Proving those keys are
        // the authority set's is what the multi-proof is for.
        MerkleMultiProof.Leaf[] memory leaves = new MerkleMultiProof.Leaf[](count);
        bytes[] memory publicKeys = new bytes[](count);
        for (uint256 i = 0; i < count; ++i) {
            publicKeys[i] = signers[i].publicKey;
            // The runtime commits the compressed encoding, so derive it here rather than asking
            // the prover to send both forms, which would then need checking for consistency.
            leaves[i] = MerkleMultiProof.Leaf({
                index: signers[i].authorityIndex, hash: keccak256(BlsAggregate.compressG2(signers[i].publicKey))
            });
        }

        // Two levels. The relay chain commits the BLS keys as one extra leaf of the authority set
        // tree, so the per-authority leaves keep their positions and bridges verifying ECDSA
        // signatures still prove against the same root. Establish that leaf is the authority set's,
        // then prove the signers against it.
        //
        // The keyset tree therefore holds authorityCount + 1 leaves, with the BLS commitment last,
        // while the threshold above still judges against authorityCount.
        MerkleMultiProof.Leaf[] memory keysetLeaf = new MerkleMultiProof.Leaf[](1);
        keysetLeaf[0] =
            MerkleMultiProof.Leaf({index: authorityCount, hash: keccak256(abi.encodePacked(blsCommitment))});

        if (!MerkleMultiProof.VerifyProof(keysetRoot, keysetProof, keysetLeaf, authorityCount + 1)) {
            revert InvalidAuthoritiesProof();
        }

        if (!MerkleMultiProof.VerifyProof(blsCommitment, authorityProof, leaves, authorityCount)) {
            revert InvalidAuthoritiesProof();
        }

        if (!BlsAggregate.verify(commitment, aggregateSignature, publicKeys)) {
            revert InvalidAggregateSignature();
        }
    }

    // @dev Stack too deep, sigh solidity
    function verifyMmrLeaf(BeefyConsensusState memory trustedState, BlsRelayChainProof memory relay, bytes32 mmrRoot)
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

    // @dev Verifies that some parachain header has been finalized, given the current trusted state.
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

    /// @notice The BEEFY supermajority: more than two thirds of the set.
    function checkParticipationThreshold(uint256 len, uint256 total) public pure returns (bool) {
        return len >= ((2 * total) / 3) + 1;
    }

    // @dev so these structs are included in the abi
    function noOp(BeefyConsensusState memory s, BlsBeefyConsensusProof memory p) external pure {}
}
