// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {PostTimeoutMessage} from "../src/HandlerV1.sol";
import {BaseTest} from "./BaseTest.sol";
import {PostRequest} from "ismp/IIsmp.sol";

contract PostTimeoutTest is BaseTest {
    function PostTimeoutNoChallenge(
        bytes memory consensusProof,
        PostRequest memory request,
        PostTimeoutMessage memory message
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
        handler.handlePostTimeouts(host, message);
    }
}
