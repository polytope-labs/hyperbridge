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
import "../src/hosts/EvmHost.sol";

import {BaseTest} from "./BaseTest.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {DispatchPost} from "@polytope-labs/ismp-solidity/IDispatcher.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";

contract EvmHostTest is BaseTest {
    using Message for PostRequest;
    using Bytes for bytes;

    // we should only be able to set consensus state multiple times on testnet
    function testSetConsensusState() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);
        StateMachineHeight memory height = StateMachineHeight({height: 100, stateMachineId: 2000});
        StateCommitment memory commitment = StateCommitment({
            timestamp: 200,
            overlayRoot: bytes32(0),
            stateRoot: bytes32(0)
        });

        // we can set consensus state
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"deadbeef", height, commitment);
        assert(host.consensusState().equals(hex"deadbeef"));

        // as many times as we want
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"beefdead", height, commitment);
        assert(host.consensusState().equals(hex"beefdead"));

        // reset it
        vm.prank(host.hostParams().admin);
        host.setConsensusState(new bytes(0), height, commitment);
        assert(host.consensusState().equals(new bytes(0)));

        // set chain Id to mainnet
        vm.chainId(host.chainId());
        // we can set consensus state
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"beef", height, commitment);
        assert(host.consensusState().equals(hex"beef"));

        // but not anymore
        vm.startPrank(host.hostParams().admin);
        vm.expectRevert(EvmHost.UnauthorizedAction.selector);
        host.setConsensusState(hex"feeb", height, commitment);
        assert(host.consensusState().equals(hex"beef"));
    }

    function testSetHostParamsAdmin() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);
        assert(host.chainId() + 5 == block.chainid);

        HostParams memory params = host.hostParams();
        // we can set host params
        vm.prank(host.hostParams().admin);
        host.updateHostParams(params);

        // can't set on mainnet
        vm.chainId(host.chainId());
        vm.prank(host.hostParams().admin);
        vm.expectRevert(EvmHost.UnauthorizedAction.selector);
        host.updateHostParams(params);
    }

    function testSweepFeeTokenBeforeUpdate() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);
        assert(host.chainId() + 5 == block.chainid);

        feeToken.mint(address(host), 1 * 1e18);
        HostParams memory params = host.hostParams();
        params.feeToken = address(this);
        // we can't set host params
        vm.prank(host.hostParams().admin);
        vm.expectRevert(EvmHost.CannotChangeFeeToken.selector);
        host.updateHostParams(params);

        feeToken.burn(address(host), 1 * 1e18);
        // we can't set host params
        vm.prank(host.hostParams().admin);
        host.updateHostParams(params);
        assert(host.hostParams().feeToken == address(this));
    }

    function testCannotDispatchWithFrozenHost() public {
        host.setFrozenState(FrozenStatus.Outgoing);
        vm.expectRevert(EvmHost.FrozenHost.selector);
        host.dispatch(
            DispatchPost({
                body: abi.encodePacked(bytes32(0)),
                payer: address(this),
                fee: 0,
                dest: StateMachine.evm(421614),
                timeout: 0,
                to: abi.encode(bytes32(0))
            })
        );

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: new bytes(0),
            to: abi.encodePacked(address(this)),
            timeoutTimestamp: 0,
            body: bytes.concat(hex"01", abi.encode(host.hostParams()))
        });
        vm.expectRevert(EvmHost.FrozenHost.selector);
        host.dispatch(
            DispatchPostResponse({
                request: request,
                response: abi.encode(bytes32(0)),
                fee: 0,
                timeout: 0,
                payer: address(this)
            })
        );

        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encode(address(this));
        vm.expectRevert(EvmHost.FrozenHost.selector);
        host.dispatch(
            DispatchGet({
                dest: StateMachine.evm(97),
                height: 100,
                keys: keys,
                context: new bytes(0),
                timeout: 60 * 60,
                fee: 0
            })
        );

        vm.prank(host.hostParams().handler);
        host.setFrozenState(FrozenStatus.None);

        feeToken.mint(address(this), 32 * host.perByteFee(StateMachine.evm(97)));
        bytes32 commitment = host.dispatch(
            DispatchPost({
                body: abi.encodePacked(bytes32(0)),
                payer: address(this),
                fee: 0,
                dest: StateMachine.evm(421614),
                timeout: 0,
                to: abi.encode(bytes32(0))
            })
        );

        assert(host.requestCommitments(commitment).sender == address(this));
    }

    function testFundRequest() public {
        // dispatch request
        vm.prank(tx.origin);
        bytes32 commitment = host.dispatch(
            DispatchPost({
                body: new bytes(0),
                payer: tx.origin,
                fee: 0,
                dest: StateMachine.evm(421614),
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
        vm.expectRevert(EvmHost.UnknownRequest.selector);
        vm.prank(tx.origin);
        host.fundRequest(keccak256(hex"dead"), 10 * 1e18);

        // someone else can fund your request
        feeToken.mint(address(this), 10 * 1e18);
        host.fundRequest(commitment, 10 * 1e18);
    }

    function testMinimumMessagingFee() public {
        bytes memory hyperbridge = host.host();
        // dispatch request
        vm.expectRevert("ERC20: transfer amount exceeds balance");
        host.dispatch(
            DispatchPost({
                body: new bytes(0), // empty body
                payer: msg.sender,
                fee: 0,
                dest: hyperbridge,
                timeout: 0,
                to: abi.encode(address(this))
            })
        );

        feeToken.mint(address(this), host.perByteFee(StateMachine.evm(97)) * 32);
        bytes32 commitment = host.dispatch(
            DispatchPost({
                body: new bytes(0), // empty body
                payer: msg.sender,
                fee: 0,
                dest: hyperbridge,
                timeout: 0,
                to: abi.encode(address(this))
            })
        );
        // charges minimum fee
        assert(host.requestCommitments(commitment).sender == msg.sender);

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: new bytes(0),
            to: abi.encodePacked(address(manager)),
            timeoutTimestamp: 0,
            body: bytes.concat(hex"01", abi.encode(host.hostParams()))
        });
        vm.prank(address(handler));
        host.dispatchIncoming(request, address(this));
        assert(host.requestReceipts(request.hash()) == address(this));

        feeToken.mint(address(manager), host.perByteFee(StateMachine.evm(97)) * 32);
        vm.prank(address(manager));
        feeToken.approve(address(host), type(uint256).max);
        vm.prank(address(manager));
        bytes32 resp = host.dispatch(
            DispatchPostResponse({
                request: request,
                response: new bytes(0),
                fee: 0,
                timeout: 0,
                payer: address(manager)
            })
        );
        assert(host.responseCommitments(resp).sender == address(manager));
        assert(feeToken.balanceOf(address(host)) == host.perByteFee(StateMachine.evm(97)) * 32 * 2);
    }

    function testCanAddwhitelistedStateMachines() public {
        HostParams memory params = host.hostParams();
        uint256[] memory stateMachines = new uint256[](2);
        stateMachines[0] = 2000;
        stateMachines[1] = 2001;
        params.stateMachines = stateMachines;

        // create a state commitment
        StateMachineHeight memory height = StateMachineHeight({height: 100, stateMachineId: 2000});
        vm.prank(params.handler);
        host.storeStateMachineCommitment(
            height,
            StateCommitment({timestamp: 200, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );

        vm.prank(params.handler);
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

    function testHostStateMachineId() public {
        assert(StateMachine.kusama(3000).equals(bytes(host.stateMachineId(host.hyperbridge(), 3000))));

        HostParams memory params = host.hostParams();
        params.hyperbridge = StateMachine.polkadot(3367);
        vm.prank(params.admin);
        host.updateHostParams(params);

        assert(StateMachine.polkadot(3000).equals(bytes(host.stateMachineId(host.hyperbridge(), 3000))));
    }
}
