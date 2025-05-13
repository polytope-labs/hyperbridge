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
import {TeleportParams, Body, BODY_BYTES_SIZE} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";

contract TeleportForkTest is MainnetForkBaseTest {
    // Maximum slippage of 0.5%
    uint256 maxSlippagePercentage = 5; // 0.5 * 100

    function testCanTeleportAssetsUsingNativeToken() public {
        // mainnet address holding eth and dai
        address whaleAccount = address(0x47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503);

        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee(StateMachine.evm(97)));

        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(IIsmpHost(gateway.params().host).uniswapV2Router()).WETH();
        path[1] = address(feeToken);

        uint256 _fromTokenAmountIn = _uniswapV2Router.getAmountsIn(messagingFee, path)[0];

        // Handling Slippage Implementation
        uint256 _slippageAmount = (_fromTokenAmountIn * maxSlippagePercentage) / 10_000; // Adjusted for percentage times 100
        uint256 _amountInMax = _fromTokenAmountIn + _slippageAmount;

        // mainnet forking - impersonation
        vm.startPrank(whaleAccount);
        dai.approve(address(gateway), 1_000 * 1e6); // approve amount to spend

        gateway.teleport{value: _amountInMax}(
            TeleportParams({
                amount: 1_000 * 1e6, // $1000
                redeem: false,
                dest: StateMachine.evm(97),
                relayerFee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                nativeCost: _amountInMax
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function testCanTeleportNativeToken() public {
        // mainnet address holding eth and dai
        address whaleAccount = address(0x47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503);

        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee(StateMachine.evm(97)));

        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(IIsmpHost(gateway.params().host).uniswapV2Router()).WETH();
        path[1] = address(feeToken);

        uint256 messagingFeeNative = _uniswapV2Router.getAmountsIn(messagingFee, path)[0];
        uint256 teleportAmount = 1 * 1e18;
        uint256 total = messagingFeeNative + teleportAmount;

        // mainnet forking - impersonation
        vm.startPrank(whaleAccount);
        gateway.teleport{value: total}(
            TeleportParams({
                amount: teleportAmount, // $1000
                redeem: false,
                dest: StateMachine.evm(97),
                relayerFee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(this)),
                assetId: keccak256("WETH"),
                data: new bytes(0),
                nativeCost: messagingFeeNative
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function addressToBytes32(address _address) public pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }
}
