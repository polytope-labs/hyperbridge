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
pragma solidity 0.8.17;

import "solidity-merkle-trees/MerkleMultiProof.sol";
import "solidity-merkle-trees/trie/substrate/ScaleCodec.sol";
import "solidity-merkle-trees/trie/Bytes.sol";
import "./Header.sol";

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

/// type encoding stuff
library Codec {
    uint8 internal constant DIGEST_ITEM_OTHER = 0;
    uint8 internal constant DIGEST_ITEM_CONSENSUS = 4;
    uint8 internal constant DIGEST_ITEM_SEAL = 5;
    uint8 internal constant DIGEST_ITEM_PRERUNTIME = 6;
    uint8 internal constant DIGEST_ITEM_RUNTIME_ENVIRONMENT_UPDATED = 8;

    function Encode(Commitment memory commitment) internal pure returns (bytes memory) {
        uint256 payloadLen = commitment.payload.length;
        bytes memory accumulator = bytes("");
        for (uint256 i = 0; i < payloadLen; i++) {
            accumulator = bytes.concat(
                abi.encodePacked(commitment.payload[i].id), ScaleCodec.encodeBytes(commitment.payload[i].data)
            );
        }

        bytes memory payload = bytes.concat(ScaleCodec.encodeUintCompact(payloadLen), accumulator);

        bytes memory rest = bytes.concat(
            ScaleCodec.encode32(uint32(commitment.blockNumber)), ScaleCodec.encode64(uint64(commitment.validatorSetId))
        );

        return bytes.concat(payload, rest);
    }

    function Encode(BeefyMmrLeaf memory leaf) internal pure returns (bytes memory) {
        bytes memory first =
            bytes.concat(abi.encodePacked(uint8(leaf.version)), ScaleCodec.encode32(uint32(leaf.parentNumber)));
        bytes memory second =
            bytes.concat(bytes.concat(leaf.parentHash), ScaleCodec.encode64(uint64(leaf.nextAuthoritySet.id)));
        bytes memory third = bytes.concat(
            ScaleCodec.encode32(uint32(leaf.nextAuthoritySet.len)), bytes.concat(leaf.nextAuthoritySet.root)
        );
        return bytes.concat(bytes.concat(first, second), bytes.concat(third, bytes.concat(leaf.extra)));
    }

    function DecodeHeader(bytes memory encoded) internal pure returns (Header memory) {
        ByteSlice memory slice = ByteSlice(encoded, 0);
        bytes32 parentHash = Bytes.toBytes32(Bytes.read(slice, 32));
        uint256 blockNumber = ScaleCodec.decodeUintCompact(slice);
        bytes32 stateRoot = Bytes.toBytes32(Bytes.read(slice, 32));
        bytes32 extrinsicsRoot = Bytes.toBytes32(Bytes.read(slice, 32));

        uint256 length = ScaleCodec.decodeUintCompact(slice);
        Digest[] memory digests = new Digest[](length);

        for (uint256 i = 0; i < length; i++) {
            uint8 kind = Bytes.readByte(slice);
            Digest memory digest;
            if (kind == DIGEST_ITEM_OTHER) {
                digest.isOther = true;
            } else if (kind == DIGEST_ITEM_CONSENSUS) {
                digest.isConsensus = true;
                digest.consensus = decodeDigestItem(slice);
            } else if (kind == DIGEST_ITEM_SEAL) {
                digest.isSeal = true;
                digest.seal = decodeDigestItem(slice);
            } else if (kind == DIGEST_ITEM_PRERUNTIME) {
                digest.isPreRuntime = true;
                digest.preruntime = decodeDigestItem(slice);
            } else if (kind == DIGEST_ITEM_RUNTIME_ENVIRONMENT_UPDATED) {
                digest.isRuntimeEnvironmentUpdated = true;
            }
            digests[i] = digest;
        }

        return Header(parentHash, blockNumber, stateRoot, extrinsicsRoot, digests);
    }

    function decodeDigestItem(ByteSlice memory slice) internal pure returns (DigestItem memory) {
        bytes4 id = Bytes.toBytes4(read(slice, 4), 0);
        uint256 length = ScaleCodec.decodeUintCompact(slice);
        bytes memory data = Bytes.read(slice, length);
        return DigestItem(id, data);
    }

    function read(ByteSlice memory self, uint256 len) internal pure returns (bytes memory) {
        require(self.offset + len <= self.data.length);
        if (len == 0) {
            return "";
        }
        uint256 addr = Memory.dataPtr(self.data);
        bytes memory slice = Memory.toBytes(addr + self.offset, len);
        self.offset += len;
        return slice;
    }

    function readByte(ByteSlice memory self) internal pure returns (uint8) {
        require(self.offset + 1 <= self.data.length);

        uint8 b = uint8(self.data[self.offset]);
        self.offset += 1;

        return b;
    }

    // Decodes a SCALE encoded compact unsigned integer
    function decodeUintCompact(ByteSlice memory data) internal pure returns (uint256 v) {
        uint8 b = readByte(data); // read the first byte
        uint8 mode = b & 3; // bitwise operation

        uint256 value;
        if (mode == 0) {
            // [0, 63]
            value = b >> 2; // right shift to remove mode bits
        } else if (mode == 1) {
            // [64, 16383]
            uint8 bb = readByte(data); // read the second byte
            uint64 r = bb; // convert to uint64
            r <<= 6; // multiply by * 2^6
            r += b >> 2; // right shift to remove mode bits
            value = r;
        } else if (mode == 2) {
            // [16384, 1073741823]
            uint8 b2 = readByte(data); // read the next 3 bytes
            uint8 b3 = readByte(data);
            uint8 b4 = readByte(data);

            uint32 x1 = uint32(b) | (uint32(b2) << 8); // convert to little endian
            uint32 x2 = x1 | (uint32(b3) << 16);
            uint32 x3 = x2 | (uint32(b4) << 24);

            x3 >>= 2; // remove the last 2 mode bits
            value = uint256(x3);
        } else if (mode == 3) {
            // [1073741824, 4503599627370496]
            uint8 l = (b >> 2) + 4; // remove mode bits
            require(l <= 8, "unexpected prefix decoding Compact<Uint>");
            return ScaleCodec.decodeUint256(read(data, l));
        } else {
            revert("Code should be unreachable");
        }
        return (value);
    }

    // Convert the provided type to a bn254 field element
    function toFieldElements(bytes32 source) internal pure returns (bytes32, bytes32) {
        // is assembly cheaper?
        bytes32 left = bytes32(uint256(uint128(bytes16(source))));
        bytes32 right = bytes32(uint256(uint128(uint256(source))));

        return (left, right);
    }
}
