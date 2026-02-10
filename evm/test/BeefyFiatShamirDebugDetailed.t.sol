// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "../src/consensus/BeefyV1FiatShamir.sol";
import {RelayChainProof, ParachainProof, BeefyConsensusState} from "../src/consensus/Types.sol";

contract BeefyFiatShamirDebugDetailedTest is Test {
    BeefyV1FiatShamir public consensus;

    function setUp() public {
        consensus = new BeefyV1FiatShamir();
    }

    function testDebugWithManualDecode() public view {
        // The consensus state from the TRON test
        bytes memory trustedState =
            hex"0000000000000000000000000000000000000000000000000000000001c7518b00000000000000000000000000000000000000000000000000000000012a531800000000000000000000000000000000000000000000000000000000000010d5000000000000000000000000000000000000000000000000000000000000025837755bf75b316fc437f3e3a9b02ca51432c7db995b2b5504b5f1185802a0d72c00000000000000000000000000000000000000000000000000000000000010d6000000000000000000000000000000000000000000000000000000000000025837755bf75b316fc437f3e3a9b02ca51432c7db995b2b5504b5f1185802a0d72c";

        // Read the proof (already stripped of the 0x02 prefix byte)
        string memory proofStr = vm.readFile("test/consensus_proof.txt");
        bytes memory proof = vm.parseBytes(proofStr);

        console.log("Proof length:", proof.length);

        // Decode the proof to see what's inside
        (RelayChainProof memory relay, ParachainProof memory p, uint256[4] memory signersBitmap) =
            abi.decode(proof, (RelayChainProof, ParachainProof, uint256[4]));

        console.log("=== Decoded Proof ===");
        console.log("Block number:", relay.signedCommitment.commitment.blockNumber);
        console.log("Validator set ID:", relay.signedCommitment.commitment.validatorSetId);
        console.log("Number of votes:", relay.signedCommitment.votes.length);

        // Decode consensus state
        BeefyConsensusState memory state = abi.decode(trustedState, (BeefyConsensusState));
        console.log("\n=== Consensus State ===");
        console.log("Latest height:", state.latestHeight);
        console.log("Current authority set ID:", state.currentAuthoritySet.id);
        console.log("Current authority set len:", state.currentAuthoritySet.len);
        console.log("Next authority set ID:", state.nextAuthoritySet.id);
        console.log("Next authority set len:", state.nextAuthoritySet.len);

        // Check which authority set will be used
        bool isCurrentAuthorities = relay.signedCommitment.commitment.validatorSetId == state.currentAuthoritySet.id;
        console.log("\nUsing current authority set:", isCurrentAuthorities);

        uint256 authoritySetLen = isCurrentAuthorities ? state.currentAuthoritySet.len : state.nextAuthoritySet.len;
        console.log("Authority set length:", authoritySetLen);

        // Count set bits manually to see if that's where it fails
        console.log("\n=== Testing countSetBits ===");
        console.log("Bitmap word 0:", signersBitmap[0]);
        console.log("Bitmap word 1:", signersBitmap[1]);
        console.log("Bitmap word 2:", signersBitmap[2]);
        console.log("Bitmap word 3:", signersBitmap[3]);

        // Try to verify - this should cause Panic(0x11)
        (bytes memory newState,) = consensus.verifyConsensus(trustedState, proof);

        console.log("Verification succeeded!");
        console.log("New state length:", newState.length);

        // Decode and log the new state
        BeefyConsensusState memory newBeefyState = abi.decode(newState, (BeefyConsensusState));
        console.log("\n=== New Consensus State ===");
        console.log("New latest height:", newBeefyState.latestHeight);
        console.log("New current authority set ID:", newBeefyState.currentAuthoritySet.id);
        console.log("New current authority set len:", newBeefyState.currentAuthoritySet.len);
        console.log("New current authority set root:");
        console.logBytes32(newBeefyState.currentAuthoritySet.root);
        console.log("New next authority set ID:", newBeefyState.nextAuthoritySet.id);
        console.log("New next authority set len:", newBeefyState.nextAuthoritySet.len);
        console.log("New next authority set root:");
        console.logBytes32(newBeefyState.nextAuthoritySet.root);
    }
}
