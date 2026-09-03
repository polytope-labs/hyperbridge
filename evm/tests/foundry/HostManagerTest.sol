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

import {BaseTest} from "./BaseTest.sol";
import {PostRequest, Message} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {IHandlerV2} from "@hyperbridge/core/interfaces/IHandlerV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";
import {HostManagerParams, HostManager} from "../../src/core/HostManager.sol";
import {HostParams, EvmHost} from "../../src/core/EvmHost.sol";

/// @dev What an attacker would install as the host's handler: it passes the host's interface
/// check, verifies nothing, and reports whatever relayer address it is told to.
contract MaliciousHandler is ERC165 {
    function supportsInterface(bytes4 interfaceId) public view override returns (bool) {
        return interfaceId == type(IHandlerV2).interfaceId || super.supportsInterface(interfaceId);
    }

    function deliver(EvmHost host, PostRequest memory request, address claimedRelayer) external {
        host.dispatchIncoming(request, claimedRelayer);
    }
}

contract HostManagerTest is BaseTest {
    using Message for PostRequest;

    address internal constant OUTSIDER = address(0xD00D);

    // ---------- relayer gate ----------

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

    function testRelayerIsSetInSetup() public view {
        assertEq(manager.relayer(), address(this));
    }

    function testSetRelayerOnlyHostAdmin() public {
        assertEq(host.admin(), address(this), "precondition: this contract is the host admin");

        vm.prank(OUTSIDER);
        vm.expectRevert(HostManager.UnauthorizedAction.selector);
        manager.setRelayer(OUTSIDER);

        // The host delivers governance messages but does not administer the manager.
        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedAction.selector);
        manager.setRelayer(OUTSIDER);

        assertEq(manager.relayer(), address(this), "relayer unchanged");
    }

    function testOnAcceptRejectsUnlistedRelayer() public {
        HostParams memory params = host.hostParams();
        uint256 previousPeriod = params.challengePeriod;
        params.challengePeriod = previousPeriod + 1234;
        PostRequest memory request = _setHostParamRequest(params);

        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        manager.onAccept(IncomingPostRequest(request, OUTSIDER));
        assertEq(host.hostParams().challengePeriod, previousPeriod, "params unchanged");

        // The relayer as reported by the host is never zero, and zero must not match an unset slot.
        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        manager.onAccept(IncomingPostRequest(request, address(0)));

        // The same message goes through from the authorised relayer.
        vm.prank(address(host));
        manager.onAccept(IncomingPostRequest(request, address(this)));
        assertEq(host.hostParams().challengePeriod, previousPeriod + 1234, "params applied");
    }

    function testFreshManagerRejectsEveryoneUntilRelayerSet() public {
        HostManager fresh = new HostManager(HostManagerParams({admin: address(this), host: address(0)}));
        fresh.setIsmpHost(address(host));
        assertEq(fresh.relayer(), address(0));
        PostRequest memory request = _setHostParamRequest(host.hostParams());

        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        fresh.onAccept(IncomingPostRequest(request, address(this)));
        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        fresh.onAccept(IncomingPostRequest(request, OUTSIDER));

        // Once set, the gate passes and the call reaches the host, which rejects this manager
        // because it is not the one the host is bound to: the gate was the only thing in the way.
        fresh.setRelayer(address(this));
        vm.prank(address(host));
        vm.expectRevert(EvmHost.UnauthorizedAction.selector);
        fresh.onAccept(IncomingPostRequest(request, address(this)));
    }

    function testSetRelayerRotates() public {
        address next = makeAddr("nextRelayer");
        HostParams memory params = host.hostParams();
        params.challengePeriod += 1;
        PostRequest memory request = _setHostParamRequest(params);

        vm.expectEmit(true, true, true, true, address(manager));
        emit HostManager.RelayerUpdated(address(this), next);
        manager.setRelayer(next);
        assertEq(manager.relayer(), next);

        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        manager.onAccept(IncomingPostRequest(request, address(this)));

        vm.prank(address(host));
        manager.onAccept(IncomingPostRequest(request, next));
        assertEq(host.hostParams().challengePeriod, params.challengePeriod);
    }

    function testSetRelayerToZeroFailsClosed() public {
        manager.setRelayer(address(0));
        PostRequest memory request = _setHostParamRequest(host.hostParams());

        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        manager.onAccept(IncomingPostRequest(request, address(this)));
        vm.prank(address(host));
        vm.expectRevert(HostManager.UnauthorizedRelayer.selector);
        manager.onAccept(IncomingPostRequest(request, address(0)));
    }

    /// Through the real host: a refused governance delivery leaves no receipt, so the authorised
    /// relayer can deliver the same message afterwards.
    function testRejectedDeliveryStaysRetryableThroughHost() public {
        HostParams memory params = host.hostParams();
        uint256 previousPeriod = params.challengePeriod;
        params.challengePeriod = previousPeriod + 99;
        PostRequest memory request = _setHostParamRequest(params);
        bytes32 commitment = request.hash();

        vm.prank(address(handler));
        host.dispatchIncoming(request, OUTSIDER);
        assertEq(host.requestReceipts(commitment), address(0), "refused delivery leaves no receipt");
        assertEq(host.hostParams().challengePeriod, previousPeriod, "params unchanged");

        vm.prank(address(handler));
        host.dispatchIncoming(request, address(this));
        assertEq(host.requestReceipts(commitment), address(this), "delivery recorded");
        assertEq(host.hostParams().challengePeriod, previousPeriod + 99, "params applied");
    }

    /// The attack the gate exists for: a forged SetHostParam that swaps the host's handler for a
    /// contract that will report any relayer address. With the gate, an arbitrary relayer cannot
    /// deliver the swap, so the handler stays honest and the attacker's contract never becomes
    /// able to call the host.
    function testForgedHandlerSwapIsRefused() public {
        MaliciousHandler malicious = new MaliciousHandler();
        HostParams memory params = host.hostParams();
        address honestHandler = params.handler;
        params.handler = address(malicious);
        PostRequest memory swap = _setHostParamRequest(params);

        // Delivered by the attacker (through the honest handler, proof assumed forged).
        vm.prank(address(handler));
        host.dispatchIncoming(swap, OUTSIDER);
        assertEq(host.hostParams().handler, honestHandler, "handler unchanged");

        // The attacker's contract is not the handler, so it cannot inject a relayer address.
        PostRequest memory forged = _setHostParamRequest(host.hostParams());
        vm.expectRevert(EvmHost.UnauthorizedAction.selector);
        malicious.deliver(EvmHost(payable(address(host))), forged, address(this));
    }

    // ---------- pre-existing helpers and tests ----------

    function HostManagerWithdraw(PostRequest memory request) public {
        // add balance to the host
        feeToken.mint(address(host), 1000e18);

        require(feeToken.balanceOf(address(host)) == 1000e18, "Failed to mint user tokens");

        vm.startPrank(address(host));
        HostManager(payable(host.hostParams().hostManager)).onAccept(IncomingPostRequest(request, tx.origin));

        require(feeToken.balanceOf(address(host)) == 500e18, "Failed to process request");
    }

    function HostManagerSetParams(PostRequest calldata request) public {
        vm.startPrank(address(host));

        HostManager(payable(host.hostParams().hostManager)).onAccept(IncomingPostRequest(request, tx.origin));
        HostParams memory params = abi.decode(request.body[1:], (HostParams));
        console.logUint(host.hostParams().challengePeriod);

        require(host.hostParams().challengePeriod == params.challengePeriod, "Failed to process request");
    }

    function testCannotSetInvalidAddresses() public {
        HostParams memory params = host.hostParams();

        // host manager address
        address manager = params.hostManager;
        params.hostManager = address(0);

        vm.startPrank(manager);
        vm.expectRevert(EvmHost.InvalidHostManager.selector);
        host.updateHostParams(params);

        params.hostManager = msg.sender;
        vm.expectRevert(EvmHost.InvalidHostManager.selector);
        host.updateHostParams(params);

        params.hostManager = address(this);
        vm.expectRevert();
        host.updateHostParams(params);
        params.hostManager = manager;

        // handler address
        address handler = params.handler;
        params.handler = address(0);
        vm.expectRevert(EvmHost.InvalidHandler.selector);
        host.updateHostParams(params);

        params.handler = msg.sender;
        vm.expectRevert(EvmHost.InvalidHandler.selector);
        host.updateHostParams(params);

        params.handler = address(this);
        vm.expectRevert();
        host.updateHostParams(params);
        params.handler = handler;

        // consensusClient address
        address consensusClient = params.consensusClient;
        params.consensusClient = address(0);

        vm.expectRevert(EvmHost.InvalidConsensusClient.selector);
        host.updateHostParams(params);

        params.consensusClient = msg.sender;
        vm.expectRevert(EvmHost.InvalidConsensusClient.selector);
        host.updateHostParams(params);

        params.consensusClient = address(this);
        vm.expectRevert();
        host.updateHostParams(params);
        params.consensusClient = consensusClient;

        params.hyperbridge = new bytes(0);
        vm.expectRevert(EvmHost.InvalidHyperbridgeId.selector);
        host.updateHostParams(params);
    }

    function HostManagerOnAccept(PostRequest calldata request) public {
        vm.startPrank(address(host));

        HostManager(payable(host.hostParams().hostManager)).onAccept(IncomingPostRequest(request, tx.origin));
    }

    function hostParamsInternal() public view returns (HostParams memory) {
        return host.hostParams();
    }
}
