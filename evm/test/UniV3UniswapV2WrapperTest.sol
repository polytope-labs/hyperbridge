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

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {UniV3UniswapV2Wrapper} from "../src/modules/UniV3UniswapV2Wrapper.sol";

contract UniV3UniswapV2WrapperTest is MainnetForkBaseTest {
    address private constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    address private constant UNISWAP_V3_QUOTER = 0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6;
    address private constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address private constant DAI = 0x6B175474E89094C44Da98b954EedeAC495271d0F;
    address private constant WHALE = address(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045);

    UniV3UniswapV2Wrapper private wrapper;

    function setUp() public override {
        vm.selectFork(vm.createFork(vm.envString("MAINNET_FORK_URL")));

        wrapper = new UniV3UniswapV2Wrapper(address(this));
        wrapper.init(
            UniV3UniswapV2Wrapper.Params({WETH: WETH, swapRouter: UNISWAP_V3_ROUTER, quoter: UNISWAP_V3_QUOTER})
        );
    }

    /*function testSwapETHForExactTokens() public {
        // Create path for the swap (ETH -> DAI)
        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = DAI;

        // Amount of DAI we want to receive
        uint256 amountOut = 1_000 * 1e18; // 1000 DAI
        uint256 amountsIn = wrapper.getAmountsIn(amountOut, path)[0]; // 1000 DAI

        // Get initial DAI balance
        uint256 initialDaiBalance = IERC20(DAI).balanceOf(WHALE);

        // Get initial ETH balance
        uint256 initialEthBalance = WHALE.balance;

        // Set deadline to 1 hour from now
        uint256 deadline = block.timestamp;

        // Execute swap with more ETH than needed to ensure it succeeds
        uint256 slippage = amountsIn * 50 / 10_000; // 0.5% slippage
        vm.prank(WHALE);
        uint256[] memory amounts = wrapper.swapETHForExactTokens{value: amountsIn + slippage}(
            amountOut,
            path,
            WHALE,
            deadline
        );

        // Verify the swap results
        assertEq(
            IERC20(DAI).balanceOf(WHALE),
            initialDaiBalance + amountOut,
            "DAI balance should increase by exact amount"
        );
        assertTrue(amounts[0] > 0, "ETH spent should be greater than 0");
        assertEq(amounts[1], amountOut, "Amount out should match requested amount");
        assertTrue(WHALE.balance < initialEthBalance, "ETH balance should decrease");
        assertTrue(WHALE.balance == initialEthBalance - amounts[0], "Should receive refund for unused ETH");
    }*/

    // Required to receive ETH refunds
    receive() external payable {}
}
