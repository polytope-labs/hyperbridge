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
        bytes32 commitment = testModule.dispatch(request);
        assert(host.requestCommitments(commitment).sender == address(this));

        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handleGetResponses(host, message);

        assert(host.responseReceipts(commitment).relayer == tx.origin);
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
