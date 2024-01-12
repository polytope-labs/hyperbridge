// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {
    PostRequest,
    PostResponse,
    PostRequestMessage,
    PostResponseMessage,
    PostResponseTimeoutMessage
} from "ismp/IIsmp.sol";
import {IHandler} from "ismp/IHandler.sol";
import {BaseTest} from "./BaseTest.sol";

contract PostResponseTest is BaseTest {
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

        handler.handleConsensus(host, consensusProof2);
        vm.warp(20);
        handler.handlePostResponseTimeouts(host, timeout);
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
        testModule.dispatchPostResponse(response);

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
