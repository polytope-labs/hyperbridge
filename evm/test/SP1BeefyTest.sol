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
// import {SP1Verifier} from "@sp1-contracts/v5.0.0/SP1VerifierGroth16.sol";
import {SP1Verifier} from "@sp1-contracts/v4.0.0-rc.3/SP1VerifierGroth16.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "../src/consensus/SP1Beefy.sol";

contract SP1BeefyTest is Test {
    SP1Verifier internal sp1;

    function setUp() public virtual {
        sp1 = new SP1Verifier();
    }

    function testDecodeConsensusState() public view {
        bytes memory encodedState = hex"0000000000000000000000000000000000000000000000000000000001771a6a00000000000000000000000000000000000000000000000000000000012a5318000000000000000000000000000000000000000000000000000000000000083a00000000000000000000000000000000000000000000000000000000000001f442c444bf993527f25cdeb8cca93b6632fbcacb30cf3e037748e5ca8f39ef9ade000000000000000000000000000000000000000000000000000000000000083b00000000000000000000000000000000000000000000000000000000000001f442c444bf993527f25cdeb8cca93b6632fbcacb30cf3e037748e5ca8f39ef9ade";

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
        bytes
            memory proof = hex"11b6a09d15ead29da124f6bafcb61cef7146ee5d2be1c07f65916b5f9afbc077746a4fef207c58c5ce968578ba9ada2ed98d8596aa1eed1ad3ca336184c8da77aafb6ccc2c952a8683b310ee61d0aea58db36a9de7e2e36b81f617e26bd439b822240cfb1c136bc88c36c0312d0b88442484392339645dd0bf57085490df0fd53ecf91671d19b24cf9be4fefaa8b129ba8a542f4a8de0803a4026bf7edbb49f6040998e5097d96daa464e03f46986415e368ea89e116cb017a926d4a4baa117bc462c3221849395223a3a939ad5d0fb456d40022c889650f8d5fff8a75385129011e193027f6a7dce896661a63f2138a1fba30924506c735bbe3988e87e2397ef6f57240";

        bytes
            memory publicInputs = hex"0000000000000000000000000000000000000000000000000000000000000020bea1ea741f3a85e9d200e3dc6fd7d929a82c313566c2e3ed8e75cd141b5849830000000000000000000000000000000000000000000000000000000000000258a291c8362a0ea21efc57eae6e87a2c2d829fb2f319c102c1255e3110bcd51e600000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000137eebee40035586228fd1681702e3931395cc5fc2000bdbc86141d446dcde7f4";

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
