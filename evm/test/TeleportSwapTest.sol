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
import "forge-std/console.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "ismp/Message.sol";
import {TeleportParams, Body, BODY_BYTES_SIZE} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract TeleportSwapTest is MainnetForkBaseTest {
    // Maximum slippage of 0.5%
    uint256 maxSlippagePercentage = 50; // 0.5 * 100

    function testCanTeleportAssetsUsingUsdcForFee() public {
        // mainnet address holding usdc and dai
        address mainnetUsdcHolder = address(0xf584F8728B874a6a5c7A8d4d387C9aae9172D621);

        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee());

        address[] memory path = new address[](2);
        path[0] = address(usdc);
        path[1] = address(feeToken);

        uint256 _fromTokenAmountIn = _uniswapV2Router.getAmountsIn(messagingFee, path)[0];

        // Handling Slippage Implementation
        uint256 _slippageAmount = (_fromTokenAmountIn * maxSlippagePercentage) / 10000; // Adjusted for percentage times 100
        uint256 _amountInMax = _fromTokenAmountIn + _slippageAmount;

        // mainnet forking - impersonation
        vm.startPrank(mainnetUsdcHolder);

        dai.approve(address(gateway), 10000 * 1e18);
        dai.approve(address(host), messagingFee);
        usdc.approve(address(gateway), 10000 * 1e18);

        gateway.teleport(
            TeleportParams({
                feeToken: address(usdc),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                amountInMax: _amountInMax
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function testCanTeleportAssetsUsingNativeTokenForFee() public {
        // mainnet address holding eth
        address mainnetEthHolder = address(0xf584F8728B874a6a5c7A8d4d387C9aae9172D621);

        // relayer fee + per-byte fee
        uint256 messagingFee = (9 * 1e17) + (BODY_BYTES_SIZE * host.perByteFee());

        address[] memory path = new address[](2);
        path[0] = gateway.params().erc20NativeToken;
        path[1] = address(feeToken);

        uint256 _fromTokenAmountIn = _uniswapV2Router.getAmountsIn(messagingFee, path)[0];

        // Handling Slippage Implementation
        uint256 _slippageAmount = (_fromTokenAmountIn * maxSlippagePercentage) / 10000; // Adjusted for percentage times 100
        uint256 _amountInMax = _fromTokenAmountIn + _slippageAmount;

        // mainnet forking - impersonation
        vm.startPrank(mainnetEthHolder);

        dai.approve(address(gateway), 10000 * 1e18);

        gateway.teleport{value: _amountInMax}(
            TeleportParams({
                feeToken: address(usdc),
                amount: 1000 * 1e18, // $1000
                redeem: false,
                dest: StateMachine.bsc(),
                fee: 9 * 1e17, // $0.9
                timeout: 0,
                to: addressToBytes32(address(this)),
                assetId: keccak256("USD.h"),
                data: new bytes(0),
                amountInMax: _amountInMax
            })
        );

        assert(feeToken.balanceOf(address(this)) == 0);
        assert(feeToken.balanceOf(address(host)) == messagingFee);
    }

    function addressToBytes32(address _address) public pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }
}
