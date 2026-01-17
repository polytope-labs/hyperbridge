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
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    CancelOptions
} from "../src/apps/IntentGatewayV2.sol";
import {ICallDispatcher, Call} from "../src/interfaces/ICallDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/**
 * @title IntentGatewayV2SameChainTest
 * @notice Comprehensive tests for same-chain swap functionality in IntentGatewayV2
 */
contract IntentGatewayV2SameChainTest is MainnetForkBaseTest {
    IntentGatewayV2 public intentGateway;

    // Test users
    address public user;
    address public solver;
    address public otherUser;

    // Protocol fee in BPS (30 BPS = 0.3%)
    uint256 public constant PROTOCOL_FEE_BPS = 30;
    uint256 public constant SURPLUS_SHARE_BPS = 5000; // 50% to protocol, 50% to beneficiary

    // Events to test
    event OrderPlaced(
        bytes32 indexed commitment,
        address indexed user,
        bytes source,
        bytes destination,
        uint256 deadline,
        uint256 nonce,
        TokenInfo[] inputs,
        PaymentInfo output
    );
    event OrderFilled(bytes32 indexed commitment, address indexed filler);
    event EscrowReleased(bytes32 indexed commitment);
    event EscrowRefunded(bytes32 indexed commitment);
    event DustCollected(address indexed token, uint256 amount);

    function setUp() public override {
        super.setUp();

        // Setup test accounts
        user = makeAddr("user");
        solver = makeAddr("solver");
        otherUser = makeAddr("otherUser");

        // Deploy IntentGatewayV2
        intentGateway = new IntentGatewayV2(address(this));

        // Set params with surplus sharing but no protocol fees (to simplify tests)
        Params memory intentParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: SURPLUS_SHARE_BPS,
            protocolFeeBps: 0, // No protocol fees for most tests
            priceOracle: address(0)
        });
        intentGateway.setParams(intentParams);

        // Fund test accounts
        _fundTestAccounts();
    }

    function _fundTestAccounts() internal {
        // Fund user with ETH and tokens
        vm.deal(user, 10 ether);
        deal(address(usdc), user, 100000 * 1e6); // 100,000 USDC
        deal(address(dai), user, 100000 * 1e18); // 100,000 DAI

        // Fund solver with ETH and tokens
        vm.deal(solver, 100 ether);
        deal(address(usdc), solver, 100000 * 1e6);
        deal(address(dai), solver, 100000 * 1e18);

        // Fund otherUser
        vm.deal(otherUser, 10 ether);
        deal(address(usdc), otherUser, 10000 * 1e6);
        deal(address(dai), otherUser, 10000 * 1e18);
    }

    /*//////////////////////////////////////////////////////////////
                        BASIC SAME-CHAIN SWAP TESTS
    //////////////////////////////////////////////////////////////*/

    function testSameChainSwap_BasicFill() public {
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 900 * 1e18; // 900 DAI

        // User wants to swap USDC for DAI
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        // Note: order.user, order.source, and order.nonce will be set by placeOrder
        Order memory order = Order({
            user: bytes32(0), // Will be set to msg.sender
            source: "", // Will be set to current chain
            destination: host.host(), // Same chain
            deadline: block.number + 100,
            nonce: 0, // Will be assigned by contract
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        uint256 userUsdcBefore = usdc.balanceOf(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // After placeOrder, order fields are set: user, source, nonce
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0; // First order

        // Verify escrow
        assertEq(usdc.balanceOf(user), userUsdcBefore - inputAmount, "User USDC should be escrowed");
        assertEq(usdc.balanceOf(address(intentGateway)), inputAmount, "Gateway should hold escrowed USDC");

        // Solver fills order
        vm.startPrank(solver);
        uint256 userDaiBefore = dai.balanceOf(user);
        uint256 solverUsdcBefore = usdc.balanceOf(solver);

        dai.approve(address(intentGateway), outputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // Verify swap completed
        assertEq(dai.balanceOf(user), userDaiBefore + outputAmount, "User should receive DAI");
        assertEq(usdc.balanceOf(solver), solverUsdcBefore + inputAmount, "Solver should receive escrowed USDC");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC left");
    }

    function testSameChainSwap_WithProtocolFee() public {
        // Deploy a new gateway with protocol fees enabled
        IntentGatewayV2 gatewayWithFees = new IntentGatewayV2(address(this));
        Params memory intentParams = Params({
            host: address(host),
            dispatcher: address(dispatcher),
            solverSelection: false,
            surplusShareBps: SURPLUS_SHARE_BPS,
            protocolFeeBps: PROTOCOL_FEE_BPS,
            priceOracle: address(0)
        });
        gatewayWithFees.setParams(intentParams);

        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 900 * 1e18; // 900 DAI

        // Calculate protocol fee (30 BPS = 0.3%)
        uint256 expectedFee = (inputAmount * PROTOCOL_FEE_BPS) / 10_000;
        uint256 amountAfterFee = inputAmount - expectedFee;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0), // Will be set by placeOrder
            source: "", // Will be set by placeOrder
            destination: host.host(),
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
        uint256 userUsdcBefore = usdc.balanceOf(user);
        usdc.approve(address(gatewayWithFees), inputAmount);
        gatewayWithFees.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Verify protocol fee was taken
        // Gateway holds the full amount including protocol fee initially
        assertEq(usdc.balanceOf(address(gatewayWithFees)), inputAmount, "Gateway should hold full amount");
        assertEq(usdc.balanceOf(user), userUsdcBefore - inputAmount, "Full amount taken from user");

        // Update order fields as they would be after placeOrder
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;
        // Update inputs to reduced amounts (after protocol fee deduction)
        // The commitment is calculated with reduced amounts when protocol fees are enabled
        order.inputs[0].amount = amountAfterFee;

        // Solver fills order
        vm.startPrank(solver);
        uint256 solverUsdcBefore = usdc.balanceOf(solver);
        dai.approve(address(gatewayWithFees), outputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        gatewayWithFees.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // Verify solver received amount after fee
        assertEq(
            usdc.balanceOf(solver), solverUsdcBefore + amountAfterFee, "Solver should receive amount after protocol fee"
        );
    }

    function testSameChainSwap_WithSurplus() public {
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 requestedAmount = 900 * 1e18; // 900 DAI requested
        uint256 solverAmount = 920 * 1e18; // Solver provides 920 DAI (20 DAI surplus)

        uint256 surplus = solverAmount - requestedAmount;
        uint256 protocolShare = (surplus * SURPLUS_SHARE_BPS) / 10_000; // 50%
        uint256 beneficiaryShare = surplus - protocolShare; // 50%

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: requestedAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order with surplus
        vm.startPrank(solver);
        uint256 userDaiBefore = dai.balanceOf(user);
        uint256 gatewayDaiBefore = dai.balanceOf(address(intentGateway));

        dai.approve(address(intentGateway), solverAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // Verify surplus split
        assertEq(
            dai.balanceOf(user),
            userDaiBefore + requestedAmount + beneficiaryShare,
            "User should receive requested amount + beneficiary share"
        );
        assertEq(
            dai.balanceOf(address(intentGateway)),
            gatewayDaiBefore + protocolShare,
            "Gateway should receive protocol share"
        );
    }

    /*//////////////////////////////////////////////////////////////
                        SAME-CHAIN CANCEL TESTS
    //////////////////////////////////////////////////////////////*/

    function testSameChainCancel_BeforeDeadline() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        uint256 userUsdcBefore = usdc.balanceOf(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // User cancels order before deadline (same-chain allows this)
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});

        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();

        // Verify refund
        assertEq(usdc.balanceOf(user), userUsdcBefore, "User should receive full refund");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC");
    }

    function testSameChainCancel_AfterDeadline() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
            deadline: block.number + 10,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order
        vm.startPrank(user);
        uint256 userUsdcBefore = usdc.balanceOf(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Move past deadline
        vm.roll(block.number + 20);

        // User cancels order after deadline (same-chain allows this)
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});

        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();

        // Verify refund
        assertEq(usdc.balanceOf(user), userUsdcBefore, "User should receive full refund");
    }

    function testSameChainCancel_AfterFill_ShouldRevert() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order
        vm.startPrank(solver);
        dai.approve(address(intentGateway), outputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // User tries to cancel already-filled order
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});

        vm.expectRevert(IntentGatewayV2.Filled.selector);
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();
    }

    function testSameChainCancel_UnauthorizedUser_ShouldRevert() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Other user tries to cancel
        vm.startPrank(otherUser);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});

        vm.expectRevert(IntentGatewayV2.Unauthorized.selector);
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();
    }

    /*//////////////////////////////////////////////////////////////
                        NATIVE TOKEN TESTS
    //////////////////////////////////////////////////////////////*/

    function testSameChainSwap_WithNativeETH() public {
        uint256 ethAmount = 1 ether;
        uint256 daiAmount = 3000 * 1e18; // 3000 DAI for 1 ETH

        // User wants to swap ETH for DAI
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(0), amount: ethAmount}); // address(0) = native ETH

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: output
        });

        // User places order with ETH
        vm.startPrank(user);
        uint256 userEthBefore = user.balance;
        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));
        vm.stopPrank();

        // Verify ETH escrowed
        assertEq(user.balance, userEthBefore - ethAmount, "User ETH should be escrowed");
        assertEq(address(intentGateway).balance, ethAmount, "Gateway should hold escrowed ETH");

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order
        vm.startPrank(solver);
        uint256 solverEthBefore = solver.balance;
        dai.approve(address(intentGateway), daiAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // Verify swap completed
        assertEq(solver.balance, solverEthBefore + ethAmount, "Solver should receive ETH");
        assertEq(address(intentGateway).balance, 0, "Gateway should have no ETH left");
    }

    function testSameChainCancel_WithETH() public {
        uint256 ethAmount = 1 ether;
        uint256 daiAmount = 3000 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(0), amount: ethAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        uint256 userEthBefore = user.balance;
        intentGateway.placeOrder{value: ethAmount}(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Cancel
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();

        // Verify ETH refunded
        assertEq(user.balance, userEthBefore, "User should receive full ETH refund");
        assertEq(address(intentGateway).balance, 0, "Gateway should have no ETH");
    }

    /*//////////////////////////////////////////////////////////////
                        RACE CONDITION TESTS
    //////////////////////////////////////////////////////////////*/

    function testSameChainRaceCondition_FillBeatsCancel() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order first
        vm.startPrank(solver);
        dai.approve(address(intentGateway), outputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // User tries to cancel after fill
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});

        vm.expectRevert(IntentGatewayV2.Filled.selector);
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();
    }

    function testSameChainRaceCondition_CancelBeatsFill() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // User cancels first
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();

        // Solver tries to fill after cancel
        vm.startPrank(solver);
        dai.approve(address(intentGateway), outputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        vm.expectRevert(IntentGatewayV2.Filled.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    /*//////////////////////////////////////////////////////////////
                        MULTIPLE TOKEN TESTS
    //////////////////////////////////////////////////////////////*/

    function testSameChainSwap_MultipleInputs() public {
        uint256 usdcAmount = 1000 * 1e6; // 1000 USDC
        uint256 daiInputAmount = 500 * 1e18; // 500 DAI
        uint256 ethOutputAmount = 1 ether; // 1 ETH output

        // User wants to swap USDC + DAI for ETH
        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcAmount});
        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiInputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: ethOutputAmount}); // Native ETH

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        uint256 userUsdcBefore = usdc.balanceOf(user);
        uint256 userDaiBefore = dai.balanceOf(user);
        usdc.approve(address(intentGateway), usdcAmount);
        dai.approve(address(intentGateway), daiInputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Verify both tokens escrowed
        assertEq(usdc.balanceOf(user), userUsdcBefore - usdcAmount, "USDC should be escrowed");
        assertEq(dai.balanceOf(user), userDaiBefore - daiInputAmount, "DAI should be escrowed");

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order
        vm.startPrank(solver);
        uint256 userEthBefore = user.balance;
        uint256 solverUsdcBefore = usdc.balanceOf(solver);
        uint256 solverDaiBefore = dai.balanceOf(solver);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: ethOutputAmount});

        intentGateway.fillOrder{value: ethOutputAmount}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();

        // Verify all tokens swapped
        assertEq(user.balance, userEthBefore + ethOutputAmount, "User should receive ETH");
        assertEq(usdc.balanceOf(solver), solverUsdcBefore + usdcAmount, "Solver should receive USDC");
        assertEq(dai.balanceOf(solver), solverDaiBefore + daiInputAmount, "Solver should receive DAI");
    }

    function testSameChainSwap_MultipleOutputs() public {
        uint256 ethInputAmount = 2 ether;
        uint256 usdcOutputAmount = 3000 * 1e6; // 3000 USDC
        uint256 daiOutputAmount = 3000 * 1e18; // 3000 DAI

        // User wants to swap ETH for USDC + DAI
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(0), amount: ethInputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcOutputAmount});
        outputAssets[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiOutputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        uint256 userEthBefore = user.balance;
        intentGateway.placeOrder{value: ethInputAmount}(order, bytes32(0));
        vm.stopPrank();

        assertEq(user.balance, userEthBefore - ethInputAmount, "ETH should be escrowed");

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order
        vm.startPrank(solver);
        uint256 userUsdcBefore = usdc.balanceOf(user);
        uint256 userDaiBefore = dai.balanceOf(user);
        uint256 solverEthBefore = solver.balance;

        usdc.approve(address(intentGateway), usdcOutputAmount);
        dai.approve(address(intentGateway), daiOutputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](2);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcOutputAmount});
        solverOutputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiOutputAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // Verify all tokens swapped
        assertEq(usdc.balanceOf(user), userUsdcBefore + usdcOutputAmount, "User should receive USDC");
        assertEq(dai.balanceOf(user), userDaiBefore + daiOutputAmount, "User should receive DAI");
        assertEq(solver.balance, solverEthBefore + ethInputAmount, "Solver should receive ETH");
    }

    /*//////////////////////////////////////////////////////////////
                        ERROR CASE TESTS
    //////////////////////////////////////////////////////////////*/

    function testSameChainFill_ExpiredOrder_ShouldRevert() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
            deadline: block.number + 10,
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

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Move past deadline
        vm.roll(block.number + 20);

        // Solver tries to fill expired order
        vm.startPrank(solver);
        dai.approve(address(intentGateway), outputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        vm.expectRevert(IntentGatewayV2.Expired.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testSameChainFill_InsufficientSolverAmount_ShouldRevert() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 requestedAmount = 900 * 1e18;
        uint256 insufficientAmount = 800 * 1e18; // Less than requested

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: requestedAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver tries to fill with insufficient amount
        vm.startPrank(solver);
        dai.approve(address(intentGateway), insufficientAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: insufficientAmount});

        vm.expectRevert(IntentGatewayV2.InvalidInput.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testSameChainFill_AlreadyFilled_ShouldRevert() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(0),
            source: "",
            destination: host.host(),
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
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Update order fields
        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills order
        vm.startPrank(solver);
        dai.approve(address(intentGateway), outputAmount * 2); // Approve enough for two attempts

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));

        // Try to fill again
        vm.expectRevert(IntentGatewayV2.Filled.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();
    }

    function testSameChainCancel_UnknownOrder_ShouldRevert() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        Order memory order = Order({
            user: bytes32(uint256(uint160(user))), // Set to user so it passes Unauthorized check
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

        // User tries to cancel an order that was never placed
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});

        vm.expectRevert(IntentGatewayV2.UnknownOrder.selector);
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();
    }
}
