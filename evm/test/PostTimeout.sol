// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";

import "./TestConsensusClient.sol";
import "../src/EvmHost.sol";
import "./TestHost.sol";
import {PingModule} from "./PingModule.sol";
import "../src/HandlerV1.sol";
import {ERC20} from "openzeppelin/token/ERC20/ERC20.sol";

contract FeeToken is ERC20 {
    constructor(uint256 initialSupply) ERC20("Fee Token", "FTK") {
        _mint(tx.origin, initialSupply);
    }

    function superApprove(address owner, address spender) public {
        _approve(owner, spender, type(uint256).max);
    }
}

contract PostTimeoutTest is Test {
    // needs a test method so that integration-tests can detect it
    function testPostTimeout() public {}

    IConsensusClient internal consensusClient;
    EvmHost internal host;
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
            baseGetRequestFee: 0,
            perByteFee: 1000000000000000000, // 1FTK
            feeTokenAddress: address(feeToken)
        });
        host = new TestHost(params);
        // approve the host address to spend the fee token.
        feeToken.superApprove(tx.origin, address(host));
        testModule = new PingModule(address(host));
    }

    function module() public view returns (address) {
        return address(testModule);
    }

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
