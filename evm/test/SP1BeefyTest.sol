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
pragma solidity 0.8.20;

import "forge-std/Test.sol";
import {SP1Verifier} from "@sp1-contracts/v2.0.0/SP1VerifierPlonk.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "../src/consensus/SP1Beefy.sol";


contract SP1BeefyTest is Test {
    SP1Verifier internal sp1;

    function setUp() public virtual {
        sp1 = new SP1Verifier();
    }

    function testPolkadotVerifier() public view {
        bytes
            memory proof = hex"4aca240a2f0bf9b6c922d4c57353e59a29634035217467e193e91eadb17f3a7b7a91a3291967b4aa263ba370a0830e0d9da34e7987f60622558069d9731095937ce8f041260474e34c9abd180f2fa8f988317928ba13e25d79e268977a0cfd116cd0c3f125ab28ed01834d191fc41164f692b5c91a638cef79ad034a96c2c2890e3ac2e11b9d64740c81b0219bba352ffc494cc5f9944910e56e4dbda1850c53a205571c1c8c624fe4aacaf428a5ed186e1a464d35a92b680177bb2cb0207c49eb866a170203307dafc76f0405b1ab43127c2762cd86e85b8825e227a3723e732cfd96f100a9b0690e5338e1231e062fd23960ce080bcf0d46f5bc5c7e457d979346fa4b2d9b69f5683dd9360d9d10a51871361030d9ff02016fcde7bcb1c0a9db8378a62fb85bdb183ecdbc692e0f099a92de0b11fceffdc1f438277d8cf11d5106b13f22fee6270c1530177e8ccc0953be5e76cfb4c3e33a885e8e8dc103f89159775b1b5e5b93cabdc7481968b7decd981f5fa60651f678b2cb2df6b51d15ca24dda82516f906a72ec8353f4f49d806b102fecd2afc85102cc31bc8c6a3799233521222e7d8d862bfac8293c98c2f09de923669c3228bf4e15e50546114b6a0e96d210e05ef7f03696d7299af003f7957554abc93f6c162beaadf81adefacbfec391723d2bf77d1dceee0999aa52757540d63297026b8b3d9c41aeeac0349bf4d7bcf2592621d5a92792f5297872fc8bc1114019a59e1b0f9e03ae9cb78d06ed5063c21dd10ec61794d01a2faed5be9a80bd60174175f2ae009e92910aadb56ebe3e0087a0d2572f0e414b32f14ac9ddfe2e1b81bfb5ab16270b893017878081db7420116da1d4fd8b8d1abc2421f735ff3304fb025f9b316100d8f7fd93940c072da19cff581ca5a32b8d306284ff4d7f94bcecaba41d241ae0791ae8d1a30557d4315f58c0b4a3aae6d14323d57c225f1c2cc00232d71cb3dbab1f7f56123e253f12f9736d8d77978525b999ea60ff8cb58ab0ca9b9f952f606199e70b6ab7041bf1492c0f8424be9339a510736d417aa2f5b08282fdd247b2832d0ad53c9c58be3050e864347f375ea621d43895b79a54f46a5deb14c961402dc8d22114adb4d3e207b70aea83e63dd641188ddf276225c7ca08b80e99957e4e829a5f979f07d28266de9dcaec71fc71787348e514a2d17048ed1517fd88a4eb125dfb07f6194b7";

        bytes memory publicInputs =             hex"000000000000000000000000000000000000000000000000000000000000002067f3dc54dc343f46a92d13212d585f81026cf15b4e3ed44f32b4677e2c44742c000000000000000000000000000000000000000000000000000000000000019053e540e08e5aea29dddd9bdef3e31930a713d9e0e2da97eb4a319dff0bff4b15000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000019b99dd7a3a537811a27a58712d443203d0d57211e04a07966be9d8f174d2d654";

        bytes32 verificationKey = bytes32(0x00b3830a7bcbd368596446801391435c29bb5319827319de0acb83fb7490ef49);

        ISP1Verifier(address(sp1)).verifyProof(verificationKey, publicInputs, proof);

        PublicInputs memory inputs = abi.decode(publicInputs, (PublicInputs));

        console.log("authority: ");
        console.logBytes32(inputs.authorities_root);

        console.log("MMR Leaf: ");
        console.logBytes32(inputs.leaf_hash);

        console.log("authority length: ");
        console.log(inputs.authorities_len);
    }
}
