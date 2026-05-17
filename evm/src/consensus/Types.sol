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

import {StateCommitment} from "@hyperbridge/core/interfaces/IConsensus.sol";
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
}
