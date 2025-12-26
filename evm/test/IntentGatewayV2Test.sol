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
import {
    IntentGatewayV2,
    Order,
    Params,
    TokenInfo,
    SweepDust,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    CancelOptions,
    NewDeployment,
    RequestBody,
    SelectOptions
} from "../src/apps/IntentGatewayV2.sol";
import {ICallDispatcher, Call} from "../src/interfaces/ICallDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {ISwapRouter} from "@uniswap/v3-periphery/contracts/interfaces/ISwapRouter.sol";
import {IQuoter} from "@uniswap/v3-periphery/contracts/interfaces/IQuoter.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {GetRequest, GetResponse, StorageValue} from "@hyperbridge/core/libraries/Message.sol";

contract IntentGatewayV2Test is MainnetForkBaseTest {
    IntentGatewayV2 public intentGateway;

    // Mainnet addresses
    address public constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address public constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    address public constant UNISWAP_V3_QUOTER = 0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6;

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

        // Set params
        Params memory intentParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000 // 100% to protocol, 0% to beneficiary (default)
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

    /// @dev Helper function to create EIP-712 signature for solver selection
    function _createSelectSolverSignature(bytes32 commitment, address solver, uint256 privateKey, address gateway)
        internal
        view
        returns (bytes memory)
    {
        // Compute the EIP-712 digest using public constants
        IntentGatewayV2 gatewayContract = IntentGatewayV2(payable(gateway));
        bytes32 structHash = keccak256(abi.encode(gatewayContract.SELECT_SOLVER_TYPEHASH(), commitment, solver));

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", gatewayContract.DOMAIN_SEPARATOR(), structHash));

        // Sign the digest
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, digest);

        // Return the signature in the expected format
        return abi.encodePacked(r, s, v);
    }

    function testDustCollectionFromPredispatchSwapWithUniswapV2() public {
        // Test scenario: User wants to swap 1 ETH for DAI using UniswapV2, then escrow the DAI
        uint256 ethAmount = 1 ether;

        // Prepare predispatch call to swap ETH -> DAI via UniswapV2
        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        // Get quote for expected output
        uint256[] memory amounts = _uniswapV2Router.getAmountsOut(ethAmount, path);
        uint256 expectedDaiAmount = amounts[1];
        uint256 minDaiAmount = (expectedDaiAmount * 95) / 100; // 5% slippage tolerance

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            minDaiAmount,
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({to: address(_uniswapV2Router), value: ethAmount, data: swapCalldata});

        // Setup predispatch info
        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0), // Native token (ETH)
            amount: ethAmount
        });

        DispatchInfo memory predispatch = DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)});

        // Setup order inputs (what will be escrowed)
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: minDaiAmount});

        // Setup order output assets (what filler will provide on destination chain)
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6 // 2000 USDC
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create order
        Order memory order = Order({
            user: bytes32(0), // Will be set by contract
            source: "", // Will be set by contract
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0, // Will be set by contract
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
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
        assertGe(daiBalanceAfter - daiBalanceBefore, minDaiAmount, "Minimum DAI not escrowed");

        // Check for DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
                break;
            }
        }

        assertTrue(dustCollectedFound, "DustCollected event should be emitted");
    }

    function testDustCollectionFromPredispatchSwapWithUniswapV3() public {
        // Test scenario: User wants to swap 1 ETH for USDC using UniswapV3
        uint256 ethAmount = 1 ether;

        // Get quote for expected output and calculate minimum with slippage
        uint256 minUsdcAmount =
            (IQuoter(UNISWAP_V3_QUOTER)
                        .quoteExactInputSingle(
                            WETH,
                            address(usdc),
                            3000, // 0.3% fee tier
                            ethAmount,
                            0
                        )
                    * 95) / 100; // 5% slippage tolerance

        // Prepare predispatch call to swap ETH -> USDC via UniswapV3
        bytes memory swapCalldata = abi.encodeWithSelector(
            ISwapRouter.exactInputSingle.selector,
            ISwapRouter.ExactInputSingleParams({
                tokenIn: WETH,
                tokenOut: address(usdc),
                fee: 3000, // 0.3% fee tier
                recipient: address(dispatcher),
                deadline: block.timestamp + 3600,
                amountIn: ethAmount,
                amountOutMinimum: minUsdcAmount,
                sqrtPriceLimitX96: 0
            })
        );

        // Setup calls: wrap ETH, approve WETH, and swap
        Call[] memory calls = new Call[](3);
        calls[0] = Call({to: WETH, value: ethAmount, data: abi.encodeWithSignature("deposit()")});
        calls[1] = Call({
            to: WETH, value: 0, data: abi.encodeWithSelector(IERC20.approve.selector, UNISWAP_V3_ROUTER, ethAmount)
        });
        calls[2] = Call({to: UNISWAP_V3_ROUTER, value: 0, data: swapCalldata});

        // Setup predispatch info
        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({
            token: bytes32(0), // Native token (ETH)
            amount: ethAmount
        });

        DispatchInfo memory predispatch = DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)});

        // Setup order inputs
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: minUsdcAmount});

        // Setup order outputs
        // Setup order output assets (what filler will provide on destination chain)
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6 // 2000 USDC
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Create order
        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // Place order
        vm.startPrank(user);
        vm.recordLogs();

        uint256 usdcBalanceBefore = usdc.balanceOf(address(intentGateway));

        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));

        uint256 usdcBalanceAfter = usdc.balanceOf(address(intentGateway));

        vm.stopPrank();

        // Verify USDC was received
        assertGe(usdcBalanceAfter - usdcBalanceBefore, minUsdcAmount, "Minimum USDC not escrowed");

        // Check for DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustCollectedFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                dustCollectedFound = true;
                break;
            }
        }

        assertTrue(dustCollectedFound, "DustCollected event should be emitted");
    }

    function testDustCollectionFromSolverSingleToken() public {
        // Test that dust is correctly collected when solver provides extra tokens
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1000 * 1e18; // 1000 DAI
        uint256 dust = 3 * 1e18; // 3 DAI extra as dust

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Record gateway DAI balance before fill
        uint256 gatewayDaiBalanceBefore = dai.balanceOf(address(intentGateway));
        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        // Filler fills order with extra tokens (dust)
        vm.startPrank(filler);
        dai.approve(address(intentGateway), outputAmount + dust);
        // Approve fee token for dispatch costs
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount + dust});

        FillOptions memory fillOptions = FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs});
        intentGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // Verify user received exact requested amount
        assertEq(
            dai.balanceOf(user) - userDaiBalanceBefore, outputAmount, "User should receive exactly the requested amount"
        );

        // Verify gateway collected the exact dust amount
        assertEq(
            dai.balanceOf(address(intentGateway)) - gatewayDaiBalanceBefore,
            dust,
            "Gateway should hold exactly the dust amount"
        );

        // Check DustCollected event was emitted with correct values
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        uint256 dustAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                eventFound = true;
                // Decode event to verify values
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(token, address(dai), "Token should be DAI");
                dustAmountFromEvent = amount;
                break;
            }
        }

        assertTrue(eventFound, "DustCollected event should be emitted");
        assertEq(dustAmountFromEvent, dust, "Event dust amount should match expected");
    }

    function testDustCollectionFromSolverNativeToken() public {
        // Test that dust is correctly collected when solver provides extra native tokens
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1 ether; // 1 ETH
        uint256 dust = 0.1 ether; // 0.1 ETH extra as dust

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        // Setup order output assets
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(0), // Native token
            amount: outputAmount
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Record gateway ETH balance before fill
        uint256 gatewayEthBalanceBefore = address(intentGateway).balance;
        uint256 userEthBalanceBefore = user.balance;

        // Filler fills order with extra native tokens (dust)
        vm.startPrank(filler);
        // Approve fee token for dispatch costs
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: outputAmount + dust});

        FillOptions memory fillOptions = FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs});
        intentGateway.fillOrder{value: outputAmount + dust}(order, fillOptions);

        vm.stopPrank();

        // Verify user received exact requested amount
        assertEq(
            user.balance - userEthBalanceBefore, outputAmount, "User should receive exactly the requested ETH amount"
        );

        // Verify gateway collected the exact dust amount
        assertEq(
            address(intentGateway).balance - gatewayEthBalanceBefore,
            dust,
            "Gateway should hold exactly the ETH dust amount"
        );

        // Check DustCollected event was emitted with correct values
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        uint256 dustAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                // Decode event to verify values
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                if (token == address(0)) {
                    eventFound = true;
                    dustAmountFromEvent = amount;
                    break;
                }
            }
        }

        assertTrue(eventFound, "DustCollected event should be emitted for native token");
        assertEq(dustAmountFromEvent, dust, "Event dust amount should match expected");
    }

    function testNoDustCollectionWhenExactAmount() public {
        // Test that no dust is collected when solver provides exact amount
        IntentGatewayV2 zeroFeeGateway = new IntentGatewayV2(address(this));

        Params memory zeroFeeParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000 // 100% to protocol, 0% to beneficiary
        });
        zeroFeeGateway.setParams(zeroFeeParams);

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
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

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        FillOptions memory fillOptions = FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs});
        zeroFeeGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // No DustCollected event should be emitted when solver provides exact amounts
        Vm.Log[] memory entries = vm.getRecordedLogs();
        for (uint256 i = 0; i < entries.length; i++) {
            assertTrue(
                entries[i].topics[0] != keccak256("DustCollected(address,uint256)"),
                "DustCollected event should not be emitted when no dust"
            );
        }
    }

    function testPredispatchFailsWithInsufficientBalance() public {
        // Test that predispatch reverts if swap doesn't produce enough tokens
        uint256 ethAmount = 0.01 ether; // Very small amount

        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = address(dai);

        // Get quote for expected output
        uint256[] memory amounts = _uniswapV2Router.getAmountsOut(ethAmount, path);
        uint256 expectedDaiFromSwap = amounts[1];
        uint256 unrealisticDaiAmount = expectedDaiFromSwap * 10; // Request 10x more than possible
        uint256 minDaiAmount = (expectedDaiFromSwap * 95) / 100; // 5% slippage tolerance

        bytes memory swapCalldata = abi.encodeWithSelector(
            _uniswapV2Router.swapExactETHForTokens.selector,
            minDaiAmount,
            path,
            address(dispatcher),
            block.timestamp + 3600
        );

        Call[] memory calls = new Call[](1);
        calls[0] = Call({to: address(_uniswapV2Router), value: ethAmount, data: swapCalldata});

        TokenInfo[] memory predispatchAssets = new TokenInfo[](1);
        predispatchAssets[0] = TokenInfo({token: bytes32(0), amount: ethAmount});

        DispatchInfo memory predispatch = DispatchInfo({assets: predispatchAssets, call: abi.encode(calls)});

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: unrealisticDaiAmount});

        // Setup order output assets
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: 2000 * 1e6 // 2000 USDC
        });

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: abi.encodePacked("DEST_CHAIN"),
            deadline: 0,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        vm.expectRevert(IntentGatewayV2.InvalidInput.selector);
        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));
        vm.stopPrank();
    }

    function testSweepDustERC20() public {
        // Simulate accumulated dust in the gateway
        uint256 feeAmount = 1000 * 1e6;

        // Transfer tokens to gateway instead of using deal
        vm.prank(user);
        usdc.transfer(address(intentGateway), feeAmount);

        // Setup fee collection request
        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({
            token: bytes32(uint256(uint160(address(usdc)))),
            amount: feeAmount // Sweep exact amount
        });

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        // Create sweep dust request from hyperbridge
        bytes memory data = abi.encode(sweepDustReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        uint256 treasuryBalanceBefore = usdc.balanceOf(treasury);
        uint256 gatewayBalanceBefore = usdc.balanceOf(address(intentGateway));

        // Verify gateway has the funds before collection
        assertEq(gatewayBalanceBefore, feeAmount, "Gateway should have funds before collection");

        // Execute dust sweep
        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Verify dust was transferred
        assertEq(usdc.balanceOf(treasury) - treasuryBalanceBefore, feeAmount, "Treasury should receive protocol fees");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC left");

        // Verify DustSwept event was emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustSwept(address,uint256,address)")) {
                eventFound = true;
                break;
            }
        }
        assertTrue(eventFound, "DustSwept event should be emitted");
    }

    function testSweepDustNative() public {
        // Simulate accumulated ETH dust
        uint256 feeAmount = 1 ether;
        vm.deal(address(intentGateway), feeAmount);

        address treasury = user; // Use existing user address that can receive ETH
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(0), amount: feeAmount});

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        bytes memory data = abi.encode(sweepDustReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        uint256 treasuryBalanceBefore = treasury.balance;

        // Verify gateway has the funds before sweep
        assertEq(address(intentGateway).balance, feeAmount, "Gateway should have ETH before sweep");

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        assertEq(treasury.balance - treasuryBalanceBefore, feeAmount, "Treasury should receive ETH dust");
        assertEq(address(intentGateway).balance, 0, "Gateway should have no ETH left");
    }

    function testSweepMultipleTokenDust() public {
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
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcAmount});
        outputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiAmount});
        outputs[2] = TokenInfo({token: bytes32(0), amount: ethAmount});

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        bytes memory data = abi.encode(sweepDustReq);
        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.SweepDust)), data),
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

        // Verify all dust was swept
        assertEq(usdc.balanceOf(treasury) - usdcBalanceBefore, usdcAmount, "Treasury should receive USDC");
        assertEq(dai.balanceOf(treasury) - daiBalanceBefore, daiAmount, "Treasury should receive DAI");
        assertEq(treasury.balance - ethBalanceBefore, ethAmount, "Treasury should receive ETH");

        // Gateway should be empty
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway USDC should be 0");
        assertEq(dai.balanceOf(address(intentGateway)), 0, "Gateway DAI should be 0");
        assertEq(address(intentGateway).balance, 0, "Gateway ETH should be 0");
    }

    function testSurplusSplitBetweenBeneficiaryAndProtocol() public {
        // Test 50/50 split: solver provides 2100 DAI, user gets 2050, protocol gets 50
        IntentGatewayV2 customGateway = new IntentGatewayV2(address(this));
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 5000 // 50% to protocol, 50% to beneficiary
        });
        customGateway.setParams(customParams);

        uint256 solverOutputAmount = 2100 * 1e18;

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with surplus
        vm.startPrank(filler);
        dai.approve(address(customGateway), 2200 * 1e18); // Approve surplus + fees

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverOutputAmount});

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        vm.recordLogs();
        customGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        // Verify beneficiary received 2050 DAI (2000 + 50% of 100 surplus)
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, 2050 * 1e18, "Beneficiary gets 50% surplus");

        // Verify protocol received 50 DAI (50% of 100 surplus)
        assertEq(dai.balanceOf(address(customGateway)), 50 * 1e18, "Protocol gets 50% surplus");

        // Verify DustCollected event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool found = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                found = true;
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(token, address(dai), "Token should be DAI");
                assertEq(amount, 50 * 1e18, "Amount should be 50 DAI");
                break;
            }
        }
        assertTrue(found, "DustCollected event should be emitted");
    }

    function testSurplusSplitWith100PercentToBeneficiary() public {
        // Test with 100% surplus going to beneficiary (0% to protocol)
        IntentGatewayV2 customGateway = new IntentGatewayV2(address(this));
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 0 // 0% to protocol, 100% to beneficiary
        });
        customGateway.setParams(customParams);

        uint256 solverOutputAmount = 2100 * 1e18; // 100 DAI surplus

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with surplus
        vm.startPrank(filler);
        dai.approve(address(customGateway), 2200 * 1e18); // Approve surplus + fees

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverOutputAmount});

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        customGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        // Verify beneficiary received requested amount + all surplus (2000 + 100 = 2100 DAI)
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, 2100 * 1e18, "Beneficiary should receive 100% of surplus");

        // Verify protocol received nothing
        assertEq(dai.balanceOf(address(customGateway)), 0, "Protocol should receive 0%");
    }

    function testSurplusSplitWith0PercentToBeneficiary() public {
        // Test 0/100 split: solver provides 2100 DAI, user gets 2000, protocol gets 100
        IntentGatewayV2 customGateway = new IntentGatewayV2(address(this));
        Params memory customParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: 10000 // 100% to protocol, 0% to beneficiary
        });
        customGateway.setParams(customParams);

        uint256 solverOutputAmount = 2100 * 1e18; // 100 DAI surplus

        // Setup order
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        customGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with surplus
        vm.startPrank(filler);
        dai.approve(address(customGateway), 2200 * 1e18); // Approve surplus + fees

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverOutputAmount});

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        vm.recordLogs();
        customGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        // Verify beneficiary received only requested amount (2000 DAI, no surplus)
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, 2000 * 1e18, "Beneficiary should receive only requested");

        // Verify protocol received all surplus (100 DAI)
        assertEq(dai.balanceOf(address(customGateway)), 100 * 1e18, "Protocol should receive 100% of surplus");

        // Verify DustCollected event was emitted
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool found = false;
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                found = true;
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(token, address(dai), "Token should be DAI");
                assertEq(amount, 100 * 1e18, "Amount should be 100 DAI");
                break;
            }
        }
        assertTrue(found, "DustCollected event should be emitted");
    }

    function testSurplusWithCalldataGoesToProtocol() public {
        // Test that when calldata is present, ALL surplus goes to protocol
        // Compare: without calldata and 50% split, protocol gets 50 DAI
        //          with calldata and 50% split, protocol gets 100 DAI (all surplus)
        IntentGatewayV2 customGateway = new IntentGatewayV2(address(this));
        customGateway.setParams(
            Params({
                host: address(host), dispatcher: address(dispatcher), solverSelection: false, surplusShareBps: 5000
            })
        );

        // Setup order WITH calldata
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        // Create postdispatch calls with a simple token approval (non-reverting)
        Call[] memory postdispatchCalls = new Call[](1);
        postdispatchCalls[0] = Call({
            to: address(dai),
            value: 0,
            data: abi.encodeWithSelector(IERC20.approve.selector, address(intentGateway), 1000 * 1e18)
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: abi.encodePacked(host.host()),
            destination: abi.encodePacked(host.host()),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({
                beneficiary: bytes32(uint256(uint160(user))),
                assets: outputAssets,
                call: abi.encode(postdispatchCalls) // Valid non-reverting calldata
            })
        });

        // Place and fill order
        vm.prank(user);
        usdc.approve(address(customGateway), 1000 * 1e6);
        vm.prank(user);
        customGateway.placeOrder(order, bytes32(0));

        vm.prank(filler);
        dai.approve(address(customGateway), 2200 * 1e18);

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2100 * 1e18});

        uint256 userBalanceBefore = dai.balanceOf(user);
        uint256 gatewayBalanceBefore = dai.balanceOf(address(customGateway));

        vm.recordLogs();
        vm.prank(filler);
        customGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));

        // Verify beneficiary got ONLY requested amount (2000 DAI, no surplus)
        assertEq(
            dai.balanceOf(user) - userBalanceBefore, 2000 * 1e18, "Beneficiary should get 0% surplus with calldata"
        );

        // Verify protocol got ALL surplus (100 DAI, not 50 which would be with 50% split)
        assertEq(
            dai.balanceOf(address(customGateway)) - gatewayBalanceBefore,
            100 * 1e18,
            "Protocol should get 100% surplus with calldata"
        );

        // Verify DustCollected event shows full 100 DAI
        Vm.Log[] memory entries = vm.getRecordedLogs();
        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                (, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                assertEq(amount, 100 * 1e18, "All surplus should go to protocol with calldata");
                return; // Test passed
            }
        }
        fail("DustCollected event not found");
    }

    function testSweepDustUnauthorized() public {
        // Fund gateway
        vm.prank(user);
        usdc.transfer(address(intentGateway), 1000 * 1e6);

        address treasury = user; // Use existing user address
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        SweepDust memory sweepDustReq = SweepDust({beneficiary: treasury, outputs: outputs});

        bytes memory data = abi.encode(sweepDustReq);

        // Request NOT from hyperbridge
        PostRequest memory request = PostRequest({
            source: bytes("UNAUTHORIZED_CHAIN"),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(0x1234)),
            to: abi.encodePacked(address(intentGateway)),
            body: bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.SweepDust)), data),
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        vm.expectRevert(IntentGatewayV2.Unauthorized.selector);
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));
    }

    function testFillOrderWithNoPostdispatch() public {
        // Test that fillOrder works correctly when there's no postdispatch calldata
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1000 * 1e18; // 1000 DAI

        // Setup order with no postdispatch
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        DispatchInfo memory predispatch = DispatchInfo({assets: new TokenInfo[](0), call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: predispatch,
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Filler fills order with no postdispatch in dispatcher
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        FillOptions memory fillOptions = FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs});
        intentGateway.fillOrder(order, fillOptions);

        vm.stopPrank();

        // Verify user received DAI
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, outputAmount, "User should receive output amount");
    }

    function testPostdispatchTokenSweep() public {
        // Test realistic postdispatch: exact output swap on Uniswap V2 where refunded input tokens are swept
        // Scenario: User wants 1000 DAI on destination, solver sends USDC to dispatcher,
        // dispatcher swaps exact output for DAI, refunded USDC is swept back to gateway

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC escrow
        uint256 daiOutputAmount = 1000 * 1e18; // Exact 1000 DAI output wanted

        // Setup order inputs
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        // Create postdispatch calls that:
        // 1. Approve Uniswap router to spend USDC
        // 2. Execute exact output swap (swapTokensForExactTokens) - USDC -> DAI
        // 3. Transfer DAI to user
        Call[] memory postdispatchCalls = new Call[](3);

        // Get quote for how much USDC needed for 1000 DAI (will be less than what solver sends)
        address[] memory path = new address[](2);
        path[0] = address(usdc);
        path[1] = address(dai);
        address uniswapRouter = 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D;
        uint256[] memory amounts = IUniswapV2Router02(uniswapRouter).getAmountsIn(daiOutputAmount, path);
        uint256 usdcNeeded = amounts[0];

        // Call 1: Approve Uniswap router
        postdispatchCalls[0] = Call({
            to: address(usdc),
            value: 0,
            data: abi.encodeWithSelector(IERC20.approve.selector, uniswapRouter, type(uint256).max)
        });

        // Call 2: Exact output swap - swap USDC for exactly 1000 DAI
        postdispatchCalls[1] = Call({
            to: uniswapRouter,
            value: 0,
            data: abi.encodeWithSelector(
                bytes4(keccak256("swapTokensForExactTokens(uint256,uint256,address[],address,uint256)")),
                daiOutputAmount, // exact amount out
                type(uint256).max, // max amount in
                path,
                address(dispatcher), // tokens come back to dispatcher
                block.timestamp
            )
        });

        // Call 3: Transfer DAI to user
        postdispatchCalls[2] = Call({
            to: address(dai), value: 0, data: abi.encodeWithSelector(IERC20.transfer.selector, user, daiOutputAmount)
        });

        // Setup order output - beneficiary is dispatcher, it will receive USDC from solver
        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcNeeded + 100 * 1e6}); // Solver sends more than needed

        PaymentInfo memory output = PaymentInfo({
            beneficiary: bytes32(uint256(uint160(address(dispatcher)))), // Dispatcher receives USDC
            assets: outputAssets,
            call: abi.encode(postdispatchCalls)
        });

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Record balances before fill
        uint256 gatewayUsdcBefore = usdc.balanceOf(address(intentGateway));
        uint256 userDaiBalanceBefore = dai.balanceOf(user);

        // Filler fills order - sends USDC to dispatcher
        vm.startPrank(filler);
        uint256 solverUsdcAmount = usdcNeeded + 100 * 1e6; // Solver sends extra USDC
        usdc.approve(address(intentGateway), solverUsdcAmount);
        dai.approve(address(intentGateway), type(uint256).max);

        vm.recordLogs();

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: solverUsdcAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));

        vm.stopPrank();

        // Verify user received exact DAI output
        assertEq(dai.balanceOf(user) - userDaiBalanceBefore, daiOutputAmount, "User should receive exact DAI output");

        // Verify dispatcher has 0 USDC balance (refunded tokens were swept)
        assertEq(usdc.balanceOf(address(dispatcher)), 0, "Dispatcher should have 0 USDC after sweep");

        // Verify IntentGateway received the refunded USDC (difference between what solver sent and what swap used)
        uint256 gatewayUsdcAfter = usdc.balanceOf(address(intentGateway));
        uint256 refundedUsdc = solverUsdcAmount - usdcNeeded;
        assertGt(gatewayUsdcAfter - gatewayUsdcBefore, 0, "IntentGateway should receive refunded USDC");

        // The refunded amount should be approximately the difference (allowing for small swap variance)
        assertApproxEqAbs(
            gatewayUsdcAfter - gatewayUsdcBefore,
            refundedUsdc,
            5 * 1e6, // 5 USDC tolerance for swap price variance
            "Gateway should receive approximately the refunded USDC"
        );

        // Check DustCollected event was emitted for swept USDC
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool dustEventFound = false;
        uint256 dustAmountFromEvent = 0;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("DustCollected(address,uint256)")) {
                (address token, uint256 amount) = abi.decode(entries[i].data, (address, uint256));
                if (token == address(usdc) && amount > 0) {
                    dustEventFound = true;
                    dustAmountFromEvent = amount;
                    break;
                }
            }
        }

        assertTrue(dustEventFound, "DustCollected event should be emitted for swept refunded USDC");
        assertGt(dustAmountFromEvent, 0, "Dust amount should be greater than 0");
    }

    // ============================================
    // Solver Selection Tests
    // ============================================

    function testSelect() public {
        // Test solver selection with valid session signature
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: vm.addr(1), // Session key
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create EIP-712 signature from session key
        bytes memory sessionSignature = _createSelectSolverSignature(
            commitment,
            filler,
            1, // Session key private key
            address(intentGateway)
        );

        // Solver selects themselves
        vm.prank(filler);
        intentGateway.select(SelectOptions({commitment: commitment, solver: filler, signature: sessionSignature}));
    }

    function testFillOrderWithSolverSelection() public {
        // Enable solver selection
        Params memory newParams = Params({
            host: address(host), dispatcher: address(dispatcher), solverSelection: true, surplusShareBps: 10000
        });

        IntentGatewayV2 gatewayWithSelection = new IntentGatewayV2(address(this));
        gatewayWithSelection.setParams(newParams);

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: vm.addr(1), // Session key
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(gatewayWithSelection), inputAmount);
        gatewayWithSelection.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create EIP-712 signature from session key
        bytes memory sessionSignature = _createSelectSolverSignature(
            commitment,
            filler,
            1, // Session key private key
            address(gatewayWithSelection)
        );

        // Solver selects themselves
        vm.startPrank(filler);
        gatewayWithSelection.select(
            SelectOptions({commitment: commitment, solver: filler, signature: sessionSignature})
        );

        // Filler fills order
        dai.approve(address(gatewayWithSelection), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        gatewayWithSelection.fillOrder(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();
    }

    function testFillOrderWithWrongSolver() public {
        // Enable solver selection
        Params memory newParams = Params({
            host: address(host), dispatcher: address(dispatcher), solverSelection: true, surplusShareBps: 10000
        });

        IntentGatewayV2 gatewayWithSelection = new IntentGatewayV2(address(this));
        gatewayWithSelection.setParams(newParams);

        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: vm.addr(1),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(gatewayWithSelection), inputAmount);
        gatewayWithSelection.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create EIP-712 signature from session key for filler
        bytes memory sessionSignature = _createSelectSolverSignature(
            commitment,
            filler,
            1, // Session key private key
            address(gatewayWithSelection)
        );

        // Solver selects filler
        vm.prank(filler);
        gatewayWithSelection.select(
            SelectOptions({commitment: commitment, solver: filler, signature: sessionSignature})
        );

        // Different address tries to fill - should revert
        address wrongSolver = address(0x9999);
        deal(address(dai), wrongSolver, 10000 * 1e18);

        vm.startPrank(wrongSolver);
        dai.approve(address(gatewayWithSelection), 1000 * 1e18);
        dai.approve(address(gatewayWithSelection), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentGatewayV2.Unauthorized.selector);
        gatewayWithSelection.fillOrder(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();
    }

    // ============================================
    // fillOrder Edge Case Tests
    // ============================================

    function testFillOrderExpired() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 10,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Roll past deadline
        vm.roll(block.number + 11);

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 1000 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentGatewayV2.Expired.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testFillOrderAlreadyFilled() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 2000 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        // Fill once
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));

        // Try to fill again - should revert
        vm.expectRevert(IntentGatewayV2.Filled.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testFillOrderWrongChain() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: bytes("DIFFERENT_CHAIN"),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 1000 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        vm.expectRevert(IntentGatewayV2.WrongChain.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testFillOrderInsufficientSolverAmount() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), 500 * 1e18);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18}); // Less than requested

        vm.expectRevert(IntentGatewayV2.InvalidInput.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testFillOrderInsufficientNativeToken() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: 1 ether});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: 1 ether});

        vm.expectRevert(IntentGatewayV2.InsufficientNativeToken.selector);
        intentGateway.fillOrder{value: 0.5 ether}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();
    }

    // ============================================
    // Order Cancellation Tests
    // ============================================

    function testCancelOrder() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Roll past deadline
        vm.roll(block.number + 101);

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: order.deadline + 1});

        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    function testCancelOrderUnauthorized() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        vm.roll(block.number + 101);

        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: block.number + 100});

        // Different user tries to cancel
        vm.startPrank(filler);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectRevert(IntentGatewayV2.Unauthorized.selector);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    function testCancelOrderNotExpired() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Try to cancel before deadline
        CancelOptions memory cancelOptions = CancelOptions({relayerFee: 0, height: block.number + 50});

        vm.startPrank(user);
        dai.approve(address(intentGateway), type(uint256).max);
        vm.expectRevert(IntentGatewayV2.NotExpired.selector);
        intentGateway.cancelOrder(order, cancelOptions);
        vm.stopPrank();
    }

    // ============================================
    // placeOrder Edge Case Tests
    // ============================================

    function testPlaceOrderWithFees() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 feeAmount = 100 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: feeAmount,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        dai.approve(address(intentGateway), feeAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
    }

    function testPlaceOrderInvalidInput() public {
        TokenInfo[] memory inputs = new TokenInfo[](0); // Empty inputs

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        vm.expectRevert(IntentGatewayV2.InvalidInput.selector);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
    }

    // ============================================
    // onAccept Variant Tests
    // ============================================

    function testOnAcceptRedeemEscrow() public {
        // This is tested indirectly by fillOrder tests
        // The fillOrder dispatches RedeemEscrow message which is handled by onAccept
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Simulate RedeemEscrow request from IntentGateway on another chain
        bytes memory body = bytes.concat(
            bytes1(uint8(IntentGatewayV2.RequestKind.RedeemEscrow)),
            abi.encode(
                RequestBody({commitment: commitment, tokens: inputs, beneficiary: bytes32(uint256(uint160(filler)))})
            )
        );

        PostRequest memory request = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        uint256 fillerBalanceBefore = usdc.balanceOf(filler);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        assertEq(usdc.balanceOf(filler) - fillerBalanceBefore, inputAmount, "Filler should receive escrowed tokens");
    }

    function testOnAcceptRefundEscrow() public {
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 1000,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Simulate RefundEscrow request
        bytes memory body = bytes.concat(
            bytes1(uint8(IntentGatewayV2.RequestKind.RefundEscrow)),
            abi.encode(
                RequestBody({commitment: commitment, tokens: inputs, beneficiary: bytes32(uint256(uint160(user)))})
            )
        );

        PostRequest memory request = PostRequest({
            source: host.host(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        uint256 userBalanceBefore = usdc.balanceOf(user);

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        assertEq(usdc.balanceOf(user) - userBalanceBefore, inputAmount, "User should receive refunded tokens");
    }

    function testOnAcceptNewDeployment() public {
        bytes memory stateMachineId = bytes("NEW_CHAIN");
        address gateway = address(0x1234);

        NewDeployment memory deployment = NewDeployment({stateMachineId: stateMachineId, gateway: gateway});

        bytes memory body =
            bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.NewDeployment)), abi.encode(deployment));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Check NewDeploymentAdded event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (entries[i].topics[0] == keccak256("NewDeploymentAdded(bytes,address)")) {
                eventFound = true;
                break;
            }
        }

        assertTrue(eventFound, "NewDeploymentAdded event should be emitted");

        // Verify instance was stored
        assertEq(intentGateway.instance(stateMachineId), gateway, "Gateway instance should be stored");
    }

    function testOnAcceptUpdateParams() public {
        Params memory newParams =
            Params({host: address(0x5678), dispatcher: address(0x9ABC), solverSelection: true, surplusShareBps: 10000});

        bytes memory body = bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.UpdateParams)), abi.encode(newParams));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.recordLogs();

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Check ParamsUpdated event
        Vm.Log[] memory entries = vm.getRecordedLogs();
        bool eventFound = false;

        for (uint256 i = 0; i < entries.length; i++) {
            if (
                entries[i].topics[0]
                    == keccak256("ParamsUpdated((address,address,bool,uint256),(address,address,bool,uint256))")
            ) {
                eventFound = true;
                break;
            }
        }

        assertTrue(eventFound, "ParamsUpdated event should be emitted");

        // Verify params were updated
        Params memory updatedParams = intentGateway.params();
        assertEq(updatedParams.host, newParams.host, "Host should be updated");
        assertEq(updatedParams.dispatcher, newParams.dispatcher, "Dispatcher should be updated");
        assertEq(updatedParams.solverSelection, newParams.solverSelection, "SolverSelection should be updated");
    }

    // ============================================
    // Helper Function Tests
    // ============================================

    function testInstance() public {
        bytes memory stateMachineId = bytes("TEST_CHAIN");

        // Before adding deployment, should return this contract's address
        address instance = intentGateway.instance(stateMachineId);
        assertEq(instance, address(intentGateway), "Should return self address by default");

        // Add a new deployment
        address gateway = address(0xABCD);
        NewDeployment memory deployment = NewDeployment({stateMachineId: stateMachineId, gateway: gateway});

        bytes memory body =
            bytes.concat(bytes1(uint8(IntentGatewayV2.RequestKind.NewDeployment)), abi.encode(deployment));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(intentGateway)),
            to: abi.encodePacked(address(intentGateway)),
            body: body,
            timeoutTimestamp: 0
        });

        vm.prank(address(host));
        intentGateway.onAccept(IncomingPostRequest({relayer: address(0), request: request}));

        // Now should return the stored gateway
        instance = intentGateway.instance(stateMachineId);
        assertEq(instance, gateway, "Should return stored gateway address");
    }

    function testCalculateCommitmentSlotHash() public view {
        bytes32 commitment = keccak256("test_commitment");
        bytes memory slotHash = intentGateway.calculateCommitmentSlotHash(commitment);

        assertGt(slotHash.length, 0, "Should return non-empty slot hash");
    }

    function testParams() public view {
        Params memory currentParams = intentGateway.params();

        assertEq(currentParams.host, address(host), "Host should match");
        assertEq(currentParams.dispatcher, address(dispatcher), "Dispatcher should match");
        assertEq(currentParams.solverSelection, false, "SolverSelection should be false");
    }

    function testHost() public view {
        address hostAddr = intentGateway.host();
        assertEq(hostAddr, address(host), "Host address should match");
    }

    function testReceive() public {
        uint256 amount = 1 ether;
        uint256 balanceBefore = address(intentGateway).balance;

        vm.deal(user, 10 ether);
        vm.prank(user);
        (bool sent,) = address(intentGateway).call{value: amount}("");

        assertTrue(sent, "ETH transfer should succeed");
        assertEq(address(intentGateway).balance, balanceBefore + amount, "Contract should receive ETH");
    }

    function testOnGetResponse() public {
        // Test successful cancellation via GET response
        uint256 inputAmount = 1000 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        bytes32 commitment = keccak256(abi.encode(order));

        // Create GET response with empty value (order not filled)
        bytes memory context = abi.encode(
            RequestBody({commitment: commitment, tokens: inputs, beneficiary: bytes32(uint256(uint160(user)))})
        );

        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue({key: new bytes(0), value: new bytes(0)}); // Empty value = not filled

        GetRequest memory getRequest = GetRequest({
            source: host.host(),
            dest: order.destination,
            nonce: 0,
            from: address(intentGateway),
            keys: new bytes[](0),
            height: 0,
            timeoutTimestamp: 0,
            context: context
        });

        GetResponse memory getResponse = GetResponse({request: getRequest, values: values});

        IncomingGetResponse memory incoming = IncomingGetResponse({response: getResponse, relayer: address(0)});

        uint256 userBalanceBefore = usdc.balanceOf(user);

        vm.prank(address(host));
        intentGateway.onGetResponse(incoming);

        assertEq(usdc.balanceOf(user) - userBalanceBefore, inputAmount, "User should receive refunded tokens");
    }
}
