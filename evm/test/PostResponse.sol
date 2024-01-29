// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {
    PostRequest,
    PostResponse,
    PostRequestMessage,
    PostResponseMessage,
    Message,
    PostResponseTimeoutMessage
} from "ismp/IIsmp.sol";
import {IHandler} from "ismp/IHandler.sol";
import {BaseTest} from "./BaseTest.sol";

contract PostResponseTest is BaseTest {
    using Message for PostRequest;
    using Message for PostResponse;
    // needs a test method so that integration-tests can detect it

    function testPostResponse() public {}

    function PostResponseNoChallengeNoTimeout(
        bytes memory consensusProof,
        PostRequest memory request,
        PostResponseMessage memory message
    ) public {
        testModule.dispatch(request);
        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handlePostResponses(host, message);

        // assert that we acknowledge the response
        assert(host.responseReceipts(message.responses[0].response.request.hash()).relayer != address(0));
    }

    function PostResponseTimeoutNoChallenge(
        bytes memory consensusProof1,
        bytes memory consensusProof2,
        PostRequestMessage memory request,
        PostResponse memory response,
        PostResponseTimeoutMessage memory timeout
    ) public {
        handler.handleConsensus(host, consensusProof1);
        vm.warp(10);
        handler.handlePostRequests(host, request);
        response.timeoutTimestamp -= 10;
        testModule.dispatchPostResponse(response);
        response.timeoutTimestamp += 10;
        // we should know this response
        assert(host.responseCommitments(response.hash()).sender != address(0));

        handler.handleConsensus(host, consensusProof2);
        vm.warp(20);
        handler.handlePostResponseTimeouts(host, timeout);
        // we should no longer know this response
        assert(host.responseCommitments(response.hash()).sender == address(0));
    }

    function PostResponseMaliciousTimeoutNoChallenge(
        bytes memory consensusProof1,
        bytes memory consensusProof2,
        PostRequestMessage memory request,
        PostResponse memory response,
        PostResponseTimeoutMessage memory timeout
    ) public {
        handler.handleConsensus(host, consensusProof1);
        vm.warp(10);
        handler.handlePostRequests(host, request);
        response.timeoutTimestamp -= 10;
        (bool status,) = address(testModule).call(abi.encodeCall(testModule.dispatchPostResponse, response));
        assert(status);

        (bool ok,) = address(testModule).call(abi.encodeCall(testModule.dispatchPostResponse, response));
        // attempting to dispatch duplicate response should fail
        assert(!ok);

        handler.handleConsensus(host, consensusProof2);
        vm.warp(20);
        bytes memory callData =
            abi.encodeWithSelector(IHandler.handlePostResponseTimeouts.selector, address(host), timeout);
        (bool success,) = address(handler).call(callData);
        // non-membership proof actually contains the response
        assert(!success);
    }
}
