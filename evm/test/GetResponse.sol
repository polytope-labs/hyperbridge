// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {GetResponseMessage} from "../src/HandlerV1.sol";
import {BaseTest} from "./BaseTest.sol";
import {GetRequest} from "ismp/IIsmp.sol";

contract GetResponseTest is BaseTest {
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
}
