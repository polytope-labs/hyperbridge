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

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {PostRequest} from "ismp/Message.sol";
import {IncomingPostRequest} from "ismp/IIsmpModule.sol";
import {HostManagerParams, HostManager} from "../contracts/modules/HostManager.sol";
import {HostParams, EvmHost} from "../contracts/hosts/EvmHost.sol";

contract HostManagerTest is BaseTest {
    function HostManagerWithdraw(PostRequest memory request) public {
        // add balance to the host
        feeToken.mint(address(host), 1000e18);

        require(feeToken.balanceOf(address(host)) == 1000e18, "Failed to mint user tokens");

        vm.startPrank(address(host));
        HostManager(host.hostParams().hostManager).onAccept(IncomingPostRequest(request, tx.origin));

        require(feeToken.balanceOf(address(host)) == 500e18, "Failed to process request");
    }

    function HostManagerSetParams(PostRequest calldata request) public {
        vm.startPrank(address(host));

        HostManager(host.hostParams().hostManager).onAccept(IncomingPostRequest(request, tx.origin));
        HostParams memory params = abi.decode(request.body[1:], (HostParams));
        console.logUint(host.hostParams().challengePeriod);

        require(host.hostParams().challengePeriod == params.challengePeriod, "Failed to process request");
    }

    function testCannotSetInvalidHostManagerAddress() public {
        HostParams memory params = host.hostParams();
        address manager = params.hostManager;
        params.hostManager = address(0);

        vm.startPrank(manager);
        vm.expectRevert(EvmHost.InvalidHostManagerAddress.selector);
        host.updateHostParams(params);

        params.hostManager = msg.sender;
        vm.expectRevert(EvmHost.InvalidHostManagerAddress.selector);
        host.updateHostParams(params);
    }

    function HostManagerOnAccept(PostRequest calldata request) public {
        vm.startPrank(address(host));

        HostManager(host.hostParams().hostManager).onAccept(IncomingPostRequest(request, tx.origin));
    }
}
