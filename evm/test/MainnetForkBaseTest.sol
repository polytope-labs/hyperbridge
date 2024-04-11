// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Test.sol";
import {TestConsensusClient} from "./TestConsensusClient.sol";
import {TestHost} from "./TestHost.sol";
import {PingModule} from "../examples/PingModule.sol";
import {HandlerV1} from "../src/modules/HandlerV1.sol";
import {CallDispatcher} from "../src/modules/CallDispatcher.sol";
import {FeeToken} from "./FeeToken.sol";
import {HostParams} from "../src/hosts/EvmHost.sol";
import {HostManagerParams, HostManager} from "../src/modules/HostManager.sol";
import {TokenGateway, Asset, InitParams} from "../src/modules/TokenGateway.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {IUniswapV2Router} from "../src/interfaces/IUniswapV2Router.sol";

contract MainnetForkBaseTest is Test {
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
    TokenGateway internal gateway;
    IERC20 internal usdc;
    IERC20 internal dai;
    IERC20 internal feeToken;
    IUniswapV2Router internal _uniswapV2Router;

    uint256 internal mainnetFork;

    function setUp() public virtual {
        usdc = IERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        dai = IERC20(0x6B175474E89094C44Da98b954EedeAC495271d0F);
        feeToken = dai;
        _uniswapV2Router = IUniswapV2Router(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

        string memory fork_url = vm.envString("MAINNET_FORK_URL");

        // mainnet fork creation
        mainnetFork = vm.createFork(fork_url);

        // mainnet fork selection
        vm.selectFork(mainnetFork);

        consensusClient = new TestConsensusClient();
        handler = new HandlerV1();
        CallDispatcher dispatcher = new CallDispatcher();

        HostManagerParams memory gParams = HostManagerParams({admin: address(this), host: address(0)});
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
            latestStateMachineHeight: 0,
            hyperbridge: StateMachine.kusama(2000)
        });

        host = new TestHost(params);

        testModule = new PingModule(address(this));
        testModule.setIsmpHost(address(host));
        manager.setIsmpHost(address(host));
        gateway = new TokenGateway(address(this));
        Asset[] memory assets = new Asset[](1);
        assets[0] = Asset({identifier: keccak256("USD.h"), erc20: address(feeToken), erc6160: address(0)});

        gateway.init(
            InitParams({
                hyperbridge: StateMachine.kusama(2000),
                host: address(host),
                uniswapV2Router: 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D,
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300, // 0.3
                assets: assets,
                callDispatcher: address(dispatcher)
            })
        );
    }

    function module() public view returns (address) {
        return address(testModule);
    }
}
