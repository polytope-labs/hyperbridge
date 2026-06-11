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

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

import {StreamingYieldVault} from "../contracts/vaults/StreamingYieldVault.sol";

/// @dev Minimal mintable ERC-20 used as the vault's underlying asset. Standard behaviour:
///      no transfer fee, no rebasing, no callbacks.
contract MockERC20 is ERC20 {
    constructor() ERC20("Mock", "MOCK") {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @notice Adversarial and invariant tests for {StreamingYieldVault}, focused on the known
///         ERC-4626 / yield-vault vulnerability classes:
///          - first-depositor inflation / donation attack
///          - yield sniping (sandwiching a yield event)
///          - vesting correctness & the no-backlog `addYield` guard
///          - underflow safety, access control, input validation
///          - rounding-in-the-vault's-favour and ERC-4626 preview parity
contract StreamingYieldVaultTest is Test {
    MockERC20 internal asset;
    StreamingYieldVault internal vault;

    address internal owner = makeAddr("owner");
    address internal alice = makeAddr("alice"); // long-term, honest LP
    address internal bob = makeAddr("bob");
    address internal attacker = makeAddr("attacker");
    address internal seeder = makeAddr("seeder");

    uint256 internal constant VEST = 23 hours;
    uint256 internal constant OFFSET = 6;

    function setUp() public {
        // Warp to a realistic, mainnet-like timestamp so vesting/guard arithmetic behaves as
        // it would in production (and never near the chain-genesis edge).
        vm.warp(1_700_000_000);

        asset = new MockERC20();
        vault = new StreamingYieldVault(IERC20(address(asset)), "Vault MOCK", "vMOCK", owner);

        address[4] memory actors = [alice, bob, attacker, seeder];
        for (uint256 i = 0; i < actors.length; i++) {
            asset.mint(actors[i], 2_000_000 ether);
            vm.prank(actors[i]);
            asset.approve(address(vault), type(uint256).max);
        }
        // The owner funds yield, so it needs balance + allowance too.
        asset.mint(owner, 2_000_000 ether);
        vm.prank(owner);
        asset.approve(address(vault), type(uint256).max);
    }

    // --------------------------------------------------------------------------------------
    // helpers
    // --------------------------------------------------------------------------------------

    function _deposit(address who, uint256 amount) internal returns (uint256 shares) {
        vm.prank(who);
        shares = vault.deposit(amount, who);
    }

    function _redeemAll(address who) internal returns (uint256 assets) {
        uint256 shares = vault.balanceOf(who);
        vm.prank(who);
        assets = vault.redeem(shares, who, who);
    }

    function _addYield(uint256 amount) internal {
        vm.prank(owner);
        vault.addYield(amount);
    }

    /// @dev Seed the vault with a real first deposit and burn the shares. Together with the
    ///      decimals offset this removes the empty-vault edge for the economic tests.
    function _seedAndBurn(uint256 amount) internal {
        vm.prank(seeder);
        uint256 shares = vault.deposit(amount, seeder);
        vm.prank(seeder);
        vault.transfer(address(0xdead), shares);
    }

    // --------------------------------------------------------------------------------------
    // 1. first-depositor inflation / donation attack
    // --------------------------------------------------------------------------------------

    /// @notice Classic attack on an empty vault: deposit 1 wei, donate a large amount directly
    ///         to spike the price, then let the victim deposit. The virtual-share offset must
    ///         leave the victim with non-zero shares and make the attack unprofitable.
    function test_InflationAttack_isUnprofitable_andVictimKeptWhole() public {
        uint256 donation = 100 ether;
        uint256 victimDeposit = 100 ether;

        uint256 attackerSpent;

        // 1 wei first deposit.
        vm.prank(attacker);
        vault.deposit(1, attacker);
        attackerSpent += 1;

        // Direct donation to inflate balanceOf (and thus the naive price).
        vm.prank(attacker);
        asset.transfer(address(vault), donation);
        attackerSpent += donation;

        // Victim deposits.
        uint256 victimShares = _deposit(alice, victimDeposit);

        // The offset prevents the victim from being rounded down to nothing.
        assertGt(victimShares, 0, "victim minted zero shares");

        // Attacker unwinds; must not profit from the manipulation.
        uint256 attackerOut = _redeemAll(attacker);
        assertLt(attackerOut, attackerSpent, "attack was profitable");

        // Victim can recover essentially their whole deposit (>= 99%).
        uint256 victimOut = _redeemAll(alice);
        assertGe(victimOut, (victimDeposit * 99) / 100, "victim lost value to the attacker");
    }

    /// @notice With a seed-and-burn at deployment, the same attack is strictly weaker: the
    ///         victim is fully made whole.
    function test_InflationAttack_defeatedBySeed() public {
        _seedAndBurn(10 ether);

        vm.prank(attacker);
        asset.transfer(address(vault), 100 ether);

        uint256 victimShares = _deposit(alice, 100 ether);
        assertGt(victimShares, 0);

        uint256 victimOut = _redeemAll(alice);
        assertGe(victimOut, (100 ether * 99) / 100, "victim lost value despite seed");
    }

    // --------------------------------------------------------------------------------------
    // 2. yield sniping
    // --------------------------------------------------------------------------------------

    /// @notice Same-block sandwich: deposit -> addYield -> redeem all at one timestamp. Because
    ///         the freshly added tranche is fully locked, the sniper captures nothing.
    function test_Snipe_sameBlock_capturesNothing() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 1_000 ether); // honest holder

        uint256 snipe = 1_000_000 ether;
        uint256 spent = snipe;
        _deposit(attacker, snipe);

        _addYield(100 ether); // same block.timestamp as the deposit & redeem

        uint256 out = _redeemAll(attacker);
        assertLe(out, spent, "same-block snipe extracted yield");
    }

    /// @notice Crossing a single block (~12s) lets the sniper capture only the dust that vested
    ///         in that interval — far below any meaningful slice of the tranche.
    function test_Snipe_oneBlock_capturesOnlyDust() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 1_000 ether);

        uint256 snipe = 1_000_000 ether;
        _deposit(attacker, snipe);

        uint256 yield = 100 ether;
        _addYield(yield);

        vm.warp(block.timestamp + 12); // one mainnet block
        uint256 out = _redeemAll(attacker);

        uint256 profit = out > snipe ? out - snipe : 0;
        // Captured dust must be well under 1% of the tranche.
        assertLt(profit, yield / 100, "one-block snipe captured a meaningful share");
    }

    /// @notice The yield a sniper is denied accrues to the honest long-term holder once the
    ///         tranche has fully vested.
    function test_Yield_accruesToLongTermHolder() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 1_000 ether);

        uint256 before = vault.convertToAssets(vault.balanceOf(alice));
        _addYield(100 ether);
        vm.warp(block.timestamp + VEST);

        uint256 afterVest = vault.convertToAssets(vault.balanceOf(alice));
        assertGt(afterVest, before, "long-term holder did not accrue yield");
    }

    // --------------------------------------------------------------------------------------
    // 3. vesting correctness
    // --------------------------------------------------------------------------------------

    /// @notice Adding yield must not move `totalAssets` in the same block (no instant jump to
    ///         snipe), then must unlock linearly to the full amount over `VEST`.
    function test_Vesting_noJump_thenLinear() public {
        _seedAndBurn(1_000 ether);

        uint256 base = vault.totalAssets();
        _addYield(100 ether);
        assertEq(vault.totalAssets(), base, "yield recognized instantly (jump)");

        vm.warp(block.timestamp + VEST / 2);
        assertApproxEqAbs(vault.totalAssets(), base + 50 ether, 1e6, "half not recognized");

        vm.warp(block.timestamp + VEST / 2);
        assertEq(vault.totalAssets(), base + 100 ether, "full not recognized after VEST");
        assertEq(vault.lockedYield(), 0, "locked yield remained after full vest");
    }

    /// @notice `totalAssets` is monotonically non-decreasing while a tranche vests with no other
    ///         flows.
    function testFuzz_Vesting_monotonic(uint256 amount, uint96 dt1, uint96 dt2) public {
        amount = bound(amount, 1, 1_000_000 ether);
        uint256 e1 = bound(dt1, 0, VEST);
        uint256 e2 = bound(dt2, e1, VEST);

        _seedAndBurn(1_000 ether);
        uint256 t0 = block.timestamp;
        _addYield(amount);

        vm.warp(t0 + e1);
        uint256 a1 = vault.totalAssets();
        vm.warp(t0 + e2);
        uint256 a2 = vault.totalAssets();

        assertGe(a2, a1, "totalAssets decreased during vesting");
    }

    // --------------------------------------------------------------------------------------
    // 4. addYield guard / no backlog
    // --------------------------------------------------------------------------------------

    function test_AddYield_revertsWhileVesting() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(block.timestamp + VEST - 1);
        uint256 expectedVestedAt = vault.vestedAt(); // evaluate before prank so it isn't consumed
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(StreamingYieldVault.YieldStillVesting.selector, expectedVestedAt));
        vault.addYield(100 ether);
    }

    function test_AddYield_allowedExactlyAtVestedAt() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(vault.vestedAt());
        _addYield(50 ether); // must not revert
    }

    /// @notice A full add -> vest -> add cycle leaves no permanently locked backlog: after the
    ///         first tranche vests, all of it is recognized and the second tranche starts clean.
    function test_AddYield_noBacklogAcrossCycles() public {
        _seedAndBurn(1_000 ether);
        uint256 base = vault.totalAssets();

        _addYield(100 ether);
        vm.warp(block.timestamp + VEST);
        assertEq(vault.totalAssets(), base + 100 ether, "first tranche not fully recognized");

        _addYield(40 ether);
        assertEq(vault.lockedYield(), 40 ether, "second tranche did not start at full lock");
        vm.warp(block.timestamp + VEST);
        assertEq(vault.totalAssets(), base + 140 ether, "backlog left locked after two cycles");
    }

    // --------------------------------------------------------------------------------------
    // 5. underflow safety, access control, validation
    // --------------------------------------------------------------------------------------

    /// @notice The first `addYield` on an empty vault must not underflow `totalAssets` despite
    ///         the tranche exceeding the (zero) backing.
    function test_AddYield_firstTrancheExceedingBacking_noUnderflow() public {
        _addYield(100 ether); // empty vault
        assertEq(vault.totalAssets(), 0, "locked tranche leaked into totalAssets");

        vm.warp(block.timestamp + VEST);
        assertEq(vault.totalAssets(), 100 ether);
    }

    function test_AddYield_onlyOwner() public {
        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, attacker));
        vault.addYield(1 ether);
    }

    function test_AddYield_zeroAmountReverts() public {
        vm.prank(owner);
        vm.expectRevert(StreamingYieldVault.ZeroAmount.selector);
        vault.addYield(0);
    }

    // --------------------------------------------------------------------------------------
    // 6. rounding & ERC-4626 parity
    // --------------------------------------------------------------------------------------

    /// @notice A deposit immediately followed by a full redeem must never return more than was
    ///         put in — rounding always favours the vault.
    function testFuzz_depositRedeem_roundsInVaultFavour(uint256 amount) public {
        amount = bound(amount, 1, 1_000_000 ether);
        _seedAndBurn(1_000 ether);

        uint256 shares = _deposit(alice, amount);
        vm.prank(alice);
        uint256 out = vault.redeem(shares, alice, alice);

        assertLe(out, amount, "user extracted value via rounding");
    }

    /// @notice `previewDeposit`/`previewRedeem` must match the realized amounts.
    function test_previewParity() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);
        vm.warp(block.timestamp + VEST / 3);

        uint256 pd = vault.previewDeposit(123 ether);
        uint256 shares = _deposit(bob, 123 ether);
        assertEq(shares, pd, "previewDeposit != deposit");

        uint256 pr = vault.previewRedeem(shares);
        vm.prank(bob);
        uint256 out = vault.redeem(shares, bob, bob);
        assertEq(out, pr, "previewRedeem != redeem");
    }

    function test_decimalsOffsetApplied() public view {
        assertEq(vault.decimals(), asset.decimals() + OFFSET);
    }

    // --------------------------------------------------------------------------------------
    // 7. pausing
    // --------------------------------------------------------------------------------------

    function test_Pause_blocksShareTransfer() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 100 ether);

        vm.prank(owner);
        vault.pause();

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.transfer(bob, 1);
    }

    function test_Pause_blocksDeposit() public {
        vm.prank(owner);
        vault.pause();

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.deposit(100 ether, alice);
    }

    function test_Pause_blocksWithdraw() public {
        _seedAndBurn(1_000 ether);
        uint256 shares = _deposit(alice, 100 ether);

        vm.prank(owner);
        vault.pause();

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.redeem(shares, alice, alice);
    }

    function test_Unpause_restoresShareMovement() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 100 ether);

        vm.prank(owner);
        vault.pause();
        vm.prank(owner);
        vault.unpause();

        vm.prank(alice);
        vault.transfer(bob, 1); // must not revert
        assertEq(vault.balanceOf(bob), 1);
    }

    function test_Pause_onlyOwner() public {
        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, attacker));
        vault.pause();
    }

    function test_Unpause_onlyOwner() public {
        vm.prank(owner);
        vault.pause();

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, attacker));
        vault.unpause();
    }

    /// @notice Adding yield touches no shares, so it remains possible while paused; the funds
    ///         simply cannot be withdrawn until the vault is unpaused.
    function test_AddYield_worksWhilePaused() public {
        _seedAndBurn(1_000 ether);

        vm.prank(owner);
        vault.pause();

        _addYield(100 ether);
        vm.warp(block.timestamp + VEST);
        assertApproxEqAbs(vault.totalAssets(), 1_000 ether + 100 ether, 1e6);
    }

    // --------------------------------------------------------------------------------------
    // 8. the `_update` chokepoint
    //
    // Every share movement — mint (deposit/mint), burn (withdraw/redeem) and transfer
    // (transfer/transferFrom) — funnels through `_update`. These tests assert each distinct
    // entry into `_update` is gated, that the gate sits at `_update` itself (so even a
    // zero-value move reverts), and that it is a pure no-op when unpaused.
    // --------------------------------------------------------------------------------------

    /// @notice The share-denominated mint path (`mint`) hits `_update` via `_mint`.
    function test_Pause_chokepoint_blocksMint() public {
        _seedAndBurn(1_000 ether);

        vm.prank(owner);
        vault.pause();

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.mint(100 ether, alice);
    }

    /// @notice The asset-denominated burn path (`withdraw`) hits `_update` via `_burn`.
    function test_Pause_chokepoint_blocksWithdrawByAssets() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 100 ether);

        vm.prank(owner);
        vault.pause();

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.withdraw(50 ether, alice, alice);
    }

    /// @notice The delegated-transfer path (`transferFrom`) hits `_update` via `_transfer`.
    function test_Pause_chokepoint_blocksTransferFrom() public {
        _seedAndBurn(1_000 ether);
        uint256 shares = _deposit(alice, 100 ether);

        vm.prank(alice);
        vault.approve(bob, shares);

        vm.prank(owner);
        vault.pause();

        vm.prank(bob);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.transferFrom(alice, bob, shares);
    }

    /// @notice The gate lives in `_update`, not in any amount-specific branch: even a zero-value
    ///         transfer (which still calls `_update`) reverts while paused.
    function test_Pause_chokepoint_blocksZeroValueTransfer() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 100 ether);

        vm.prank(owner);
        vault.pause();

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.transfer(bob, 0);
    }

    /// @notice `paused()` tracks the toggle, and the chokepoint re-engages after a pause/unpause
    ///         cycle (no sticky state).
    function test_Pause_chokepoint_isRepausable() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 100 ether);

        assertFalse(vault.paused());

        vm.prank(owner);
        vault.pause();
        assertTrue(vault.paused());

        vm.prank(owner);
        vault.unpause();
        assertFalse(vault.paused());

        // pause again — movement must be blocked once more
        vm.prank(owner);
        vault.pause();
        assertTrue(vault.paused());

        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        vault.transfer(bob, 1);
    }

    /// @notice When unpaused the chokepoint is a no-op: every `_update` path (mint, transferFrom,
    ///         burn) succeeds.
    function test_Unpause_chokepoint_allPathsFlow() public {
        _seedAndBurn(1_000 ether);

        vm.prank(owner);
        vault.pause();
        vm.prank(owner);
        vault.unpause();

        // mint (mint -> _mint -> _update); mint() takes a share amount and returns assets paid
        vm.prank(alice);
        vault.mint(100 ether, alice);
        uint256 shares = vault.balanceOf(alice);
        assertEq(shares, 100 ether);

        // transferFrom (-> _transfer -> _update)
        vm.prank(alice);
        vault.approve(bob, shares);
        vm.prank(bob);
        vault.transferFrom(alice, bob, shares);
        assertEq(vault.balanceOf(bob), shares);

        // burn (redeem -> _burn -> _update)
        vm.prank(bob);
        uint256 out = vault.redeem(shares, bob, bob);
        assertGt(out, 0);
        assertEq(vault.balanceOf(bob), 0);
    }
}
