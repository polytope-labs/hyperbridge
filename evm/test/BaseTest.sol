// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";
import {TestConsensusClient} from "./TestConsensusClient.sol";
import {TestHost} from "./TestHost.sol";
import {PingModule} from "../examples/PingModule.sol";
import {HandlerV1} from "../src/modules/HandlerV1.sol";
import {FeeToken} from "./FeeToken.sol";
import {HostParams} from "../src/hosts/EvmHost.sol";
import {HostManagerParams, HostManager} from "../src/modules/HostManager.sol";
import {TokenGateway, Asset, InitParams} from "../src/modules/TokenGateway.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract BaseTest is Test {
    /// @notice The Id of Role required to mint token
    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");

    /// @notice The Id of Role required to burn token
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    // needs a test method so that integration-tests can detect it
    function testPostTimeout() public {}

    TestConsensusClient internal consensusClient;
    TestHost internal host;
    HandlerV1 internal handler;
    PingModule internal testModule;
    FeeToken internal feeToken;
    TokenGateway internal gateway;

    function setUp() public virtual {
        consensusClient = new TestConsensusClient();
        handler = new HandlerV1();
        feeToken = new FeeToken(address(this), "HyperUSD", "USD.h");

        HostManagerParams memory gParams =
            HostManagerParams({admin: address(this), host: address(0), hyperbridge: StateMachine.kusama(2000)});
        HostManager manager = new HostManager(gParams);

        HostParams memory params = HostParams({
            admin: address(0),
            hostManager: address(manager),
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
        manager.setIsmpHost(address(host));
        gateway = new TokenGateway(address(this));
        Asset[] memory assets = new Asset[](1);
        assets[0] = Asset({
            localIdentifier: keccak256("USD.h"),
            foreignIdentifier: keccak256("USD.h"),
            erc20: address(0),
            erc6160: address(feeToken)
        });

        gateway.init(
            InitParams({
                hyperbridge: StateMachine.kusama(2000),
                host: address(host),
                uniswapV2Router: address(1),
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300, // 0.3
                assets: assets
            })
        );

        feeToken.grantRole(MINTER_ROLE, address(gateway));
        feeToken.grantRole(BURNER_ROLE, address(gateway));
    }

    function module() public view returns (address) {
        return address(testModule);
    }
}
