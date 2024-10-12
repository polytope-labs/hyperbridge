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
import "forge-std/console.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "@polytope-labs/ismp-solidity/Message.sol";
import {IncomingPostRequest} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";
import {TeleportParams, Body, BODY_BYTES_SIZE} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {RequestBody, RegistrarParams, TokenRegistrar} from "../src/modules/Registrar.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";

contract TokenRegistrarTest is MainnetForkBaseTest {
    // Maximum slippage of 0.5%
    uint256 maxSlippagePercentage = 50; // 0.5 * 100

    function testCanRegisterAssetsUsingNativeToken() public {
        // mainnet address holding eth
        address mainnetEthHolder = address(0xf584F8728B874a6a5c7A8d4d387C9aae9172D621);

        // relayer fee + per-byte fee
        uint256 messagingFee = 64 * host.perByteFee(StateMachine.evm(97));
        uint256 registrationFee = _registrar.params().baseFee + messagingFee;

        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(IIsmpHost(_registrar.params().host).uniswapV2Router()).WETH();
        path[1] = address(feeToken);

        uint256 _fromTokenAmountIn = _uniswapV2Router.getAmountsIn(registrationFee, path)[0];

        // Handling Slippage Implementation
        uint256 _slippageAmount = (_fromTokenAmountIn * maxSlippagePercentage) / 10_000; // Adjusted for percentage times 100
        uint256 _amountInMax = _fromTokenAmountIn + _slippageAmount;

        vm.startPrank(mainnetEthHolder);
        _registrar.registerAsset{value: _amountInMax}(keccak256("USD.h"));

        assert(feeToken.balanceOf(address(host)) == registrationFee);
    }

    function testCanRegisterAssetsUsingFeeToken() public {
        // mainnet address holding eth
        address mainnetEthHolder = address(0xf584F8728B874a6a5c7A8d4d387C9aae9172D621);

        // relayer fee + per-byte fee
        uint256 messagingFee = 64 * host.perByteFee(StateMachine.evm(97));
        uint256 registrationFee = _registrar.params().baseFee + messagingFee;

        vm.startPrank(mainnetEthHolder);
        dai.approve(address(_registrar), registrationFee);
        _registrar.registerAsset(keccak256("USD.h"));

        assert(feeToken.balanceOf(address(host)) == registrationFee);
    }

    function testGovernanceParameterUpdate() public {
        assert(_registrar.params().baseFee == 100 * 1e18);

        vm.startPrank(address(host));
        RegistrarParams memory params = _registrar.params();
        params.baseFee = 200 * 1e18;
        bytes memory body = abi.encode(params);

        vm.expectRevert(TokenRegistrar.UnauthorizedAction.selector);
        _registrar.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    source: new bytes(0),
                    dest: new bytes(0),
                    nonce: 0,
                    from: new bytes(0),
                    to: new bytes(0),
                    timeoutTimestamp: 0,
                    body: body
                }),
                relayer: msg.sender
            })
        );

        _registrar.onAccept(
            IncomingPostRequest({
                request: PostRequest({
                    source: host.hyperbridge(),
                    dest: new bytes(0),
                    nonce: 0,
                    from: new bytes(0),
                    to: new bytes(0),
                    timeoutTimestamp: 0,
                    body: body
                }),
                relayer: msg.sender
            })
        );
        assert(_registrar.params().baseFee == params.baseFee);
    }

    function addressToBytes32(address _address) public pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }
}
