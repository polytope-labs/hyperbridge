// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";
import "ismp/IConsensusClient.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {Header} from "../src/consensus/Header.sol";
import {BeefyMmrLeaf, Commitment, Codec} from "../src/consensus/Codec.sol";

contract BeefyConsensusClientTest is Test {
    BeefyV1 internal beefy;

    function setUp() public virtual {
        beefy = new BeefyV1(2000);
    }

    function testFieldElementConversion() public pure {
        bytes32 message = 0x3d2fc8e85afd38a3b23610fae5cbcbf424ab7ba5c4b5df37241e921e4b2fb164;
        bytes32 root = 0xad19f07c487a8497b6e4f1e9296c363448a3f47bf8d72c875d822fc36d306d4f;

        (bytes32 left, bytes32 right) = Codec.toFieldElements(message);
        assert(left == 0x000000000000000000000000000000003d2fc8e85afd38a3b23610fae5cbcbf4);
        assert(right == 0x0000000000000000000000000000000024ab7ba5c4b5df37241e921e4b2fb164);

        (bytes32 limb1, bytes32 limb2) = Codec.toFieldElements(root);
        assert(limb1 == 0x00000000000000000000000000000000ad19f07c487a8497b6e4f1e9296c3634);
        assert(limb2 == 0x0000000000000000000000000000000048a3f47bf8d72c875d822fc36d306d4f);
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
