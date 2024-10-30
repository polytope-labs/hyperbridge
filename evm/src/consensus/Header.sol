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

import {StateCommitment} from "@polytope-labs/ismp-solidity/IConsensusClient.sol";
import "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import "@polytope-labs/solidity-merkle-trees/src/trie/substrate/ScaleCodec.sol";

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
    /// ConsensusID for aura
    bytes4 public constant AURA_CONSENSUS_ID = bytes4("aura");
    /// Slot duration in milliseconds
    uint256 public constant SLOT_DURATION = 12000;

    error TimestampNotFound();
    error ChildTrieRootNotFound();

    // Extracts the `StateCommitment` from the provided header.
    function stateCommitment(Header calldata self) public pure returns (StateCommitment memory) {
        bytes32 mmrRoot;
        bytes32 childTrieRoot;
        uint256 timestamp;

        for (uint256 j = 0; j < self.digests.length; j++) {
            if (self.digests[j].isConsensus && self.digests[j].consensus.consensusId == ISMP_CONSENSUS_ID) {
                mmrRoot = Bytes.toBytes32(self.digests[j].consensus.data[:32]);
                childTrieRoot = Bytes.toBytes32(self.digests[j].consensus.data[32:]);
            }

            if (self.digests[j].isPreRuntime && self.digests[j].preruntime.consensusId == AURA_CONSENSUS_ID) {
                uint256 slot = ScaleCodec.decodeUint256(self.digests[j].preruntime.data);
                timestamp = slot * SLOT_DURATION;
            }
        }

        // sanity check
        if (timestamp == 0) revert TimestampNotFound();
        if (childTrieRoot == bytes32(0)) revert ChildTrieRootNotFound();

        return StateCommitment({timestamp: timestamp, overlayRoot: mmrRoot, stateRoot: childTrieRoot});
    }
}
