// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
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
import {TokenRegistrar, RegistrarParams} from "../src/modules/Registrar.sol";
import {
    TokenGateway,
    Asset,
    TokenGatewayParams,
    TokenGatewayParamsExt,
    AssetMetadata
} from "../src/modules/TokenGateway.sol";
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
    TokenRegistrar internal _registrar;

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

        uint256 paraId = 2000;
        HostManagerParams memory gParams = HostManagerParams({admin: address(this), host: address(0)});
        HostManager manager = new HostManager(gParams);
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = paraId;
        address[] memory fishermen = new address[](0);
        HostParams memory params = HostParams({
            fishermen: fishermen,
            admin: address(0),
            hostManager: address(manager),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 5000,
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            perByteFee: 3 * 1e15, // $0.003/byte
            feeToken: address(feeToken),
            hyperbridge: StateMachine.kusama(paraId),
            stateMachines: stateMachines
        });

        host = new TestHost(params);

        testModule = new PingModule(address(this));
        testModule.setIsmpHost(address(host));
        manager.setIsmpHost(address(host));
        gateway = new TokenGateway(address(this));
        AssetMetadata[] memory assets = new AssetMetadata[](1);
        assets[0] = AssetMetadata({
            erc20: address(usdc),
            erc6160: address(0),
            name: "Hyperbridge USD",
            symbol: "USD.h",
            beneficiary: address(0),
            initialSupply: 0
        });

        gateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({
                    host: address(host),
                    uniswapV2: 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D,
                    dispatcher: address(dispatcher)
                }),
                assets: assets
            })
        );

        _registrar = new TokenRegistrar(address(this));
        _registrar.init(
            RegistrarParams({
                host: address(host),
                uniswapV2: 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D,
                erc20NativeToken: 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2,
                baseFee: 100 * 1e18
            })
        );
    }

    function module() public view returns (address) {
        return address(testModule);
    }
}
