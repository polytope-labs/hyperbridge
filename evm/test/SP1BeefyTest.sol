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
import {SP1Verifier} from "@sp1-contracts/v4.0.0-rc.3/SP1VerifierGroth16.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "../src/consensus/SP1Beefy.sol";

contract SP1BeefyTest is Test {
    SP1Verifier internal sp1;

    function setUp() public virtual {
        sp1 = new SP1Verifier();
    }

    function testPolkadotVerifier() public view {
        bytes
            memory proof = hex"11b6a09d0602783a739ff23a34879d0f31c9742293407605ad35b2ab9f2112445436251c2929a1f895ee9c0a7185734a57da30feaa42de756ef44ac2383ad0146f525ed41ea56e5485fe3d329cec126256c2c19918560e93fea6e69d6c1287aba55359e020f8368e1be185cabcaf87407e6e700be708bb4f0e3d800fea43e7e06f95d8a7141f29d97bbdec0ead4c051dc1a8ff931bf470e7f48bf8edba580d1a1e7f93bf2c7fc4b3124ccbbcaa0cb0243a3be77001d71fc4eb020bfce79d88ce520a9cf611b8032714b5f7429e0bd89c76474f67e0b8f02c0a1cc9fd3cbc8fdf6039a89904236d309f7153c388b5c66d5e5bdc484f4272b71aa246b121bbac05fd50bbe5";

        bytes
            memory publicInputs = hex"00000000000000000000000000000000000000000000000000000000000000209b5eebd2ca5ae7248ba20456bd8866e6a233ea23c4079303c5b19971f05c40c900000000000000000000000000000000000000000000000000000000000001f38055badaa1bf16bbbb8cd6fe066815bb9e1f23ae46d9641cb88d88aeff1b9569000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000016482008a0af7995d30781c057b8decb26eac6bcebf646283fe386a919b833822";

        bytes32 verificationKey = bytes32(0x004609733a0366baf52880d2a058a858c8c83479d4b1fca39c1a14666375419f);

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
