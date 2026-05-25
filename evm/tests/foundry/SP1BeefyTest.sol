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
    bytes32 internal verificationKey = bytes32(0x0095a067623354f4cc98faa79652b2d1e0e6c11ac235abe65ad7e45a5135ead0);

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
            hex"0000000000000000000000000000000000000000000000000000000001de0e3500000000000000000000000000000000000000000000000000000000012a53180000000000000000000000000000000000000000000000000000000000001344000000000000000000000000000000000000000000000000000000000000025831dc27bc79c0e5fc4e99d8900d13573d668a472e1d0e1ce53729448fa82512c00000000000000000000000000000000000000000000000000000000000001345000000000000000000000000000000000000000000000000000000000000025831dc27bc79c0e5fc4e99d8900d13573d668a472e1d0e1ce53729448fa82512c0";

        BeefyConsensusState memory initial = abi.decode(state, (BeefyConsensusState));

        // ABI-encoded SP1BeefyProof (abi_encode_params output; no outer offset).
        bytes memory proof =
            hex"0000000000000000000000000000000000000000000000000000000001de0e3d000000000000000000000000000000000000000000000000000000000000134400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001de0e3ce74b53a94acbdcc4aeece36a4c0785c6a2d2646f3f65bc7cb19a78d4840788440000000000000000000000000000000000000000000000000000000000001345000000000000000000000000000000000000000000000000000000000000025831dc27bc79c0e5fc4e99d8900d13573d668a472e1d0e1ce53729448fa82512c064298798662bb0861e07e05dfc74f7a83ed106f73fb90625089170cb8e1c3e1e00000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000340000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000d2700000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000139412d4391f1a149646243282eb12ad535048e899da224f4fdc3845897389473d5527974022ba08c7cd425cb6bab3b2d35acae88d3a6bf6fab235795ac03df6ec3e8c9effcb028ed52db3ca8c912f4e0b6fb2c7421ee9dcc7d6d2330ae1e3fe45b56c91543140661757261204992d60800000000045250535290681565089c893fd0952438961dc6c1cef3fb59a2351d0c094c0bb7bbbdcdbf39e63878070449534d5001010000000000000000000000000000000000000000000000000000000000000000bc36789e7a1e281436464229828f817d6612f7b477d66591ff96a9e064bcc98a044953544d2072db0e6a000000000561757261010140131f77567ddaa5e89f190abf466473c7d6254bc3ac1bb0d81a0595a353a6305cfd249460e628250f79da53a951eea20233bf840af2f4ddecce784847dabd820000000000000000000000000000000000000000000000000000000000000000000000000001644388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f25352000000000000000000000000000000000000000000000000000000000000000020b467bd1a1907e2951b90149711e993984c3b28dba9ff2f57b52d298b848c45267700a9ad82cedeba582dda1348f53f3e1e13b959092212a04db546a441551f1bfd01282476c2e2d3f0003dddf4446b182affb5c3de0113b26f4cda8abf7b77078051434cd45a1a37fcf11512df9d632ad0999df50b8b359a364eb2bd83d63e0b2d2bb5b50e73b300063d0f74ca7b7aeb5c784395784a5a3dcfccd167562482100c9ef96bd609318278de308a7c15838c0a7da26f5cc145a5d1434a4df89ead07647eba40127e056fd622957e13e746a5ff1b6540951bd4293d0d838b6fc69c2a180738486efb2bda0f6e39c7ceb7b50c1466e224e3e3a17efba358215f306c00000000000000000000000000000000000000000000000000000000";

        (bytes memory newState, IntermediateState[] memory intermediates,) = beefy.verify(state, proof);

        BeefyConsensusState memory updated = abi.decode(newState, (BeefyConsensusState));

        // Consensus advanced past the trusted height.
        assertGt(updated.latestHeight, initial.latestHeight, "latest height should advance");

        // One Hyperbridge Nexus (para_id 3367) header committed in this proof.
        assertEq(intermediates.length, 1, "expected one intermediate state");
        IntermediateState memory nexus = intermediates[0];
        assertEq(nexus.stateMachineId, 3367, "state machine id");
        assertEq(nexus.height, 10296916, "height");
        // Timestamp (seconds) is read from the ISMP `ISTM` timestamp digest.
        assertEq(nexus.commitment.timestamp, 1779358578, "timestamp");
        assertEq(
            nexus.commitment.stateRoot,
            bytes32(0xbc36789e7a1e281436464229828f817d6612f7b477d66591ff96a9e064bcc98a),
            "state root"
        );
        assertEq(nexus.commitment.overlayRoot, bytes32(0x0000000000000000000000000000000000000000000000000000000000000000), "overlay root");
    }
}
