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

import {IConsensusClient, IntermediateState} from "@polytope-labs/ismp-solidity/IConsensusClient.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "./Codec.sol";
import "./Types.sol";

/**
 * @title The SP1 BEEFY Consensus Client.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Similar to the BeefyV1 client but delegates secp256k1 signature verification, authority set membership proof checks
 * and mmr leaf to an SP1 program.
 */
contract SP1Beefy is IConsensusClient, ERC165 {
    using HeaderImpl for Header;

    // SP1 verification key
    bytes32 public verificationKey = bytes32(0x004609733a0366baf52880d2a058a858c8c83479d4b1fca39c1a14666375419f);

    // Sp1 verifier contract
    ISP1Verifier internal _verifier;

    // Provided authority set id was unknown
    error UnknownAuthoritySet();

    // Provided consensus proof height is stale
    error StaleHeight();

    // Genesis block should not be provided
    error IllegalGenesisBlock();

    constructor(ISP1Verifier verifier) {
        _verifier = verifier;
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusClient).interfaceId || super.supportsInterface(interfaceId);
    }

    /*
     * @dev Given some opaque consensus proof, produce the new consensus state and newly finalized intermediate states.
     */
    function verifyConsensus(
        bytes calldata encodedState,
        bytes calldata encodedProof
    ) external view returns (bytes memory, IntermediateState[] memory) {
        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));
        (
            MiniCommitment memory commitment,
            PartialBeefyMmrLeaf memory leaf,
            ParachainHeader[] memory headers,
            bytes memory plonkProof
        ) = abi.decode(encodedProof, (MiniCommitment, PartialBeefyMmrLeaf, ParachainHeader[], bytes));
        SP1BeefyProof memory proof = SP1BeefyProof({
            commitment: commitment,
            mmrLeaf: leaf,
            headers: headers,
            proof: plonkProof
        });

        (BeefyConsensusState memory newState, IntermediateState[] memory intermediates) = verifyConsensus(
            consensusState,
            proof
        );

        return (abi.encode(newState), intermediates);
    }

    /**
     * @dev Verifies an SP1 proof of consensus.
     */
    function verifyConsensus(
        BeefyConsensusState memory trustedState,
        SP1BeefyProof memory proof
    ) internal view returns (BeefyConsensusState memory, IntermediateState[] memory) {
        MiniCommitment memory commitment = proof.commitment;
        if (trustedState.latestHeight >= commitment.blockNumber) revert StaleHeight();

        AuthoritySetCommitment memory authority;
        if (commitment.validatorSetId == trustedState.nextAuthoritySet.id) {
            authority = trustedState.nextAuthoritySet;
        } else if (commitment.validatorSetId == trustedState.currentAuthoritySet.id) {
            authority = trustedState.currentAuthoritySet;
        } else {
            revert UnknownAuthoritySet();
        }

        uint256 headers_len = proof.headers.length;
        bytes32[] memory headers = new bytes32[](headers_len);
        for (uint256 i = 0; i < headers_len; i++) {
            headers[i] = keccak256(proof.headers[i].header);
        }

        bytes memory publicInputs = abi.encode(
            PublicInputs({
                authorities_len: authority.len,
                authorities_root: authority.root,
                headers: headers,
                leaf_hash: keccak256(Codec.Encode(proof.mmrLeaf))
            })
        );

        _verifier.verifyProof(verificationKey, publicInputs, proof.proof);

        uint256 statesLen = proof.headers.length;
        IntermediateState[] memory intermediates = new IntermediateState[](statesLen);
        for (uint256 i = 0; i < statesLen; i++) {
            ParachainHeader memory para = proof.headers[i];
            Header memory header = Codec.DecodeHeader(para.header);
            if (header.number == 0) revert IllegalGenesisBlock();
            StateCommitment memory stateCommitment = header.stateCommitment();
            IntermediateState memory intermediate = IntermediateState({
                stateMachineId: para.id,
                height: header.number,
                commitment: stateCommitment
            });
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
