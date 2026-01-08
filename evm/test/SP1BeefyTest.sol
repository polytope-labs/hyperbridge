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
import {SP1Verifier} from "@sp1-contracts/v5.0.0/SP1VerifierGroth16.sol";
// import {SP1Verifier} from "@sp1-contracts/v4.0.0-rc.3/SP1VerifierGroth16.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "../src/consensus/SP1Beefy.sol";

contract SP1BeefyTest is Test {
    SP1Verifier internal sp1;

    function setUp() public virtual {
        sp1 = new SP1Verifier();
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
            hex"a4594c591589c3cbab5792a9e93295644b16a2a3a2872c6ec01b9866a507d32eba27edac0bada0ad86e985e1dea2bc6781e08c850f8c3d130aa02584d640ef8207efef7818f88fd78eff0e43aef4641aff8b376f8162dc9592a712c54adb0e25db6854011255222e763eef01e6e6d308b3279a327a89b101f05de35aa87c76806e6e65f412f8b2efd64d56e5c7df9021f97187b3b67c3876a11743ff053beb0d4d139157191544e924ddeba35d6b9b314e37f399af881326e0bec405a376c8208294bb9d0d11619202c42a0ab6e9ed48918fc83af620715a8722652f554b5a3e8413e98806615b4a17ae3249c1fd8c298d7e666e6e908f78eba25a9d9a5acad5fb579229";

        bytes memory publicInputs =
            hex"0000000000000000000000000000000000000000000000000000000000000020ee305ec1c15776f3d1ff1d8c49cae9a66caeea20edc5d36a9290efde7f8f06af00000000000000000000000000000000000000000000000000000000000002583403fa8283308c2899bc5f4d67760941b77e53383323a19c50be0d8d94bd9b390000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000193f2de96c85bcb51d1880438241cc2e0850610218ac7e3a08c8f3109323d79f5";

        bytes32 verificationKey = bytes32(0x00468fb8911ecec00d7fd724627ea49172c901dfe0af8749b0af7a90c33e1948);

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
