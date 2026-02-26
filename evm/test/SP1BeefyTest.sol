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

import "forge-std/Test.sol";
import {SP1Verifier} from "@sp1-contracts/v6.0.0/SP1VerifierGroth16.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "../src/consensus/SP1Beefy.sol";

contract SP1BeefyTest is Test {
    SP1Verifier internal sp1;
    SP1Beefy internal beefy;
    bytes32 internal verificationKey = bytes32(0x00a5353172d0aaf457b4e1440ef826f2ba5516ce384b56f41206a3a52d892499);

    function setUp() public virtual {
        sp1 = new SP1Verifier();
        beefy = new SP1Beefy(sp1, verificationKey);
    }

    function testDecodeConsensusState() public pure {
        bytes memory encodedState =
            hex"0000000000000000000000000000000000000000000000000000000001771a6a00000000000000000000000000000000000000000000000000000000012a5318000000000000000000000000000000000000000000000000000000000000083a00000000000000000000000000000000000000000000000000000000000001f442c444bf993527f25cdeb8cca93b6632fbcacb30cf3e037748e5ca8f39ef9ade000000000000000000000000000000000000000000000000000000000000083b00000000000000000000000000000000000000000000000000000000000001f442c444bf993527f25cdeb8cca93b6632fbcacb30cf3e037748e5ca8f39ef9ade";

        BeefyConsensusState memory consensusState = abi.decode(encodedState, (BeefyConsensusState));

        console.log("latestHeight: ");
        console.log(consensusState.latestHeight);

        console.log("currentAuthoritySet.id: ");
        console.log(consensusState.currentAuthoritySet.id);
        console.log("currentAuthoritySet.len: ");
        console.log(consensusState.currentAuthoritySet.len);
        console.log("currentAuthoritySet.root: ");
        console.logBytes32(consensusState.currentAuthoritySet.root);

        console.log("nextAuthoritySet.id: ");
        console.log(consensusState.nextAuthoritySet.id);
        console.log("nextAuthoritySet.len: ");
        console.log(consensusState.nextAuthoritySet.len);
        console.log("nextAuthoritySet.root: ");
        console.logBytes32(consensusState.nextAuthoritySet.root);
    }

    function testPolkadotVerifier() public view {
        bytes memory proof =
            hex"0e78f4db0000000000000000000000000000000000000000000000000000000000000000008cd56e10c2fe24795cff1e1d1f40d3a324528d315674da45d26afb376e867000000000000000000000000000000000000000000000000000000000000000002475da2d1616711b2b9f3bfddc78edafa231274ca12a451f3a00724719d1b7511d4a3a4db0d2574ad96fd6c80a53bdbc37893de0315da056b5766b7d9289b7271e60553a21336842d96435863d83dccf08fa661a547779e11366b6e66e75d35b187fe8be724ffbdfb2b460f1efaeea5abd6073b081f2db9ec4e771e65752774b20a4c828a5522be82c714a8d14749ae5ddaa90b3c70ac65060d948f1a9f2c6622f0dbbe73d1e09ff9c453eee8a9d4a63c344383cd28c501d68f8c6f107e747e7119e7aa0a9a37da29ace9d089dc4992ec912b5a074350b3974044c2de0f3eb0c205216f4c1dabe2eb5e84a027532f811693a32396077f08afaf26746f61d5649";

        bytes memory publicInputs =
            hex"00000000000000000000000000000000000000000000000000000000000000206705674a4ce6fc9bb9bc6fae69ba9779eec3c6f9eccb62c2e8a476f8e7c0b30300000000000000000000000000000000000000000000000000000000000002587c28582d958b780d458f7992759fdf22fb839846204239911d77034170e2d1b5000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000013506e6643f4a0c1880c870e2babc5b7a8ace1ade1d0bb89d74ca05d28e15576e";

        ISP1Verifier(address(sp1)).verifyProof(verificationKey, publicInputs, proof);

        PublicInputs memory inputs = abi.decode(publicInputs, (PublicInputs));

        console.log("authority: ");
        console.logBytes32(inputs.authorities_root);

        console.log("MMR Leaf: ");
        console.logBytes32(inputs.leaf_hash);

        console.log("authority length: ");
        console.log(inputs.authorities_len);
    }

    function testVerifySp1Optional() public view {
        bytes memory state =
            hex"0000000000000000000000000000000000000000000000000000000001ca140400000000000000000000000000000000000000000000000000000000012a531800000000000000000000000000000000000000000000000000000000000011210000000000000000000000000000000000000000000000000000000000000258a52a75e9530615b7b372b5035d6485a3743289ce29761d36e1574d4139d0420400000000000000000000000000000000000000000000000000000000000011220000000000000000000000000000000000000000000000000000000000000258a52a75e9530615b7b372b5035d6485a3743289ce29761d36e1574d4139d04204";

        BeefyConsensusState memory initial = abi.decode(state, (BeefyConsensusState));

        console.log("initial.latestHeight: ", initial.latestHeight);

        bytes memory proof =
            hex"0000000000000000000000000000000000000000000000000000000001ca140c000000000000000000000000000000000000000000000000000000000000112100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001ca140b7157bbf49e23c59bf06fe177e9f10dd48354f743d7ea5effcf73dbc8da5e110500000000000000000000000000000000000000000000000000000000000011220000000000000000000000000000000000000000000000000000000000000258a52a75e9530615b7b372b5035d6485a3743289ce29761d36e1574d4139d0420409005ffc0c1c636e7d60b8190b56b63b6ff498b7ece37f494b744bc986e8373600000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001640e78f4db0000000000000000000000000000000000000000000000000000000000000000008cd56e10c2fe24795cff1e1d1f40d3a324528d315674da45d26afb376e867000000000000000000000000000000000000000000000000000000000000000002ae2743f1dfadebba6d34be6ec143fc1f496265361e8fb3fff700595b015e2090526f04ff7adb7f1f25c0fe4bb5ca4e3b3fe431c9eef000e6e87e61e65ec9dea2046d38042068b12911e0802e049717c84fb7bd7358efffcb5d2cc67c38512532090a6356f6ef29314be5894266bfa4f519a8fe5b218f3cd65ed6d45bf562baa15bfb7e1a91a04c015f4928408a300202ae30d083b00c97a82648b8308ee21d61f062d8e40b59b4a44ab4d272aeb5583e8ed3ac8c6e9b7ddf2fac7a13656674529de777ea8320892f5a2ae43e3a3fad32b4e65c435339d83800156d0ff37533c255600290c5848152f649f34a6242e9cd01e51dc4eb672b1dc56ddd53ec13aa700000000000000000000000000000000000000000000000000000000";
        (bytes memory newState, IntermediateState[] memory intermediates) = beefy.verifyConsensus(state, proof);

        assert(intermediates.length == 0);

        BeefyConsensusState memory updated = abi.decode(newState, (BeefyConsensusState));

        console.log("updated.latestHeight: ", updated.latestHeight);

        assert(updated.latestHeight > initial.latestHeight);
    }
}
