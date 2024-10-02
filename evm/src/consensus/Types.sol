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

import {Node} from "@polytope-labs/solidity-merkle-trees/src/Types.sol";

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

/// The public values encoded as a struct that can be easily deserialized inside Solidity.
struct PublicInputs {
    // merkle commitment to all authorities
    bytes32 authorities_root;
    // size of the authority set
    uint256 authorities_len;
    // BEEFY mmr leaf hash
    bytes32 leaf_hash;
    // Parachain header hashes
    bytes32[] headers;
}

struct Payload {
    bytes2 id;
    bytes data;
}

struct Commitment {
    Payload[] payload;
    uint256 blockNumber;
    uint256 validatorSetId;
}

struct AuthoritySetCommitment {
    /// Id of the set.
    uint256 id;
    /// Number of validators in the set.
    uint256 len;
    /// Merkle Root Hash built from BEEFY AuthorityIds.
    bytes32 root;
}

struct BeefyMmrLeaf {
    uint256 version;
    uint256 parentNumber;
    bytes32 parentHash;
    AuthoritySetCommitment nextAuthoritySet;
    bytes32 extra;
    uint256 kIndex;
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
    uint256 version;
    uint256 parentNumber;
    bytes32 parentHash;
    AuthoritySetCommitment nextAuthoritySet;
    bytes32 extra;
}

struct Parachain {
    /// k-index for latestHeadsRoot
    uint256 index;
    /// Parachain Id
    uint256 id;
    /// SCALE encoded header
    bytes header;
}

struct ParachainProof {
    Parachain parachain;
    Node[][] proof;
}
