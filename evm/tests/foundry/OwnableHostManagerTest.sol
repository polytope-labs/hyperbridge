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
import {OwnableHostManager} from "../../src/core/OwnableHostManager.sol";
import {HandlerV2} from "../../src/core/HandlerV2.sol";
import {FeeToken} from "./FeeToken.sol";
import {HostParams, EvmHost} from "../../src/core/EvmHost.sol";
import {HostManagerParams, HostManager} from "../../src/core/HostManager.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest, IApp} from "@hyperbridge/core/interfaces/IApp.sol";

contract OwnableHostManagerTest is Test {
    uint256 internal constant PARACHAIN_ID = 2000;

    address internal managerOwner = address(0xA11CE);
    address internal governanceRelayer;
    address internal stranger = address(0xB0B);

    TestConsensusClientV2 internal consensusClient;
    MainnetTestHost internal host;
    OwnableHostManager internal manager;
    HandlerV2 internal handler;
    FeeToken internal feeToken;

    function setUp() public {
        governanceRelayer = address(this);

        consensusClient = new TestConsensusClientV2();
        handler = new HandlerV2();
        feeToken = new FeeToken(address(this), "HyperUSD", "USD.h");

        manager = new OwnableHostManager(
            managerOwner, HostManagerParams({admin: governanceRelayer, host: address(0)})
        );

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

        vm.chainId(1);
    }

    /// @dev A SetHostParam governance request carrying `params`, addressed to the live HostManager.
    function _setHostParamRequest(HostParams memory params) internal view returns (PostRequest memory) {
        return PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: new bytes(0),
            to: abi.encodePacked(host.hostParams().hostManager),
            timeoutTimestamp: 0,
            body: bytes.concat(bytes1(uint8(HostManager.OnAcceptActions.SetHostParam)), abi.encode(params))
        });
    }

    /// @dev A SetAdmin governance request installing `newAdmin`, addressed to the live HostManager.
    function _setAdminRequest(address newAdmin) internal view returns (PostRequest memory) {
        PostRequest memory request = _setHostParamRequest(host.hostParams());
        request.body = bytes.concat(bytes1(uint8(HostManager.OnAcceptActions.SetAdmin)), abi.encode(newAdmin));
        return request;
    }

    function _deliverGovernance(PostRequest memory request, address relayer) internal {
        vm.prank(address(host));
        manager.onAccept(IncomingPostRequest(request, relayer));
    }

    function testConstructorSetsOwnerAndGovernanceRelayer() public view {
        assertEq(manager.owner(), managerOwner);
        assertEq(manager.params().admin, governanceRelayer);
        assertEq(manager.relayer(), governanceRelayer);
        assertEq(manager.host(), address(host));
    }

    function testConstructorRejectsZeroAddresses() public {
        HostManagerParams memory managerParams = HostManagerParams({admin: governanceRelayer, host: address(0)});

        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableInvalidOwner.selector, address(0)));
        new OwnableHostManager(address(0), managerParams);

        managerParams.admin = address(0);
        vm.expectRevert(HostManager.InvalidAdmin.selector);
        new OwnableHostManager(managerOwner, managerParams);
    }

    function testOwnerUpdatesHostParams() public {
        HostParams memory params = host.hostParams();
        uint256 previousPeriod = params.challengePeriod;
        params.challengePeriod = previousPeriod + 1234;

        vm.prank(managerOwner);
        manager.updateHostParams(params);

        assertEq(host.hostParams().challengePeriod, previousPeriod + 1234);
    }

    function testHostStillValidatesOwnerParams() public {
        address honestHandler = host.hostParams().handler;
        HostParams memory params = host.hostParams();
        params.handler = address(0);

        vm.prank(managerOwner);
        vm.expectRevert(EvmHost.InvalidHandler.selector);
        manager.updateHostParams(params);

        assertEq(host.hostParams().handler, honestHandler);
    }

    function testOnlyOwnerUpdatesHostParams() public {
        HostParams memory params = host.hostParams();
        uint256 previousPeriod = params.challengePeriod;
        params.challengePeriod = previousPeriod + 1234;

        vm.prank(stranger);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, stranger));
        manager.updateHostParams(params);

        // The governance relayer is not the owner.
        vm.prank(governanceRelayer);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, governanceRelayer));
        manager.updateHostParams(params);

        assertEq(host.hostParams().challengePeriod, previousPeriod);
    }

    function testGovernanceStillUpdatesHostParams() public {
        HostParams memory params = host.hostParams();
        uint256 previousPeriod = params.challengePeriod;
        params.challengePeriod = previousPeriod + 4321;

        _deliverGovernance(_setHostParamRequest(params), governanceRelayer);

        assertEq(host.hostParams().challengePeriod, previousPeriod + 4321);
    }

    function testGovernanceAndOwnershipAreIndependent() public {
        address nextRelayer = makeAddr("nextRelayer");

        _deliverGovernance(_setAdminRequest(nextRelayer), governanceRelayer);

        assertEq(manager.relayer(), nextRelayer);
        assertEq(manager.owner(), managerOwner);

        address nextOwner = makeAddr("nextOwner");
        vm.prank(managerOwner);
        manager.transferOwnership(nextOwner);

        assertEq(manager.owner(), nextOwner);
        assertEq(manager.relayer(), nextRelayer);

        HostParams memory params = host.hostParams();
        params.challengePeriod += 1;

        vm.prank(managerOwner);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, managerOwner));
        manager.updateHostParams(params);

        vm.prank(nextOwner);
        manager.updateHostParams(params);
        assertEq(host.hostParams().challengePeriod, params.challengePeriod);
    }

    function testSupportsInterface() public view {
        assertTrue(manager.supportsInterface(type(IApp).interfaceId));
    }
}
