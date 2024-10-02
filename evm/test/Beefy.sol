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

import "forge-std/Test.sol";
import "@polytope-labs/ismp-solidity/IConsensusClient.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {Header} from "../src/consensus/Header.sol";
import {BeefyMmrLeaf, Commitment, Codec, PartialBeefyMmrLeaf} from "../src/consensus/Codec.sol";

contract BeefyConsensusClientTest is Test {
    BeefyV1 internal beefy;

    function setUp() public virtual {
        beefy = new BeefyV1();
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

    function VerifyV1(
        bytes memory trustedConsensusState,
        bytes memory proof
    ) public view returns (bytes memory, IntermediateState[] memory) {
        return beefy.verifyConsensus(trustedConsensusState, proof);
    }

    function DecodeHeader(bytes memory encoded) public pure returns (Header memory) {
        return Codec.DecodeHeader(encoded);
    }

    function EncodeLeaf(PartialBeefyMmrLeaf memory leaf) public pure returns (bytes memory) {
        return Codec.Encode(leaf);
    }

    function EncodeCommitment(Commitment memory commitment) public pure returns (bytes memory) {
        return Codec.Encode(commitment);
    }
}
