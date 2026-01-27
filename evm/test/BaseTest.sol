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
import {MockUSCDC} from "./MockUSDC.sol";
import {HostParams, PerByteFee} from "../src/core/EvmHost.sol";
import {HostManagerParams, HostManager} from "../src/core/HostManager.sol";
import {TokenGateway, TokenGatewayParams, AssetMetadata} from "../src/apps/TokenGateway.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {ERC20Token} from "./mocks/ERC20Token.sol";
import {MiniStaking} from "./mocks/MiniStakingContract.sol";
import {TokenFaucet} from "../src/utils/TokenFaucet.sol";
import {HyperFungibleTokenImpl} from "../src/utils/HyperFungibleTokenImpl.sol";

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
    MockUSCDC internal mockUSDC;
    HostManager internal manager;
    TokenGateway internal gateway;
    ERC20Token stakedToken;
    MiniStaking miniStaking;

    MockUSCDC internal hyperInu;
    FeeToken internal hyperInu_h;

    function setUp() public virtual {
        consensusClient = new TestConsensusClient();
        handler = new HandlerV1();
        feeToken = new FeeToken(address(this), "HyperUSD", "USD.h");

        mockUSDC = new MockUSCDC("MockUSDC", "USDC.h");
        CallDispatcher dispatcher = new CallDispatcher();

        hyperInu = new MockUSCDC("HyperInu", "HINU");
        hyperInu_h = new FeeToken(address(this), "HyperInu", "HINU.h");
        uint256 paraId = 2000;
        HostManagerParams memory gParams = HostManagerParams({admin: address(this), host: address(0)});
        manager = new HostManager(gParams);
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = paraId;
        PerByteFee[] memory perByteFees = new PerByteFee[](0);
        HostParams memory params = HostParams({
            uniswapV2: address(0),
            perByteFees: perByteFees,
            admin: address(this),
            hostManager: address(manager),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 21 * (60 * 60 * 24),
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            defaultPerByteFee: 1000000000000000000, // 1FTK
            stateCommitmentFee: 10 * 1e18, // $10
            feeToken: address(feeToken),
            hyperbridge: StateMachine.kusama(paraId),
            stateMachines: stateMachines
        });
        host = new TestHost(params);

        // and token faucet
        TokenFaucet faucet = new TokenFaucet();
        feeToken.grantMinterRole(address(faucet));
        // Grant minter and burner roles to test contract for direct token operations
        feeToken.grantMinterRole(address(this));
        feeToken.grantBurnerRole(address(this));

        testModule = new PingModule(address(this));
        uint256 oldTime = block.timestamp;
        vm.warp(100_000);
        testModule.setIsmpHost(address(host), address(faucet));
        vm.warp(oldTime);

        manager.setIsmpHost(address(host));
        gateway = new TokenGateway(address(this));

        // Grant minter and burner roles to gateway for feeToken
        feeToken.grantMinterRole(address(gateway));
        feeToken.grantBurnerRole(address(gateway));

        // Grant minter and burner roles to gateway for hyperInu_h
        hyperInu_h.grantMinterRole(address(gateway));
        hyperInu_h.grantBurnerRole(address(gateway));
        // Grant minter and burner roles to test contract for hyperInu_h
        hyperInu_h.grantMinterRole(address(this));
        hyperInu_h.grantBurnerRole(address(this));

        mockUSDC.superApprove(tx.origin, address(host));
        mockUSDC.superApprove(address(this), address(host));
        AssetMetadata[] memory assets = new AssetMetadata[](1);
        assets[0] = AssetMetadata({
            erc20: address(0),
            erc6160: address(feeToken),
            name: "Hyperbridge USD",
            symbol: "USD.h",
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

        // HyperFungibleToken uses immutable gateway pattern
        // Gateway is set at deployment and can mint/burn tokens

        // some approvals
        feeToken.superApprove(address(this), address(gateway));
        feeToken.superApprove(address(tx.origin), address(testModule));
        feeToken.superApprove(address(tx.origin), address(host));
        feeToken.superApprove(address(testModule), address(host));
        feeToken.superApprove(address(this), address(host));

        miniStaking = new MiniStaking(address(feeToken));

        vm.chainId(1);
    }

    function module() public view returns (address) {
        return address(testModule);
    }
}
