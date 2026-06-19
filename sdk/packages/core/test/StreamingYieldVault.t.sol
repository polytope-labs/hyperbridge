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
import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {ERC1363} from "@openzeppelin/contracts/token/ERC20/extensions/ERC1363.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {StreamingYieldVault} from "../contracts/vaults/StreamingYieldVault.sol";

/// @dev Minimal mintable ERC-20 used as the vault's underlying asset. Standard behaviour:
///      no transfer fee, no rebasing, no callbacks.
contract MockERC20 is ERC20 {
    constructor() ERC20("Mock", "MOCK") {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @dev Mintable ERC-1363 ("payable token") used to exercise the single-tx `transferAndCall`
///      deposit path against the vault's `onTransferReceived` hook.
contract MockERC1363 is ERC1363 {
    constructor() ERC20("Mock1363", "M1363") {}

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

    uint256 internal constant VEST = 22 hours;
    uint256 internal constant MIN_WINDOW = 2 hours;
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

    /// @notice After the vest ends, `addYield` is still blocked throughout the guaranteed deposit
    ///         window, so new capital always has a chance to enter.
    function test_AddYield_revertsDuringDepositWindow() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        uint256 closesAt = vault.nextYieldAt();

        vm.warp(vault.vestedAt()); // window just opened
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(StreamingYieldVault.DepositWindowOpen.selector, closesAt));
        vault.addYield(50 ether);

        vm.warp(closesAt - 1); // one second before it closes
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(StreamingYieldVault.DepositWindowOpen.selector, closesAt));
        vault.addYield(50 ether);
    }

    function test_AddYield_allowedAtNextYieldAt() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(vault.nextYieldAt());
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

        vm.warp(block.timestamp + MIN_WINDOW); // clear the guaranteed deposit window before re-arming
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
        // Deposits are locked while vesting, so exercise parity in the post-vest window where the
        // price already reflects the fully recognized tranche.
        vm.warp(vault.vestedAt());

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
    // 7. deposit lock while vesting + guaranteed deposit window
    //
    // Deposits/mints are disabled while a tranche vests, so no one joins mid-tranche and shares
    // the in-flight yield. `addYield` must then wait `MIN_WINDOW` past the vest end, guaranteeing
    // a deposit window every cycle regardless of how eagerly the keeper re-arms.
    // --------------------------------------------------------------------------------------

    /// @notice Test constants track the contract; the guards below assume this parity.
    function test_constants_match() public view {
        assertEq(vault.VEST(), VEST, "VEST drift");
        assertEq(vault.MIN_WINDOW(), MIN_WINDOW, "MIN_WINDOW drift");
        assertEq(vault.nextYieldAt(), vault.vestedAt() + MIN_WINDOW, "window math");
    }

    /// @notice Before any tranche exists (`_vestingStart == 0`) deposits are open, so the vault can
    ///         be bootstrapped.
    function test_Deposit_allowedBeforeFirstYield() public {
        assertEq(vault.maxDeposit(alice), type(uint256).max);
        assertGt(_deposit(alice, 100 ether), 0);
    }

    /// @notice A deposit while a tranche is vesting is blocked: `maxDeposit` is 0, so `deposit`
    ///         reverts at its limit check with the standard ERC-4626 error.
    function test_Deposit_revertsWhileVesting() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(block.timestamp + VEST / 2);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, alice, 100 ether, 0));
        vault.deposit(100 ether, alice);
    }

    /// @notice `mint` is gated by `maxMint` the same way.
    function test_Mint_revertsWhileVesting() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(block.timestamp + VEST / 2);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxMint.selector, alice, 100 ether, 0));
        vault.mint(100 ether, alice);
    }

    /// @notice The lock lifts at the vest end; deposits succeed throughout the window, right up to
    ///         the moment the next tranche could be armed.
    function test_Deposit_succeedsAcrossWholeWindow() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(vault.vestedAt()); // window opens
        assertGt(_deposit(alice, 100 ether), 0, "deposit blocked at window open");

        vm.warp(vault.nextYieldAt()); // last moment before addYield can fire
        assertGt(_deposit(bob, 100 ether), 0, "deposit blocked later in window");
    }

    /// @notice `maxDeposit`/`maxMint` report 0 while vesting and unbounded otherwise, so integrators
    ///         can detect the closed window instead of hitting a surprise revert.
    function test_MaxDepositMint_reflectLock() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(block.timestamp + VEST / 2);
        assertEq(vault.maxDeposit(alice), 0, "maxDeposit not zero while vesting");
        assertEq(vault.maxMint(alice), 0, "maxMint not zero while vesting");

        vm.warp(vault.vestedAt());
        assertEq(vault.maxDeposit(alice), type(uint256).max, "maxDeposit still zero in window");
        assertEq(vault.maxMint(alice), type(uint256).max, "maxMint still zero in window");
    }

    /// @notice The window is guaranteed: the instant vesting ends, an eager keeper's `addYield`
    ///         still reverts while deposits are open — the window cannot be squeezed shut.
    function test_GuaranteedWindow_keeperCannotSqueezeShut() public {
        _seedAndBurn(1_000 ether);
        _addYield(100 ether);

        vm.warp(vault.vestedAt());
        uint256 closesAt = vault.nextYieldAt(); // read before prank so the view call doesn't consume it
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(StreamingYieldVault.DepositWindowOpen.selector, closesAt));
        vault.addYield(100 ether);

        assertEq(vault.maxDeposit(alice), type(uint256).max, "deposits not open during window");
        assertGt(_deposit(alice, 100 ether), 0);
    }

    /// @notice End-to-end: a depositor who enters in the window captures none of the tranche that
    ///         was already streaming when they arrived, but earns the next one. Because no one can
    ///         join mid-vest, the in-flight tranche stays with the prior holders.
    function test_WindowDepositor_earnsNextTrancheNotCurrent() public {
        _seedAndBurn(1_000 ether);
        _deposit(alice, 1_000 ether); // present for tranche 1

        _addYield(100 ether); // tranche 1 — streams only to seed + alice
        vm.warp(vault.vestedAt());

        // Window: bob joins for the next cycle and records what he can immediately redeem.
        uint256 aliceAfterT1 = vault.convertToAssets(vault.balanceOf(alice));
        uint256 bobShares = _deposit(bob, 1_000 ether);
        uint256 bobIn = vault.convertToAssets(bobShares);
        assertApproxEqAbs(bobIn, 1_000 ether, 1e6, "window depositor did not enter at par");

        // Arm tranche 2 and let it fully vest.
        vm.warp(vault.nextYieldAt());
        _addYield(100 ether);
        vm.warp(vault.vestedAt());

        assertGt(vault.convertToAssets(vault.balanceOf(bob)), bobIn, "window depositor earned nothing next cycle");
        assertGt(vault.convertToAssets(vault.balanceOf(alice)), aliceAfterT1, "prior holder lost ground next cycle");
    }

    // --------------------------------------------------------------------------------------
    // 8. ERC-1363 single-transaction yield top-up (`transferAndCall` -> `onTransferReceived`)
    // --------------------------------------------------------------------------------------

    /// @dev Deploy a vault over an ERC-1363 asset, funded + approved for the usual actors.
    function _deploy1363() internal returns (MockERC1363 a, StreamingYieldVault v) {
        a = new MockERC1363();
        v = new StreamingYieldVault(IERC20(address(a)), "Vault 1363", "v1363", owner);
        address[3] memory who = [alice, bob, owner];
        for (uint256 i = 0; i < who.length; i++) {
            a.mint(who[i], 1_000_000 ether);
            vm.prank(who[i]);
            a.approve(address(v), type(uint256).max);
        }
    }

    /// @notice The owner funds a tranche in one transaction via `transferAndCall`; it streams over
    ///         `VEST` exactly like `addYield`, with no instant jump.
    function test_ERC1363_transferAndCall_addsYield() public {
        (MockERC1363 a, StreamingYieldVault v) = _deploy1363();
        vm.prank(alice);
        v.deposit(1_000 ether, alice); // a holder for the yield to accrue to
        uint256 base = v.totalAssets();

        vm.prank(owner);
        a.transferAndCall(address(v), 100 ether);

        assertEq(v.lockedYield(), 100 ether, "tranche not locked");
        assertEq(v.totalAssets(), base, "yield recognized instantly");
        vm.warp(block.timestamp + VEST);
        assertEq(v.totalAssets(), base + 100 ether, "tranche not fully recognized after VEST");
    }

    /// @notice Only the owner may fund yield this way: a non-owner `transferAndCall` reverts and the
    ///         asset transfer rolls back.
    function test_ERC1363_onlyOwnerMayFund() public {
        (MockERC1363 a, StreamingYieldVault v) = _deploy1363();

        uint256 balBefore = a.balanceOf(alice);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, alice));
        a.transferAndCall(address(v), 100 ether);

        assertEq(a.balanceOf(alice), balBefore, "asset transfer was not rolled back");
        assertEq(v.lockedYield(), 0, "yield was armed by a non-owner");
    }

    /// @notice The hook is only callable by the asset; a direct call cannot arm a tranche.
    function test_ERC1363_onTransferReceived_onlyAsset() public {
        (, StreamingYieldVault v) = _deploy1363();

        vm.prank(owner); // even the owner can't call it directly — it must come from the asset
        vm.expectRevert(abi.encodeWithSelector(StreamingYieldVault.CallerNotAsset.selector, owner));
        v.onTransferReceived(owner, owner, 100 ether, "");
    }

    /// @notice The single-tx path honours the no-overlap guard: funding again while a tranche is
    ///         vesting reverts (and the asset transfer rolls back).
    function test_ERC1363_respectsVestingGuard() public {
        (MockERC1363 a, StreamingYieldVault v) = _deploy1363();

        vm.prank(owner);
        a.transferAndCall(address(v), 100 ether); // tranche 1
        vm.warp(block.timestamp + 1 hours); // mid-vest

        uint256 vestEnd = v.vestedAt(); // read before prank so the view doesn't consume it
        uint256 balBefore = a.balanceOf(owner);
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(StreamingYieldVault.YieldStillVesting.selector, vestEnd));
        a.transferAndCall(address(v), 50 ether);

        assertEq(a.balanceOf(owner), balBefore, "asset transfer was not rolled back");
    }
}
