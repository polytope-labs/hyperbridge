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
import {SP1Verifier} from "@sp1-contracts/v6.1.0/SP1VerifierGroth16.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

import "../../src/consensus/SP1Beefy.sol";

contract SP1BeefyTest is Test {
    SP1Verifier internal sp1;
    SP1Beefy internal beefy;
    bytes32 internal verificationKey = bytes32(0x007d1720c695842ed647a1a72e981751f9b5e26fc5ca038523b23430a1292f08);

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

    function skip_testPolkadotVerifier() public view {
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

    function testVerifySp1Beefy() public view {
        // Initial trusted consensus state (previous_state) captured from Polkadot
        // mainnet when the proof below was generated against sp1-beefy v1.0.0.
        bytes memory state =
            hex"0000000000000000000000000000000000000000000000000000000001df6bd100000000000000000000000000000000000000000000000000000000012a5318000000000000000000000000000000000000000000000000000000000000136a00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741000000000000000000000000000000000000000000000000000000000000136b00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741";

        BeefyConsensusState memory initial = abi.decode(state, (BeefyConsensusState));

        // ABI-encoded SP1BeefyProof (abi_encode_params output; no outer offset).
        bytes memory proof =
            hex"0000000000000000000000000000000000000000000000000000000001df6bd9000000000000000000000000000000000000000000000000000000000000136a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001df6bd8b06c82d25b39550a06ab64cf89004fce1f913b27190ab108320812295591fa89000000000000000000000000000000000000000000000000000000000000136b00000000000000000000000000000000000000000000000000000000000002582cd28e2a83ddf10dbcc7da45533a44c70d5bc52be1868649ab8c30f7ec6dc741ed96e512661b155ef81e590ca5ad1bacf2ccce06e7e822ca521daa71efb4ff91000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000003608eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139557ed2657ce1e450327c6006e17e64425bb2154a7e6a55514e3d37fc7fd5d9884697790283bf3e632f74afab019365ed730a6deb0bc3e70bb229635fcf769d28febdf61f520234bde985bfdd18c0b04baf50ddae8f48860b9546184888ce393886265396140661757261209441d70800000000045250535290b43da1ab3f398f7008b0bd1374925ba70102ff77f33c1acce60e98a4e40fb8cf56af7d070449534d500101af5c78d7d0420a25ee6b68dc946d9919da3799b923cff420aa27ab1b646f355794a54ad343c04bc95bb013d63caecc98e97b65edb739cacc2c6e97f7d5aba5c9044953544d20f612176a000000000561757261010162f8da99803bec263b758f801ed06717d9af6178ac74e3c5ecba6e9cf6ab5c33e4daf9b75a18945bc3bb2221e1aceef06b67c87a0c6756872789dffbcd747b810000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f2535200000000000000000000000000000000000000000000000000000000000000002607774c88245bcad79f2414d5829f9b61771e86fb92366a5d224ba9a42cea9b16e91bc69ca90c8f455e8973ca2d522e460b371e95a8cdd298e00558bb25e39c1046ac2fb71dfc17f57e39ae5309c9522d97cc181e836aa679be1c168e25180b1556c8c21b6537ff21a57ecb73d497301f5fc9fe8f8d312d03720e401684da5e16440efe811bc61f2bfa210171efc4745d1b7461ce5593c8bcd9a9b2f6489f6e0c8cfbf59f1489e3e9a93143084cd57df0bf06cbf1fce9a7098154abcfb984a90fd4708053142c7043ce767492db2f5c0055f8791ef0cfb31173e9cac6ab47600e3953c2efe616bbd960b6048026dd1e0bb8bba4a29f3ccdb5ac21ce3899751300000000000000000000000000000000000000000000000000000000";

        (bytes memory newState, IntermediateState[] memory intermediates,) = beefy.verify(state, proof);

        BeefyConsensusState memory updated = abi.decode(newState, (BeefyConsensusState));

        // Consensus advanced past the trusted height.
        assertGt(updated.latestHeight, initial.latestHeight, "latest height should advance");

        // One Hyperbridge Nexus (para_id 3367) header committed in this proof.
        assertEq(intermediates.length, 1, "expected one intermediate state");
        IntermediateState memory nexus = intermediates[0];
        assertEq(nexus.stateMachineId, 3367, "state machine id");
        assertEq(nexus.height, 10380753, "height");
        // Timestamp (seconds) is read from the ISMP `ISTM` timestamp digest.
        assertEq(nexus.commitment.timestamp, 1779897078, "timestamp");
        assertEq(
            nexus.commitment.stateRoot,
            bytes32(0x94a54ad343c04bc95bb013d63caecc98e97b65edb739cacc2c6e97f7d5aba5c9),
            "state root"
        );
        assertEq(nexus.commitment.overlayRoot, bytes32(0xaf5c78d7d0420a25ee6b68dc946d9919da3799b923cff420aa27ab1b646f3557), "overlay root");
    }
}
