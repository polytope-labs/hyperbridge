// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";
import "../src/beefy/BeefyV1.sol";

contract BeefyConsensusClientTest is Test {
    // needs a test method so that integration-tests can detect it
    function testConsensusClient() public {}

    BeefyV1 internal beefy;

    function setUp() public virtual {
        beefy = new BeefyV1(2000);
    }

    function VerifyV1(bytes memory trustedConsensusState, bytes memory proof)
        public
        view
        returns (bytes memory, IntermediateState memory)
    {
        return beefy.verifyConsensus(trustedConsensusState, proof);
    }

    function DecodeHeader(bytes memory encoded) public pure returns (Header memory) {
        return Codec.DecodeHeader(encoded);
    }

    function EncodeLeaf(BeefyMmrLeaf memory leaf) public pure returns (bytes memory) {
        return Codec.Encode(leaf);
    }

    function EncodeCommitment(Commitment memory commitment) public pure returns (bytes memory) {
        return Codec.Encode(commitment);
    }
}
