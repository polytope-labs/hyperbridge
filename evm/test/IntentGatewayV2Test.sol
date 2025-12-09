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
import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {IntentGatewayV2, Order, Params, TokenInfo, CollectFees, PaymentInfo, PredispatchInfo, FillOptions} from "../src/modules/IntentGatewayV2.sol";
import {ICallDispatcher, Call} from "../src/interfaces/ICallDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {ISwapRouter} from "@uniswap/v3-periphery/contracts/interfaces/ISwapRouter.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";

contract IntentGatewayV2Test is MainnetForkBaseTest {
    IntentGatewayV2 public intentGateway;

    // Mainnet addresses
    address public constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address public constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;

    // Test users
    address public user;
    address public filler;

    // Protocol fee in BPS (30 BPS = 0.3%)
    uint256 public constant PROTOCOL_FEE_BPS = 30;

    function setUp() public override {
        super.setUp();

        // Setup test accounts
        user = makeAddr("user");
        filler = makeAddr("filler");

        // Deploy IntentGatewayV2
        intentGateway = new IntentGatewayV2(address(this));

        // Set params with protocol fee
        Params memory intentParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            protocolFeeBps: PROTOCOL_FEE_BPS
        });
        intentGateway.setParams(intentParams);

        // Fund test accounts
        _fundTestAccounts();
    }

    function _fundTestAccounts() internal {
        // Fund user with ETH and tokens using deal
        vm.deal(user, 10 ether);
        deal(address(usdc), user, 10000 * 1e6); // 10,000 USDC
        deal(address(dai), user, 10000 * 1e18); // 10,000 DAI

        // Fund filler with ETH and tokens
        vm.deal(filler, 100 ether);
        deal(address(usdc), filler, 10000 * 1e6);
        deal(address(dai), filler, 10000 * 1e18);
    }

    function testPredispatchSwapWithUniswapV2() public {
        // Test scenario: User wants to swap 1 ETH for DAI using UniswapV2, then escrow the DAI
        uint256 ethAmount = 1 ether;
        uint256 expectedDaiAmount = 2000 * 1e18; // Approximate amount, adjust based on current prices

        // Prepare predispatch call to swap ETH -> DAI via UniswapV2
        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            0, // amountOutMin (set to 0 for test)
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(_uniswapV2Router),
            value: ethAmount,
            data: swapCalldata
        });

        // Setup predispatch info
        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0), // Native token (ETH)
            amount: ethAmount
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: predispatchAssets,
            call: abi.encode(calls)
        });

        // Setup order inputs (what will be escrowed)
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: expectedDaiAmount
        });

        // Setup order outputs (what filler will provide on destination chain)
        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6, // 2000 USDC
            beneficiary: bytes32(uint256(uint160(user)))
        });

        // Create order
        Order memory order = Order({
            user: bytes32(0), // Will be set by contract
            sourceChain: "", // Will be set by contract
            destChain: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0, // Will be set by contract
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        // Place order
        vm.startPrank(user);

        // Record events
        vm.recordLogs();

        uint256 daiBalanceBefore = dai.balanceOf(address(intentGateway));

        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));

        uint256 daiBalanceAfter = dai.balanceOf(address(intentGateway));

        vm.stopPrank();

        // Verify DAI was received
        assertGe(daiBalanceAfter - daiBalanceBefore, expectedDaiAmount, "Expected DAI not escrowed");

        // Check for DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
                break;
            }
        }

        // Dust should be collected if swap returned more than expected
        if (daiBalanceAfter - daiBalanceBefore > expectedDaiAmount) {
            assertTrue(dustCollectedFound, "DustCollected event should be emitted");
        }
    }

    function testPredispatchSwapWithUniswapV3() public {
        // Test scenario: User wants to swap 1 ETH for USDC using UniswapV3
        uint256 ethAmount = 1 ether;
        uint256 expectedUsdcAmount = 2000 * 1e6; // Approximate amount

        // Prepare predispatch call to swap ETH -> USDC via UniswapV3
        ISwapRouter swapRouter = ISwapRouter(UNISWAP_V3_ROUTER);

        ISwapRouter.ExactInputSingleParams memory swapParams = ISwapRouter.ExactInputSingleParams({
            tokenIn: WETH,
            tokenOut: address(usdc),
            fee: 3000, // 0.3% fee tier
            recipient: address(dispatcher),
            deadline: block.timestamp + 3600,
            amountIn: ethAmount,
            amountOutMinimum: 0,
            sqrtPriceLimitX96: 0
        });

        bytes memory swapCalldata = abi.encodeWithSelector(
            swapRouter.exactInputSingle.selector,
            swapParams
        );

        // First, we need to wrap ETH to WETH
        bytes memory wrapCalldata = abi.encodeWithSignature("deposit()");

        Call[] memory calls = new Call[](3);
        calls[0] = Call({
            to: WETH,
            value: ethAmount,
            data: wrapCalldata
        });

        // Approve WETH to UniswapV3 router
        calls[1] = Call({
            to: WETH,
            value: 0,
            data: abi.encodeWithSelector(IERC20.approve.selector, UNISWAP_V3_ROUTER, ethAmount)
        });

        calls[2] = Call({
            to: UNISWAP_V3_ROUTER,
            value: 0,
            data: swapCalldata
        });

        // Setup predispatch info
        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0), // Native token (ETH)
            amount: ethAmount
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: predispatchAssets,
            call: abi.encode(calls)
        });

        // Setup order inputs
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: expectedUsdcAmount
        });

        // Setup order outputs
        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: 2000 * 1e18,
            beneficiary: bytes32(uint256(uint160(user)))
        });

        // Create order
        Order memory order = Order({
            user: bytes32(0),
            sourceChain: "",
            destChain: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        // Place order
        vm.startPrank(user);
        vm.recordLogs();

        uint256 usdcBalanceBefore = usdc.balanceOf(address(intentGateway));

        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));

        uint256 usdcBalanceAfter = usdc.balanceOf(address(intentGateway));

        vm.stopPrank();

        // Verify USDC was received
        assertGe(usdcBalanceAfter - usdcBalanceBefore, expectedUsdcAmount, "Expected USDC not escrowed");
    }

    function testProtocolDustCollection() public {
        // Test that excess tokens from swaps are collected as dust
        uint256 ethAmount = 1 ether;
        uint256 requestedDaiAmount = 1500 * 1e18; // Request less than what swap will return

        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            0,
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(_uniswapV2Router),
            value: ethAmount,
            data: swapCalldata
        });

        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0),
            amount: ethAmount
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: predispatchAssets,
            call: abi.encode(calls)
        });

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: requestedDaiAmount
        });

        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 1500 * 1e6,
            beneficiary: bytes32(uint256(uint160(user)))
        });

        Order memory order = Order({
            user: bytes32(0),
            sourceChain: "",
            destChain: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        vm.startPrank(user);
        vm.recordLogs();

        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));

        vm.stopPrank();

        // Check DustCollected event was emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;
        uint256 dustAmount = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
                // Decode the amount from event data
                (, dustAmount) = abi.decode(entries[i].data, (address, uint256));
                break;
            }
        }

        assertTrue(dustCollectedFound, "DustCollected event should be emitted");
        assertGt(dustAmount, 0, "Dust amount should be greater than 0");
    }

    function testProtocolFeeBpsChargedToFiller() public {
        // Test that protocol fee event is correctly emitted when filler fills order
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1000 * 1e18; // 1000 DAI

        // Calculate expected protocol fee (30 BPS = 0.3%)
        uint256 expectedProtocolFee = (inputAmount * PROTOCOL_FEE_BPS) / 10_000;

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: inputAmount
        });

        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: outputAmount,
            beneficiary: bytes32(uint256(uint160(user)))
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: new TokenInfo[](0),
            call: ""
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            sourceChain: host.host(),
            destChain: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order
        vm.startPrank(filler);
        dai.approve(address(intentGateway), outputAmount);
        // Approve fee token for dispatch costs
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        FillOptions memory fillOptions = FillOptions({relayerFee: 0});
        intentGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // Check FeeCollected event was emitted with correct values
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        uint256 feeAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("FeeCollected(address,uint256,bytes)")) {
                eventFound = true;
                // Decode event to verify values
                (address token, uint256 amount, ) = abi.decode(entries[i].data, (address, uint256, bytes));
                assertEq(token, address(usdc), "Token should be USDC");
                feeAmountFromEvent = amount;
                break;
            }
        }

        assertTrue(eventFound, "FeeCollected event should be emitted");
        assertEq(feeAmountFromEvent, expectedProtocolFee, "Protocol fee amount should match expected");
    }

    function testProtocolFeeWithMultipleTokens() public {
        // Test protocol fee event collection with multiple input tokens
        uint256 usdcAmount = 1000 * 1e6;
        uint256 daiAmount = 1000 * 1e18;

        uint256 expectedUsdcFee = (usdcAmount * PROTOCOL_FEE_BPS) / 10_000;
        uint256 expectedDaiFee = (daiAmount * PROTOCOL_FEE_BPS) / 10_000;

        // Setup order with multiple inputs
        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: usdcAmount
        });
        inputs[1] = TokenInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: daiAmount
        });

        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: 2000 * 1e18,
            beneficiary: bytes32(uint256(uint160(user)))
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: new TokenInfo[](0),
            call: ""
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            sourceChain: host.host(),
            destChain: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), usdcAmount);
        dai.approve(address(intentGateway), daiAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order
        vm.startPrank(filler);
        dai.approve(address(intentGateway), 2000 * 1e18);
        // Approve fee token for dispatch costs
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        FillOptions memory fillOptions = FillOptions({relayerFee: 0});
        intentGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // Verify two FeeCollected events were emitted with correct values
        Vm.Log[] memory entries = vm.getRecordedLogs();
        uint256 protocolFeeEventCount = 0;
        bool usdcFeeFound = false;
        bool daiFeeFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("FeeCollected(address,uint256,bytes)")) {
                protocolFeeEventCount++;
                (address token, uint256 amount, ) = abi.decode(entries[i].data, (address, uint256, bytes));

                if (token == address(usdc) && amount == expectedUsdcFee) {
                    usdcFeeFound = true;
                } else if (token == address(dai) && amount == expectedDaiFee) {
                    daiFeeFound = true;
                }
            }
        }

        assertEq(protocolFeeEventCount, 2, "Should emit two FeeCollected events");
        assertTrue(usdcFeeFound, "USDC fee event not found");
        assertTrue(daiFeeFound, "DAI fee event not found");
    }

    function testNoProtocolFeeWhenBpsIsZero() public {
        // Deploy a new IntentGateway with 0 BPS
        IntentGatewayV2 zeroFeeGateway = new IntentGatewayV2(address(this));

        Params memory zeroFeeParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            protocolFeeBps: 0 // No protocol fee
        });
        zeroFeeGateway.setParams(zeroFeeParams);

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: inputAmount
        });

        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: 1000 * 1e18,
            beneficiary: bytes32(uint256(uint160(user)))
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: new TokenInfo[](0),
            call: ""
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            sourceChain: host.host(),
            destChain: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(zeroFeeGateway), inputAmount);
        zeroFeeGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order
        vm.startPrank(filler);
        dai.approve(address(zeroFeeGateway), 1000 * 1e18);
        // Approve fee token for dispatch costs
        dai.approve(address(zeroFeeGateway), type(uint256).max);

        vm.recordLogs();

        FillOptions memory fillOptions = FillOptions({relayerFee: 0});
        zeroFeeGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // No FeeCollected event should be emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        for (uint256 i = 0; i < entries.length; i++) {
            assertTrue(
                entries[i].topics[0] != keccak256("FeeCollected(address,uint256,bytes)"),
                "FeeCollected event should not be emitted"
            );
        }
    }

    function testPredispatchFailsWithInsufficientBalance() public {
        // Test that predispatch reverts if swap doesn't produce enough tokens
        uint256 ethAmount = 0.01 ether; // Very small amount
        uint256 unrealisticDaiAmount = 10000 * 1e18; // Unrealistically high expectation

        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            0,
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(_uniswapV2Router),
            value: ethAmount,
            data: swapCalldata
        });

        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0),
            amount: ethAmount
        });

        PredispatchInfo memory predispatch = PredispatchInfo({
            assets: predispatchAssets,
            call: abi.encode(calls)
        });

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: unrealisticDaiAmount
        });

        PaymentInfo[] memory outputs = new PaymentInfo[](1);
        outputs[0] = PaymentInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 1000 * 1e6,
            beneficiary: bytes32(uint256(uint160(user)))
        });

        Order memory order = Order({
            user: bytes32(0),
            sourceChain: "",
            destChain: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            outputs: outputs,
            inputs: inputs,
            predispatch: predispatch,
            callData: ""
        });

        vm.startPrank(user);
        vm.expectRevert(IntentGatewayV2.InvalidInput.selector);
        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));
        vm.stopPrank();
    }

    function testCollectProtocolFeesERC20() public {
        // Simulate accumulated protocol fees in the gateway
        uint256 feeAmount = 1000 * 1e6;

        // Transfer tokens to gateway instead of using deal
        vm.prank(user);
        usdc.transfer(address(intentGateway), feeAmount);

        // Setup fee collection request
        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: feeAmount // Collect exact amount
        });

        CollectFees memory collectFeesReq = CollectFees({
            beneficiary: treasury,
            outputs: outputs
        });

        // Create collect fees request from hyperbridge
        bytes memory data = abi.encode(collectFeesReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.CollectFees)), data),
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        uint256 treasuryBalanceBefore = usdc.balanceOf(treasury);
        uint256 gatewayBalanceBefore = usdc.balanceOf(address(intentGateway));

        // Verify gateway has the funds before collection
        assertEq(gatewayBalanceBefore, feeAmount, "Gateway should have funds before collection");

        // Execute fee collection
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Verify fees were transferred
        assertEq(usdc.balanceOf(treasury) - treasuryBalanceBefore, feeAmount, "Treasury should receive protocol fees");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC left");

        // Verify RevenueWithdrawn event was emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("RevenueWithdrawn(address,uint256,address)")) {
                eventFound = true;
                break;
            }
        }
        assertTrue(eventFound, "RevenueWithdrawn event should be emitted");
    }

    function testCollectProtocolFeesNative() public {
        // Simulate accumulated ETH fees
        uint256 feeAmount = 1 ether;
        vm.deal(address(intentGateway), feeAmount);

        address treasury = user; // Use existing user address that can receive ETH
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(0),
            amount: feeAmount
        });

        CollectFees memory collectFeesReq = CollectFees({
            beneficiary: treasury,
            outputs: outputs
        });

        bytes memory data = abi.encode(collectFeesReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.CollectFees)), data),
            timeoutTimestamp: 0
        });

        uint256 treasuryBalanceBefore = treasury.balance;

        // Verify gateway has the funds before collection
        assertEq(address(intentGateway).balance, feeAmount, "Gateway should have ETH before collection");

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        assertEq(treasury.balance - treasuryBalanceBefore, feeAmount, "Treasury should receive ETH fees");
        assertEq(address(intentGateway).balance, 0, "Gateway should have no ETH left");
    }

    function testCollectMultipleTokenFees() public {
        // Fund gateway with multiple tokens
        uint256 usdcAmount = 500 * 1e6;
        uint256 daiAmount = 1000 * 1e18;
        uint256 ethAmount = 0.5 ether;

        // Transfer tokens to gateway
        vm.startPrank(user);
        usdc.transfer(address(intentGateway), usdcAmount);
        dai.transfer(address(intentGateway), daiAmount);
        vm.stopPrank();
        vm.deal(address(intentGateway), ethAmount);

        address treasury = user; // Use existing user address that can receive ETH
        TokenInfo[] memory outputs = new TokenInfo[](3);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: usdcAmount
        });
        outputs[1] = TokenInfo({
            token: bytes32(uint256(uint160(address(dai)))),
            amount: daiAmount
        });
        outputs[2] = TokenInfo({
            token: bytes32(0),
            amount: ethAmount
        });

        CollectFees memory collectFeesReq = CollectFees({
            beneficiary: treasury,
            outputs: outputs
        });

        bytes memory data = abi.encode(collectFeesReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.CollectFees)), data),
            timeoutTimestamp: 0
        });

        uint256 usdcBalanceBefore = usdc.balanceOf(treasury);
        uint256 daiBalanceBefore = dai.balanceOf(treasury);
        uint256 ethBalanceBefore = treasury.balance;

        // Verify gateway has all funds before collection
        assertEq(usdc.balanceOf(address(intentGateway)), usdcAmount, "Gateway should have USDC");
        assertEq(dai.balanceOf(address(intentGateway)), daiAmount, "Gateway should have DAI");
        assertEq(address(intentGateway).balance, ethAmount, "Gateway should have ETH");

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Verify all fees were collected
        assertEq(usdc.balanceOf(treasury) - usdcBalanceBefore, usdcAmount, "Treasury should receive USDC");
        assertEq(dai.balanceOf(treasury) - daiBalanceBefore, daiAmount, "Treasury should receive DAI");
        assertEq(treasury.balance - ethBalanceBefore, ethAmount, "Treasury should receive ETH");

        // Gateway should be empty
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway USDC should be 0");
        assertEq(dai.balanceOf(address(intentGateway)), 0, "Gateway DAI should be 0");
        assertEq(address(intentGateway).balance, 0, "Gateway ETH should be 0");
    }

    function testCollectPartialFees() public {
        // Fund gateway with tokens
        uint256 totalAmount = 1000 * 1e6;
        uint256 collectAmount = 300 * 1e6;

        // Transfer tokens to gateway
        vm.prank(user);
        usdc.transfer(address(intentGateway), totalAmount);

        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: collectAmount // Collect specific amount
        });

        CollectFees memory collectFeesReq = CollectFees({
            beneficiary: treasury,
            outputs: outputs
        });

        bytes memory data = abi.encode(collectFeesReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.CollectFees)), data),
            timeoutTimestamp: 0
        });

        uint256 treasuryBalanceBefore = usdc.balanceOf(treasury);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Verify partial collection
        assertEq(usdc.balanceOf(treasury) - treasuryBalanceBefore, collectAmount, "Treasury should receive partial amount");
        assertEq(usdc.balanceOf(address(intentGateway)), totalAmount - collectAmount, "Gateway should keep remainder");
    }

    function testCollectFeesUnauthorized() public {
        // Fund gateway
        vm.prank(user);
        usdc.transfer(address(intentGateway), 1000 * 1e6);

        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 1000 * 1e6
        });

        CollectFees memory collectFeesReq = CollectFees({
            beneficiary: treasury,
            outputs: outputs
        });

        bytes memory data = abi.encode(collectFeesReq);

        // Request NOT from hyperbridge
        PostRequest memory request = PostRequest({
            source: bytes("UNAUTHORIZED_CHAIN"),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(0x1234))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.CollectFees)), data),
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        vm.expectRevert(IntentGatewayV2.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));
    }

    function testCollectFeesExceedsBalance() public {
        // Fund gateway with less than requested
        uint256 actualBalance = 100 * 1e6;
        uint256 requestedAmount = 1000 * 1e6;

        // Transfer tokens to gateway
        vm.prank(user);
        usdc.transfer(address(intentGateway), actualBalance);

        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: requestedAmount // Request more than available
        });

        CollectFees memory collectFeesReq = CollectFees({
            beneficiary: treasury,
            outputs: outputs
        });

        bytes memory data = abi.encode(collectFeesReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            to: abi.encodePacked(bytes32(uint256(uint160(address(intentGateway))))),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.CollectFees)), data),
            timeoutTimestamp: 0
        });

        // Should revert when requesting more than available balance
        vm.prank(address(host));
        vm.expectRevert();
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));
    }
}
