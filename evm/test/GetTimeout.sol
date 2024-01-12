// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {GetResponseMessage, GetTimeoutMessage} from "ismp/IIsmp.sol";
import {BaseTest} from "./BaseTest.sol";
import {GetRequest} from "ismp/IIsmp.sol";

contract GetTimeoutTest is BaseTest {
    function GetTimeoutNoChallengeNoTimeout(GetRequest memory request) public {
        testModule.dispatch(request);
        request.timeoutTimestamp += uint64(block.timestamp);
        vm.warp(1000);

        GetRequest[] memory timeouts = new GetRequest[](1);
        timeouts[0] = request;
        GetTimeoutMessage memory message = GetTimeoutMessage({timeouts: timeouts});
        handler.handleGetRequestTimeouts(host, message);
    }
}
