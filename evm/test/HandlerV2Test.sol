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
import {PingModule} from "../src/utils/PingModule.sol";
import {HandlerV2} from "../src/core/HandlerV2.sol";
import {FeeToken} from "./FeeToken.sol";
import {MockUSCDC} from "./MockUSDC.sol";
import {HostParams, PerByteFee} from "../src/core/EvmHost.sol";
import {HostManagerParams, HostManager} from "../src/core/HostManager.sol";
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
} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {IHandlerV2} from "@hyperbridge/core/interfaces/IHandlerV2.sol";

contract HandlerV2Test is Test {
    using Message for PostRequest;

    TestConsensusClientV2 internal consensusClient;
    TestHost internal host;
    HandlerV2 internal handler;
    PingModule internal testModule;
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
        PerByteFee[] memory perByteFees = new PerByteFee[](0);
        HostParams memory params = HostParams({
            uniswapV2: address(0),
            perByteFees: perByteFees,
            admin: address(this),
            hostManager: address(manager),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 21 * (60 * 60 * 24),
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            defaultPerByteFee: 1000000000000000000,
            stateCommitmentFee: 10 * 1e18,
            feeToken: address(feeToken),
            hyperbridge: StateMachine.kusama(paraId),
            stateMachines: stateMachines
        });
        host = new TestHost(params);

        testModule = new PingModule(address(this));
        uint256 oldTime = block.timestamp;
        vm.warp(100_000);
        testModule.setIsmpHost(address(host), address(0));
        vm.warp(oldTime);

        manager.setIsmpHost(address(host));

        feeToken.superApprove(address(tx.origin), address(host));
        feeToken.superApprove(address(this), address(host));
        feeToken.superApprove(address(testModule), address(host));

        vm.chainId(1);
    }


    function _makeConsensusProof(uint256 stateMachineId, uint256 height, uint256 epoch)
        internal
        pure
        returns (bytes memory)
    {
        IntermediateState memory intermediate = IntermediateState({
            stateMachineId: stateMachineId,
            height: height,
            commitment: StateCommitment({timestamp: 20000, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        });

        bytes memory innerProof = abi.encode(bytes("newState"), intermediate, epoch);
        return abi.encode(bytes("proofId"), innerProof);
    }

    function testBatchCallEmpty() public {
        bytes[] memory calls = new bytes[](0);
        handler.batchCall(calls);
    }

    function testBatchCallSingleConsensus() public {
        bytes memory proof = _makeConsensusProof(2000, 1, 0);

        bytes[] memory calls = new bytes[](1);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, proof);

        vm.prank(tx.origin);
        handler.batchCall(calls);
    }

    function testHandleConsensusV2StoresState() public {
        bytes memory proof = _makeConsensusProof(2000, 1, 0);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        bytes memory stored = host.consensusState();
        assertEq(keccak256(stored), keccak256(bytes("newState")));
    }

    function testHandleConsensusV2RecordsRelayerOnEpochChange() public {
        // epoch must be exactly currentEpoch + 1, starting from 0
        bytes memory proof = _makeConsensusProof(2000, 1, 1);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        assertEq(handler.relayerOf(1), tx.origin);
        assertEq(handler.currentEpoch(), 1);
    }

    function testHandleConsensusV2NoEpochChange() public {
        bytes memory proof = _makeConsensusProof(2000, 1, 0);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        assertEq(handler.relayerOf(0), address(0));
    }

    function testRelayerOfUnknownEpoch() public view {
        assertEq(handler.relayerOf(999), address(0));
    }

    function testBatchCallRevertsAtomically() public {
        bytes memory validProof = _makeConsensusProof(2000, 1, 1);

        // second call is invalid (empty proof)
        bytes[] memory calls = new bytes[](2);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, validProof);
        calls[1] = abi.encodeWithSelector(handler.handleConsensus.selector, host, bytes(""));

        vm.prank(tx.origin);
        vm.expectRevert();
        handler.batchCall(calls);

        // relayer mapping should not have been set since batch reverted
        assertEq(handler.relayerOf(1), address(0));
    }

    function testBatchCallPreservesMsgSender() public {
        bytes memory proof = _makeConsensusProof(2000, 1, 1);

        bytes[] memory calls = new bytes[](1);
        calls[0] = abi.encodeWithSelector(handler.handleConsensus.selector, host, proof);

        address relayer = address(0xBEEF);
        vm.prank(relayer);
        handler.batchCall(calls);

        assertEq(handler.relayerOf(1), relayer);
    }

    function testSupportsInterfaceV2() public view {
        assertTrue(handler.supportsInterface(type(IHandlerV2).interfaceId));
    }

    function testBackwardCompatDirectCall() public {
        bytes memory proof = _makeConsensusProof(2000, 1, 1);

        vm.prank(tx.origin);
        handler.handleConsensus(host, proof);

        assertEq(handler.relayerOf(1), tx.origin);
    }

    function testInvalidEpochReverts() public {
        // skip epoch 1, try to go straight to 2 — should revert
        bytes memory proof = _makeConsensusProof(2000, 1, 2);

        vm.prank(tx.origin);
        vm.expectRevert(abi.encodeWithSelector(HandlerV2.InvalidEpoch.selector, 1, 2));
        handler.handleConsensus(host, proof);
    }

    function testSequentialEpochs() public {
        // epoch 1
        bytes memory proof1 = _makeConsensusProof(2000, 1, 1);
        vm.prank(tx.origin);
        handler.handleConsensus(host, proof1);
        assertEq(handler.currentEpoch(), 1);

        // epoch 2
        bytes memory proof2 = _makeConsensusProof(2000, 2, 2);
        vm.prank(tx.origin);
        handler.handleConsensus(host, proof2);
        assertEq(handler.currentEpoch(), 2);
        assertEq(handler.relayerOf(2), tx.origin);
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
