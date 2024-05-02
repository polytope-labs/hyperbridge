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
pragma solidity 0.8.17;

import {BaseTest} from "./BaseTest.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import "forge-std/Test.sol";
import "../src/hosts/EvmHost.sol";
import {DispatchPost} from "ismp/IDispatcher.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract EvmHostTest is BaseTest {
    using Bytes for bytes;

    // we should only be able to set consensus state multiple times on testnet
    function testSetConsensusState() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);

        // we can set consensus state
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"deadbeef");
        assert(host.consensusState().equals(hex"deadbeef"));

        // as many times as we want
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"beefdead");
        assert(host.consensusState().equals(hex"beefdead"));

        // reset it
        vm.prank(host.hostParams().admin);
        host.setConsensusState(new bytes(0));
        assert(host.consensusState().equals(new bytes(0)));

        // set chain Id to mainnet
        vm.chainId(host.chainId());
        // we can set consensus state
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"beef");
        assert(host.consensusState().equals(hex"beef"));

        // but not anymore
        vm.startPrank(host.hostParams().admin);
        vm.expectRevert("Unauthorized action");
        host.setConsensusState(hex"feeb");
        assert(host.consensusState().equals(hex"beef"));
    }

    function testSetHostParamsAdmin() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);
        assert(host.chainId() + 5 == block.chainid);

        HostParams memory params = host.hostParams();
        // we can set host params
        vm.prank(host.hostParams().admin);
        host.setHostParamsAdmin(params);

        // can't set on mainnet
        vm.chainId(host.chainId());
        vm.prank(host.hostParams().admin);
        vm.expectRevert("Cannot set params on mainnet");
        host.setHostParamsAdmin(params);
    }

    function testFundRequest() public {
        // dispatch request
        vm.prank(tx.origin);
        bytes32 commitment = host.dispatch(
            DispatchPost({
                body: new bytes(0),
                payer: tx.origin,
                fee: 0,
                dest: StateMachine.arbitrum(),
                timeout: 0,
                to: new bytes(0)
            })
        );
        assert(host.requestCommitments(commitment).fee == 0);

        // fund request
        vm.prank(tx.origin);
        host.fundRequest(commitment, 10 * 1e18);
        assert(host.requestCommitments(commitment).fee == 10 * 1e18);

        // can't fund unknown requests
        vm.expectRevert("Unknown request");
        vm.prank(tx.origin);
        host.fundRequest(keccak256(hex"dead"), 10 * 1e18);

        // another person can fund your request
        feeToken.mint(address(this), 10 * 1e18, "");
        host.fundRequest(commitment, 10 * 1e18);
    }

    function testVetoStateCommitment() public {
        // add tx.origin to fishermen
        HostParams memory params = host.hostParams();
        address[] memory fishermen = new address[](1);
        fishermen[0] = tx.origin;
        params.fishermen = fishermen;
        vm.prank(params.admin);
        host.setHostParamsAdmin(params);

        // create a state commitment
        StateMachineHeight memory height = StateMachineHeight({height: 100, stateMachineId: 2000});
        vm.prank(params.handler);
        host.storeStateMachineCommitment(
            height, StateCommitment({timestamp: 200, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );
        assert(host.stateMachineCommitment(height).timestamp == 200);

        // can't veto if not in fishermen set
        vm.expectRevert("EvmHost: Account is not in the fishermen set");
        host.vetoStateCommitment(height);

        // veto with fisherman
        vm.prank(tx.origin);
        host.vetoStateCommitment(height);
        assert(host.stateMachineCommitment(height).timestamp == 0);
    }

    function testCanAddwhitelistedStateMachines() public {
        HostParams memory params = host.hostParams();
        uint256[] memory stateMachineWhitelist = new uint256[](2);
        stateMachineWhitelist[0] = 2000;
        stateMachineWhitelist[1] = 2001;
        params.stateMachineWhitelist = stateMachineWhitelist;

        // create a state commitment
        StateMachineHeight memory height = StateMachineHeight({height: 100, stateMachineId: 2000});
        vm.prank(params.handler);
        host.storeStateMachineCommitment(
            height, StateCommitment({timestamp: 200, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );
        assert(host.stateMachineCommitment(height).timestamp == 200);
        assert(host.latestStateMachineHeight(height.stateMachineId) == 100);

        // add the new state machine
        vm.prank(params.hostManager);
        host.updateHostParams(params);
        // should be unchanged
        assert(host.latestStateMachineHeight(height.stateMachineId) == 100);
        // should be set to 1
        assert(host.latestStateMachineHeight(2001) == 1);
    }

    function testCanAddandRemoveFishermen() public {
        // add tx.origin & this to fishermen
        HostParams memory params = host.hostParams();
        address[] memory fishermen = new address[](2);
        fishermen[0] = tx.origin;
        fishermen[1] = address(this);
        params.fishermen = fishermen;
        vm.prank(params.admin);
        host.setHostParamsAdmin(params);

        // create a state commitment
        StateMachineHeight memory height = StateMachineHeight({height: 100, stateMachineId: 2000});
        vm.prank(params.handler);
        host.storeStateMachineCommitment(
            height, StateCommitment({timestamp: 200, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );
        assert(host.stateMachineCommitment(height).timestamp == 200);
        // veto with fisherman
        vm.prank(tx.origin);
        host.vetoStateCommitment(height);
        assert(host.stateMachineCommitment(height).timestamp == 0);

        // create a state commitment
        vm.prank(params.handler);
        host.storeStateMachineCommitment(
            height, StateCommitment({timestamp: 200, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );
        assert(host.stateMachineCommitment(height).timestamp == 200);
        // veto with fisherman
        host.vetoStateCommitment(height);
        assert(host.stateMachineCommitment(height).timestamp == 0);

        // remove fishermen
        address[] memory newFishermen = new address[](0);
        params.fishermen = newFishermen;
        vm.prank(params.admin);
        host.setHostParamsAdmin(params);

        // create a state commitment
        vm.prank(params.handler);
        host.storeStateMachineCommitment(
            height, StateCommitment({timestamp: 200, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );
        assert(host.stateMachineCommitment(height).timestamp == 200);

        // cannot veto
        vm.expectRevert("EvmHost: Account is not in the fishermen set");
        host.vetoStateCommitment(height);
        assert(host.stateMachineCommitment(height).timestamp == 200);

        // cannot veto
        vm.prank(tx.origin);
        vm.expectRevert("EvmHost: Account is not in the fishermen set");
        host.vetoStateCommitment(height);
        assert(host.stateMachineCommitment(height).timestamp == 200);
    }

    function testHostStateMachineId() public {
        assert(StateMachine.kusama(3000).equals(host.stateMachineId(3000)));

        HostParams memory params = host.hostParams();
        params.hyperbridge = StateMachine.polkadot(3367);
        vm.prank(params.admin);
        host.setHostParamsAdmin(params);

        assert(StateMachine.polkadot(3000).equals(host.stateMachineId(3000)));
    }
}
