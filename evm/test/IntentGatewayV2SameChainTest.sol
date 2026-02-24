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

        // Verify protocol retained its fee
        assertEq(usdc.balanceOf(address(gatewayWithFees)), expectedFee, "Gateway should retain protocol fee");
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

        // Verify gateway holds the USDC
        assertEq(usdc.balanceOf(address(intentGateway)), inputAmount, "Gateway should hold full amount");
        assertEq(usdc.balanceOf(user), userUsdcBefore - inputAmount, "User balance should decrease by input amount");

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
        uint256 userDaiBefore = dai.balanceOf(user);
        uint256 solverEthBefore = solver.balance;
        dai.approve(address(intentGateway), daiAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // Verify swap completed
        assertEq(solver.balance, solverEthBefore + ethAmount, "Solver should receive ETH");
        assertEq(address(intentGateway).balance, 0, "Gateway should have no ETH left");
        assertEq(dai.balanceOf(user), userDaiBefore + daiAmount, "User should receive DAI");
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

    function testSameChainSwap_MultiplePairs() public {
        // 1:1 pairing: USDC->ETH and DAI->USDC
        uint256 usdcInputAmount = 1000 * 1e6;
        uint256 daiInputAmount = 500 * 1e18;
        uint256 ethOutputAmount = 1 ether;
        uint256 usdcOutputAmount = 400 * 1e6;

        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcInputAmount});
        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiInputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: ethOutputAmount});
        outputAssets[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcOutputAmount});

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
        usdc.approve(address(intentGateway), usdcInputAmount);
        dai.approve(address(intentGateway), daiInputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        assertEq(usdc.balanceOf(user), userUsdcBefore - usdcInputAmount, "USDC should be escrowed");
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

        usdc.approve(address(intentGateway), usdcOutputAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](2);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: ethOutputAmount});
        solverOutputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcOutputAmount});

        intentGateway.fillOrder{value: ethOutputAmount}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();

        // Verify all tokens swapped
        assertEq(user.balance, userEthBefore + ethOutputAmount, "User should receive ETH");
        assertEq(
            usdc.balanceOf(user), userUsdcBefore - usdcInputAmount + usdcOutputAmount, "User should receive USDC output"
        );
        assertEq(
            usdc.balanceOf(solver),
            solverUsdcBefore + usdcInputAmount - usdcOutputAmount,
            "Solver should receive USDC input"
        );
        assertEq(dai.balanceOf(solver), solverDaiBefore + daiInputAmount, "Solver should receive DAI");
    }

    function testSameChainSwap_MismatchedLengths_ShouldRevert() public {
        // 2 inputs, 1 output should revert with InvalidInput
        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});
        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: 1 ether});

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

        vm.startPrank(user);
        usdc.approve(address(intentGateway), 1000 * 1e6);
        dai.approve(address(intentGateway), 500 * 1e18);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        vm.startPrank(solver);
        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: 1 ether});

        vm.expectRevert(IntentGatewayV2.InvalidInput.selector);
        intentGateway.fillOrder{value: 1 ether}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();
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

    function testSameChainFill_PartialAmount_IsValidPartialFill() public {
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 requestedAmount = 900 * 1e18; // 900 DAI
        uint256 partialAmount = 800 * 1e18; // 800 DAI (partial fill)

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

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver partially fills with 800 DAI
        vm.startPrank(solver);
        uint256 userDaiBefore = dai.balanceOf(user);
        uint256 solverUsdcBefore = usdc.balanceOf(solver);
        dai.approve(address(intentGateway), partialAmount);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: partialAmount});

        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs}));
        vm.stopPrank();

        // User receives partial output
        assertEq(dai.balanceOf(user), userDaiBefore + partialAmount, "User should receive partial DAI");

        // Solver receives proportional input: 1000 * 800 / 900 = 888.888... USDC
        uint256 expectedInputRelease = (inputAmount * partialAmount) / requestedAmount;
        assertEq(
            usdc.balanceOf(solver), solverUsdcBefore + expectedInputRelease, "Solver should receive proportional USDC"
        );

        // Gateway still holds remaining escrowed USDC
        assertEq(
            usdc.balanceOf(address(intentGateway)),
            inputAmount - expectedInputRelease,
            "Gateway should hold remaining USDC"
        );
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

    /*//////////////////////////////////////////////////////////////
                        PARTIAL FILL TESTS
    //////////////////////////////////////////////////////////////*/

    function _placeStandardOrder(uint256 inputAmount, uint256 outputAmount) internal returns (Order memory order) {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        PaymentInfo memory output =
            PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""});

        order = Order({
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

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;
    }

    function testPartialFill_TwoSolversCompleteOrder() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        Order memory order = _placeStandardOrder(inputAmount, outputAmount);

        address solver2 = makeAddr("solver2");
        vm.deal(solver2, 10 ether);
        deal(address(dai), solver2, 100000 * 1e18);

        // Solver 1 fills 600 DAI
        uint256 fill1 = 600 * 1e18;
        vm.startPrank(solver);
        dai.approve(address(intentGateway), fill1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fill1});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs1}));
        vm.stopPrank();

        uint256 expectedInput1 = (inputAmount * fill1) / outputAmount; // 600 USDC

        // Solver 2 fills remaining 400 DAI
        uint256 fill2 = 400 * 1e18;
        vm.startPrank(solver2);
        dai.approve(address(intentGateway), fill2);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fill2});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs2}));
        vm.stopPrank();

        // User received full output
        assertGe(dai.balanceOf(user), outputAmount, "User should receive full DAI");
        // Gateway should have no USDC left
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC");
        // Solver 1 received proportional input
        assertEq(usdc.balanceOf(solver), 100000 * 1e6 + expectedInput1, "Solver1 should receive proportional USDC");
    }

    function testPartialFill_ExcessCappedOnPartiallyFilledPair() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        Order memory order = _placeStandardOrder(inputAmount, outputAmount);

        // Solver 1 fills 600 DAI
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 600 * 1e18);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 600 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs1}));
        vm.stopPrank();

        // Solver 2 tries to fill 500 DAI but only 400 remains — excess capped, no surplus
        address solver2 = makeAddr("solver2");
        deal(address(dai), solver2, 100000 * 1e18);
        vm.startPrank(solver2);
        dai.approve(address(intentGateway), 500 * 1e18);
        uint256 solver2DaiBefore = dai.balanceOf(solver2);

        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs2}));
        vm.stopPrank();

        // Solver 2 should only spend 400 DAI (capped to remaining)
        assertEq(dai.balanceOf(solver2), solver2DaiBefore - 400 * 1e18, "Solver2 should only spend remaining 400 DAI");
        // Gateway should have no USDC left (order fully filled)
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC");
    }

    function testPartialFill_CancelAfterPartialFill() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        Order memory order = _placeStandardOrder(inputAmount, outputAmount);

        // Solver fills 500 DAI
        uint256 fill = 500 * 1e18;
        vm.startPrank(solver);
        dai.approve(address(intentGateway), fill);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fill});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        uint256 expectedInputReleased = (inputAmount * fill) / outputAmount; // 500 USDC
        uint256 remainingEscrowed = inputAmount - expectedInputReleased;

        // User cancels the partially filled order
        uint256 userUsdcBefore = usdc.balanceOf(user);
        vm.startPrank(user);
        CancelOptions memory cancelOpts = CancelOptions({height: block.number, relayerFee: 0});
        intentGateway.cancelOrder(order, cancelOpts);
        vm.stopPrank();

        // User should receive refund of remaining escrowed USDC
        assertEq(usdc.balanceOf(user), userUsdcBefore + remainingEscrowed, "User should receive remaining USDC");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC");
    }

    function testPartialFill_CannotFillAfterCancel() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        Order memory order = _placeStandardOrder(inputAmount, outputAmount);

        // Solver fills 500 DAI
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 500 * 1e18);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        // User cancels
        vm.prank(user);
        intentGateway.cancelOrder(order, CancelOptions({height: block.number, relayerFee: 0}));

        // Another solver tries to fill — should revert
        address solver2 = makeAddr("solver2");
        deal(address(dai), solver2, 100000 * 1e18);
        vm.startPrank(solver2);
        dai.approve(address(intentGateway), 500 * 1e18);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});
        vm.expectRevert(IntentGatewayV2.Filled.selector);
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs2}));
        vm.stopPrank();
    }

    function testPartialFill_WithNativeETH() public {
        uint256 inputAmount = 1000 * 1e6; // 1000 USDC
        uint256 outputAmount = 1 ether;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: outputAmount});

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

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver partially fills with 0.6 ETH
        uint256 partialETH = 0.6 ether;
        uint256 userEthBefore = user.balance;
        uint256 solverUsdcBefore = usdc.balanceOf(solver);

        vm.startPrank(solver);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(0), amount: partialETH});
        intentGateway.fillOrder{value: partialETH}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs})
        );
        vm.stopPrank();

        assertEq(user.balance, userEthBefore + partialETH, "User should receive partial ETH");
        uint256 expectedInput = (inputAmount * partialETH) / outputAmount; // 600 USDC
        assertEq(usdc.balanceOf(solver), solverUsdcBefore + expectedInput, "Solver should receive proportional USDC");

        // Complete with remaining 0.4 ETH
        address solver2 = makeAddr("solver2");
        vm.deal(solver2, 10 ether);
        uint256 remainingETH = 0.4 ether;

        vm.startPrank(solver2);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        outputs2[0] = TokenInfo({token: bytes32(0), amount: remainingETH});
        intentGateway.fillOrder{value: remainingETH}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs2})
        );
        vm.stopPrank();

        assertEq(user.balance, userEthBefore + outputAmount, "User should receive full ETH");
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have no USDC");
    }

    function testPartialFill_SurplusOnlyWhenNotPartiallyFilled() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        Order memory order = _placeStandardOrder(inputAmount, outputAmount);

        // Solver fills 500 DAI (partial)
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 500 * 1e18);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs1}));
        vm.stopPrank();

        // Solver 2 fills remaining 500 with 600 offered — excess should be capped (no surplus on partially filled pair)
        address solver2 = makeAddr("solver2");
        deal(address(dai), solver2, 100000 * 1e18);
        uint256 solver2DaiBefore = dai.balanceOf(solver2);
        uint256 userDaiBefore = dai.balanceOf(user);

        vm.startPrank(solver2);
        dai.approve(address(intentGateway), 600 * 1e18);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 600 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs2}));
        vm.stopPrank();

        // Solver 2 should only spend 500 DAI (capped)
        assertEq(dai.balanceOf(solver2), solver2DaiBefore - 500 * 1e18, "Solver2 capped to remaining");
        // User should receive exactly 500 more DAI (no surplus)
        assertEq(dai.balanceOf(user), userDaiBefore + 500 * 1e18, "User gets exact remaining, no surplus");
        // No dust in gateway
        assertEq(dai.balanceOf(address(intentGateway)), 0, "No protocol surplus on partially filled pair");
    }

    function testPartialFill_SurplusOnFullFill() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;
        uint256 solverAmount = 1100 * 1e18; // 100 DAI surplus
        Order memory order = _placeStandardOrder(inputAmount, outputAmount);

        // Single solver fills the entire order with surplus (pair was never partially filled)
        vm.startPrank(solver);
        uint256 userDaiBefore = dai.balanceOf(user);
        dai.approve(address(intentGateway), solverAmount);

        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: solverAmount});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        // surplus = 100 DAI, 50% to beneficiary, 50% to protocol
        uint256 surplus = 100 * 1e18;
        uint256 beneficiaryShare = surplus - (surplus * SURPLUS_SHARE_BPS) / 10_000;
        assertEq(
            dai.balanceOf(user),
            userDaiBefore + outputAmount + beneficiaryShare,
            "User should receive output + beneficiary surplus share"
        );
        uint256 protocolShare = (surplus * SURPLUS_SHARE_BPS) / 10_000;
        assertEq(dai.balanceOf(address(intentGateway)), protocolShare, "Gateway should hold protocol surplus share");
    }

    function testPartialFill_WithProtocolFee() public {
        IntentGatewayV2 gatewayWithFees = new IntentGatewayV2(address(this));
        gatewayWithFees.setParams(
            Params({
                host: address(host),
                dispatcher: address(dispatcher),
                solverSelection: false,
                surplusShareBps: SURPLUS_SHARE_BPS,
                protocolFeeBps: PROTOCOL_FEE_BPS,
                priceOracle: address(0)
            })
        );

        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 900 * 1e18;
        uint256 expectedFee = (inputAmount * PROTOCOL_FEE_BPS) / 10_000;
        uint256 amountAfterFee = inputAmount - expectedFee;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

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
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: ""})
        });

        vm.startPrank(user);
        usdc.approve(address(gatewayWithFees), inputAmount);
        gatewayWithFees.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;
        order.inputs[0].amount = amountAfterFee; // Commitment uses reduced amount

        // Partial fill: 450 DAI (half)
        uint256 fill = 450 * 1e18;
        vm.startPrank(solver);
        uint256 solverUsdcBefore = usdc.balanceOf(solver);
        dai.approve(address(gatewayWithFees), fill);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fill});
        gatewayWithFees.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs}));
        vm.stopPrank();

        // Solver gets proportional input after fee: amountAfterFee * 450/900 = amountAfterFee / 2
        uint256 expectedInput = (amountAfterFee * fill) / outputAmount;
        assertEq(
            usdc.balanceOf(solver), solverUsdcBefore + expectedInput, "Solver receives proportional input after fee"
        );
        // Gateway retains protocol fee + remaining escrow
        assertEq(
            usdc.balanceOf(address(gatewayWithFees)),
            inputAmount - expectedInput,
            "Gateway holds fee + remaining escrow"
        );
    }

    function testPartialFill_CalldataOnlyAfterFullFill() public {
        uint256 inputAmount = 1000 * 1e6;
        uint256 outputAmount = 1000 * 1e18;

        // Create an order with calldata (approve call that won't revert)
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: outputAmount});

        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(dai), value: 0, data: abi.encodeWithSelector(IERC20.approve.selector, address(intentGateway), 1)
        });

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
            output: PaymentInfo({
                beneficiary: bytes32(uint256(uint160(user))), assets: outputAssets, call: abi.encode(calls)
            })
        });

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Partial fill — calldata should NOT execute
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 500 * 1e18);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs1}));
        vm.stopPrank();

        // Allowance should still be 0 (calldata not executed yet)
        assertEq(
            dai.allowance(address(intentGateway.params().dispatcher), address(intentGateway)),
            0,
            "Calldata should not execute on partial fill"
        );

        // Complete the fill — calldata should execute
        address solver2 = makeAddr("solver2");
        deal(address(dai), solver2, 100000 * 1e18);
        vm.startPrank(solver2);
        dai.approve(address(intentGateway), 500 * 1e18);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 500 * 1e18});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputs2}));
        vm.stopPrank();

        // Allowance should now be 1 (calldata executed)
        assertEq(
            dai.allowance(address(intentGateway.params().dispatcher), address(intentGateway)),
            1,
            "Calldata should execute after full fill"
        );
    }

    /*//////////////////////////////////////////////////////////////
                    ROUNDING DUST IN PARTIAL FILLS (Finding #4)
    //////////////////////////////////////////////////////////////*/

    /// @notice Verifies that rounding dust from integer division in partial fills
    /// is not permanently locked. The final solver completing the order should
    /// receive the full remaining escrow balance rather than a truncated amount.
    function testPartialFill_RoundingDustReleasedToFinalSolver() public {
        // Choose amounts that produce rounding truncation:
        // input = 100 USDC (100e6), output = 3 DAI (3e18)
        // Each of 3 solvers fills 1 DAI. Proportional release per fill:
        //   100e6 * 1e18 / 3e18 = 33333333 (truncated from 33333333.33...)
        // Without fix: 3 * 33333333 = 99999999, leaving 1 unit locked.
        // With fix: final solver gets remaining balance = 100e6 - 2*33333333 = 33333334
        uint256 inputAmount = 100 * 1e6; // 100 USDC
        uint256 outputAmount = 3 * 1e18; // 3 DAI

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

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        uint256 fillPerSolver = 1e18; // Each solver fills 1 DAI
        uint256 truncatedRelease = (inputAmount * fillPerSolver) / outputAmount; // 33333333

        // --- Solver 1 fills 1 DAI ---
        address solver1 = makeAddr("solver1");
        vm.deal(solver1, 1 ether);
        deal(address(dai), solver1, 10 * 1e18);
        uint256 solver1UsdcBefore = usdc.balanceOf(solver1);

        vm.startPrank(solver1);
        dai.approve(address(intentGateway), fillPerSolver);
        TokenInfo[] memory out1 = new TokenInfo[](1);
        out1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fillPerSolver});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: out1}));
        vm.stopPrank();

        assertEq(
            usdc.balanceOf(solver1),
            solver1UsdcBefore + truncatedRelease,
            "Solver1 should receive truncated proportional USDC"
        );

        // --- Solver 2 fills 1 DAI ---
        address solver2 = makeAddr("solver2");
        vm.deal(solver2, 1 ether);
        deal(address(dai), solver2, 10 * 1e18);
        uint256 solver2UsdcBefore = usdc.balanceOf(solver2);

        vm.startPrank(solver2);
        dai.approve(address(intentGateway), fillPerSolver);
        TokenInfo[] memory out2 = new TokenInfo[](1);
        out2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fillPerSolver});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: out2}));
        vm.stopPrank();

        assertEq(
            usdc.balanceOf(solver2),
            solver2UsdcBefore + truncatedRelease,
            "Solver2 should receive truncated proportional USDC"
        );

        // --- Solver 3 fills final 1 DAI (completes the order) ---
        address solver3 = makeAddr("solver3");
        vm.deal(solver3, 1 ether);
        deal(address(dai), solver3, 10 * 1e18);
        uint256 solver3UsdcBefore = usdc.balanceOf(solver3);

        vm.startPrank(solver3);
        dai.approve(address(intentGateway), fillPerSolver);
        TokenInfo[] memory out3 = new TokenInfo[](1);
        out3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fillPerSolver});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: out3}));
        vm.stopPrank();

        // Final solver should receive the remaining balance (truncatedRelease + 1 rounding unit)
        uint256 expectedFinalRelease = inputAmount - (2 * truncatedRelease); // 33333334
        assertEq(
            usdc.balanceOf(solver3),
            solver3UsdcBefore + expectedFinalRelease,
            "Final solver should receive remaining escrow including rounding dust"
        );
        assertGt(expectedFinalRelease, truncatedRelease, "Final release should be larger due to rounding dust");

        // Gateway should have zero USDC — no dust locked
        assertEq(usdc.balanceOf(address(intentGateway)), 0, "Gateway should have zero USDC - no rounding dust locked");
    }

    /// @notice Same test with native ETH to verify rounding dust fix works for native tokens too
    function testPartialFill_RoundingDustReleasedToFinalSolver_NativeETH() public {
        // input = 1 ether, output = 3 DAI
        // Per fill: 1e18 * 1e18 / 3e18 = 333333333333333333 (truncated)
        // Without fix: 3 * 333333333333333333 = 999999999999999999, leaving 1 wei locked
        uint256 inputAmount = 1 ether;
        uint256 outputAmount = 3 * 1e18;

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(0), amount: inputAmount}); // native ETH

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

        vm.startPrank(user);
        intentGateway.placeOrder{value: inputAmount}(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        uint256 fillPerSolver = 1e18;
        uint256 truncatedRelease = (inputAmount * fillPerSolver) / outputAmount;

        // --- Solver 1 ---
        address solver1 = makeAddr("ethSolver1");
        vm.deal(solver1, 10 ether);
        deal(address(dai), solver1, 10 * 1e18);
        uint256 solver1EthBefore = solver1.balance;

        vm.startPrank(solver1);
        dai.approve(address(intentGateway), fillPerSolver);
        TokenInfo[] memory out1 = new TokenInfo[](1);
        out1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fillPerSolver});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: out1}));
        vm.stopPrank();

        assertEq(solver1.balance, solver1EthBefore + truncatedRelease, "Solver1 gets truncated ETH");

        // --- Solver 2 ---
        address solver2 = makeAddr("ethSolver2");
        vm.deal(solver2, 10 ether);
        deal(address(dai), solver2, 10 * 1e18);
        uint256 solver2EthBefore = solver2.balance;

        vm.startPrank(solver2);
        dai.approve(address(intentGateway), fillPerSolver);
        TokenInfo[] memory out2 = new TokenInfo[](1);
        out2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fillPerSolver});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: out2}));
        vm.stopPrank();

        assertEq(solver2.balance, solver2EthBefore + truncatedRelease, "Solver2 gets truncated ETH");

        // --- Solver 3 (final) ---
        address solver3 = makeAddr("ethSolver3");
        vm.deal(solver3, 10 ether);
        deal(address(dai), solver3, 10 * 1e18);
        uint256 solver3EthBefore = solver3.balance;

        vm.startPrank(solver3);
        dai.approve(address(intentGateway), fillPerSolver);
        TokenInfo[] memory out3 = new TokenInfo[](1);
        out3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: fillPerSolver});
        intentGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: out3}));
        vm.stopPrank();

        uint256 expectedFinalRelease = inputAmount - (2 * truncatedRelease);
        assertEq(solver3.balance, solver3EthBefore + expectedFinalRelease, "Final solver gets remaining ETH + dust");
        assertGt(expectedFinalRelease, truncatedRelease, "Final release includes rounding dust");

        // Gateway should have zero ETH locked
        assertEq(address(intentGateway).balance, 0, "Gateway should have zero ETH - no rounding dust locked");
    }

    /*//////////////////////////////////////////////////////////////
              NATIVE ETH SURPLUS ACCOUNTING (Finding #2)
    //////////////////////////////////////////////////////////////*/

    /// @notice Verifies that native ETH surplus (protocol share) is correctly accounted
    /// for in the msgValue tracker during same-chain fills, preventing the protocol's
    /// dust from being consumed by subsequent operations.
    function testSameChainSwap_NativeETHSurplus_CorrectAccounting() public {
        // Order: user escrows 1000 USDC, wants 1 ETH output
        // Solver provides 1.1 ETH (0.1 ETH surplus)
        // With 50% surplusShareBps: 0.05 ETH to beneficiary, 0.05 ETH to protocol
        uint256 inputAmount = 1000 * 1e6;
        uint256 requestedETH = 1 ether;
        uint256 solverETH = 1.1 ether; // 0.1 ETH surplus

        uint256 surplus = solverETH - requestedETH;
        uint256 protocolShare = (surplus * SURPLUS_SHARE_BPS) / 10_000; // 0.05 ETH
        uint256 beneficiaryShare = surplus - protocolShare; // 0.05 ETH

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: inputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: requestedETH}); // native ETH output

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

        vm.startPrank(user);
        usdc.approve(address(intentGateway), inputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills with surplus native ETH
        vm.startPrank(solver);
        uint256 userEthBefore = user.balance;
        uint256 gatewayEthBefore = address(intentGateway).balance;
        uint256 solverUsdcBefore = usdc.balanceOf(solver);

        TokenInfo[] memory solverOutputs = new TokenInfo[](1);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: solverETH});

        intentGateway.fillOrder{value: solverETH}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();

        // User receives requested + beneficiary share
        assertEq(
            user.balance,
            userEthBefore + requestedETH + beneficiaryShare,
            "User should receive requested ETH + beneficiary surplus share"
        );

        // Protocol dust retained in gateway
        assertEq(
            address(intentGateway).balance,
            gatewayEthBefore + protocolShare,
            "Gateway should retain protocol's ETH surplus share as dust"
        );

        // Solver receives escrowed USDC
        assertEq(usdc.balanceOf(solver), solverUsdcBefore + inputAmount, "Solver should receive escrowed USDC");
    }

    /// @notice Tests that with mixed output tokens (native ETH + ERC20) and surplus,
    /// the msgValue tracker correctly accounts for protocolShare on the native ETH pair,
    /// preventing protocol dust from being consumed by subsequent operations.
    function testSameChainSwap_MixedOutputs_NativeETHSurplusAccounting() public {
        // 2 pairs: USDC->ETH (with surplus) and DAI->USDC (with surplus)
        uint256 usdcInputAmount = 1000 * 1e6;
        uint256 daiInputAmount = 500 * 1e18;
        uint256 ethOutputRequired = 0.5 ether;
        uint256 usdcOutputRequired = 400 * 1e6;
        uint256 solverEth = 0.6 ether; // 0.1 ETH surplus
        uint256 solverUsdc = 420 * 1e6; // 20 USDC surplus

        uint256 ethSurplus = solverEth - ethOutputRequired;
        uint256 ethProtocolShare = (ethSurplus * SURPLUS_SHARE_BPS) / 10_000;
        uint256 ethBeneficiaryShare = ethSurplus - ethProtocolShare;

        uint256 usdcSurplus = solverUsdc - usdcOutputRequired;
        uint256 usdcProtocolShare = (usdcSurplus * SURPLUS_SHARE_BPS) / 10_000;
        uint256 usdcBeneficiaryShare = usdcSurplus - usdcProtocolShare;

        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcInputAmount});
        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: daiInputAmount});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: ethOutputRequired}); // native ETH
        outputAssets[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: usdcOutputRequired});

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

        vm.startPrank(user);
        usdc.approve(address(intentGateway), usdcInputAmount);
        dai.approve(address(intentGateway), daiInputAmount);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(user)));
        order.source = host.host();
        order.nonce = 0;

        // Solver fills both pairs with surplus
        vm.startPrank(solver);
        uint256 userEthBefore = user.balance;
        uint256 userUsdcBefore = usdc.balanceOf(user);
        uint256 solverEthBefore = solver.balance;

        usdc.approve(address(intentGateway), solverUsdc);

        TokenInfo[] memory solverOutputs = new TokenInfo[](2);
        solverOutputs[0] = TokenInfo({token: bytes32(0), amount: solverEth});
        solverOutputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: solverUsdc});

        // Solver sends exactly solverEth as msg.value — no extra ETH for USDC transfer
        intentGateway.fillOrder{value: solverEth}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: solverOutputs})
        );
        vm.stopPrank();

        // User receives ETH output + beneficiary share
        assertEq(
            user.balance,
            userEthBefore + ethOutputRequired + ethBeneficiaryShare,
            "User should receive ETH output + beneficiary surplus"
        );

        // User receives USDC output + beneficiary share
        assertEq(
            usdc.balanceOf(user),
            userUsdcBefore + usdcOutputRequired + usdcBeneficiaryShare,
            "User should receive USDC output + beneficiary surplus"
        );

        // Gateway retains ETH protocol share + USDC protocol share
        // (gateway also had the escrowed USDC input which was released to solver)
        assertEq(address(intentGateway).balance, ethProtocolShare, "Gateway should retain ETH protocol share as dust");

        // Solver spent exactly solverEth in ETH
        assertEq(solverEthBefore - solver.balance, solverEth, "Solver should have spent exactly solverEth");
    }
}
