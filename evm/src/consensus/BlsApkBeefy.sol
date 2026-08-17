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

import {IConsensusV2, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {MerkleMountainRange} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";
import {MerkleMultiProof} from "@polytope-labs/solidity-merkle-trees/src/MerkleMultiProof.sol";
import {ScaleCodec} from "@polytope-labs/solidity-merkle-trees/src/trie/polkadot/ScaleCodec.sol";

import {Codec} from "./Codec.sol";
import {
    AuthoritySetCommitment,
    ApkDigest,
    BlsApkBeefyConsensusProof,
    BeefyConsensusState,
    BlsApkRelayChainProof,
    BeefyMmrLeaf,
    Commitment,
    Digest,
    Header,
    HeaderImpl,
    Parachain,
    ParachainProof,
    PartialBeefyMmrLeaf
} from "./Types.sol";

interface IApkProof {
    function verify(
        uint256 publicKeysCommitment,
        uint256[5] calldata bitlist,
        bytes32[3] calldata apk,
        bytes calldata apkProof,
        bytes32[3] calldata message,
        bytes32[3] calldata signature,
        bytes32[6] calldata apk2
    ) external view;

    function hashToG1(bytes memory message) external view returns (bytes32[3] memory);
}

/**
 * @title BEEFY consensus verified by an aggregate public key proof
 * @notice Verifies BEEFY finality without touching individual validator keys.
 *
 * @dev Naming each signer and proving their public key against the authority set's keyset root
 * costs roughly 17k gas per signer, for the G2 addition and the compression needed to rebuild the
 * leaf. At a few hundred validators that dominates everything else.
 *
 * Here a SNARK does that work instead. The prover shows that an aggregate public key corresponds
 * to exactly the validators named in a bitlist, against a commitment to the whole set, and the
 * contract checks one proof and one pairing. Nothing in the calldata or the verification grows
 * with the number of signers.
 *
 * The commitment the proof is checked against does not come from the relay chain's MMR leaf, the
 * way the keyset root does. Hyperbridge computes it over the relay's next authority set and
 * publishes it in a header digest, so a client picks it up from a header it has already verified
 * and carries it in its consensus state. That is what `AuthoritySetCommitment.root` holds, and
 * why the state has to be seeded with the starting set's commitment at initialisation.
 *
 * Requires Prague for the EIP-2537 precompiles.
 */
contract BlsApkBeefy is IConsensusV2, ERC165 {
    /// The payload id for the mmr root in a BEEFY commitment, "mh"
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

    /// The APK proof verifier, holding the circuit's verifying key.
    IApkProof public immutable _apk;

    /// The parachain whose header digests carry apk commitments, which is hyperbridge.
    ///
    /// Every parachain in a proof is proven against the heads root, so a digest from any of them
    /// is authentic; only this one's says anything about the relay chain's authorities.
    uint32 public immutable _digestParaId;

    /// The commitment was signed by a set this client does not know.
    error UnknownAuthoritySet();
    /// Fewer than two thirds of the set signed.
    error SuperMajorityRequired();
    /// The APK proof or the aggregate signature did not verify.
    error InvalidAggregateProof();
    /// The commitment carried no mmr root payload.
    error MmrRootHashMissing();
    /// The mmr leaf was not in the tree the commitment attests to.
    error InvalidMmrProof();
    /// A parachain header was not in the heads root.
    error InvalidParachainHeaderProof();
    /// The authority set has no APK commitment yet, so no proof against it can be checked.
    error MissingApkCommitment();

    constructor(address apkProof, uint32 digestParaId) {
        _apk = IApkProof(apkProof);
        _digestParaId = digestParaId;
    }

    function supportsInterface(bytes4 interfaceId) public view virtual override(ERC165) returns (bool) {
        return interfaceId == type(IConsensusV2).interfaceId || super.supportsInterface(interfaceId);
    }

    /// @dev IConsensusV2 entry point.
    function verify(bytes calldata previousState, bytes calldata proof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        BeefyConsensusState memory consensusState = abi.decode(previousState, (BeefyConsensusState));
        (BlsApkRelayChainProof memory relay, ParachainProof memory parachain) =
            abi.decode(proof, (BlsApkRelayChainProof, ParachainProof));

        // Replays are idempotent rather than reverting, matching the ecdsa client.
        if (consensusState.latestHeight >= relay.commitment.blockNumber) {
            return (abi.encode(consensusState), new IntermediateState[](0), consensusState.nextAuthoritySet.id);
        }

        (BeefyConsensusState memory newState, bytes32 headsRoot) = verifyMmrUpdateProof(consensusState, relay);
        (IntermediateState[] memory intermediates, ApkDigest memory digest) =
            verifyParachainHeaderProof(headsRoot, parachain, _digestParaId);

        // Forward chaining: a verified header may carry the commitment for a set this client does
        // not have keys for yet. Picking it up here is what lets the next update be verified at
        // all, and is the reason the digest names the *next* set rather than the current one.
        if (
            digest.setId == newState.nextAuthoritySet.id && newState.nextAuthoritySet.root == bytes32(0)
        ) {
            newState.nextAuthoritySet.root = bytes32(digest.commitment);
        }

        return (abi.encode(newState), intermediates, newState.nextAuthoritySet.id);
    }

    /// @dev Verify the signed mmr root, then roll the authority sets forward.
    function verifyMmrUpdateProof(BeefyConsensusState memory trustedState, BlsApkRelayChainProof memory relayProof)
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

        bool isCurrent = commitment.validatorSetId == trustedState.currentAuthoritySet.id;
        AuthoritySetCommitment memory authoritySet = isCurrent ? trustedState.currentAuthoritySet : trustedState.nextAuthoritySet;

        // A set whose commitment has not been learned from a digest yet cannot be verified against.
        // Reverting here is deliberate: silently accepting would mean checking the proof against a
        // zero commitment.
        if (uint256(authoritySet.root) == 0) revert MissingApkCommitment();

        verifySignedByApk(Codec.Encode(commitment), relayProof, authoritySet);

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
            // The incoming set's size comes from the mmr leaf, but its APK commitment is not known
            // until a header digest supplies it, so it starts empty.
            trustedState.nextAuthoritySet = AuthoritySetCommitment({
                id: relayProof.latestMmrLeaf.nextAuthoritySet.id,
                len: relayProof.latestMmrLeaf.nextAuthoritySet.len,
                root: bytes32(0)
            });
        }
        trustedState.latestHeight = commitment.blockNumber;

        return (trustedState, relayProof.latestMmrLeaf.extra);
    }

    /**
     * @notice Establish that a supermajority of `authoritySet` signed `encodedCommitment`.
     * @dev Two things are checked, and the split matters. The SNARK proves that `apk` is the
     * aggregate of exactly the validators set in `bitlist`, against the set's commitment. The
     * pairing inside `ApkProof` then checks the aggregate signature against that key, and binds
     * `apk2` to `apk` so a caller cannot supply an unrelated G2 point. The threshold is counted
     * here, because the circuit proves who signed but has no opinion on whether that is enough.
     */
    function verifySignedByApk(
        bytes memory encodedCommitment,
        BlsApkRelayChainProof memory relayProof,
        AuthoritySetCommitment memory authoritySet
    ) internal view {
        uint256 signed = countSigners(relayProof.bitlist);
        if (!checkParticipationThreshold(signed, authoritySet.len)) revert SuperMajorityRequired();

        bytes32[3] memory message = _apk.hashToG1(encodedCommitment);

        // `verify` reverts on failure rather than returning false, so a successful call is the
        // whole result. Wrapped so the reason surfaces as this contract's error.
        try _apk.verify(
            uint256(authoritySet.root),
            relayProof.bitlist,
            relayProof.apk,
            relayProof.apkProof,
            message,
            relayProof.signature,
            relayProof.apk2
        ) {} catch {
            revert InvalidAggregateProof();
        }
    }

    /// @dev Population count over the bitlist. Fixed cost regardless of how many signed, which is
    /// the point of the whole scheme.
    ///
    /// The 1024 slots are packed 250 to a word across the first four and 24 into the last, which
    /// is what the circuit decomposes with `ToBinary`. Bits above those ranges are constrained to
    /// zero there, so a proof carrying any would not verify; masking them off here keeps the count
    /// honest on its own terms rather than relying on that.
    function countSigners(uint256[5] memory bitlist) internal pure returns (uint256 count) {
        for (uint256 w = 0; w < 5; w++) {
            uint256 width = w == 4 ? 24 : 250;
            uint256 word = bitlist[w] & ((uint256(1) << width) - 1);
            while (word != 0) {
                word &= word - 1;
                count++;
            }
        }
    }

    /// @dev Two thirds plus one, matching the ecdsa client and substrate's own rule.
    function checkParticipationThreshold(uint256 signed, uint256 total) internal pure returns (bool) {
        return total > 0 && signed * 3 > total * 2;
    }

    /// @dev The signed mmr root must attest to the leaf carrying the parachain heads.
    function verifyMmrLeaf(
        BeefyConsensusState memory trustedState,
        BlsApkRelayChainProof memory relay,
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

        MerkleMountainRange.Leaf[] memory leaves = new MerkleMountainRange.Leaf[](1);
        leaves[0] = MerkleMountainRange.Leaf({index: relay.latestMmrLeaf.leafIndex, hash: hash});

        bool valid = MerkleMountainRange.VerifyProof(mmrRoot, relay.mmrProof, leaves, leafCount);
        if (!valid) revert InvalidMmrProof();
    }

    /// @dev Verify the parachain headers against the heads root, and surface any APK commitment
    /// they carry so the caller can roll it into the consensus state.
    function verifyParachainHeaderProof(bytes32 headsRoot, ParachainProof memory proof, uint32 digestParaId)
        internal
        pure
        returns (IntermediateState[] memory, ApkDigest memory digest)
    {
        uint256 len = proof.parachains.length;
        MerkleMultiProof.Leaf[] memory leaves = new MerkleMultiProof.Leaf[](len);
        IntermediateState[] memory intermediates = new IntermediateState[](len);

        for (uint256 i = 0; i < len; i++) {
            Parachain memory para = proof.parachains[i];
            Header memory header = Codec.DecodeHeader(para.header);

            leaves[i] = MerkleMultiProof.Leaf(
                para.index,
                keccak256(bytes.concat(ScaleCodec.encode32(uint32(para.id)), ScaleCodec.encodeBytes(para.header)))
            );

            intermediates[i] = IntermediateState({
                stateMachineId: para.id,
                height: header.number,
                commitment: HeaderImpl.stateCommitment(header)
            });

            // Only the first commitment found is used; the pallet writes at most one per block.
            // Read it from hyperbridge's header alone, or any parachain in the proof could name
            // the keys this client trusts next.
            if (digest.setId == 0 && para.id == digestParaId) {
                digest = HeaderImpl.apkCommitment(header);
            }
        }

        if (len > 0) {
            bool valid = MerkleMultiProof.VerifyProof(headsRoot, proof.proof, leaves, proof.leafCount);
            if (!valid) revert InvalidParachainHeaderProof();
        }

        return (intermediates, digest);
    }

    /// @dev Leaf index for a relay chain block, given where beefy was activated.
    function leafIndex(uint256 activationBlock, uint256 parentNumber) internal pure returns (uint256) {
        return activationBlock == 0 ? parentNumber : parentNumber - activationBlock;
    }

    /// @dev Only here so the structs appear in the ABI, which is what the Rust bindings are
    /// generated from. `verify` takes bytes, so without this they would be invisible.
    function noOp(BeefyConsensusState memory s, BlsApkBeefyConsensusProof memory p) external pure {}
}
