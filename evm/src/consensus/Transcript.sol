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
     * Uses rejection sampling with sorted insertion to ensure uniqueness and
     * maintain sorted order in a single pass. Binary search is used for duplicate
     * detection, reducing complexity from O(n²) to O(n log n).
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

            // Binary search to find insertion position and check for duplicates
            // Since the array is kept sorted, we can use binary search for O(log n) lookup
            (bool isDuplicate, uint256 insertPos) = binarySearchInsertPos(indices, found, candidate);

            if (!isDuplicate) {
                // Shift elements to the right to make room for insertion
                unchecked {
                    for (uint256 k = found; k > insertPos; --k) {
                        indices[k] = indices[k - 1];
                    }
                }
                indices[insertPos] = candidate;
                unchecked {
                    ++found;
                }
            }
        }
    }

    /**
     * @dev Binary search to find the insertion position for a value in a sorted array.
     * Also returns whether the value already exists (is a duplicate).
     *
     * @param arr The sorted array to search.
     * @param len The number of valid elements in the array.
     * @param value The value to search for.
     * @return isDuplicate True if the value already exists in the array.
     * @return insertPos The position where the value should be inserted to maintain sorted order.
     */
    function binarySearchInsertPos(uint256[] memory arr, uint256 len, uint256 value)
        internal
        pure
        returns (bool isDuplicate, uint256 insertPos)
    {
        if (len == 0) {
            return (false, 0);
        }

        uint256 left = 0;
        uint256 right = len;

        unchecked {
            while (left < right) {
                uint256 mid = (left + right) >> 1; // (left + right) / 2

                if (arr[mid] == value) {
                    return (true, mid);
                } else if (arr[mid] < value) {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }
        }

        return (false, left);
    }
}
