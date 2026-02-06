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
 * @title Fiat-Shamir Transcript
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice A minimal Fiat-Shamir transcript implementation using keccak256.
 * The transcript maintains a running hash state that absorbs data and
 * squeezes out pseudo-random challenges. This is used to deterministically
 * derive random validator indices from a BEEFY commitment, enabling
 * probabilistic verification of a subset of validator signatures.
 */
library Transcript {
    struct State {
        bytes32 hash;
    }

    /**
     * @dev Initialize a new transcript with a domain separator.
     * The domain separator ensures that transcripts used in different
     * contexts produce independent random outputs.
     * @param domainSeparator The domain separation tag for this transcript context.
     * @return A new transcript state initialized with the domain separator.
     */
    function init(bytes memory domainSeparator) internal pure returns (State memory) {
        return State(keccak256(domainSeparator));
    }

    /**
     * @dev Absorb arbitrary data into the transcript.
     * This mixes the data into the running state so that subsequent
     * squeeze operations are bound to all previously absorbed data.
     * @param self The transcript state to absorb into.
     * @param data The data to absorb.
     */
    function absorb(State memory self, bytes memory data) internal pure {
        self.hash = keccak256(abi.encodePacked(self.hash, data));
    }

    /**
     * @dev Absorb a bytes32 value into the transcript.
     * @param self The transcript state to absorb into.
     * @param data The bytes32 value to absorb.
     */
    function absorbBytes32(State memory self, bytes32 data) internal pure {
        self.hash = keccak256(abi.encodePacked(self.hash, data));
    }

    /**
     * @dev Absorb a uint256 value into the transcript.
     * @param self The transcript state to absorb into.
     * @param data The uint256 value to absorb.
     */
    function absorbUint256(State memory self, uint256 data) internal pure {
        self.hash = keccak256(abi.encodePacked(self.hash, data));
    }

    /**
     * @dev Squeeze a pseudo-random bytes32 challenge from the transcript.
     * Each call produces a fresh challenge and advances the internal state,
     * so sequential calls yield independent outputs.
     * @param self The transcript state to squeeze from.
     * @return challenge A pseudo-random bytes32 derived from all absorbed data.
     */
    function squeeze(State memory self) internal pure returns (bytes32 challenge) {
        challenge = keccak256(abi.encodePacked(self.hash, bytes8("squeeze")));
        // Advance the state so the next squeeze produces a different value
        self.hash = keccak256(abi.encodePacked(self.hash, challenge));
    }

    /**
     * @dev Squeeze a pseudo-random uint256 from the transcript, reduced
     * modulo `modulus`. Useful for sampling random indices within a range.
     * @param self The transcript state to squeeze from.
     * @param modulus The upper bound (exclusive) for the output.
     * @return A pseudo-random uint256 in [0, modulus).
     */
    function squeezeIndex(State memory self, uint256 modulus) internal pure returns (uint256) {
        bytes32 challenge = squeeze(self);
        return uint256(challenge) % modulus;
    }

    /**
     * @dev Sample `count` unique random indices in [0, modulus) from the transcript.
     * Uses rejection sampling to ensure uniqueness. Reverts if count > modulus
     * (impossible to select that many unique indices).
     *
     * @param self The transcript state to squeeze from.
     * @param count The number of unique indices to sample.
     * @param modulus The upper bound (exclusive) for each index.
     * @return indices An array of `count` unique random indices, sorted ascending.
     */
    function sampleUniqueIndices(State memory self, uint256 count, uint256 modulus)
        internal
        pure
        returns (uint256[] memory indices)
    {
        require(count <= modulus, "Transcript: count exceeds modulus");

        indices = new uint256[](count);
        uint256 found = 0;

        while (found < count) {
            uint256 candidate = squeezeIndex(self, modulus);

            // Check for duplicates using a simple linear scan.
            // This is acceptable because count is small (e.g. 10).
            bool isDuplicate = false;
            for (uint256 j = 0; j < found; j++) {
                if (indices[j] == candidate) {
                    isDuplicate = true;
                    break;
                }
            }

            if (!isDuplicate) {
                indices[found] = candidate;
                found++;
            }
        }

        // Sort indices ascending (insertion sort, fine for small arrays)
        for (uint256 i = 1; i < count; i++) {
            uint256 key = indices[i];
            uint256 j = i;
            while (j > 0 && indices[j - 1] > key) {
                indices[j] = indices[j - 1];
                j--;
            }
            indices[j] = key;
        }
    }
}
