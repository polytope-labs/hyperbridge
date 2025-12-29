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
pragma solidity ^0.8.26;

import "forge-std/Test.sol";
import "forge-std/console.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {UniV4UniswapV2Wrapper} from "../src/utils/uniswapv2/UniV4UniswapV2Wrapper.sol";

contract UniV4UniswapV2WrapperTest is MainnetForkBaseTest {
    address private constant UNIVERSAL_ROUTER = 0x66a9893cC07D91D95644AEDD05D03f95e1dBA8Af;
    address private constant V4_QUOTER = 0x52F0E24D1c21C8A0cB1e5a5dD6198556BD9E1203;

    address private constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address private constant DAI = 0x6B175474E89094C44Da98b954EedeAC495271d0F;
    address private constant WHALE = address(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045);
    address private DEPLOYER;

    UniV4UniswapV2Wrapper private wrapper;

    function setUp() public override {
        vm.selectFork(vm.createFork(vm.envString("MAINNET_FORK_URL")));

        DEPLOYER = address(this);
        wrapper = new UniV4UniswapV2Wrapper(address(this));
        wrapper.init(
            UniV4UniswapV2Wrapper.Params({
                universalRouter: UNIVERSAL_ROUTER,
                quoter: V4_QUOTER,
                WETH: WETH,
                defaultFee: 3000, // 0.3% fee tier
                defaultTickSpacing: 60 // Tick spacing for 0.3% pools
            })
        );
    }

    function testSwapExactETHForTokensV4() public {
        address[] memory path = new address[](2);
        path[0] = address(0); // Native ETH
        path[1] = DAI;

        uint256 exactEthAmount = 1 ether;
        uint256[] memory expectedAmounts = wrapper.getAmountsOut(exactEthAmount, path);
        uint256 amountOutMin = expectedAmounts[1];

        uint256 initialDaiBalance = IERC20(DAI).balanceOf(WHALE);
        uint256 initialEthBalance = WHALE.balance;

        uint256 deadline = block.timestamp + 1 hours;

        vm.prank(WHALE);
        uint256[] memory amounts =
            wrapper.swapExactETHForTokens{value: exactEthAmount}(amountOutMin, path, WHALE, deadline);

        uint256 newDaiBalance = IERC20(DAI).balanceOf(WHALE);
        uint256 newEthBalance = WHALE.balance;

        assertEq(amounts[0], exactEthAmount, "Should spend exact ETH amount");
        assertEq(initialEthBalance - newEthBalance, exactEthAmount, "ETH balance should decrease by exact amount");

        console.log("ETH spent:", amounts[0]);
        console.log("DAI received:", amounts[1]);
        console.log("Actual DAI received:", newDaiBalance - initialDaiBalance);

        assertTrue(amounts[1] > 0, "Should receive some DAI");
        assertTrue(newDaiBalance > initialDaiBalance, "DAI balance should increase");
    }

    function testSwapETHForExactTokensV4() public {
        address[] memory path = new address[](2);
        path[0] = address(0);
        path[1] = DAI;

        uint256 amountOut = 1000 * 1e18;
        uint256 maxEthIn = 2 ether;

        uint256 initialEthBalance = WHALE.balance;
        uint256 initialDeployerBalance = DEPLOYER.balance;

        uint256 deadline = block.timestamp + 1 hours;

        vm.prank(WHALE);
        uint256[] memory amounts = wrapper.swapETHForExactTokens{value: maxEthIn}(amountOut, path, WHALE, deadline);

        uint256 newEthBalance = WHALE.balance;
        uint256 newDeployerBalance = DEPLOYER.balance;

        console.log("Max ETH sent:", maxEthIn);
        console.log("ETH actually spent (returned):", amounts[0]);
        console.log("WHALE ETH spent:", initialEthBalance - newEthBalance);
        console.log("Deployer received refund:", newDeployerBalance - initialDeployerBalance);
        console.log("DAI received:", amounts[1]);

        assertEq(initialEthBalance - newEthBalance, maxEthIn, "WHALE spent full amount");

        assertEq(newDeployerBalance - initialDeployerBalance, maxEthIn - amounts[0], "Deployer got refund");
    }

    function testGetAmountsOut() public {
        address[] memory path = new address[](2);
        path[0] = address(0);
        path[1] = DAI;

        uint256 amountIn = 1 ether;

        uint256[] memory amounts = wrapper.getAmountsOut(amountIn, path);

        console.log("Quote for 1 ETH:");
        console.log("  ETH in:", amounts[0]);
        console.log("  DAI out:", amounts[1]);

        assertEq(amounts[0], amountIn, "Input amount should match");
        assertTrue(amounts[1] > 0, "Should quote some DAI output");
    }

    function testGetAmountsIn() public {
        address[] memory path = new address[](2);
        path[0] = address(0);
        path[1] = DAI;

        uint256 amountOut = 1000 * 1e18; // Want 1000 DAI

        uint256[] memory amounts = wrapper.getAmountsIn(amountOut, path);

        console.log("Quote for 1000 DAI:");
        console.log("  ETH in:", amounts[0]);
        console.log("  DAI out:", amounts[1]);

        assertTrue(amounts[0] > 0, "Should quote some ETH input");
        assertEq(amounts[1], amountOut, "Output amount should match");
    }

    // Required to receive ETH refunds
    receive() external payable {}
}
