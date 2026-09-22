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
import {IntentQuoteTestUtils} from "./IntentQuoteTestUtils.sol";

import "forge-std/Test.sol";
import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {IntentGatewayV2} from "../../src/apps/IntentGatewayV2.sol";
import {
    Order,
    Params,
    InitParams,
    TokenInfo,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {deployIntentGatewayImpl, deployIntentModules} from "./IntentGatewayDeploy.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/**
 * @title ReentrantBeneficiary
 * @notice Malicious beneficiary contract that attempts to re-enter `fillOrder` during
 *         the ETH transfer made while `fillOrder` pays a leg.
 *
 * Attack window (pre-fix):
 *
 *   fillOrder, in either module's `_fillOrder`:
 *     beneficiary.call{value: ...}("")   ← RE-ENTRY HERE
 *     // _filled still == address(0) pre-fix, now set at the top (CEI)
 *
 * With the CEI fix in place, `_filled[commitment]` is set to `msg.sender` by
 * `fillOrder` before either module runs. Any reentrant `fillOrder` call therefore hits
 * the `if (_filled[commitment] != address(0)) revert Filled()` guard and reverts.
 * That revert propagates through `receive()`, causing the outer ETH transfer to
 * return `(false, ...)`, which triggers `InsufficientNativeToken()` in the outer
 * call — rolling back all state changes atomically.
 */
contract ReentrantBeneficiary {
    IntentGatewayV2 public immutable gateway;

    Order private storedOrder;
    FillOptions private storedOptions;
    bool private armed;
    bool private reentered;

    constructor(address payable _gateway) {
        gateway = IntentGatewayV2(_gateway);
    }

    /// @notice Pre-approve the gateway to pull an ERC-20 from this contract.
    function approveGateway(address token, uint256 amount) external {
        IERC20(token).approve(address(gateway), amount);
    }

    /// @notice Load the reentrant payload before the outer fill is triggered.
    function arm(Order calldata order, FillOptions calldata options) external {
        storedOrder = order;
        storedOptions = options;
        armed = true;
    }

    /// @notice Triggered by the ETH transfer inside the fill loop.
    ///         Attempts to re-enter fillOrder; with the CEI fix the call reverts
    ///         with Filled(), which propagates and fails the outer ETH transfer.
    receive() external payable {
        if (armed && !reentered) {
            reentered = true;
            gateway.fillOrder(storedOrder, storedOptions);
        }
    }
}

/**
 * @title IntrinsicIntentsReentrancyTest
 * @notice Forge tests that confirm the CEI fix for same-chain fills and verify that
 *         cross-chain fills are also resistant to reentrancy attacks.
 *
 * `IntentGatewayV2.fillOrder` sets `_filled[commitment] = msg.sender` before it
 * delegates to either module, so a reentrant `fillOrder` attempt is always blocked by
 * its `Filled()` guard.
 *
 * Test matrix
 * ───────────
 *  testReentrancy_FeeTheft                    same-chain, 1 ETH output   → InsufficientNativeToken
 *  testReentrancy_EscrowTheft_MultiOutput     same-chain, two legs selling one token → InsufficientNativeToken
 *  testCrossChain_ReentrancyBlocked           cross-chain, 1 ETH output  → InsufficientNativeToken
 *  testCrossChain_ReentrancyBlocked_MultiOutput cross-chain, two legs selling one token → InsufficientNativeToken
 */
contract IntrinsicIntentsReentrancyTest is MainnetForkBaseTest {
    // ── constants ────────────────────────────────────────────────────────────

    uint256 constant INPUT_USDC = 1_000 * 1e6; // 1 000 USDC
    uint256 constant INPUT_DAI = 1_000 * 1e18; // 1 000 DAI
    uint256 constant OUTPUT_ETH = 1 ether;
    uint256 constant TX_FEES = 10 * 1e18; // 10 DAI (fee token)

    /// @dev Sentinel `_orders` key under which the gateway escrows tx fees.
    uint256 internal constant TRANSACTION_FEES = uint160(uint256(keccak256("txFees")));

    /// @dev 4-byte selector for the custom error thrown when a re-entered ETH
    ///      transfer returns false (the upstream Filled() revert is swallowed by
    ///      the .call return value, but then InsufficientNativeToken is thrown).
    bytes4 internal constant ERR_INSUFFICIENT_NATIVE = bytes4(keccak256("InsufficientNativeToken()"));

    // ── state ─────────────────────────────────────────────────────────────────

    IntentGatewayV2 public intentGateway;
    ReentrantBeneficiary public maliciousBeneficiary;

    address public attacker;
    address public legitimateSolver;

    // ── setup ─────────────────────────────────────────────────────────────────

    function _deployGatewayProxy() internal returns (IntentGatewayV2) {
        IntentGatewayV2 implementation = deployIntentGatewayImpl();
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), "");
        return IntentGatewayV2(payable(address(proxy)));
    }

    function setUp() public override {
        super.setUp();

        attacker = makeAddr("attacker");
        legitimateSolver = makeAddr("legitimateSolver");

        intentGateway = _deployGatewayProxy();
        intentGateway.initialize(
            InitParams({
                params: Params({
                    host: address(host),
                    dispatcher: address(dispatcher),
                    solverSelection: false,
                    surplusShareBps: 0,
                    protocolFeeBps: 0,
                    priceOracle: address(0)
                }),
                peerChains: new bytes[](0),
                relayer: address(0),
                owner: address(this)
            })
        );

        maliciousBeneficiary = new ReentrantBeneficiary(payable(address(intentGateway)));

        deal(address(usdc), attacker, INPUT_USDC + 1_000 * 1e6);
        deal(address(dai), attacker, INPUT_DAI + TX_FEES);
        vm.deal(legitimateSolver, OUTPUT_ETH * 2);
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    /// @dev Builds a same-chain order (source == destination == current chain).
    function _sameChainOrder(TokenInfo[] memory inputs, TokenInfo[] memory outputs, uint256 fees)
        internal
        view
        returns (Order memory)
    {
        return Order({
            user: bytes32(0), // stamped by placeOrder
            source: "", // stamped by placeOrder
            destination: host.host(), // same chain
            deadline: block.number + 100,
            nonce: 0,
            fees: fees,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({
                beneficiary: bytes32(uint256(uint160(address(maliciousBeneficiary)))), assets: outputs, call: ""
            })
        });
    }

    /// @dev Builds a cross-chain order (source = remote chain, destination = current chain).
    ///      No placeOrder needed — the source-chain escrow is out of scope for the
    ///      destination-side fill, which only transfers output tokens and dispatches
    ///      a RedeemEscrow message back.
    function _crossChainOrder(TokenInfo[] memory inputs, TokenInfo[] memory outputs)
        internal
        view
        returns (Order memory)
    {
        return Order({
            user: bytes32(uint256(uint160(attacker))),
            source: "EVM-2", // remote source chain (not current)
            destination: host.host(), // current chain (where fill happens)
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({
                beneficiary: bytes32(uint256(uint160(address(maliciousBeneficiary)))), assets: outputs, call: ""
            })
        });
    }

    // ── SAME-CHAIN TESTS (IntrinsicIntents._fillOrder) ───────────────────────

    /**
     * @dev Same-chain fee theft is now blocked by the CEI fix.
     *
     * Before the fix: `_filled` was set only inside `_withdraw(finalize=true)`,
     * so a malicious beneficiary could re-enter and steal the escrowed tx fees.
     *
     * After the fix: `_filled[commitment] = msg.sender` is set by `fillOrder`
     * before the module's output loop. The reentrant `fillOrder` call
     * therefore hits `Filled()`, propagates through `receive()`, causes the ETH
     * transfer to return false, and the outer call reverts with
     * `InsufficientNativeToken()` — rolling back all state changes.
     */
    function testReentrancy_FeeTheft() public {
        // ── 1. Place a same-chain order (input=USDC, output=ETH, fees=DAI) ───

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: INPUT_USDC});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: OUTPUT_ETH});

        Order memory order = _sameChainOrder(inputs, outputAssets, TX_FEES);

        vm.startPrank(attacker);
        usdc.approve(address(intentGateway), INPUT_USDC);
        dai.approve(address(intentGateway), TX_FEES);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Reconstruct the stamped order for commitment computation.
        order.user = bytes32(uint256(uint160(attacker)));
        order.source = host.host();
        order.nonce = 0;

        bytes32 commitment = keccak256(abi.encode(order));

        // Sanity: confirm fees are escrowed.
        assertEq(intentGateway._orders(commitment, TRANSACTION_FEES), TX_FEES);

        // ── 2. Arm the malicious beneficiary ─────────────────────────────────
        //
        // The reentrant FillOptions passes amount=0 so the re-entered loop's
        // `remaining == 0 || solverAmount == 0` branch is taken — but this
        // code path is never reached because _filled[commitment] is already set.

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](1);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0), amount: 0});

        maliciousBeneficiary.arm(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: reentrantOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, reentrantOutputs)
            })
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputAssets,
                inputs: IntentQuoteTestUtils.inputs(order, outputAssets)
            })
        );

        // ── 4. State is completely rolled back ───────────────────────────────

        assertEq(
            intentGateway._orders(commitment, TRANSACTION_FEES), TX_FEES, "fees must still be escrowed after revert"
        );
        assertEq(intentGateway._filled(commitment), address(0), "order must not be marked filled after revert");
        assertEq(dai.balanceOf(address(maliciousBeneficiary)), 0, "malicious beneficiary must not receive stolen fees");
        assertEq(usdc.balanceOf(legitimateSolver), 0, "solver must not have received any escrow");
    }

    /**
     * @dev Same-chain order selling USDC for ETH at two prices. The reentrant payload skips leg 0
     * and self-fills leg 1 to claim that leg's USDC escrow. Reentrancy is blocked, and both legs'
     * escrow survives the revert.
     */
    function testReentrancy_EscrowTheft_MultiOutput() public {
        uint256 outputEth2 = 0.5 ether;
        bytes32 usdcToken = bytes32(uint256(uint160(address(usdc))));

        // ── 1. Place a two-leg same-chain order ──────────────────────────────

        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: usdcToken, amount: INPUT_USDC});
        inputs[1] = TokenInfo({token: usdcToken, amount: INPUT_USDC});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: OUTPUT_ETH});
        outputAssets[1] = TokenInfo({token: bytes32(0), amount: outputEth2});

        Order memory order = _sameChainOrder(inputs, outputAssets, 0);

        vm.startPrank(attacker);
        usdc.approve(address(intentGateway), 2 * INPUT_USDC);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user = bytes32(uint256(uint160(attacker)));
        order.source = host.host();
        order.nonce = 0;

        bytes32 commitment = keccak256(abi.encode(order));

        // ── 2. Arm the malicious beneficiary ─────────────────────────────────

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](2);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0), amount: 0});
        reentrantOutputs[1] = TokenInfo({token: bytes32(0), amount: outputEth2});

        maliciousBeneficiary.arm(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: reentrantOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, reentrantOutputs)
            })
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH + outputEth2}(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputAssets,
                inputs: IntentQuoteTestUtils.inputs(order, outputAssets)
            })
        );

        // ── 4. State is completely rolled back ───────────────────────────────

        assertEq(intentGateway._orders(commitment, 0), INPUT_USDC, "leg 0 escrow must be intact after revert");
        assertEq(intentGateway._orders(commitment, 1), INPUT_USDC, "leg 1 escrow must be intact after revert");
        assertEq(intentGateway._filled(commitment), address(0), "order must not be marked filled after revert");
        assertEq(
            usdc.balanceOf(address(maliciousBeneficiary)), 0, "malicious beneficiary must not receive leg 1's escrow"
        );
    }

    // ── CROSS-CHAIN TESTS (ExtrinsicIntents._fillOrder) ──────────────────────
    //
    // Cross-chain fills are claimed by the same `_filled[commitment] = msg.sender`
    // in `fillOrder`. These tests confirm the protection holds for single- and
    // multi-output cross-chain orders.
    //
    // Setup difference from same-chain tests:
    //  - order.source = "EVM-2" (a remote chain, not the current host)
    //  - order.destination = host.host() (current chain — where fill happens)
    //  - No placeOrder needed; cross-chain fills don't access source-chain escrow.
    //
    // Attack flow (blocked):
    //   1. fillOrder sets _filled[commitment] = msg.sender
    //   2. fillOrder routes to ExtrinsicIntents._fillOrder (source != dest, dest == current)
    //   3. ETH output loop sends ETH to maliciousBeneficiary → receive() fires
    //   4. Reentrant fillOrder hits _filled[commitment] != 0 → Filled() revert
    //   5. Revert propagates through receive() → ETH .call returns false
    //   6. _payLeg throws InsufficientNativeToken() → full tx rollback

    /**
     * @dev Cross-chain fill with a single ETH output: reentrancy is blocked.
     */
    function testCrossChain_ReentrancyBlocked() public {
        // ── 1. Build a cross-chain order (no placeOrder required) ────────────

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: INPUT_USDC});

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: OUTPUT_ETH});

        Order memory order = _crossChainOrder(inputs, outputAssets);
        bytes32 commitment = keccak256(abi.encode(order));

        // ── 2. Arm the malicious beneficiary ─────────────────────────────────

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](1);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0), amount: 0});

        maliciousBeneficiary.arm(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: reentrantOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, reentrantOutputs)
            })
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputAssets,
                inputs: IntentQuoteTestUtils.inputs(order, outputAssets)
            })
        );

        // ── 4. _filled is rolled back — order remains fillable ───────────────

        assertEq(intentGateway._filled(commitment), address(0), "cross-chain: _filled must be 0 after revert");
    }

    /**
     * @dev Cross-chain fill of two legs selling the same token (USDC for ETH, USDC for DAI): the
     * reentrant payload skips the ETH leg and self-fills the DAI leg. `_filled` is set by `fillOrder`
     * before the module runs, so the reentrant call is blocked before any leg's progress is recorded.
     */
    function testCrossChain_ReentrancyBlocked_MultiOutput() public {
        uint256 outputDai = 500 * 1e18;
        bytes32 usdcToken = bytes32(uint256(uint160(address(usdc))));
        bytes32 daiToken = bytes32(uint256(uint160(address(dai))));

        // ── 1. Build a two-leg cross-chain order ─────────────────────────────

        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: usdcToken, amount: INPUT_USDC});
        inputs[1] = TokenInfo({token: usdcToken, amount: INPUT_USDC});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: OUTPUT_ETH});
        outputAssets[1] = TokenInfo({token: daiToken, amount: outputDai});

        Order memory order = _crossChainOrder(inputs, outputAssets);
        bytes32 commitment = keccak256(abi.encode(order));

        // ── 2. Arm with a self-fill reentrant payload ────────────────────────

        deal(address(dai), address(maliciousBeneficiary), outputDai);
        maliciousBeneficiary.approveGateway(address(dai), outputDai);

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](2);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0), amount: 0});
        reentrantOutputs[1] = TokenInfo({token: daiToken, amount: outputDai});

        maliciousBeneficiary.arm(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: reentrantOutputs,
                inputs: IntentQuoteTestUtils.inputs(order, reentrantOutputs)
            })
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({
                relayerFee: 0,
                nativeDispatchFee: 0,
                validUntil: 0,
                outputs: outputAssets,
                inputs: IntentQuoteTestUtils.inputs(order, outputAssets)
            })
        );

        // ── 4. Nothing was recorded for either leg ───────────────────────────

        assertEq(intentGateway._filled(commitment), address(0), "cross-chain multi-leg: _filled must be 0 after revert");
        assertEq(intentGateway._partialFills(commitment, 0), 0, "leg 0 progress rolled back");
        assertEq(intentGateway._partialFills(commitment, 1), 0, "leg 1 progress rolled back");
    }
}
