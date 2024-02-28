// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {PostRequest} from "ismp/Message.sol";
import {HostManagerParams, HostManager} from "../src/modules/HostManager.sol";
import {HostParams} from "../src/hosts/EvmHost.sol";

contract HostManagerTest is BaseTest {
    function HostManagerWithdraw(PostRequest memory request) public {
        vm.startPrank(address(host));

        // add balance to the host
        feeToken.mint(address(host), 1000e18, "");
        require(feeToken.balanceOf(address(host)) == 1000e18, "Failed to mint user tokens");

        HostManager(host.hostParams().hostManager).onAccept(request);

        require(feeToken.balanceOf(address(host)) == 500e18, "Failed to process request");
    }

    function HostManagerSetParams(PostRequest calldata request) public {
        vm.startPrank(address(host));

        HostManager(host.hostParams().hostManager).onAccept(request);
        HostParams memory params = abi.decode(request.body[1:], (HostParams));
        console.logUint(host.hostParams().challengePeriod);

        require(host.hostParams().challengePeriod == params.challengePeriod, "Failed to process request");
    }

    function HostManagerUnauthorizedRequest(PostRequest calldata request) public {
        vm.startPrank(address(host));

        HostManager(host.hostParams().hostManager).onAccept(request);
    }
}
