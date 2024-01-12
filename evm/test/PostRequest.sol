// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {PostRequestMessage, PostRequestTimeoutMessage, PostRequest} from "ismp/IIsmp.sol";
import {BaseTest} from "./BaseTest.sol";

contract PostRequestTest is BaseTest {
    function PostRequestNoChallengeNoTimeout(bytes memory consensusProof, PostRequestMessage memory message) public {
        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handlePostRequests(host, message);
    }

    function PostRequestTimeoutNoChallenge(
        bytes memory consensusProof,
        PostRequest memory request,
        PostRequestTimeoutMessage memory message
    ) public {
        uint256 fee = host.hostParams().perByteFee * request.body.length;
        uint256 balanceBefore = feeToken.balanceOf(tx.origin);

        testModule.dispatch(request);

        uint256 balanceAfter = feeToken.balanceOf(tx.origin);
        uint256 hostBalance = feeToken.balanceOf(address(host));

        assert(fee == hostBalance);
        assert(balanceBefore == balanceAfter + fee);

        handler.handleConsensus(host, consensusProof);
        vm.warp(5000);
        handler.handlePostRequestTimeouts(host, message);
    }
}
