// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {PostRequestMessage, PostRequestTimeoutMessage, PostRequest, Message} from "ismp/Message.sol";
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
        uint256 fee = host.hostParams().perByteFee * request.body.length;
        uint256 balanceBefore = feeToken.balanceOf(tx.origin);

        vm.prank(tx.origin);
        testModule.dispatch(request);

        bytes32 commitment = message.timeouts[0].hash();
        assert(host.requestCommitments(commitment).sender != address(0));

        uint256 balanceAfter = feeToken.balanceOf(tx.origin);
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
