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

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {TestConsensusClientV2} from "./TestConsensusClientV2.sol";
import {MainnetTestHost} from "./TestHost.sol";
import {OwnableHandler} from "../../src/core/OwnableHandler.sol";
import {HandlerV2} from "../../src/core/HandlerV2.sol";
import {FeeToken} from "./FeeToken.sol";
import {HostParams} from "../../src/core/EvmHost.sol";
import {HostManagerParams, HostManager} from "../../src/core/HostManager.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {IntermediateState, StateCommitment, StateMachineHeight} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {IHandlerV2} from "@hyperbridge/core/interfaces/IHandlerV2.sol";

contract OwnableHandlerTest is Test {
    uint256 internal constant PARACHAIN_ID = 2000;

    bytes internal constant NEW_CONSENSUS_STATE = hex"c0ffee";
    uint256 internal constant NEW_COMMITMENT_TIMESTAMP = 500;
    bytes32 internal constant NEW_OVERLAY_ROOT = keccak256("new_overlay_root");
    bytes32 internal constant NEW_STATE_ROOT = keccak256("new_state_root");

    address internal admin = address(0xA11CE);
    address internal stranger = address(0xB0B);

    TestConsensusClientV2 internal consensusClient;
    MainnetTestHost internal host;
    OwnableHandler internal handler;
    FeeToken internal feeToken;
    HostManager internal manager;

    function setUp() public {
        consensusClient = new TestConsensusClientV2();
        handler = new OwnableHandler(admin);
        feeToken = new FeeToken(address(this), "HyperUSD", "USD.h");

        HostManagerParams memory gParams = HostManagerParams({admin: address(this), host: address(0)});
        manager = new HostManager(gParams);
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = PARACHAIN_ID;
        HostParams memory params = HostParams({
            uniswapV2: address(0),
            admin: address(this),
            hostManager: address(manager),
            handler: address(handler),
            unStakingPeriod: 21 * (60 * 60 * 24),
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            feeToken: address(feeToken),
            hyperbridge: StateMachine.kusama(PARACHAIN_ID),
            stateMachines: stateMachines
        });
        host = new MainnetTestHost(params);

        manager.init(address(host));

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

    function _relayConsensusProof(address relayer, uint256 height) internal {
        bytes memory proof = _makeConsensusProof(PARACHAIN_ID, height, 0);

        vm.prank(relayer);
        handler.handleConsensus(host, proof);
    }

    function _stateMachineHeight(uint256 height) internal pure returns (StateMachineHeight memory) {
        return StateMachineHeight({stateMachineId: PARACHAIN_ID, height: height});
    }

    function _newCommitment() internal pure returns (StateCommitment memory) {
        return StateCommitment({
            timestamp: NEW_COMMITMENT_TIMESTAMP,
            overlayRoot: NEW_OVERLAY_ROOT,
            stateRoot: NEW_STATE_ROOT
        });
    }

    function _ownerSetsConsensusState(uint256 height) internal {
        vm.prank(admin);
        handler.setConsensusState(host, NEW_CONSENSUS_STATE, _stateMachineHeight(height), _newCommitment());
    }

    function testConstructorSetsOwner() public view {
        assertEq(handler.owner(), admin);
    }

    function testConstructorRejectsZeroOwner() public {
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableInvalidOwner.selector, address(0)));
        new OwnableHandler(address(0));
    }

    function testOwnerSetsConsensusStateAndResetsHeight() public {
        _relayConsensusProof(stranger, 100);
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 100);

        vm.warp(block.timestamp + 1 hours);
        _ownerSetsConsensusState(5);

        assertEq(keccak256(host.consensusState()), keccak256(NEW_CONSENSUS_STATE));
        assertEq(host.consensusUpdateTime(), block.timestamp);
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 5);

        StateCommitment memory stored = host.stateMachineCommitment(_stateMachineHeight(5));
        assertEq(stored.timestamp, NEW_COMMITMENT_TIMESTAMP);
        assertEq(stored.overlayRoot, NEW_OVERLAY_ROOT);
        assertEq(stored.stateRoot, NEW_STATE_ROOT);
        assertEq(host.stateMachineCommitmentUpdateTime(_stateMachineHeight(5)), block.timestamp);
    }

    function testOwnerResetLetsLowerHeightBeRelayed() public {
        _relayConsensusProof(stranger, 100);
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 100);

        vm.warp(block.timestamp + 1 hours);
        _ownerSetsConsensusState(5);

        _relayConsensusProof(stranger, 6);
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 6);
    }

    function testStrangerCannotSetConsensusState() public {
        bytes memory stateBefore = host.consensusState();
        uint256 latestHeightBefore = host.latestStateMachineHeight(PARACHAIN_ID);

        vm.prank(stranger);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, stranger));
        handler.setConsensusState(host, NEW_CONSENSUS_STATE, _stateMachineHeight(5), _newCommitment());

        assertEq(keccak256(host.consensusState()), keccak256(stateBefore));
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), latestHeightBefore);
    }

    function testHandleConsensusStaysPermissionless() public {
        bytes memory stateBefore = host.consensusState();

        _relayConsensusProof(stranger, 100);

        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 100);
        assertEq(keccak256(host.consensusState()), keccak256(abi.encode(stateBefore, uint256(0))));
    }

    function testOwnerRevivesExpiredConsensusClient() public {
        vm.warp(block.timestamp + 21 days + 1);

        bytes memory proof = _makeConsensusProof(PARACHAIN_ID, 6, 0);

        vm.prank(stranger);
        vm.expectRevert(HandlerV2.ConsensusClientExpired.selector);
        handler.handleConsensus(host, proof);

        _ownerSetsConsensusState(5);

        vm.prank(stranger);
        handler.handleConsensus(host, proof);
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 6);
    }

    function testBatchCallKeepsOwnerCheck() public {
        bytes[] memory calls = new bytes[](1);
        calls[0] = abi.encodeWithSelector(
            handler.setConsensusState.selector, host, NEW_CONSENSUS_STATE, _stateMachineHeight(5), _newCommitment()
        );

        vm.prank(stranger);
        vm.expectRevert(
            abi.encodeWithSelector(
                HandlerV2.BatchCallFailed.selector,
                0,
                abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, stranger)
            )
        );
        handler.batchCall(calls);

        vm.prank(admin);
        handler.batchCall(calls);

        assertEq(keccak256(host.consensusState()), keccak256(NEW_CONSENSUS_STATE));
        assertEq(host.latestStateMachineHeight(PARACHAIN_ID), 5);
    }

    function testSupportsInterface() public view {
        assertTrue(handler.supportsInterface(type(IHandlerV2).interfaceId));
    }
}
