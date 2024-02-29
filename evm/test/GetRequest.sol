// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {BaseTest} from "./BaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "ismp/Message.sol";

contract GetRequestTest is BaseTest {
    using Message for PostRequest;
    using Message for GetRequest;

    function GetResponseNoChallengeNoTimeout(
        bytes memory consensusProof,
        GetRequest memory request,
        GetResponseMessage memory message
    ) public {
        uint256 fee = host.hostParams().baseGetRequestFee;
        uint256 balanceBefore = feeToken.balanceOf(tx.origin);

        testModule.dispatch(request);

        uint256 balanceAfter = feeToken.balanceOf(tx.origin);
        uint256 hostBalance = feeToken.balanceOf(address(host));

        assert(fee == hostBalance);
        assert(balanceBefore == balanceAfter + fee);

        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handleGetResponses(host, message);

        uint256 cost = host.hostParams().perByteFee * 32;
        uint256 hostBalanceAfter = feeToken.balanceOf(address(host));

        assert(hostBalance + cost == hostBalanceAfter);
    }

    function GetTimeoutNoChallenge(GetRequest memory request) public {
        testModule.dispatch(request);
        request.timeoutTimestamp += uint64(block.timestamp);
        assert(host.requestCommitments(request.hash()).sender != address(0));
        vm.warp(1000);

        GetRequest[] memory timeouts = new GetRequest[](1);
        timeouts[0] = request;
        GetTimeoutMessage memory message = GetTimeoutMessage({timeouts: timeouts});
        handler.handleGetRequestTimeouts(host, message);

        assert(host.requestCommitments(request.hash()).sender == address(0));
    }
}
