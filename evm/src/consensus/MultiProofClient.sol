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
pragma solidity ^0.8.20;

import {IConsensus, IntermediateState} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";
import {SP1Beefy} from "./SP1Beefy.sol";
import {BeefyV1} from "./BeefyV1.sol";

/**
 * @title The Multi-Proof Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Routes consensus verification to either SP1Beefy (ZK proof) or BeefyV1 (naive proof)
 * based on the first byte of the proof.
 */
contract MultiProofClient is IConsensus, ERC165 {
    // Proof type enum
    enum ProofType {
        // 0x00 - BeefyV1 naive proof
        Naive, 
        // 0x01 - SP1Beefy ZK proof
        ZK 
    }

    // SP1 Beefy consensus client
    SP1Beefy public immutable sp1Beefy;

    // BeefyV1 consensus client
    BeefyV1 public immutable beefyV1;

    // Invalid proof type provided
    error InvalidProofType(uint8 proofType);

    // Empty proof provided
    error EmptyProof();

    constructor(SP1Beefy _sp1Beefy, BeefyV1 _beefyV1) {
        sp1Beefy = _sp1Beefy;
        beefyV1 = _beefyV1;
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensus).interfaceId || super.supportsInterface(interfaceId);
    }

    /**
     * @dev Given some opaque consensus proof, routes to the appropriate verifier based on the first byte.
     * First byte 0x00 -> BeefyV1 (naive proof)
     * First byte 0x01 -> SP1Beefy (ZK proof)
     */
    function verifyConsensus(bytes calldata encodedState, bytes calldata encodedProof)
        external
        view
        returns (bytes memory, IntermediateState[] memory)
    {
        if (encodedProof.length == 0) revert EmptyProof();

        uint8 proofTypeByte = uint8(encodedProof[0]);

        // Validate proof type is within enum range
        if (proofTypeByte > uint8(type(ProofType).max)) {
            revert InvalidProofType(proofTypeByte);
        }

        ProofType proofType = ProofType(proofTypeByte);

        // Extract the actual proof data (skip the first byte)
        bytes calldata actualProof = encodedProof[1:];

        if (proofType == ProofType.ZK) {
            // Route to SP1Beefy for ZK proof verification
            return sp1Beefy.verifyConsensus(encodedState, actualProof);
        } else if (proofType == ProofType.Naive) {
            // Route to BeefyV1 for naive proof verification
            return beefyV1.verifyConsensus(encodedState, actualProof);
        } else {
            revert InvalidProofType(proofTypeByte);
        }
    }
}
