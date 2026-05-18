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

import {IConsensusV2, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import {Codec} from "./Codec.sol";
import {
    Header,
    HeaderImpl,
    AuthoritySetCommitment,
    BeefyConsensusState,
    MiniCommitment,
    ParachainHeader,
    ParachainHeaderHash,
    PartialBeefyMmrLeaf,
    PublicInputs,
    SP1BeefyProof
} from "./Types.sol";

/**
 * @title The SP1 BEEFY Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Verifies BEEFY consensus proofs using an SP1 zero-knowledge proof, offloading
 * secp256k1 signature verification, authority set membership checks, and MMR leaf
 * inclusion to an off-chain SP1 program. The on-chain contract only verifies the
 * resulting proof against a fixed verification key and a set of public inputs
 * (authority set commitment, parachain header hashes, MMR leaf hash, and block number).
 *
 * @dev The verification key is set at construction time as an immutable. Stale proofs
 * (where the commitment block number <= the trusted latest height) are treated as no-ops
 * and return the existing state with no intermediates.
 */
contract SP1Beefy is IConsensusV2, ERC165 {
    using HeaderImpl for Header;

    // SP1 verification key
    bytes32 public immutable verificationKey;

    // Sp1 verifier contract
    ISP1Verifier public immutable verifier;

    // Provided authority set id was unknown
    error UnknownAuthoritySet();

    // Genesis block should not be provided
    error IllegalGenesisBlock();

    constructor(ISP1Verifier v, bytes32 vk) {
        verifier = v;
        verificationKey = vk;
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusV2).interfaceId
            || super.supportsInterface(interfaceId);
    }

    /// @dev IConsensusV2 entry point. Decodes the proof, verifies the SP1 ZK proof, and returns
    /// the updated state along with the latest authority set id.
    function verify(bytes calldata previousState, bytes calldata proof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        BeefyConsensusState memory consensusState = abi.decode(previousState, (BeefyConsensusState));
        (
            MiniCommitment memory commitment,
            PartialBeefyMmrLeaf memory leaf,
            ParachainHeader[] memory headers,
            bytes memory proofBytes
        ) = abi.decode(proof, (MiniCommitment, PartialBeefyMmrLeaf, ParachainHeader[], bytes));
        SP1BeefyProof memory sp1Proof =
            SP1BeefyProof({commitment: commitment, mmrLeaf: leaf, headers: headers, proof: proofBytes});

        (BeefyConsensusState memory newState, IntermediateState[] memory intermediates) =
            verifyConsensus(consensusState, sp1Proof);

        return (abi.encode(newState), intermediates, newState.nextAuthoritySet.id);
    }

    /**
     * @dev Verifies an SP1 proof of consensus.
     */
    function verifyConsensus(BeefyConsensusState memory trustedState, SP1BeefyProof memory proof)
        internal
        view
        returns (BeefyConsensusState memory, IntermediateState[] memory)
    {
        MiniCommitment memory commitment = proof.commitment;
        // Stale proofs are a no-op
        if (trustedState.latestHeight >= commitment.blockNumber) {
            return (trustedState, new IntermediateState[](0));
        }

        AuthoritySetCommitment memory authority;
        if (commitment.validatorSetId == trustedState.nextAuthoritySet.id) {
            authority = trustedState.nextAuthoritySet;
        } else if (commitment.validatorSetId == trustedState.currentAuthoritySet.id) {
            authority = trustedState.currentAuthoritySet;
        } else {
            revert UnknownAuthoritySet();
        }

        uint256 headers_len = proof.headers.length;
        ParachainHeaderHash[] memory headers = new ParachainHeaderHash[](headers_len);
        for (uint256 i = 0; i < headers_len; i++) {
            headers[i] = ParachainHeaderHash({
                id: proof.headers[i].id,
                hash: keccak256(proof.headers[i].header)
            });
        }

        bytes memory publicInputs = abi.encode(
            PublicInputs({
                authorities_len: authority.len,
                authorities_root: authority.root,
                headers: headers,
                block_number: commitment.blockNumber,
                leaf_hash: keccak256(Codec.Encode(proof.mmrLeaf))
            })
        );
        verifier.verifyProof(verificationKey, publicInputs, proof.proof);

        uint256 statesLen = proof.headers.length;
        IntermediateState[] memory intermediates = new IntermediateState[](statesLen);
        for (uint256 i = 0; i < statesLen; i++) {
            ParachainHeader memory para = proof.headers[i];
            Header memory header = Codec.DecodeHeader(para.header);
            if (header.number == 0) revert IllegalGenesisBlock();

            StateCommitment memory stateCommitment = header.stateCommitment();
            IntermediateState memory intermediate =
                IntermediateState({stateMachineId: para.id, height: header.number, commitment: stateCommitment});
            intermediates[i] = intermediate;
        }

        if (proof.mmrLeaf.nextAuthoritySet.id > trustedState.nextAuthoritySet.id) {
            trustedState.currentAuthoritySet = trustedState.nextAuthoritySet;
            trustedState.nextAuthoritySet = proof.mmrLeaf.nextAuthoritySet;
        }
        trustedState.latestHeight = commitment.blockNumber;

        return (trustedState, intermediates);
    }

    // @dev so these structs are included in the abi
    function noOp(SP1BeefyProof memory s, PublicInputs memory p) external pure {}
}
