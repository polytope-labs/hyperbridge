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
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {TestConsensusClient} from "./TestConsensusClient.sol";
import {TestHost} from "./TestHost.sol";
import {PingModule} from "../src/utils/PingModule.sol";
import {HandlerV1} from "../src/core/HandlerV1.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {FeeToken} from "./FeeToken.sol";
import {HostParams, PerByteFee} from "../src/core/EvmHost.sol";
import {HostManagerParams, HostManager} from "../src/core/HostManager.sol";

import {HyperFungibleTokenImpl} from "../src/utils/HyperFungibleTokenImpl.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {TokenGateway, TokenGatewayParams, AssetMetadata} from "../src/apps/TokenGateway.sol";
import {PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {TokenFaucet} from "../src/utils/TokenFaucet.sol";

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
    HostManager internal manager;
    IERC20 internal usdc;
    IERC20 internal dai;
    IERC20 internal feeToken;
    IUniswapV2Router02 internal _uniswapV2Router;

    CallDispatcher internal dispatcher;

    uint256 internal mainnetFork;

    function setUp() public virtual {
        usdc = IERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        dai = IERC20(0x6B175474E89094C44Da98b954EedeAC495271d0F);
        feeToken = dai;
        _uniswapV2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

        string memory fork_url = vm.envString("MAINNET_FORK_URL");

        // mainnet fork creation
        mainnetFork = vm.createFork(fork_url);

        // mainnet fork selection
        vm.selectFork(mainnetFork);

        consensusClient = new TestConsensusClient();
        handler = new HandlerV1();
        dispatcher = new CallDispatcher();

        uint256 paraId = 2000;
        HostManagerParams memory gParams = HostManagerParams({admin: address(this), host: address(0)});
        manager = new HostManager(gParams);
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = paraId;
        PerByteFee[] memory perByteFees = new PerByteFee[](0);
        HostParams memory params = HostParams({
            uniswapV2: address(_uniswapV2Router),
            perByteFees: perByteFees,
            admin: address(0),
            hostManager: address(manager),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 21 * (60 * 60 * 24),
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            defaultPerByteFee: 3 * 1e15, // $0.003/byte
            stateCommitmentFee: 10 * 1e18, // $10
            feeToken: address(feeToken),
            hyperbridge: StateMachine.kusama(paraId),
            stateMachines: stateMachines
        });

        host = new TestHost(params);

        testModule = new PingModule(address(this));
        testModule.setIsmpHost(address(host), address(0));
        manager.setIsmpHost(address(host));
        gateway = new TokenGateway(address(this));

        // Deploy HyperFungibleTokens with address(this) as admin
        HyperFungibleTokenImpl daiToken = new HyperFungibleTokenImpl(address(this), "Hyperbridge USD", "USD.h");
        // Grant minter and burner roles to gateway
        daiToken.grantMinterRole(address(gateway));
        daiToken.grantBurnerRole(address(gateway));

        HyperFungibleTokenImpl wethToken = new HyperFungibleTokenImpl(address(this), "Wrapped ETH", "WETH");
        // Grant minter and burner roles to gateway
        wethToken.grantMinterRole(address(gateway));
        wethToken.grantBurnerRole(address(gateway));

        AssetMetadata[] memory assets = new AssetMetadata[](2);
        assets[0] = AssetMetadata({
            erc20: address(dai),
            erc6160: address(daiToken),
            name: "Hyperbridge USD",
            symbol: "USD.h",
            beneficiary: address(0),
            initialSupply: 0
        });

        assets[1] = AssetMetadata({
            erc20: _uniswapV2Router.WETH(),
            erc6160: address(wethToken),
            name: "Wrapped ETH",
            symbol: "WETH",
            beneficiary: address(0),
            initialSupply: 0
        });

        gateway.init(TokenGatewayParams({host: address(host), dispatcher: address(dispatcher)}));

        // Add assets via governance
        for (uint256 i = 0; i < assets.length; i++) {
            AssetMetadata[] memory singleAsset = new AssetMetadata[](1);
            singleAsset[0] = assets[i];
            bytes memory body = bytes.concat(hex"02", abi.encode(singleAsset[0]));

            vm.prank(address(host));
            gateway.onAccept(
                IncomingPostRequest({
                    request: PostRequest({
                        to: abi.encodePacked(address(0)),
                        from: abi.encodePacked(address(gateway)),
                        dest: new bytes(0),
                        body: body,
                        nonce: 0,
                        source: StateMachine.kusama(2000),
                        timeoutTimestamp: 0
                    }),
                    relayer: address(0)
                })
            );
        }
    }

    function module() public view returns (address) {
        return address(testModule);
    }
}
