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

import {PostRequest, PostResponse, PostRequestMessage, PostResponseMessage, Message, PostResponseTimeoutMessage} from "@polytope-labs/ismp-solidity/Message.sol";
import {StateMachineHeight} from "@polytope-labs/ismp-solidity/IConsensusClient.sol";
import {IHandler} from "@polytope-labs/ismp-solidity/IHandler.sol";
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
        vm.prank(tx.origin);
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
        feeToken.mint(address(this), 1_000_000_000 * 1e18);
        feeToken.approve(address(testModule), type(uint256).max);
        handler.handleConsensus(host, consensusProof1);
        vm.warp(10);
        handler.handlePostRequests(host, request);
        assert(host.requestReceipts(request.requests[0].request.hash()) == address(this));

        response.timeoutTimestamp -= uint64(block.timestamp);
        testModule.dispatchPostResponse(response);
        response.timeoutTimestamp += uint64(block.timestamp);
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
        feeToken.mint(address(this), 1_000_000_000 * 1e18);
        feeToken.approve(address(testModule), type(uint256).max);
        handler.handleConsensus(host, consensusProof1);
        vm.warp(10);
        handler.handlePostRequests(host, request);
        response.timeoutTimestamp -= 10;
        testModule.dispatchPostResponse(response);

        vm.expectRevert();
        testModule.dispatchPostResponse(response);

        handler.handleConsensus(host, consensusProof2);
        vm.warp(20);
        bytes memory callData = abi.encodeWithSelector(
            IHandler.handlePostResponseTimeouts.selector,
            address(host),
            timeout
        );
        (bool success, ) = address(handler).call(callData);
        // non-membership proof actually contains the response
        assert(!success);
    }
}
