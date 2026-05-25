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

import {TestConsensusClientV2} from "./TestConsensusClientV2.sol";
import {TestHost} from "./TestHost.sol";
import {HandlerV2} from "../../src/core/HandlerV2.sol";
import {FeeToken} from "./FeeToken.sol";
import {MockUSCDC} from "./MockUSDC.sol";
import {HostParams} from "../../src/core/EvmHost.sol";
import {HostManagerParams, HostManager} from "../../src/core/HostManager.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {
    PostRequestMessage,
    PostRequest,
    PostRequestLeaf,
    Proof,
    Message
} from "@hyperbridge/core/libraries/Message.sol";
import {
    IntermediateState,
    StateCommitment,
    StateMachineHeight
} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {IHandlerV2} from "@hyperbridge/core/interfaces/IHandlerV2.sol";

contract HandlerV2Test is Test {
    using Message for PostRequest;

    TestConsensusClientV2 internal consensusClient;
    TestHost internal host;
    HandlerV2 internal handler;
    FeeToken internal feeToken;
    HostManager internal manager;

    function setUp() public virtual {
        consensusClient = new TestConsensusClientV2();
        handler = new HandlerV2();
        feeToken = new FeeToken(address(this), "HyperUSD", "USD.h");

        uint256 paraId = 2000;
        HostManagerParams memory gParams = HostManagerParams({admin: address(this), host: address(0)});
        manager = new HostManager(gParams);
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = paraId;
        HostParams memory params = HostParams({
            uniswapV2: address(0),
            admin: address(this),
            hostManager: address(manager),
            handler: address(handler),
            unStakingPeriod: 21 * (60 * 60 * 24),
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            feeToken: address(feeToken),
            hyperbridge: StateMachine.kusama(paraId),
            stateMachines: stateMachines
        });
        host = new TestHost(params);

        manager.setIsmpHost(address(host));

        feeToken.superApprove(address(tx.origin), address(host));
        feeToken.superApprove(address(this), address(host));

        vm.chainId(1);
    }


    function _makeConsensusProof(uint256 stateMachineId, uint256 height, uint256 nextAuthoritySetId)
        internal
        pure
        returns (bytes memory)
    {
        IntermediateState memory intermediate = IntermediateState({
            stateMachineId: stateMachineId,
            height: height,
            commitment: StateCommitment({timestamp: 20000, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        });

        return abi.encode(intermediate, nextAuthoritySetId);
    }

    function testBatchCallEmpty() public {
        bytes[] memory calls = new bytes[](0);
        handler.batchCall(calls);
    }

    function testBatchCallSingleConsensus() public {
        // no epoch change: both IDs are 0
        bytes memory proof = _makeConsensusProof(2000, 1, 0);

        bytes[] memory calls = new bytes[](1);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, proof);

        vm.prank(tx.origin);
        handler.batchCall(calls);
    }

    function testHandleConsensusV2StoresState() public {
        bytes memory stateBefore = host.consensusState();
        bytes memory proof = _makeConsensusProof(2000, 1, 0);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        // TestConsensusClientV2.verify returns `abi.encode(previousState, nextAuthoritySetId)`
        // as the new state so handleConsensus can observe a state change.
        bytes memory stateAfter = host.consensusState();
        assertEq(keccak256(stateAfter), keccak256(abi.encode(stateBefore, uint256(0))));
    }

    function testHandleConsensusV2RecordsRelayerOnEpochChange() public {
        // next authority set is 2 → relayer is credited for the just-ended epoch 1
        bytes memory proof = _makeConsensusProof(2000, 1, 2);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        assertEq(host.relayerOf(1), tx.origin);
        assertEq(host.currentEpoch(), 1);
    }

    function testHandleConsensusV2NoEpochChange() public {
        // nextAuthoritySetId of 0 means no rotation has occurred
        bytes memory proof = _makeConsensusProof(2000, 1, 0);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        assertEq(host.relayerOf(0), address(0));
        assertEq(host.currentEpoch(), 0);
    }

    function testRelayerOfUnknownEpoch() public view {
        assertEq(host.relayerOf(999), address(0));
    }

    function testBatchCallRevertsAtomically() public {
        bytes memory validProof = _makeConsensusProof(2000, 1, 2);

        // second call is invalid (empty proof)
        bytes[] memory calls = new bytes[](2);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, validProof);
        calls[1] = abi.encodeWithSelector(handler.handleConsensus.selector, host, bytes(""));

        vm.prank(tx.origin);
        vm.expectRevert();
        handler.batchCall(calls);

        // relayer mapping should not have been set since batch reverted
        assertEq(host.relayerOf(1), address(0));
    }

    function testBatchCallPreservesMsgSender() public {
        // next authority set is 2 → relayer is credited for the just-ended epoch 1
        bytes memory proof = _makeConsensusProof(2000, 1, 2);

        bytes[] memory calls = new bytes[](1);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, proof);

        address relayer = address(0xBEEF);
        vm.prank(relayer);
        handler.batchCall(calls);

        assertEq(host.relayerOf(1), relayer);
    }

    function testSupportsInterfaceV2() public view {
        assertTrue(handler.supportsInterface(type(IHandlerV2).interfaceId));
    }

    function testBackwardCompatDirectCall() public {
        bytes memory proof = _makeConsensusProof(2000, 1, 2);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        assertEq(host.relayerOf(1), tx.origin);
    }

    function testStaleEpochIgnored() public {
        // first, advance to epoch 1 (nextAuthoritySetId = 2)
        bytes memory proof1 = _makeConsensusProof(2000, 1, 2);
        vm.prank(tx.origin);
        handler.handleConsensus(host, proof1);
        assertEq(host.currentEpoch(), 1);

        // submit proof with same nextAuthoritySetId — should not update relayer
        address otherRelayer = address(0xDEAD);
        bytes memory proof2 = _makeConsensusProof(2000, 2, 2);
        vm.prank(otherRelayer);
        handler.handleConsensus(host, proof2);

        // epoch unchanged, relayer for epoch 1 still the original
        assertEq(host.currentEpoch(), 1);
        assertEq(host.relayerOf(1), tx.origin);
    }

    function testSequentialEpochs() public {
        // epoch 0 -> 1 (nextAuthoritySetId = 2)
        bytes memory proof1 = _makeConsensusProof(2000, 1, 2);
        vm.prank(tx.origin);
        handler.handleConsensus(host, proof1);
        assertEq(host.currentEpoch(), 1);

        // epoch 1 -> 2 (nextAuthoritySetId = 3)
        bytes memory proof2 = _makeConsensusProof(2000, 2, 3);
        vm.prank(tx.origin);
        handler.handleConsensus(host, proof2);
        assertEq(host.currentEpoch(), 2);
        assertEq(host.relayerOf(2), tx.origin);
    }


    function BatchConsensusAndPostRequest(bytes memory consensusProof, PostRequestMessage memory message) public {
        bytes[] memory calls = new bytes[](2);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, consensusProof);
        calls[1] = abi.encodeWithSelector(handler.handlePostRequests.selector, host, message);

        vm.prank(tx.origin);
        handler.batchCall(calls);
        vm.warp(10);

        bytes32 commitment = message.requests[0].request.hash();
        assert(host.requestReceipts(commitment) != address(0));
    }
}
