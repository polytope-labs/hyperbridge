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

    // Slot duration in milliseconds
    uint256 public constant SLOT_DURATION = 12_000;

    // The PayloadId for the mmr root.
    bytes2 public constant MMR_ROOT_PAYLOAD_ID = bytes2("mh");

    // Digest Item ID
    bytes4 public constant ISMP_CONSENSUS_ID = bytes4("ISMP");

    // ConsensusID for aura
    bytes4 public constant AURA_CONSENSUS_ID = bytes4("aura");

    // SP1 verification key
    bytes32 verificationKey = bytes32(0x00b3830a7bcbd368596446801391435c29bb5319827319de0acb83fb7490ef49);

    // Sp1 verifier contract
    ISP1Verifier internal _verifier;

    // Provided authority set id was unknown
    error UnknownAuthoritySet();

    // Provided consensus proof height is stale
    error StaleHeight();

    // Provided ultra plonk proof was invalid
    error InvalidUltraPlonkProof();

    // Mmr root hash was not found in header digests
    error MmrRootHashMissing();

    // Provided Mmr Proof was invalid
    error InvalidMmrProof();

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

    function verifyConsensus(
        bytes memory encodedState,
        bytes memory encodedProof
    ) external view returns (bytes memory, IntermediateState[] memory) {
        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));
        SP1BeefyProof memory proof = abi.decode(encodedProof, (SP1BeefyProof));

        (BeefyConsensusState memory newState, IntermediateState[] memory intermediates) = verifyConsensus(
            consensusState,
            proof
        );

        return (abi.encode(newState), intermediates);
    }

    /**
     * @dev Verify the consensus proof and return the new trusted consensus state and any
     * intermediate states finalized by this consensus proof.
     */
    function verifyConsensus(
        BeefyConsensusState memory trustedState,
        SP1BeefyProof memory proof
    ) internal view returns (BeefyConsensusState memory, IntermediateState[] memory) {
        Commitment memory commitment = proof.commitment;
        uint256 latestHeight = commitment.blockNumber;
        if (trustedState.latestHeight >= latestHeight) revert StaleHeight();

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

        trustedState.latestHeight = latestHeight;

        return (trustedState, intermediates);
    }
}
