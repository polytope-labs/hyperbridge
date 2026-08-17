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

/**
 * @notice Shared type definitions for the BEEFY consensus client suite. Contains all structs
 * used across EcdsaBeefy, SP1Beefy, and the ConsensusRouter, as well as
 * the HeaderImpl library for extracting state commitments from Substrate block headers.
 */

import {StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {ScaleCodec} from "@polytope-labs/solidity-merkle-trees/src/trie/polkadot/ScaleCodec.sol";

struct SP1BeefyProof {
    // BEEFY Commitment message
    MiniCommitment commitment;
    // Latest leaf added to mmr
    PartialBeefyMmrLeaf mmrLeaf;
    // Parachain headers finalized by the commitment
    ParachainHeader[] headers;
    // SP1 plonk proof for BEEFY consensus
    bytes proof;
    // Prover-chosen nonce committed into the proof's public values. Carried verbatim so the
    // verifier can reconstruct the committed public inputs; rewarding verifiers bind it to the
    // submission account to make a proof non-transferable.
    bytes32 nonce;
}

struct MiniCommitment {
    uint256 blockNumber;
    uint256 validatorSetId;
}

struct ParachainHeader {
    /// Parachain Id
    uint256 id;
    /// SCALE encoded header
    bytes header;
}

struct ParachainHeaderHash {
    // Parachain Id
    uint256 id;
    // header hash
    bytes32 hash;
}

/// The public values encoded as a struct that can be easily deserialized inside Solidity.
struct PublicInputs {
    // merkle commitment to all authorities
    bytes32 authorities_root;
    // size of the authority set
    uint256 authorities_len;
    // BEEFY mmr leaf hash
    bytes32 leaf_hash;
    // commitment block number
    uint256 block_number;
    // Parachain header hashes
    ParachainHeaderHash[] headers;
    // Prover-chosen nonce, committed verbatim by the SP1 program
    bytes32 nonce;
}

struct Payload {
    bytes2 id;
    bytes data;
}

struct Commitment {
    Payload[] payload;
    uint32 blockNumber;
    uint64 validatorSetId;
}

struct AuthoritySetCommitment {
    /// Id of the set.
    uint64 id;
    /// Number of validators in the set.
    uint32 len;
    /// Merkle Root Hash built from BEEFY AuthorityIds.
    bytes32 root;
}

struct BeefyMmrLeaf {
    uint8 version;
    uint32 parentNumber;
    bytes32 parentHash;
    AuthoritySetCommitment nextAuthoritySet;
    bytes32 extra;
    uint256 leafIndex;
}

struct BeefyConsensusState {
    /// block number for the latest mmr_root_hash
    uint256 latestHeight;
    /// Block number that the beefy protocol was activated on the relay chain.
    /// This should be the first block in the merkle-mountain-range tree.
    uint256 beefyActivationBlock;
    /// authorities for the current round
    AuthoritySetCommitment currentAuthoritySet;
    /// authorities for the next round
    AuthoritySetCommitment nextAuthoritySet;
}

struct PartialBeefyMmrLeaf {
    uint8 version;
    uint32 parentNumber;
    bytes32 parentHash;
    AuthoritySetCommitment nextAuthoritySet;
    bytes32 extra;
}

struct Parachain {
    /// 0-based leaf index in the parachain heads merkle tree
    uint256 index;
    /// Parachain Id
    uint256 id;
    /// SCALE encoded header
    bytes header;
}

struct ParachainProof {
    Parachain[] parachains;
    bytes32[] proof;
    uint256 leafCount;
}

struct Vote {
    // secp256k1 signature from a member of the authority set
    bytes signature;
    // 0-based index of the authority in the authority set
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
    bytes32[] proof;
}

struct BeefyConsensusProof {
    // The proof items for the relay chain consensus
    RelayChainProof relay;
    // Proof items for parachain headers
    ParachainProof parachain;
}

// An authority set identified by its APK commitment rather than by a merkle root over keys.
//
// The commitment is Poseidon2 over the validators' BLS12-381 G1 keys, padded to the circuit's
// fixed width. Unlike the keyset root it does not come from the MMR leaf: hyperbridge publishes it
// in a header digest, and a client picks it up from a header it has already verified. That is why
// it is carried in the consensus state rather than supplied with each proof.
/// An apk commitment read off a header, with `setId` zero meaning the header carried none.
struct ApkDigest {
    /// The authority set the commitment describes.
    uint64 setId;
    /// Poseidon2 commitment over that set's G1 public keys.
    uint256 commitment;
}

struct ApkAuthoritySet {
    /// Id of the set.
    uint64 id;
    /// Number of validators in the set, for the two-thirds threshold.
    uint32 len;
    /// Poseidon2 commitment over the set's G1 public keys, as `ApkProof.verify` takes it.
    uint256 apkCommitment;
}

struct BlsApkConsensusState {
    /// block number for the latest mmr_root_hash
    uint256 latestHeight;
    /// Block number that the beefy protocol was activated on the relay chain.
    uint256 beefyActivationBlock;
    /// authorities for the current round
    ApkAuthoritySet currentAuthoritySet;
    /// authorities for the next round
    ApkAuthoritySet nextAuthoritySet;
}

// A BEEFY relay chain proof verified by a SNARK over the aggregate public key, rather than by a
// merkle multi-proof of each signer's key.
//
// The saving is that nothing here grows with the number of signers: the bitlist is fixed width and
// the proof is constant size, where the merkle path costs roughly 17k gas per signer.
struct BlsApkRelayChainProof {
    // A commitment to the finalized state
    Commitment commitment;
    // Which validators signed, one bit each, 1024 slots over five words
    uint256[5] bitlist;
    // Aggregate public key in G1, proven correct against the set's APK commitment
    bytes32[3] apk;
    // The same aggregate in G2, bound to `apk` by the pairing check inside ApkProof
    bytes32[6] apk2;
    // PLONK proof that `apk` is the aggregate of exactly the validators in `bitlist`
    bytes apkProof;
    // Sum of the signers' BLS signatures, a G1 point
    bytes32[3] signature;
    // Latest leaf added to mmr
    BeefyMmrLeaf latestMmrLeaf;
    // Proof for the latest mmr leaf
    bytes32[] mmrProof;
}

struct BlsApkBeefyConsensusProof {
    // The proof items for the relay chain consensus
    BlsApkRelayChainProof relay;
    // Proof items for parachain headers
    ParachainProof parachain;
}

struct DigestItem {
    bytes4 consensusId;
    bytes data;
}

struct Digest {
    bool isPreRuntime;
    DigestItem preruntime;
    bool isConsensus;
    DigestItem consensus;
    bool isSeal;
    DigestItem seal;
    bool isOther;
    bytes other;
    bool isRuntimeEnvironmentUpdated;
}

struct Header {
    bytes32 parentHash;
    uint256 number;
    bytes32 stateRoot;
    bytes32 extrinsicRoot;
    Digest[] digests;
}

library HeaderImpl {
    /// Digest Item ID
    bytes4 public constant ISMP_CONSENSUS_ID = bytes4("ISMP");
    /// ConsensusID for the ISMP timestamp digest deposited by pallet-ismp
    bytes4 public constant ISMP_TIMESTAMP_ID = bytes4("ISTM");
    /// ConsensusID for the APK commitment digest deposited by pallet-beefy-apk-digest
    bytes4 public constant APK_COMMITMENT_ID = bytes4("APKC");

    error TimestampNotFound();

    /// @dev Extracts the ISMP MMR root, child trie root, and timestamp from the header
    /// digests and returns them as a StateCommitment. Reverts if no timestamp digest is found.
    function stateCommitment(Header memory self) internal pure returns (StateCommitment memory) {
        bytes32 mmrRoot;
        bytes32 childTrieRoot;
        uint256 timestamp;

        for (uint256 j = 0; j < self.digests.length; j++) {
            if (self.digests[j].isConsensus && self.digests[j].consensus.consensusId == ISMP_CONSENSUS_ID) {
                mmrRoot = Bytes.toBytes32(Bytes.substr(self.digests[j].consensus.data, 0, 32));
                childTrieRoot = Bytes.toBytes32(Bytes.substr(self.digests[j].consensus.data, 32));
            }

            if (self.digests[j].isConsensus && self.digests[j].consensus.consensusId == ISMP_TIMESTAMP_ID) {
                timestamp = ScaleCodec.decodeUint256(self.digests[j].consensus.data);
            }
        }

        // sanity check
        if (timestamp == 0) revert TimestampNotFound();

        return StateCommitment({timestamp: timestamp, overlayRoot: mmrRoot, stateRoot: childTrieRoot});
    }

    /// @dev The commitment to the relay chain's next BEEFY authority set, if this header carries
    /// one. Written by `pallet-beefy-apk-digest` on the block a set finishes being absorbed, so
    /// most headers do not have it and `found` is false for those.
    ///
    /// The header itself is already authenticated, through the parachain heads root in the BEEFY
    /// MMR leaf, so no further proof is needed: `commitment` can go straight to `ApkProof.verify`
    /// as `publicKeysCommitment`, and `setId` says which authority set it describes.
    ///
    /// Payload is SCALE: a u64 set id little-endian, then the 32 byte commitment.
    ///
    /// A zero `setId` reads as absent. BEEFY numbers its sets from one, so nothing legitimate
    /// names set zero, and the caller then has one thing to check rather than two.
    function apkCommitment(Header memory self) internal pure returns (ApkDigest memory digest) {
        for (uint256 j = 0; j < self.digests.length; j++) {
            if (!self.digests[j].isConsensus) continue;
            if (self.digests[j].consensus.consensusId != APK_COMMITMENT_ID) continue;

            bytes memory data = self.digests[j].consensus.data;
            // Ignore a malformed item rather than reverting: a wrong length means some other
            // producer wrote under this engine id, and the caller should see "absent", not fail.
            if (data.length != 40) continue;

            uint64 setId = uint64(ScaleCodec.decodeUint256(Bytes.substr(data, 0, 8)));
            if (setId == 0) continue;

            return ApkDigest({
                setId: setId,
                commitment: uint256(Bytes.toBytes32(Bytes.substr(data, 8)))
            });
        }
    }
}
