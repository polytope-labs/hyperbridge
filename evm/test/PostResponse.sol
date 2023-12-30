// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import "./TestConsensusClient.sol";
import "../src/EvmHost.sol";
import "./TestHost.sol";
import {PingModule} from "./PingModule.sol";
import "../src/HandlerV1.sol";

contract PostResponseTest is Test {
    // needs a test method so that integration-tests can detect it
    function testPostResponse() public {}

    IConsensusClient internal consensusClient;
    EvmHost internal host;
    HandlerV1 internal handler;
    address internal testModule;

    function setUp() public virtual {
        consensusClient = new TestConsensusClient();
        handler = new HandlerV1();

        HostParams memory params = HostParams({
            admin: address(0),
            crosschainGovernor: address(0),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 5000,
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            lastUpdated: 0,
            consensusState: new bytes(0)
        });
        host = new TestHost(params);

        PingModule test = new PingModule(address(host));
        testModule = address(test);
    }

    function module() public view returns (address) {
        return testModule;
    }

    function PostResponseNoChallengeNoTimeout(
        bytes memory consensusProof,
        PostRequest memory request,
        PostResponseMessage memory message
    ) public {
        PingModule(testModule).dispatch(request);
        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handlePostResponses(host, message);
    }
}
