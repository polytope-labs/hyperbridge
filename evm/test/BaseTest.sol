// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";
import {TestConsensusClient} from "./TestConsensusClient.sol";
import {TestHost} from "./TestHost.sol";
import {PingModule} from "../examples/PingModule.sol";
import {HandlerV1} from "../src/modules/HandlerV1.sol";
import {FeeToken} from "./FeeToken.sol";
import {HostParams} from "../src/hosts/EvmHost.sol";

contract BaseTest is Test {
    // needs a test method so that integration-tests can detect it
    function testPostTimeout() public {}

    TestConsensusClient internal consensusClient;
    TestHost internal host;
    HandlerV1 internal handler;
    PingModule internal testModule;
    FeeToken internal feeToken;

    function setUp() public virtual {
        consensusClient = new TestConsensusClient();
        handler = new HandlerV1();
        feeToken = new FeeToken(1000000000000000000000000000000); // 1,000,000,000,000 FTK

        HostParams memory params = HostParams({
            admin: address(0),
            hostManager: address(0),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 5000,
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            lastUpdated: 0,
            consensusState: new bytes(0),
            baseGetRequestFee: 10000000000000000000,
            perByteFee: 1000000000000000000, // 1FTK
            feeTokenAddress: address(feeToken),
            latestStateMachineHeight: 0
        });
        host = new TestHost(params);
        // approve the host address to spend the fee token.
        feeToken.superApprove(tx.origin, address(host));
        testModule = new PingModule(address(this));
        testModule.setIsmpHost(address(host));
    }

    function module() public view returns (address) {
        return address(testModule);
    }
}
