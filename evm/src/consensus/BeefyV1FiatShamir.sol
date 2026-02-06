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

import {Codec} from "./Codec.sol";
import {Transcript} from "./Transcript.sol";
import {Header, HeaderImpl} from "./Header.sol";
import {
    Vote,
    RelayChainProof,
    BeefyConsensusProof,
    Commitment,
    AuthoritySetCommitment,
    BeefyConsensusState,
    PartialBeefyMmrLeaf,
    Parachain,
    ParachainProof
} from "./Types.sol";

import {IConsensus, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensus.sol";

import {MerkleMultiProof} from "@polytope-labs/solidity-merkle-trees/src/MerkleMultiProof.sol";
import {MerkleMountainRange} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import {Node, MmrLeaf} from "@polytope-labs/solidity-merkle-trees/src/Types.sol";
import {ScaleCodec} from "@polytope-labs/solidity-merkle-trees/src/trie/substrate/ScaleCodec.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";

import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

/**
 * @title The BEEFY Fiat-Shamir Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice A gas-optimized variant of BeefyV1 that uses the Fiat-Shamir heuristic
 * to deterministically select a small subset of validator signatures for on-chain
 * verification, instead of verifying a full 2/3+1 supermajority.
 *
 * ## Protocol
 *
 * 1. The prover observes the BEEFY signed commitment and constructs a **signers
 *    bitmap** — a compact 4×uint256 (1024-bit) bitfield where bit `i` is set iff
 *    authority `i` signed the commitment.
 *
 * 2. Both prover and verifier build an identical Fiat-Shamir transcript seeded
 *    with the commitment hash, the authority set commitment (root + length), and
 *    the signers bitmap. The bitmap is absorbed into the transcript so the prover
 *    cannot manipulate which validators are sampled after choosing which
 *    signatures to include.
 *
 * 3. From the transcript, SAMPLE_SIZE unique indices in [0, signerCount) are
 *    derived, then mapped to actual authority indices via the bitmap (the n-th
 *    set bit corresponds to the n-th signer).
 *
 * 4. The proof contains only those SAMPLE_SIZE signatures and a merkle
 *    multi-proof of their membership in the authority set.
 *
 * ## Security
 *
 * The verifier checks that the bitmap represents a 2/3+1 supermajority before
 * accepting it. An attacker controlling fraction f < 1/3 of the set has at most
 * f/(2/3) ≈ 1/2 of the signers in the bitmap. The probability that all
 * SAMPLE_SIZE sampled signers are adversarial is at most (1/2)^10 ≈ 0.1%.
 *
 * ## Gas savings
 *
 * Only 10 ecrecover calls (30,000 gas) instead of ~200+ (600,000+ gas), and a
 * proportionally smaller merkle multi-proof. The bitmap verification and
 * transcript construction add negligible overhead.
 */
contract BeefyV1FiatShamir is IConsensus, ERC165 {
    using HeaderImpl for Header;
    using Transcript for Transcript.State;

    // ──────────────────────────────────────────────
    //  Constants
    // ──────────────────────────────────────────────

    /// @notice The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

    /// @notice Number of validator signatures to sample and verify.
    uint256 public constant SAMPLE_SIZE = 10;

    /// @notice Domain separator for the Fiat-Shamir transcript.
    bytes public constant TRANSCRIPT_DOMAIN = "BEEFY_FIAT_SHAMIR_V1";

    /// @notice Number of uint256 words in the signers bitmap (4 × 256 = 1024 bits).
    uint256 internal constant BITMAP_WORDS = 4;

    // ──────────────────────────────────────────────
    //  Errors
    // ──────────────────────────────────────────────

    /// @notice Provided authority set id was unknown.
    error UnknownAuthoritySet();

    /// @notice Provided consensus proof height is stale.
    error StaleHeight();

    /// @notice Mmr root hash was not found in commitment payload.
    error MmrRootHashMissing();

    /// @notice Provided Mmr proof was invalid.
    error InvalidMmrProof();

    /// @notice Genesis block should not be provided.
    error IllegalGenesisBlock();

    /// @notice Provided authorities merkle proof was invalid.
    error InvalidAuthoritiesProof();

    /// @notice The number of provided votes does not match SAMPLE_SIZE.
    error WrongSampleCount(uint256 expected, uint256 actual);

    /// @notice A provided vote's authorityIndex does not match the expected
    /// Fiat-Shamir challenge.
    error VoteAuthorityMismatch(uint256 position, uint256 expectedAuthority, uint256 actualAuthority);

    /// @notice The authority set is smaller than SAMPLE_SIZE.
    error AuthoritySetTooSmall(uint256 authoritySetLen, uint256 sampleSize);

    /// @notice The signers bitmap does not represent a 2/3+1 supermajority.
    error SuperMajorityRequired(uint256 signerCount, uint256 required);

    /// @notice Authority set exceeds the maximum supported by the bitmap.
    error AuthoritySetTooLarge(uint256 authoritySetLen, uint256 maxSupported);

    // ──────────────────────────────────────────────
    //  ERC-165
    // ──────────────────────────────────────────────

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensus).interfaceId || super.supportsInterface(interfaceId);
    }

    // ──────────────────────────────────────────────
    //  IConsensus entry point
    // ──────────────────────────────────────────────

    /**
     * @dev The encoded proof is expected as:
     *   abi.encode(RelayChainProof, ParachainProof, uint256[4])
     * where the uint256[4] is the signers bitmap.
     */
    function verifyConsensus(bytes memory encodedState, bytes memory encodedProof)
        external
        pure
        returns (bytes memory, IntermediateState[] memory)
    {
        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));
        (RelayChainProof memory relay, ParachainProof memory parachain, uint256[4] memory signersBitmap) =
            abi.decode(encodedProof, (RelayChainProof, ParachainProof, uint256[4]));

        (BeefyConsensusState memory newState, IntermediateState memory intermediate) =
            verifyConsensusInner(consensusState, relay, parachain, signersBitmap);

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (abi.encode(newState), intermediates);
    }

    // ──────────────────────────────────────────────
    //  Core verification
    // ──────────────────────────────────────────────

    function verifyConsensusInner(
        BeefyConsensusState memory trustedState,
        RelayChainProof memory relay,
        ParachainProof memory parachain,
        uint256[4] memory signersBitmap
    ) internal pure returns (BeefyConsensusState memory, IntermediateState memory) {
        (BeefyConsensusState memory state, bytes32 headsRoot) = verifyMmrUpdateProof(trustedState, relay, signersBitmap);
        IntermediateState memory intermediate = verifyParachainHeaderProof(headsRoot, parachain);
        return (state, intermediate);
    }

    /**
     * @dev Verifies a new MMR root update using Fiat-Shamir sampled signature
     * verification with a signers bitmap.
     *
     * Flow:
     * 1. Determine the active authority set.
     * 2. Count set bits in the bitmap and verify supermajority.
     * 3. Build a Fiat-Shamir transcript from the commitment hash, authority set
     *    commitment, and signers bitmap.
     * 4. Sample SAMPLE_SIZE indices from [0, signerCount), map each to the actual
     *    authority index (the n-th set bit in the bitmap).
     * 5. Verify the provided votes match and pass ecrecover + merkle membership.
     * 6. Verify the MMR leaf.
     */
    function verifyMmrUpdateProof(
        BeefyConsensusState memory trustedState,
        RelayChainProof memory relayProof,
        uint256[4] memory signersBitmap
    ) internal pure returns (BeefyConsensusState memory, bytes32) {
        uint256 latestHeight = relayProof.signedCommitment.commitment.blockNumber;
        if (trustedState.latestHeight >= latestHeight) revert StaleHeight();

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

        // 1024-bit bitmap supports up to 1024 validators
        if (authoritySet.len > BITMAP_WORDS * 256) {
            revert AuthoritySetTooLarge(authoritySet.len, BITMAP_WORDS * 256);
        }
        if (authoritySet.len < SAMPLE_SIZE) revert AuthoritySetTooSmall(authoritySet.len, SAMPLE_SIZE);

        // Count signers from the bitmap and verify supermajority
        uint256 signerCount = countSetBits(signersBitmap, authoritySet.len);
        uint256 required = ((2 * authoritySet.len) / 3) + 1;
        if (signerCount < required) revert SuperMajorityRequired(signerCount, required);

        // Extract MMR root
        bytes32 mmrRoot = extractMmrRoot(commitment);
        if (mmrRoot == bytes32(0)) revert MmrRootHashMissing();

        bytes32 commitmentHash = keccak256(Codec.Encode(commitment));

        // ── Fiat-Shamir challenge ──
        uint256[] memory challengedAuthorities =
            deriveAuthorityChallenge(commitmentHash, authoritySet, signersBitmap, signerCount);

        // ── Verify sampled votes ──
        verifySampledVotes(
            commitmentHash,
            relayProof.signedCommitment.votes,
            challengedAuthorities,
            authoritySet.root,
            relayProof.proof
        );

        // ── Verify MMR leaf ──
        verifyMmrLeaf(trustedState, relayProof, mmrRoot);

        // ── Update authority sets ──
        if (relayProof.latestMmrLeaf.nextAuthoritySet.id > trustedState.nextAuthoritySet.id) {
            trustedState.currentAuthoritySet = trustedState.nextAuthoritySet;
            trustedState.nextAuthoritySet = relayProof.latestMmrLeaf.nextAuthoritySet;
        }

        trustedState.latestHeight = latestHeight;
        return (trustedState, relayProof.latestMmrLeaf.extra);
    }

    // ──────────────────────────────────────────────
    //  Bitmap helpers
    // ──────────────────────────────────────────────

    /// @dev Returns true if bit `index` is set in the bitmap.
    function isBitSet(uint256[4] memory bitmap, uint256 index) internal pure returns (bool) {
        uint256 word = index >> 8; // index / 256
        uint256 bit = index & 0xFF; // index % 256
        return (bitmap[word] & (1 << bit)) != 0;
    }

    /// @dev Counts the number of set bits in the bitmap, only considering
    /// positions [0, authoritySetLen). Uses O(1)-per-word parallel popcount.
    function countSetBits(uint256[4] memory bitmap, uint256 authoritySetLen) internal pure returns (uint256 count) {
        for (uint256 w = 0; w < BITMAP_WORDS; w++) {
            uint256 remaining = authoritySetLen > w * 256 ? authoritySetLen - w * 256 : 0;
            if (remaining == 0) break;

            uint256 word = bitmap[w];
            if (remaining < 256) {
                // Mask off bits beyond authoritySetLen
                word &= (uint256(1) << remaining) - 1;
            }
            count += popcount256(word);
        }
    }

    /// @dev Returns the number of set bits in a uint256 using parallel bit counting.
    /// Runs in constant time regardless of the input value.
    function popcount256(uint256 x) internal pure returns (uint256) {
        // Step 1: pair-wise sums (each 2-bit field holds popcount of its pair)
        x = x - ((x >> 1) & 0x5555555555555555555555555555555555555555555555555555555555555555);
        // Step 2: nibble-wise sums (each 4-bit field holds popcount of its nibble)
        x = (x & 0x3333333333333333333333333333333333333333333333333333333333333333)
            + ((x >> 2) & 0x3333333333333333333333333333333333333333333333333333333333333333);
        // Step 3: byte-wise sums (each byte holds popcount of its original 8 bits, max 8)
        x = (x + (x >> 4)) & 0x0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F;
        // Step 4: multiply by 0x0101...01 to sum all 32 bytes into the top byte, then shift
        x = (x * 0x0101010101010101010101010101010101010101010101010101010101010101) >> 248;
        return x;
    }

    /// @dev Enumerates all set bit positions in the bitmap in one pass,
    /// returning an array of authority indices for every signer.
    /// Only considers positions [0, authoritySetLen).
    function enumerateSigners(uint256[4] memory bitmap, uint256 authoritySetLen, uint256 signerCount)
        internal
        pure
        returns (uint256[] memory signers)
    {
        signers = new uint256[](signerCount);
        uint256 idx = 0;
        for (uint256 i = 0; i < authoritySetLen; i++) {
            if (isBitSet(bitmap, i)) {
                signers[idx++] = i;
            }
        }
    }

    // ──────────────────────────────────────────────
    //  Fiat-Shamir challenge derivation
    // ──────────────────────────────────────────────

    /**
     * @dev Derives SAMPLE_SIZE unique authority indices from the transcript.
     *
     * The transcript absorbs:
     *   - commitmentHash (binds to this specific block)
     *   - authority set root + length (binds to the validator set)
     *   - all 4 bitmap words (binds to the exact signer set — prevents the
     *     prover from changing which signers are claimed after seeing the
     *     challenge)
     *
     * Indices are sampled from [0, signerCount) and then mapped to actual
     * authority indices via the bitmap.
     *
     * @param commitmentHash  keccak256 of the SCALE-encoded commitment.
     * @param authoritySet    The active authority set commitment.
     * @param signersBitmap   The 4×uint256 signers bitmap.
     * @param signerCount     Number of set bits in the bitmap.
     * @return authorities    SAMPLE_SIZE actual authority indices, sorted ascending.
     */
    function deriveAuthorityChallenge(
        bytes32 commitmentHash,
        AuthoritySetCommitment memory authoritySet,
        uint256[4] memory signersBitmap,
        uint256 signerCount
    ) internal pure returns (uint256[] memory authorities) {
        Transcript.State memory transcript = Transcript.init(TRANSCRIPT_DOMAIN);

        // Absorb commitment + authority set
        transcript.absorbBytes32(commitmentHash);
        transcript.absorbBytes32(authoritySet.root);
        transcript.absorbUint256(authoritySet.len);

        // Absorb the entire bitmap to bind the challenge
        for (uint256 w = 0; w < BITMAP_WORDS; w++) {
            transcript.absorbUint256(signersBitmap[w]);
        }

        // Build the signers array once — O(authoritySetLen)
        uint256[] memory signers = enumerateSigners(signersBitmap, authoritySet.len, signerCount);

        // Sample SAMPLE_SIZE unique indices in [0, signerCount)
        uint256[] memory sampledPositions = transcript.sampleUniqueIndices(SAMPLE_SIZE, signerCount);

        // Map each sampled position to the actual authority index — O(SAMPLE_SIZE)
        authorities = new uint256[](SAMPLE_SIZE);
        for (uint256 i = 0; i < SAMPLE_SIZE; i++) {
            authorities[i] = signers[sampledPositions[i]];
        }
    }

    // ──────────────────────────────────────────────
    //  Signature + membership verification
    // ──────────────────────────────────────────────

    /**
     * @dev Verifies that the provided sampled votes are valid:
     *   1. Exactly SAMPLE_SIZE votes were provided.
     *   2. Each vote's authorityIndex matches the Fiat-Shamir derived index.
     *      Votes must be ordered to match the challengedAuthorities array.
     *   3. Each signature recovers to a valid authority address.
     *   4. All recovered authorities pass the merkle membership proof.
     */
    function verifySampledVotes(
        bytes32 commitmentHash,
        Vote[] memory votes,
        uint256[] memory challengedAuthorities,
        bytes32 authorityRoot,
        Node[][] memory proof
    ) internal pure {
        uint256 sampleSize = challengedAuthorities.length;

        if (votes.length != sampleSize) {
            revert WrongSampleCount(sampleSize, votes.length);
        }

        Node[] memory authorities = new Node[](sampleSize);

        for (uint256 i = 0; i < sampleSize; i++) {
            if (votes[i].authorityIndex != challengedAuthorities[i]) {
                revert VoteAuthorityMismatch(i, challengedAuthorities[i], votes[i].authorityIndex);
            }

            address signer = ECDSA.recover(commitmentHash, votes[i].signature);
            authorities[i] = Node(votes[i].authorityIndex, keccak256(abi.encodePacked(signer)));
        }

        bool valid = MerkleMultiProof.VerifyProof(authorityRoot, proof, authorities);
        if (!valid) revert InvalidAuthoritiesProof();
    }

    // ──────────────────────────────────────────────
    //  MMR leaf verification
    // ──────────────────────────────────────────────

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

        MmrLeaf[] memory leaves = new MmrLeaf[](1);
        leaves[0] = MmrLeaf(relay.latestMmrLeaf.kIndex, relay.latestMmrLeaf.leafIndex, hash);

        bool valid = MerkleMountainRange.VerifyProof(mmrRoot, relay.mmrProof, leaves, leafCount);
        if (!valid) revert InvalidMmrProof();
    }

    // ──────────────────────────────────────────────
    //  Parachain header verification
    // ──────────────────────────────────────────────

    function verifyParachainHeaderProof(bytes32 headsRoot, ParachainProof memory proof)
        internal
        pure
        returns (IntermediateState memory)
    {
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

        StateCommitment memory stateCommitment = header.stateCommitment();
        return IntermediateState({stateMachineId: para.id, height: header.number, commitment: stateCommitment});
    }

    // ──────────────────────────────────────────────
    //  Helpers
    // ──────────────────────────────────────────────

    function extractMmrRoot(Commitment memory commitment) internal pure returns (bytes32 mmrRoot) {
        uint256 payloadLen = commitment.payload.length;
        for (uint256 i = 0; i < payloadLen; i++) {
            if (commitment.payload[i].id == MMR_ROOT_PAYLOAD_ID && commitment.payload[i].data.length == 32) {
                mmrRoot = Bytes.toBytes32(commitment.payload[i].data);
            }
        }
    }

    function leafIndex(uint256 activationBlock, uint256 parentNumber) internal pure returns (uint256) {
        if (activationBlock == 0) {
            return parentNumber;
        } else {
            return parentNumber - activationBlock;
        }
    }

    /// @dev ABI export helper so these structs appear in the generated ABI.
    function noOp(BeefyConsensusState memory s, BeefyConsensusProof memory p) external pure {}
}
