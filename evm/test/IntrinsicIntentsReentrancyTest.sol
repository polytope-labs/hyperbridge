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
    FillOptions
} from "../src/apps/IntentGatewayV2.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/**
 * @title ReentrantBeneficiary
 * @notice Malicious beneficiary contract that attempts to re-enter `fillOrder` during
 *         the ETH transfer made by `_fillSameChain` or `_fillCrossChain`.
 *
 * Attack window (pre-fix):
 *
 *   _fillSameChain / _fillCrossChain:
 *     beneficiary.call{value: ...}("")   ← RE-ENTRY HERE
 *     // _filled still == address(0) pre-fix, now set at the top (CEI)
 *
 * With the CEI fix in place, `_filled[commitment]` is set to `msg.sender` at the
 * very start of both fill functions. Any reentrant `fillOrder` call therefore hits
 * the `if (_filled[commitment] != address(0)) revert Filled()` guard and reverts.
 * That revert propagates through `receive()`, causing the outer ETH transfer to
 * return `(false, ...)`, which triggers `InsufficientNativeToken()` in the outer
 * call — rolling back all state changes atomically.
 */
contract ReentrantBeneficiary {
    IntentGatewayV2 public immutable gateway;

    Order       private storedOrder;
    FillOptions private storedOptions;
    bool        private armed;
    bool        private reentered;

    constructor(address payable _gateway) {
        gateway = IntentGatewayV2(_gateway);
    }

    /// @notice Pre-approve the gateway to pull an ERC-20 from this contract.
    function approveGateway(address token, uint256 amount) external {
        IERC20(token).approve(address(gateway), amount);
    }

    /// @notice Load the reentrant payload before the outer fill is triggered.
    function arm(Order calldata order, FillOptions calldata options) external {
        storedOrder   = order;
        storedOptions = options;
        armed         = true;
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
 * @notice Forge tests that confirm the CEI fix in `IntrinsicIntents._fillSameChain`
 *         and verify that `ExtrinsicIntents._fillCrossChain` is also resistant to
 *         reentrancy attacks.
 *
 * Both fill functions now open with `_filled[commitment] = msg.sender` before any
 * external calls, so a reentrant `fillOrder` attempt is always blocked by the
 * `Filled()` guard in `IntentGatewayV2.fillOrder`.
 *
 * Test matrix
 * ───────────
 *  testReentrancy_FeeTheft                    same-chain, 1 ETH output   → InsufficientNativeToken
 *  testReentrancy_EscrowTheft_MultiOutput     same-chain, ETH+ERC-20     → InsufficientNativeToken
 *  testCrossChain_ReentrancyBlocked           cross-chain, 1 ETH output  → InsufficientNativeToken
 *  testCrossChain_ReentrancyBlocked_MultiOutput cross-chain, ETH+ERC-20  → InsufficientNativeToken
 */
contract IntrinsicIntentsReentrancyTest is MainnetForkBaseTest {
    // ── constants ────────────────────────────────────────────────────────────

    uint256 constant INPUT_USDC = 1_000 * 1e6;  // 1 000 USDC
    uint256 constant INPUT_DAI  = 1_000 * 1e18; // 1 000 DAI
    uint256 constant OUTPUT_ETH = 1 ether;
    uint256 constant TX_FEES    = 10 * 1e18;    // 10 DAI (fee token)

    /// @dev Sentinel address used by the gateway to key escrowed tx fees.
    address internal constant TRANSACTION_FEES =
        address(uint160(uint256(keccak256("txFees"))));

    /// @dev 4-byte selector for the custom error thrown when a re-entered ETH
    ///      transfer returns false (the upstream Filled() revert is swallowed by
    ///      the .call return value, but then InsufficientNativeToken is thrown).
    bytes4 internal constant ERR_INSUFFICIENT_NATIVE =
        bytes4(keccak256("InsufficientNativeToken()"));

    // ── state ─────────────────────────────────────────────────────────────────

    IntentGatewayV2      public intentGateway;
    ReentrantBeneficiary public maliciousBeneficiary;

    address public attacker;
    address public legitimateSolver;

    // ── setup ─────────────────────────────────────────────────────────────────

    function setUp() public override {
        super.setUp();

        attacker         = makeAddr("attacker");
        legitimateSolver = makeAddr("legitimateSolver");

        intentGateway = new IntentGatewayV2(address(this));
        intentGateway.setParams(
            Params({
                host:            address(host),
                dispatcher:      address(dispatcher),
                solverSelection: false,
                surplusShareBps: 0,
                protocolFeeBps:  0,
                priceOracle:     address(0)
            })
        );

        maliciousBeneficiary = new ReentrantBeneficiary(payable(address(intentGateway)));

        deal(address(usdc), attacker, INPUT_USDC + 1_000 * 1e6);
        deal(address(dai),  attacker, INPUT_DAI  + TX_FEES);
        vm.deal(legitimateSolver, OUTPUT_ETH * 2);
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    /// @dev Builds a same-chain order (source == destination == current chain).
    function _sameChainOrder(
        TokenInfo[] memory inputs,
        TokenInfo[] memory outputs,
        uint256 fees
    ) internal view returns (Order memory) {
        return Order({
            user:        bytes32(0),     // stamped by placeOrder
            source:      "",             // stamped by placeOrder
            destination: host.host(),   // same chain
            deadline:    block.number + 100,
            nonce:       0,
            fees:        fees,
            session:     address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs:      inputs,
            output:      PaymentInfo({
                beneficiary: bytes32(uint256(uint160(address(maliciousBeneficiary)))),
                assets:      outputs,
                call:        ""
            })
        });
    }

    /// @dev Builds a cross-chain order (source = remote chain, destination = current chain).
    ///      No placeOrder needed — the source-chain escrow is out of scope for the
    ///      destination-side fill, which only transfers output tokens and dispatches
    ///      a RedeemEscrow message back.
    function _crossChainOrder(
        TokenInfo[] memory inputs,
        TokenInfo[] memory outputs
    ) internal view returns (Order memory) {
        return Order({
            user:        bytes32(uint256(uint160(attacker))),
            source:      "EVM-2",       // remote source chain (not current)
            destination: host.host(),  // current chain (where fill happens)
            deadline:    block.number + 100,
            nonce:       0,
            fees:        0,
            session:     address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs:      inputs,
            output:      PaymentInfo({
                beneficiary: bytes32(uint256(uint160(address(maliciousBeneficiary)))),
                assets:      outputs,
                call:        ""
            })
        });
    }

    // ── SAME-CHAIN TESTS (IntrinsicIntents._fillSameChain) ───────────────────

    /**
     * @dev Same-chain fee theft is now blocked by the CEI fix.
     *
     * Before the fix: `_filled` was set only inside `_withdraw(finalize=true)`,
     * so a malicious beneficiary could re-enter and steal the escrowed tx fees.
     *
     * After the fix: `_filled[commitment] = msg.sender` is set at the top of
     * `_fillSameChain`, before the output loop. The reentrant `fillOrder` call
     * therefore hits `Filled()`, propagates through `receive()`, causes the ETH
     * transfer to return false, and the outer call reverts with
     * `InsufficientNativeToken()` — rolling back all state changes.
     */
    function testReentrancy_FeeTheft() public {
        // ── 1. Place a same-chain order (input=USDC, output=ETH, fees=DAI) ───

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token:  bytes32(uint256(uint160(address(usdc)))),
            amount: INPUT_USDC
        });

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: OUTPUT_ETH});

        Order memory order = _sameChainOrder(inputs, outputAssets, TX_FEES);

        vm.startPrank(attacker);
        usdc.approve(address(intentGateway), INPUT_USDC);
        dai.approve(address(intentGateway), TX_FEES);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        // Reconstruct the stamped order for commitment computation.
        order.user   = bytes32(uint256(uint160(attacker)));
        order.source = host.host();
        order.nonce  = 0;

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
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: reentrantOutputs})
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputAssets})
        );

        // ── 4. State is completely rolled back ───────────────────────────────

        assertEq(
            intentGateway._orders(commitment, TRANSACTION_FEES),
            TX_FEES,
            "fees must still be escrowed after revert"
        );
        assertEq(
            intentGateway._filled(commitment),
            address(0),
            "order must not be marked filled after revert"
        );
        assertEq(
            dai.balanceOf(address(maliciousBeneficiary)),
            0,
            "malicious beneficiary must not receive stolen fees"
        );
        assertEq(
            usdc.balanceOf(legitimateSolver),
            0,
            "solver must not have received any escrow"
        );
    }

    /**
     * @dev Same-chain multi-output escrow theft is blocked by the CEI fix.
     *
     * Before the fix: on a two-output order (ETH + ERC-20), the malicious
     * beneficiary could re-enter during the ETH transfer, self-fill the ERC-20
     * output (net-zero cost), trigger `_withdraw(finalize=true)`, and steal the
     * entire input[1] escrow.
     *
     * After the fix: `_filled[commitment]` is set before the loop, so the
     * reentrant call reverts with `Filled()`. The whole transaction reverts with
     * `InsufficientNativeToken()` and no state is mutated.
     */
    function testReentrancy_EscrowTheft_MultiOutput() public {
        uint256 outputUSDC = 500 * 1e6;

        // ── 1. Place a two-output same-chain order ────────────────────────────

        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: INPUT_USDC});
        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))),  amount: INPUT_DAI});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(0),                                           amount: OUTPUT_ETH});
        outputAssets[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: outputUSDC});

        Order memory order = _sameChainOrder(inputs, outputAssets, 0);

        vm.startPrank(attacker);
        usdc.approve(address(intentGateway), INPUT_USDC);
        dai.approve(address(intentGateway), INPUT_DAI);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        order.user   = bytes32(uint256(uint160(attacker)));
        order.source = host.host();
        order.nonce  = 0;

        bytes32 commitment = keccak256(abi.encode(order));

        // ── 2. Arm the malicious beneficiary ─────────────────────────────────
        //
        // Reentrant payload: skip ETH output (amount=0), self-fill USDC output.
        // The self-fill would net-zero the beneficiary's USDC balance but claim
        // the full input[1] DAI escrow — if reentrancy were not blocked.

        deal(address(usdc), address(maliciousBeneficiary), outputUSDC);
        maliciousBeneficiary.approveGateway(address(usdc), outputUSDC);

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](2);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0),                                           amount: 0});
        reentrantOutputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: outputUSDC});

        maliciousBeneficiary.arm(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: reentrantOutputs})
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputAssets})
        );

        // ── 4. State is completely rolled back ───────────────────────────────

        assertEq(
            intentGateway._orders(commitment, address(dai)),
            INPUT_DAI,
            "DAI escrow must still be intact after revert"
        );
        assertEq(
            intentGateway._orders(commitment, address(usdc)),
            INPUT_USDC,
            "USDC escrow must still be intact after revert"
        );
        assertEq(
            intentGateway._filled(commitment),
            address(0),
            "order must not be marked filled after revert"
        );
        assertEq(
            dai.balanceOf(address(maliciousBeneficiary)),
            0,
            "malicious beneficiary must not receive stolen DAI escrow"
        );
    }

    // ── CROSS-CHAIN TESTS (ExtrinsicIntents._fillCrossChain) ─────────────────
    //
    // _fillCrossChain already applied the CEI pattern from the start (the
    // `_filled[commitment] = msg.sender` statement was never commented out in
    // ExtrinsicIntents.sol, unlike _fillSameChain). These tests confirm the
    // existing protection holds for single- and multi-output cross-chain orders.
    //
    // Setup difference from same-chain tests:
    //  - order.source = "EVM-2" (a remote chain, not the current host)
    //  - order.destination = host.host() (current chain — where fill happens)
    //  - No placeOrder needed; cross-chain fills don't access source-chain escrow.
    //
    // Attack flow (blocked):
    //   1. fillOrder routes to _fillCrossChain (source != dest, dest == current)
    //   2. _fillCrossChain sets _filled[commitment] = msg.sender immediately
    //   3. ETH output loop sends ETH to maliciousBeneficiary → receive() fires
    //   4. Reentrant fillOrder hits _filled[commitment] != 0 → Filled() revert
    //   5. Revert propagates through receive() → ETH .call returns false
    //   6. _fillCrossChain throws InsufficientNativeToken() → full tx rollback

    /**
     * @dev Cross-chain fill with a single ETH output: reentrancy is blocked.
     */
    function testCrossChain_ReentrancyBlocked() public {
        // ── 1. Build a cross-chain order (no placeOrder required) ────────────

        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({
            token:  bytes32(uint256(uint160(address(usdc)))),
            amount: INPUT_USDC
        });

        TokenInfo[] memory outputAssets = new TokenInfo[](1);
        outputAssets[0] = TokenInfo({token: bytes32(0), amount: OUTPUT_ETH});

        Order memory order = _crossChainOrder(inputs, outputAssets);
        bytes32 commitment = keccak256(abi.encode(order));

        // ── 2. Arm the malicious beneficiary ─────────────────────────────────

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](1);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0), amount: 0});

        maliciousBeneficiary.arm(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: reentrantOutputs})
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputAssets})
        );

        // ── 4. _filled is rolled back — order remains fillable ───────────────

        assertEq(
            intentGateway._filled(commitment),
            address(0),
            "cross-chain: _filled must be 0 after revert"
        );
    }

    /**
     * @dev Cross-chain fill with two outputs (ETH + ERC-20): reentrancy is blocked.
     *
     * The malicious beneficiary arms with a self-fill for the ERC-20 output —
     * the same zero-net-cost technique as the same-chain multi-output test.
     * Because _fillCrossChain sets _filled at the top, the reentrant call is
     * blocked before it can execute any token transfers.
     */
    function testCrossChain_ReentrancyBlocked_MultiOutput() public {
        uint256 outputUSDC = 500 * 1e6;

        // ── 1. Build a two-output cross-chain order ───────────────────────────

        TokenInfo[] memory inputs = new TokenInfo[](2);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: INPUT_USDC});
        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))),  amount: INPUT_DAI});

        TokenInfo[] memory outputAssets = new TokenInfo[](2);
        outputAssets[0] = TokenInfo({token: bytes32(0),                                           amount: OUTPUT_ETH});
        outputAssets[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: outputUSDC});

        Order memory order = _crossChainOrder(inputs, outputAssets);
        bytes32 commitment = keccak256(abi.encode(order));

        // ── 2. Arm with a self-fill reentrant payload ─────────────────────────

        deal(address(usdc), address(maliciousBeneficiary), outputUSDC);
        maliciousBeneficiary.approveGateway(address(usdc), outputUSDC);

        TokenInfo[] memory reentrantOutputs = new TokenInfo[](2);
        reentrantOutputs[0] = TokenInfo({token: bytes32(0),                                           amount: 0});
        reentrantOutputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: outputUSDC});

        maliciousBeneficiary.arm(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: reentrantOutputs})
        );

        // ── 3. Fill attempt reverts — reentrancy is blocked ──────────────────

        vm.expectRevert(ERR_INSUFFICIENT_NATIVE);
        vm.prank(legitimateSolver);
        intentGateway.fillOrder{value: OUTPUT_ETH}(
            order,
            FillOptions({relayerFee: 0, nativeDispatchFee: 0, outputs: outputAssets})
        );

        // ── 4. _filled is rolled back — no state was mutated ─────────────────

        assertEq(
            intentGateway._filled(commitment),
            address(0),
            "cross-chain multi-output: _filled must be 0 after revert"
        );
        assertEq(
            dai.balanceOf(address(maliciousBeneficiary)),
            0,
            "malicious beneficiary must not receive any DAI"
        );
    }
}
