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
import {PingModule} from "../examples/PingModule.sol";
import {HandlerV1} from "../src/modules/HandlerV1.sol";
import {CallDispatcher} from "../src/modules/CallDispatcher.sol";
import {FeeToken} from "./FeeToken.sol";
import {MockUSCDC} from "./MockUSDC.sol";
import {HostParams, PerByteFee} from "../src/hosts/EvmHost.sol";
import {HostManagerParams, HostManager} from "../src/modules/HostManager.sol";
import {TokenGateway, Asset, TokenGatewayParamsExt, TokenGatewayParams, AssetMetadata} from "../src/modules/TokenGateway.sol";
import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {ERC20Token} from "./mocks/ERC20Token.sol";
import {MiniStaking} from "./mocks/MiniStakingContract.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";

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
        feeToken.grantRole(MINTER_ROLE, address(faucet));

        testModule = new PingModule(address(this));
        uint256 oldTime = block.timestamp;
        vm.warp(100_000);
        testModule.setIsmpHost(address(host), address(faucet));
        vm.warp(oldTime);

        manager.setIsmpHost(address(host));
        gateway = new TokenGateway(address(this));

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

        gateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({host: address(host), dispatcher: address(dispatcher)}),
                assets: assets
            })
        );

        feeToken.grantRole(MINTER_ROLE, address(this));
        feeToken.grantRole(MINTER_ROLE, address(gateway));
        feeToken.grantRole(BURNER_ROLE, address(gateway));

        hyperInu_h.grantRole(MINTER_ROLE, address(gateway));
        hyperInu_h.grantRole(BURNER_ROLE, address(gateway));

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
