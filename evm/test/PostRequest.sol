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

import {PostRequestMessage, PostRequestTimeoutMessage, PostRequest, Message} from "@polytope-labs/ismp-solidity/Message.sol";
import {BaseTest} from "./BaseTest.sol";

contract PostRequestTest is BaseTest {
    using Message for PostRequest;

    function PostRequestNoChallengeNoTimeout(bytes memory consensusProof, PostRequestMessage memory message) public {
        vm.prank(tx.origin);
        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handlePostRequests(host, message);

        // assert that request was acknowledged
        bytes32 commitment = message.requests[0].request.hash();
        assert(host.requestReceipts(commitment) != address(0));
    }

    function PostRequestTimeoutNoChallenge(
        bytes memory consensusProof,
        PostRequest memory request,
        PostRequestTimeoutMessage memory message
    ) public {
        feeToken.mint(address(this), 1_000_000_000 * 1e18);
        feeToken.approve(address(testModule), type(uint256).max);
        uint256 fee = host.hostParams().defaultPerByteFee * (32 > request.body.length ? 32 : request.body.length);
        uint256 balanceBefore = feeToken.balanceOf(address(this));

        testModule.dispatch(request);

        bytes32 commitment = message.timeouts[0].hash();
        assert(host.requestCommitments(commitment).sender != address(0));

        uint256 balanceAfter = feeToken.balanceOf(address(this));
        uint256 hostBalance = feeToken.balanceOf(address(host));

        assert(fee == hostBalance);
        assert(balanceBefore == balanceAfter + fee);

        handler.handleConsensus(host, consensusProof);
        vm.warp(5000);
        handler.handlePostRequestTimeouts(host, message);

        // assert that request no longer exists
        assert(host.requestCommitments(commitment).sender == address(0));
    }
}
