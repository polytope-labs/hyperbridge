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
            hex"0000000000000000000000000000000000000000000000000000000001ca14dc00000000000000000000000000000000000000000000000000000000012a531800000000000000000000000000000000000000000000000000000000000011210000000000000000000000000000000000000000000000000000000000000258a52a75e9530615b7b372b5035d6485a3743289ce29761d36e1574d4139d0420400000000000000000000000000000000000000000000000000000000000011220000000000000000000000000000000000000000000000000000000000000258a52a75e9530615b7b372b5035d6485a3743289ce29761d36e1574d4139d04204";

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
        BeefyConsensusState memory intialState = abi.decode(trustedState, (BeefyConsensusState));
        console.log("\n=== Consensus State ===");
        console.log("Latest height:", intialState.latestHeight);
        console.log("Current authority set ID:", intialState.currentAuthoritySet.id);
        console.log("Current authority set len:", intialState.currentAuthoritySet.len);
        console.log("Next authority set ID:", intialState.nextAuthoritySet.id);
        console.log("Next authority set len:", intialState.nextAuthoritySet.len);

        // Check which authority set will be used
        bool isCurrentAuthorities = relay.signedCommitment.commitment.validatorSetId == intialState.currentAuthoritySet.id;
        console.log("\nUsing current authority set:", isCurrentAuthorities);

        uint256 authoritySetLen = isCurrentAuthorities ? intialState.currentAuthoritySet.len : intialState.nextAuthoritySet.len;
        console.log("Authority set length:", authoritySetLen);

        // Count set bits manually to see if that's where it fails
        console.log("\n=== Testing countSetBits ===");
        console.log("Bitmap word 0:", signersBitmap[0]);
        console.log("Bitmap word 1:", signersBitmap[1]);
        console.log("Bitmap word 2:", signersBitmap[2]);
        console.log("Bitmap word 3:", signersBitmap[3]);

        // Try to verify - this should cause Panic(0x11)
        (bytes memory newState, IntermediateState[] memory intermediates) = consensus.verifyConsensus(trustedState, proof);

        assert(intermediates.length == 0);
        
        console.log("Verification succeeded!");
        console.log("New state length:", newState.length);

        // Decode and log the new state
        BeefyConsensusState memory updatedState = abi.decode(newState, (BeefyConsensusState));
        assert(updatedState.latestHeight > intialState.latestHeight);
        
        console.log("\n=== New Consensus State ===");
        console.log("New latest height:", updatedState.latestHeight);
        console.log("New current authority set ID:", updatedState.currentAuthoritySet.id);
        console.log("New current authority set len:", updatedState.currentAuthoritySet.len);
        console.log("New current authority set root:");
        console.logBytes32(updatedState.currentAuthoritySet.root);
        console.log("New next authority set ID:", updatedState.nextAuthoritySet.id);
        console.log("New next authority set len:", updatedState.nextAuthoritySet.len);
        console.log("New next authority set root:");
        console.logBytes32(updatedState.nextAuthoritySet.root);
    }
}
