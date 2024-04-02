// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {StateCommitment} from "ismp/IConsensusClient.sol";
import "solidity-merkle-trees/trie/Bytes.sol";
import "solidity-merkle-trees/trie/substrate/ScaleCodec.sol";

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
        require(timestamp != 0, "timestamp not found!");
        require(childTrieRoot != bytes32(0), "Child trie commitment not found");
        require(mmrRoot != bytes32(0), "Mmr root commitment not found");

        return StateCommitment({timestamp: timestamp, overlayRoot: mmrRoot, stateRoot: childTrieRoot});
    }
}
