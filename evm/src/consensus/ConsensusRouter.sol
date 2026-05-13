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
import {IConsensusV2} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

/**
 * @title The Consensus Router.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Routes consensus verification to the appropriate BEEFY verifier based on a
 * single-byte proof type prefix:
 *
 *   0x00 (Naive)      -> EcdsaBeefy: Verifies all secp256k1 signatures and authority set
 *                        membership proofs on-chain. Most gas-expensive but fully trustless.
 *
 *   0x01 (ZK)         -> SP1Beefy: Delegates signature verification, authority set membership,
 *                        and MMR leaf inclusion to an SP1 zero-knowledge proof. Cheapest on-chain
 *                        verification at the cost of off-chain proving.
 *
 *   0x02 (FiatShamir) -> FiatShamirBeefy: Uses the Fiat-Shamir heuristic to deterministically
 *                        sample a subset of validators to verify on-chain, reducing gas costs
 *                        compared to the naive approach while remaining fully on-chain.
 *
 * The router strips the first byte before forwarding the remaining proof bytes to the
 * selected verifier. All three verifiers implement IConsensus and IConsensusV2, and the
 * router itself exposes both interfaces.
 *
 * @dev The verifier addresses are set as immutables at construction time and cannot be changed.
 * Reverts with EmptyProof if no proof data is provided, or InvalidProofType if the prefix
 * byte is outside the valid range (0x00-0x02).
 */
contract ConsensusRouter is IConsensus, IConsensusV2, ERC165 {
    // Proof type enum
    enum ProofType {
        // 0x00 - EcdsaBeefy (full ECDSA signature verification)
        Ecdsa,
        // 0x01 - SP1Beefy (zero-knowledge proof)
        Sp1,
        // 0x02 - FiatShamirBeefy (Fiat-Shamir sampled proof)
        FiatShamir
    }

    // SP1 Beefy consensus client
    IConsensus public immutable sp1Beefy;

    // EcdsaBeefy consensus client
    IConsensus public immutable ecdsaBeefy;

    // FiatShamirBeefy consensus client
    IConsensus public immutable fiatShamirBeefy;

    // Invalid proof type provided
    error InvalidProofType(uint8 proofType);

    // Empty proof provided
    error EmptyProof();

    constructor(IConsensus _sp1Beefy, IConsensus _ecdsaBeefy, IConsensus _fiatShamirBeefy) {
        sp1Beefy = _sp1Beefy;
        ecdsaBeefy = _ecdsaBeefy;
        fiatShamirBeefy = _fiatShamirBeefy;
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensus).interfaceId || interfaceId == type(IConsensusV2).interfaceId
            || super.supportsInterface(interfaceId);
    }

    /**
     * @dev Routes to the appropriate verifier based on the first byte of the proof.
     * @param encodedState The ABI-encoded BeefyConsensusState.
     * @param encodedProof The proof prefixed with a single-byte ProofType discriminator.
     * @return The updated consensus state and any newly finalized intermediate states.
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

        if (proofType == ProofType.Sp1) {
            // Route to SP1Beefy for ZK proof verification
            return IConsensus(address(sp1Beefy)).verifyConsensus(encodedState, actualProof);
        } else if (proofType == ProofType.Ecdsa) {
            // Route to EcdsaBeefy for naive proof verification
            return IConsensus(address(ecdsaBeefy)).verifyConsensus(encodedState, actualProof);
        } else if (proofType == ProofType.FiatShamir) {
            // Route to FiatShamirBeefy for Fiat-Shamir sampled proof verification
            return IConsensus(address(fiatShamirBeefy)).verifyConsensus(encodedState, actualProof);
        } else {
            revert InvalidProofType(proofTypeByte);
        }
    }

    /**
     * @dev IConsensusV2 variant that additionally returns the latest authority set id.
     * @param previousState The ABI-encoded BeefyConsensusState.
     * @param encodedProof The proof prefixed with a single-byte ProofType discriminator.
     * @return The updated consensus state, newly finalized intermediate states, and the
     *         latest authority set id from the selected verifier.
     */
    function verify(bytes calldata previousState, bytes calldata encodedProof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        if (encodedProof.length == 0) revert EmptyProof();

        uint8 proofTypeByte = uint8(encodedProof[0]);

        if (proofTypeByte > uint8(type(ProofType).max)) {
            revert InvalidProofType(proofTypeByte);
        }

        ProofType proofType = ProofType(proofTypeByte);

        // Strip the first byte
        bytes calldata actualProof = encodedProof[1:];

        if (proofType == ProofType.Sp1) {
            return IConsensusV2(address(sp1Beefy)).verify(previousState, actualProof);
        } else if (proofType == ProofType.Ecdsa) {
            return IConsensusV2(address(ecdsaBeefy)).verify(previousState, actualProof);
        } else if (proofType == ProofType.FiatShamir) {
            return IConsensusV2(address(fiatShamirBeefy)).verify(previousState, actualProof);
        } else {
            revert InvalidProofType(proofTypeByte);
        }
    }
}
