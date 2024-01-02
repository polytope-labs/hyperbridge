// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import {PostRequestMessage} from "../src/HandlerV1.sol";
import {BaseTest} from "./BaseTest.sol";

contract PostRequestTest is BaseTest {
    function PostRequestNoChallengeNoTimeout(bytes memory consensusProof, PostRequestMessage memory message) public {
        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handlePostRequests(host, message);
    }
}
