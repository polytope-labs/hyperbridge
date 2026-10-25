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
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {IERC4626} from "@openzeppelin/contracts/interfaces/IERC4626.sol";
import {ERC7821} from "@openzeppelin/contracts/account/extensions/draft-ERC7821.sol";
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";
import {
    ERC7579Utils,
    Mode,
    ModeSelector,
    ModePayload
} from "@openzeppelin/contracts/account/utils/draft-ERC7579Utils.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

import {YieldCaptureVault} from "../contracts/vaults/YieldCaptureVault.sol";

/// @dev Mintable and burnable 6-decimal ERC-20 standing in for USDC.
contract MockUSDC is ERC20 {
    constructor() ERC20("Mock USDC", "USDC") {}

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    function burn(address from, uint256 amount) external {
        _burn(from, amount);
    }
}

/// @dev A small ERC-4626 vault that mimics Aave's StataTokenV2 ("stataUSDC"):
///      - shares and assets are related by a RAY-scaled `rate`, so 1 share is worth `rate / RAY` assets;
///      - `convertToShares` and `convertToAssets` round down, `previewMint` and `previewWithdraw` round up;
///      - a deposit that mints zero shares reverts, and so does a zero-amount withdraw or redeem;
///      - liquidity and the remaining supply cap can be limited;
///      - it models three kinds of stop. A frozen Aave reserve (`reserveFrozen`) blocks supply only: the
///        deposit limits return 0 and deposits revert, while withdrawals work. A paused reserve
///        (`reservePaused`) blocks supply and withdrawal, and every max function returns 0. StataTokenV2's own
///        pause (`tokenPaused`) makes transfers, mints and burns revert but is NOT reported by the max
///        functions, exactly like the real contract.
///      The starting `rate` is deliberately not 1.0, so rounding dust really appears in the tests.
contract MockStataVault is ERC4626, Ownable {
    using Math for uint256;

    error ReserveFrozen();
    error ReservePaused();
    error TokenPaused();
    error ZeroShares();
    error ZeroAmount();

    uint256 public constant RAY = 1e27;

    /// @dev Assets per share, scaled by RAY. Real stataUSDC sits near 1.11 today.
    uint256 public rate = 1.1129e27;

    /// @dev The most assets the vault can pay out right now, mimicking Aave pool liquidity.
    ///      `type(uint256).max` means no limit.
    uint256 public availableLiquidity = type(uint256).max;

    /// @dev The most assets that can still be deposited before the Aave supply cap is reached, mimicking
    ///      `maxDeposit`. It shrinks as deposits arrive. `type(uint256).max` means no cap.
    uint256 public supplyCapRemaining = type(uint256).max;

    /// @dev The Aave reserve is frozen: supply is blocked, withdrawal is not. `maxDeposit` and `maxMint`
    ///      report 0; `maxRedeem` and `maxWithdraw` are unchanged.
    bool public reserveFrozen;

    /// @dev The Aave reserve is paused (or inactive). Supply and withdrawal are blocked, and every max
    ///      function reports 0.
    bool public reservePaused;

    /// @dev StataTokenV2's own pause. Transfers, mints and burns revert, but the max functions do not change.
    bool public tokenPaused;

    constructor(MockUSDC usdc)
        ERC20("Mock Static Aave USDC", "mwaUSDC")
        ERC4626(IERC20(address(usdc)))
        Ownable(msg.sender)
    {}

    /// @dev Moves the rate and mints or burns the vault's USDC so it can pay every share at the new
    ///      rate. This turns a rate change into real money, like interest accruing in Aave.
    function setRate(uint256 newRate) external onlyOwner {
        rate = newRate;
        uint256 requiredBacking = totalSupply().mulDiv(newRate, RAY);
        uint256 currentBacking = IERC20(asset()).balanceOf(address(this));
        if (requiredBacking > currentBacking) {
            MockUSDC(asset()).mint(address(this), requiredBacking - currentBacking);
        } else if (currentBacking > requiredBacking) {
            MockUSDC(asset()).burn(address(this), currentBacking - requiredBacking);
        }
    }

    function setAvailableLiquidity(uint256 newAvailableLiquidity) external onlyOwner {
        availableLiquidity = newAvailableLiquidity;
    }

    function setSupplyCapRemaining(uint256 newSupplyCapRemaining) external onlyOwner {
        supplyCapRemaining = newSupplyCapRemaining;
    }

    function setReserveFrozen(bool newReserveFrozen) external onlyOwner {
        reserveFrozen = newReserveFrozen;
    }

    function setReservePaused(bool newReservePaused) external onlyOwner {
        reservePaused = newReservePaused;
    }

    function setTokenPaused(bool newTokenPaused) external onlyOwner {
        tokenPaused = newTokenPaused;
    }

    function totalAssets() public view override returns (uint256) {
        return totalSupply().mulDiv(rate, RAY);
    }

    function _convertToShares(uint256 assets, Math.Rounding rounding) internal view override returns (uint256) {
        return assets.mulDiv(RAY, rate, rounding);
    }

    function _convertToAssets(uint256 shares, Math.Rounding rounding) internal view override returns (uint256) {
        return shares.mulDiv(rate, RAY, rounding);
    }

    function maxDeposit(address) public view override returns (uint256) {
        return (reservePaused || reserveFrozen) ? 0 : supplyCapRemaining;
    }

    /// @dev Like stataUSDC, the mint limit is the deposit limit converted to shares.
    function maxMint(address receiver) public view override returns (uint256) {
        uint256 maxAssets = maxDeposit(receiver);
        if (maxAssets == type(uint256).max) return type(uint256).max;
        return _convertToShares(maxAssets, Math.Rounding.Floor);
    }

    function maxRedeem(address holder) public view override returns (uint256) {
        if (reservePaused) return 0;
        uint256 holderShares = balanceOf(holder);
        if (availableLiquidity == type(uint256).max) return holderShares;
        return Math.min(holderShares, _convertToShares(availableLiquidity, Math.Rounding.Floor));
    }

    function maxWithdraw(address holder) public view override returns (uint256) {
        return _convertToAssets(maxRedeem(holder), Math.Rounding.Floor);
    }

    function _deposit(address caller, address receiver, uint256 assets, uint256 shares) internal override {
        if (reservePaused) revert ReservePaused();
        if (reserveFrozen) revert ReserveFrozen();
        if (shares == 0) revert ZeroShares();
        super._deposit(caller, receiver, assets, shares);
        if (supplyCapRemaining != type(uint256).max) supplyCapRemaining -= assets;
    }

    function _withdraw(address caller, address receiver, address holder, uint256 assets, uint256 shares)
        internal
        override
    {
        if (reservePaused) revert ReservePaused();
        if (assets == 0 || shares == 0) revert ZeroAmount();
        super._withdraw(caller, receiver, holder, assets, shares);
    }

    /// @dev Every share transfer, mint and burn goes through here, so this is where StataTokenV2's own
    ///      pause bites.
    function _update(address from, address to, uint256 value) internal override {
        if (tokenPaused) revert TokenPaused();
        super._update(from, to, value);
    }

    /// @dev Incentives credited to each holder, per reward token. Like StataTokenV2, only the holder can claim them.
    mapping(address holder => mapping(address reward => uint256 amount)) public claimableRewards;

    function creditRewards(address holder, address reward, uint256 amount) external onlyOwner {
        claimableRewards[holder][reward] += amount;
    }

    /// @dev Pays the caller's credited rewards to `receiver`. The reward tokens are minted, as the mock holds none.
    function claimRewards(address receiver, address[] calldata rewards) external {
        for (uint256 rewardIndex = 0; rewardIndex < rewards.length; rewardIndex++) {
            uint256 amount = claimableRewards[msg.sender][rewards[rewardIndex]];
            claimableRewards[msg.sender][rewards[rewardIndex]] = 0;
            MockUSDC(rewards[rewardIndex]).mint(receiver, amount);
        }
    }
}

/// @dev Takes inner-vault shares and gives them back, to show that a batch may move shares within one transaction.
contract MockShareBorrower {
    function giveBack(address token, address to, uint256 amount) external {
        require(IERC20(token).transfer(to, amount), "transfer failed");
    }
}

/// @notice Tests for {YieldCaptureVault} against a mock of Aave's stataUSDC. They cover:
///          - shares that are exactly 1:1 with the asset, and principal to depositors with all yield to the owner
///          - the tiny-first-deposit ("dust") attack: a 2-wei first deposit leaves the inner vault 1 wei short
///          - the unharvested yield as a first-loss buffer, large losses, pro-rata exits and recovery
///          - deposits closed while depositors are short, and a total wipeout
///          - donations, rounding dust from other depositors, and `harvest` leaving depositors untouched
///          - ERC-4626 preview parity, inner-vault liquidity and supply limits, freezes and the inner vault's pauses
///          - the owner's pause of deposits, withdrawals and transfers, and the disabled `renounceOwnership`
///          - the owner's guarded `execute`: claiming rewards works, moving the inner-vault shares does not
///          - the admin surface and constructor validation
contract YieldCaptureVaultTest is Test {
    MockUSDC internal usdc;
    MockStataVault internal innerVault;
    YieldCaptureVault internal vault;

    address internal owner = makeAddr("owner");
    address internal alice = makeAddr("alice");
    address internal bob = makeAddr("bob");
    address internal carol = makeAddr("carol");
    address internal attacker = makeAddr("attacker");

    uint256 internal constant ONE_USDC = 1e6;

    /// @dev Denominator for `_scaleInnerVaultRate`: 10_000 leaves the rate unchanged.
    uint256 internal constant RATE_SCALE_DENOMINATOR = 10_000;

    /// @dev The most value one deposit or one withdrawal can lose to the inner vault's rounding. A deposit
    ///      is credited `floor(assets * RAY / rate)` shares, worth less than `rate / RAY + 1` wei under
    ///      `assets`. A withdrawal burns `ceil(assets * RAY / rate)` shares, which costs the vault less than
    ///      `rate / RAY + 1` wei more than `assets`. While the rate is below 2.0 both are at most 2 wei.
    uint256 internal constant MAX_INNER_DUST_PER_ACTION = 2;

    /// @dev Rounding budget for valuing one position: the inner vault's credit loss on a deposit (up to
    ///      2 wei) plus one floor in a conversion.
    uint256 internal constant ROUNDING_TOLERANCE_PER_DEPOSITOR = 3;

    /// @dev Rounding budget for a whole deposit, withdrawal and harvest of one depositor: the credit loss
    ///      (up to 2 wei), the withdrawal burn (up to 2 wei) and the final floor in `harvest` (1 wei).
    uint256 internal constant ROUNDING_TOLERANCE_PER_ROUND_TRIP = 5;

    function setUp() public {
        vm.warp(1_700_000_000);

        usdc = new MockUSDC();
        innerVault = new MockStataVault(usdc);
        vault = _deployVault();

        address[4] memory depositors = [alice, bob, carol, attacker];
        for (uint256 depositorIndex = 0; depositorIndex < depositors.length; depositorIndex++) {
            usdc.mint(depositors[depositorIndex], 100_000_000 * ONE_USDC);
            vm.prank(depositors[depositorIndex]);
            usdc.approve(address(vault), type(uint256).max);
        }
    }

    // --------------------------------------------------------------------------------------
    // helpers
    // --------------------------------------------------------------------------------------

    function _deployVault() internal returns (YieldCaptureVault) {
        return new YieldCaptureVault(address(innerVault), "Yield Capture USDC", "ycUSDC", owner);
    }

    /// @dev Checked after every helper action. Three checks guard the design, for the four tracked holders:
    ///      - no holder is ever worth more than par: `previewRedeem(balance) <= balance`;
    ///      - every claim is payable: the holders' redeemable values add up to no more than the inner vault's
    ///        value;
    ///      - whenever depositors are whole, `convertToAssets(totalSupply())` is exactly `totalSupply()`:
    ///        the conversions must be 1:1 while depositors are whole.
    ///      Two more are sanity checks only, because they follow algebraically from how `totalAssets` and
    ///      `unharvestedYield` are written (`min` and the part above par), so they would pass against any
    ///      implementation of them: `totalAssets() <= totalSupply()`, and depositors plus unharvested yield adding
    ///      up to
    ///      the inner vault's value.
    function _assertInvariants() internal view {
        address[4] memory holders = [alice, bob, carol, attacker];
        uint256 totalRedeemableValue;
        for (uint256 holderIndex = 0; holderIndex < holders.length; holderIndex++) {
            uint256 holderShares = vault.balanceOf(holders[holderIndex]);
            uint256 redeemableValue = vault.previewRedeem(holderShares);
            assertLe(redeemableValue, holderShares, "a holder is worth more than par");
            totalRedeemableValue += redeemableValue;
        }
        assertLe(totalRedeemableValue, vault.innerVaultValue(), "claims add up to more than the vault holds");

        uint256 totalShares = vault.totalSupply();
        if (vault.depositorShortfall() == 0) {
            assertEq(vault.convertToAssets(totalShares), totalShares, "whole depositors must redeem 1:1");
        }

        // Sanity checks (algebraic).
        assertLe(vault.totalAssets(), totalShares, "sanity: depositors are worth more than par");
        assertEq(
            vault.totalAssets() + vault.unharvestedYield(),
            vault.innerVaultValue(),
            "sanity: depositors plus unharvested yield do not add up to the inner vault value"
        );
    }

    /// @dev A deposit must mint exactly one share per asset unit, whatever state the vault is in.
    function _depositAs(address depositor, uint256 assets) internal returns (uint256 shares) {
        uint256 sharesBefore = vault.balanceOf(depositor);
        vm.prank(depositor);
        shares = vault.deposit(assets, depositor);
        assertEq(shares, assets, "a deposit must mint exactly one share per asset unit");
        assertEq(vault.balanceOf(depositor) - sharesBefore, assets, "the balance moved by a different amount");
        _assertInvariants();
    }

    function _redeemAllAs(address holder) internal returns (uint256 assetsReceived) {
        uint256 shares = vault.balanceOf(holder);
        vm.prank(holder);
        assetsReceived = vault.redeem(shares, holder, holder);
        _assertInvariants();
    }

    /// @dev The owner harvests, and the guarantees of a harvest are checked every time:
    ///      - no holder's redeemable value moves, to the wei, whether depositors are whole or short;
    ///      - if depositors were whole, the vault still holds at least `totalSupply()` in the inner vault,
    ///        which is what lets every holder exit in full;
    ///      - the owner is paid exactly what `harvest` returns and never more than the yield it had.
    function _harvestAsOwner() internal returns (uint256 assetsSentToOwner) {
        address[4] memory holders = [alice, bob, carol, attacker];
        uint256[4] memory redeemableValuesBefore;
        for (uint256 holderIndex = 0; holderIndex < holders.length; holderIndex++) {
            redeemableValuesBefore[holderIndex] = vault.previewRedeem(vault.balanceOf(holders[holderIndex]));
        }
        bool depositorsWereWhole = vault.depositorShortfall() == 0;
        uint256 unharvestedYieldBefore = vault.unharvestedYield();
        uint256 ownerBalanceBefore = usdc.balanceOf(owner);

        vm.prank(owner);
        assetsSentToOwner = vault.harvest();

        for (uint256 holderIndex = 0; holderIndex < holders.length; holderIndex++) {
            assertEq(
                vault.previewRedeem(vault.balanceOf(holders[holderIndex])),
                redeemableValuesBefore[holderIndex],
                "harvest changed a holder's redeemable value"
            );
        }
        if (depositorsWereWhole) {
            assertGe(vault.innerVaultValue(), vault.totalSupply(), "harvest took the depositors' backing");
        }
        assertEq(usdc.balanceOf(owner) - ownerBalanceBefore, assetsSentToOwner, "owner was not paid");
        assertLe(assetsSentToOwner, unharvestedYieldBefore, "harvest paid more than the unharvested yield");
        _assertInvariants();
    }

    /// @dev Multiplies the inner vault's rate by `scaleInBps / 10_000`. 10_100 is a 1% gain, 9_500 a 5% loss.
    function _scaleInnerVaultRate(uint256 scaleInBps) internal {
        innerVault.setRate((innerVault.rate() * scaleInBps) / RATE_SCALE_DENOMINATOR);
        _assertInvariants();
    }

    /// @dev Sets the inner vault's rate so that everything the vault holds in it is worth exactly
    ///      `totalSupply() - targetShortfall`, which makes `depositorShortfall()` exactly `targetShortfall`.
    ///      The inner vault values `innerShares` at `floor(innerShares * rate / RAY)`. Taking the smallest
    ///      rate that reaches the target overshoots that value by less than `innerShares / RAY`, which is far
    ///      below one wei for any realistic share count, so the floor lands exactly on the target. Needs
    ///      `targetShortfall <= totalSupply()`.
    function _setInnerVaultRateForShortfall(uint256 targetShortfall) internal {
        uint256 innerShares = innerVault.balanceOf(address(vault));
        uint256 targetInnerVaultValue = vault.totalSupply() - targetShortfall;
        innerVault.setRate(Math.ceilDiv(targetInnerVaultValue * innerVault.RAY(), innerShares));
        assertEq(vault.depositorShortfall(), targetShortfall, "could not construct the shortfall");
        _assertInvariants();
    }

    /// @dev The attacker buys inner shares with `assets` and sends them to the vault.
    function _donateInnerSharesWorth(uint256 assets) internal returns (uint256 donatedInnerShares) {
        vm.startPrank(attacker);
        usdc.approve(address(innerVault), type(uint256).max);
        donatedInnerShares = innerVault.deposit(assets, attacker);
        assertTrue(innerVault.transfer(address(vault), donatedInnerShares), "donation transfer failed");
        vm.stopPrank();
        _assertInvariants();
    }

    /// @dev The ERC-7821 batch mode: batch call type, default exec type, no selector, no payload.
    function _batchMode() internal pure returns (bytes32) {
        return Mode.unwrap(
            ERC7579Utils.encodeMode(
                ERC7579Utils.CALLTYPE_BATCH, ERC7579Utils.EXECTYPE_DEFAULT, ModeSelector.wrap(0), ModePayload.wrap(0)
            )
        );
    }

    function _call(address target, bytes memory callData) internal pure returns (Execution memory) {
        return Execution({target: target, value: 0, callData: callData});
    }

    function _batchOf(Execution memory firstCall) internal pure returns (Execution[] memory batch) {
        batch = new Execution[](1);
        batch[0] = firstCall;
    }

    function _batchOf(Execution memory firstCall, Execution memory secondCall)
        internal
        pure
        returns (Execution[] memory batch)
    {
        batch = new Execution[](2);
        batch[0] = firstCall;
        batch[1] = secondCall;
    }

    function _executeAsOwner(Execution[] memory batch) internal {
        bytes memory executionData = ERC7579Utils.encodeBatch(batch);
        vm.prank(owner);
        vault.execute(_batchMode(), executionData);
        _assertInvariants();
    }

    /// @dev Runs a batch that must be refused with `expectedError`, and checks that nothing moved: the vault and
    ///      the owner hold the same inner-vault shares and the owner the same USDC.
    function _assertGuardRefuses(Execution[] memory batch, bytes memory expectedError) internal {
        uint256 vaultInnerSharesBefore = innerVault.balanceOf(address(vault));
        uint256 ownerInnerSharesBefore = innerVault.balanceOf(owner);
        uint256 ownerUsdcBefore = usdc.balanceOf(owner);
        bytes memory executionData = ERC7579Utils.encodeBatch(batch);

        vm.prank(owner);
        vm.expectRevert(expectedError);
        vault.execute(_batchMode(), executionData);

        assertEq(innerVault.balanceOf(address(vault)), vaultInnerSharesBefore, "the vault's inner shares moved");
        assertEq(innerVault.balanceOf(owner), ownerInnerSharesBefore, "the owner got inner shares");
        assertEq(usdc.balanceOf(owner), ownerUsdcBefore, "the owner got USDC");
    }

    // --------------------------------------------------------------------------------------
    // shares are 1:1 with the asset
    // --------------------------------------------------------------------------------------

    /// @notice Shares have the asset's decimals, deposits of odd sizes mint exactly that many shares, and while
    ///         depositors are whole every conversion and preview is the identity.
    function test_Shares_areOneToOneWithTheAsset() public {
        assertEq(vault.decimals(), usdc.decimals(), "shares should have the asset's decimals");
        assertEq(vault.decimals(), 6, "USDC has 6 decimals");

        uint256[5] memory oddDepositAmounts = [uint256(3), 1_000_001, 123_456_789, 7_777_777_777, 987_654_321_012];
        address[2] memory depositors = [alice, bob];
        uint256 totalDeposited;
        for (uint256 depositIndex = 0; depositIndex < oddDepositAmounts.length; depositIndex++) {
            address depositor = depositors[depositIndex % 2];
            uint256 balanceBefore = vault.balanceOf(depositor);
            _depositAs(depositor, oddDepositAmounts[depositIndex]);
            assertEq(vault.balanceOf(depositor) - balanceBefore, oddDepositAmounts[depositIndex], "odd deposit");
            totalDeposited += oddDepositAmounts[depositIndex];
        }
        assertEq(vault.totalSupply(), totalDeposited, "supply is the sum of the deposits");

        _scaleInnerVaultRate(10_100); // +1%: more than the dust, so depositors are whole
        assertEq(vault.depositorShortfall(), 0, "depositors should be whole");

        uint256[4] memory amountsToConvert = [uint256(1), 999_999, 123_456_789, totalDeposited];
        for (uint256 amountIndex = 0; amountIndex < amountsToConvert.length; amountIndex++) {
            uint256 amount = amountsToConvert[amountIndex];
            assertEq(vault.convertToAssets(amount), amount, "convertToAssets");
            assertEq(vault.convertToShares(amount), amount, "convertToShares");
            assertEq(vault.previewRedeem(amount), amount, "previewRedeem");
            assertEq(vault.previewWithdraw(amount), amount, "previewWithdraw");
            assertEq(vault.previewDeposit(amount), amount, "previewDeposit");
            assertEq(vault.previewMint(amount), amount, "previewMint");
        }
    }

    /// @notice `mint(x)` pulls exactly `x` of the asset and mints exactly `x` shares, even when depositors
    ///         are a dust short.
    function test_Mint_pullsExactlyTheSharesRequested() public {
        _depositAs(bob, 1_000 * ONE_USDC);
        assertGt(vault.depositorShortfall(), 0, "setup: the vault should be a dust short");

        uint256 sharesToMint = 777_777_777;
        uint256 aliceUsdcBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        uint256 assetsPulled = vault.mint(sharesToMint, alice);
        _assertInvariants();

        assertEq(assetsPulled, sharesToMint, "mint should return the shares");
        assertEq(aliceUsdcBefore - usdc.balanceOf(alice), sharesToMint, "mint pulled a different amount");
        assertEq(vault.balanceOf(alice), sharesToMint, "mint minted a different amount");
    }

    /// @notice A deposit made while depositors are a dust short still mints at par, so nobody can buy below
    ///         par. The new holder takes their share of the dust as a paper loss of at most a wei.
    function test_Deposit_mintsAtParEvenDuringADustShortfall() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 shortfallBeforeBobDeposit = vault.depositorShortfall();
        assertGt(shortfallBeforeBobDeposit, 0, "setup: the inner vault's rounding should leave a dust shortfall");

        uint256 bobDepositAmount = 500 * ONE_USDC + 12_345;
        uint256 bobShares = _depositAs(bob, bobDepositAmount);
        assertEq(bobShares, bobDepositAmount, "bob must get exactly one share per asset unit");

        // Bob's deposit adds at most one action's worth of dust to the shortfall that was already there.
        assertLe(
            vault.depositorShortfall(),
            shortfallBeforeBobDeposit + MAX_INNER_DUST_PER_ACTION,
            "more than the dust is missing"
        );
        // The "under a wei" below is specific to this test's rate and amounts, not a general bound: Alice's
        // deposit loses 1 wei here, so the shortfall is at most 3 wei, shared by 1.5 times as many shares as
        // Alice alone had, and Bob holds one third of them: 1 wei at most. With the worst-case 2 wei per deposit
        // the shortfall could reach 4 wei and his loss 2.
        assertApproxEqAbs(vault.previewRedeem(bobShares), bobDepositAmount, 1, "bob's paper loss is under a wei");
    }

    // --------------------------------------------------------------------------------------
    // yield goes to the owner
    // --------------------------------------------------------------------------------------

    /// @notice A depositor gets exactly their principal back and the owner gets the whole 1% gain.
    function test_OwnerTakesAllYield_depositorGetsPrincipalOwnerGetsYield() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        assertEq(_depositAs(alice, aliceDepositAmount), aliceDepositAmount, "1,000 USDC buys 1,000 shares");

        _scaleInnerVaultRate(10_100); // +1% -> about 10 USDC of yield

        // The inner vault credited Alice's deposit with up to 2 wei less than she paid, and that came out
        // of the unharvested yield rather than out of Alice.
        assertApproxEqAbs(
            vault.unharvestedYield(),
            10 * ONE_USDC,
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "unharvested yield before the exit"
        );

        assertEq(_redeemAllAs(alice), aliceDepositAmount, "alice should get exactly her principal");
        assertEq(vault.totalSupply(), 0, "nobody is owed anything after the exit");

        // Everything left in the inner vault is the owner's, and `harvest` pays out exactly what the
        // inner vault gives for that yield.
        uint256 ownerBalanceBefore = usdc.balanceOf(owner);
        uint256 expectedInnerShares = innerVault.convertToShares(vault.unharvestedYield());
        uint256 expectedAssetsHarvested = innerVault.previewRedeem(expectedInnerShares);
        uint256 assetsHarvested = _harvestAsOwner();

        assertEq(usdc.balanceOf(owner) - ownerBalanceBefore, assetsHarvested, "owner was not paid");
        assertEq(assetsHarvested, expectedAssetsHarvested, "harvest differs from the unharvested yield");
        assertApproxEqAbs(
            assetsHarvested, 10 * ONE_USDC, ROUNDING_TOLERANCE_PER_ROUND_TRIP, "owner should get the whole gain"
        );
    }

    /// @notice Depositors who join at different times and rates each get back exactly their own principal,
    ///         and the owner gets the yield each deposit earned from its own entry rate.
    function test_OwnerTakesAllYield_severalDepositorsEachGetTheirOwnPrincipal() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        uint256 bobDepositAmount = 2_000 * ONE_USDC;
        uint256 carolDepositAmount = 500 * ONE_USDC;

        uint256 rateWhenAliceDeposited = innerVault.rate();
        _depositAs(alice, aliceDepositAmount);

        _scaleInnerVaultRate(10_100); // +1%
        uint256 rateWhenBobDeposited = innerVault.rate();
        _depositAs(bob, bobDepositAmount);

        _scaleInnerVaultRate(10_200); // +2%
        uint256 rateWhenCarolDeposited = innerVault.rate();
        _depositAs(carol, carolDepositAmount);

        _scaleInnerVaultRate(10_300); // +3%
        uint256 finalRate = innerVault.rate();

        assertEq(_redeemAllAs(alice), aliceDepositAmount, "alice should get exactly her principal");
        assertEq(_redeemAllAs(bob), bobDepositAmount, "bob should get exactly his principal");
        assertEq(_redeemAllAs(carol), carolDepositAmount, "carol should get exactly her principal");
        assertEq(vault.totalSupply(), 0, "everyone has exited");

        // Each deposit grew by finalRate / entryRate, so that is the yield the owner should collect.
        uint256 expectedUnharvestedYield = (aliceDepositAmount * finalRate) / rateWhenAliceDeposited
            - aliceDepositAmount + (bobDepositAmount * finalRate) / rateWhenBobDeposited - bobDepositAmount
            + (carolDepositAmount * finalRate) / rateWhenCarolDeposited - carolDepositAmount;
        assertApproxEqAbs(
            _harvestAsOwner(),
            expectedUnharvestedYield,
            3 * ROUNDING_TOLERANCE_PER_ROUND_TRIP,
            "unharvested yield across three depositors"
        );
    }

    // --------------------------------------------------------------------------------------
    // dust attack regression
    // --------------------------------------------------------------------------------------

    /// @notice An attacker makes a 2-wei first deposit, which leaves the inner vault 1 wei short, then a large
    ///         depositor joins and the rate rises 50%. This checks that nobody can use that rounding to take
    ///         value from later depositors: with shares pinned at par the price cannot be moved by it, so Bob
    ///         gets exactly his principal and the attacker nothing beyond the 2 wei they put in.
    function test_DustAttack_largeDepositorGetsExactlyHisPrincipalAndTheAttackerNothingExtra() public {
        uint256 attackerDepositAmount = 2;
        uint256 bobDepositAmount = 1_000_000 * ONE_USDC;

        assertEq(_depositAs(attacker, attackerDepositAmount), 2, "the attacker gets 2 shares");
        assertEq(vault.depositorShortfall(), 1, "the 2 wei buy 1 inner share worth 1 wei");
        // The vault is a dust short, which is within the allowance, so Bob can still enter, at par.
        uint256 bobShares = _depositAs(bob, bobDepositAmount);
        assertEq(bobShares, bobDepositAmount, "bob must get exactly 1,000,000e6 shares");

        _scaleInnerVaultRate(15_000); // +50%

        assertEq(vault.depositorShortfall(), 0, "the gain covers the dust");
        assertEq(vault.previewRedeem(bobShares), bobDepositAmount, "bob is worth exactly his principal");

        // The unharvested yield is the 50% gain on both deposits (the attacker's 2 wei included). The credit
        // loss of the two deposits (1 + 2 wei) grows by the 1.5x rate rise to at most 4.5 wei, and valuing
        // the shares floors once more.
        uint256 expectedUnharvestedYield = (bobDepositAmount + attackerDepositAmount) / 2;
        assertApproxEqAbs(
            vault.unharvestedYield(),
            expectedUnharvestedYield,
            2 * ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "unharvested yield"
        );

        // The attacker gets back exactly the 2 wei they put in, and Bob exactly his principal.
        assertEq(_redeemAllAs(attacker), attackerDepositAmount, "the attacker should extract nothing extra");
        assertEq(_redeemAllAs(bob), bobDepositAmount, "bob should get exactly his principal");
    }

    // --------------------------------------------------------------------------------------
    // losses and recovery
    // --------------------------------------------------------------------------------------

    /// @notice The unharvested yield is a first-loss buffer: a loss smaller than it leaves
    ///         depositors untouched and reduces the unharvested yield by the loss.
    function test_SmallLoss_isAbsorbedByUnharvestedYield() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000); // +10%: about 100 USDC of unharvested yield

        uint256 aliceShares = vault.balanceOf(alice);
        uint256 oneShareAssetsBefore = vault.convertToAssets(ONE_USDC);
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        uint256 unharvestedYieldBefore = vault.unharvestedYield();
        uint256 innerVaultValueBefore = vault.innerVaultValue();

        _scaleInnerVaultRate(9_500); // -5% of about 1,100 USDC: about 55 USDC, less than the 100 of yield

        assertEq(vault.convertToAssets(ONE_USDC), oneShareAssetsBefore, "share price moved");
        assertEq(oneShareAssetsBefore, ONE_USDC, "one share is one asset unit");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value moved");
        assertEq(vault.depositorShortfall(), 0, "depositors should still be whole");

        // The unharvested yield fell by the loss. The loss is 5% of the inner vault's value, rebuilt here
        // with plain arithmetic; the inner vault floors its value before and after (1 wei each).
        uint256 expectedLoss = (innerVaultValueBefore * 500) / RATE_SCALE_DENOMINATOR;
        assertApproxEqAbs(
            vault.unharvestedYield(), unharvestedYieldBefore - expectedLoss, 2, "unharvested yield absorbs the loss"
        );
    }

    /// @notice A loss larger than the unharvested yield takes it to zero and pushes depositors below par,
    ///         pro-rata. Holders exit pro-rata without changing the ratio for those who stay, and a recovery
    ///         refills depositors to par before the owner earns again.
    function test_LargeLoss_depositorsLoseProRataAndRecoveryRefillsThemFirst() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        uint256 bobDepositAmount = 3_000 * ONE_USDC;
        _depositAs(alice, aliceDepositAmount);
        _depositAs(bob, bobDepositAmount);

        _scaleInnerVaultRate(11_000); // +10%: about 400 USDC of unharvested yield
        assertApproxEqAbs(vault.unharvestedYield(), 400 * ONE_USDC, 2 * ROUNDING_TOLERANCE_PER_DEPOSITOR, "setup");

        // -20% of about 4,400 USDC: 880 USDC, more than the 400 of yield. The inner vault is worth about 3,520.
        _scaleInnerVaultRate(8_000);
        assertEq(vault.unharvestedYield(), 0, "the buffer should be gone");
        // The credit loss of the two deposits (up to 4 wei) shrinks with the net 0.88x rate move, and the
        // inner vault floors its value once.
        assertApproxEqAbs(
            vault.depositorShortfall(),
            480 * ONE_USDC,
            2 * ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "depositors are about 480 USDC short"
        );
        assertEq(vault.maxDeposit(carol), 0, "deposits should be closed");

        // Both lost the same fraction: Alice holds a quarter of the shares and is worth a quarter of what is left.
        uint256 innerVaultValueAfterLoss = vault.innerVaultValue();
        uint256 aliceValueAfterLoss = vault.previewRedeem(vault.balanceOf(alice));
        uint256 bobValueAfterLoss = vault.previewRedeem(vault.balanceOf(bob));
        assertApproxEqAbs(aliceValueAfterLoss, innerVaultValueAfterLoss / 4, ROUNDING_TOLERANCE_PER_DEPOSITOR, "alice");
        assertApproxEqAbs(
            bobValueAfterLoss, (innerVaultValueAfterLoss * 3) / 4, ROUNDING_TOLERANCE_PER_DEPOSITOR, "bob"
        );

        // With no yield, a harvest pays nothing and takes nothing from depositors.
        assertEq(_harvestAsOwner(), 0, "harvest should pay nothing during a shortfall");

        // Alice leaves with half her shares during the shortfall. She is paid her pro-rata value.
        uint256 sharesToRedeem = vault.balanceOf(alice) / 2;
        uint256 previewedAssets = vault.previewRedeem(sharesToRedeem);
        uint256 shortfallBeforeExit = vault.depositorShortfall();
        uint256 totalSupplyBeforeExit = vault.totalSupply();
        uint256 bobValueBeforeExit = vault.previewRedeem(vault.balanceOf(bob));
        vm.prank(alice);
        assertEq(vault.redeem(sharesToRedeem, alice, alice), previewedAssets, "alice should get her previewed value");
        _assertInvariants();

        // For the holders who stay, nothing changes beyond the dust of her exit. Bob's value moves by under 3
        // wei: the inner vault burns up to 2 wei more than Alice was paid, and Bob's own floor adds 1.
        assertApproxEqAbs(
            vault.previewRedeem(vault.balanceOf(bob)),
            bobValueBeforeExit,
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "bob's value after alice's exit"
        );
        // The shortfall shrinks in proportion to the shares that left, within the same budget: the dust (up to
        // 2 wei) and the floor in Alice's payout (under 1 wei).
        assertApproxEqAbs(
            vault.depositorShortfall(),
            (shortfallBeforeExit * vault.totalSupply()) / totalSupplyBeforeExit,
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "the shortfall ratio for those who stay"
        );
        assertEq(vault.maxDeposit(carol), 0, "still closed while Bob and Alice's half are short");
        uint256 innerVaultValueAfterExit = vault.innerVaultValue();

        // +10%: holders are refilled but still below par, so the owner earns nothing yet.
        _scaleInnerVaultRate(11_000);
        assertEq(vault.unharvestedYield(), 0, "the owner should earn nothing before depositors are whole");
        assertGt(vault.depositorShortfall(), 0, "holders are still short");

        // +20%: par is reached and the rest is the owner's.
        _scaleInnerVaultRate(12_000);
        assertEq(vault.depositorShortfall(), 0, "depositors are whole again");
        assertEq(vault.maxDeposit(carol), type(uint256).max, "deposits should reopen");
        assertEq(vault.previewRedeem(vault.balanceOf(bob)), vault.balanceOf(bob), "bob is back at par, not above it");
        // The inner vault is worth what it was after Alice's exit, times 1.1 * 1.2 = 1.32. Rebuilt from the
        // value after the exit, the first floor in that value is multiplied by 1.32 (under 1.32 wei) and the
        // inner vault floors once more (under 1 wei): under 2.32 wei, so 3 covers it.
        assertApproxEqAbs(
            vault.unharvestedYield(),
            (innerVaultValueAfterExit * 13_200) / RATE_SCALE_DENOMINATOR - vault.totalSupply(),
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "the owner earns again above par"
        );
    }

    // --------------------------------------------------------------------------------------
    // deposits are closed while depositors are short
    // --------------------------------------------------------------------------------------

    /// @notice Regression for the entry-price exploit. Alice deposits 1,000 and the inner vault falls 20%.
    ///         Without a gate Bob could deposit 800 at the depressed price, get as many shares as Alice, and
    ///         take 100 of her principal when the inner vault recovers. The vault is closed instead, while
    ///         Alice can still exit with her pro-rata share.
    function test_Deposit_isClosedDuringAShortfall_whileHoldersCanStillExit() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        uint256 aliceShares = _depositAs(alice, aliceDepositAmount);

        _scaleInnerVaultRate(8_000); // -20%: depositors are about 200 USDC short
        assertApproxEqAbs(
            vault.depositorShortfall(), 200 * ONE_USDC, ROUNDING_TOLERANCE_PER_DEPOSITOR, "setup shortfall"
        );
        assertGt(vault.depositorShortfall(), vault.MAX_SHORTFALL_FOR_DEPOSITS(), "the shortfall is not above the gate");

        assertEq(vault.maxDeposit(bob), 0, "maxDeposit should be closed");
        assertEq(vault.maxMint(bob), 0, "maxMint should be closed");

        uint256 bobDepositAmount = 800 * ONE_USDC;
        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, bob, bobDepositAmount, 0));
        vault.deposit(bobDepositAmount, bob);

        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxMint.selector, bob, bobDepositAmount, 0));
        vault.mint(bobDepositAmount, bob);

        // Alice can still leave with her pro-rata share: half her shares are worth half of what is left.
        uint256 sharesToRedeem = aliceShares / 2;
        uint256 previewedAssets = vault.previewRedeem(sharesToRedeem);
        uint256 innerVaultValueBeforeExit = vault.innerVaultValue();
        vm.prank(alice);
        uint256 aliceAssetsReceived = vault.redeem(sharesToRedeem, alice, alice);
        _assertInvariants();
        assertEq(aliceAssetsReceived, previewedAssets, "redeem differs from the preview");
        assertApproxEqAbs(
            aliceAssetsReceived,
            innerVaultValueBeforeExit / 2,
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "alice should get half of what is left"
        );

        assertEq(vault.maxDeposit(bob), 0, "still closed: Alice's half is still short");
    }

    /// @notice Once the inner vault recovers past par, deposits reopen, and Alice, who did not exit, is whole:
    ///         Bob cannot have taken any of her principal.
    function test_Deposit_reopensAfterRecovery_andTheHolderWhoStayedIsWhole() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        uint256 bobDepositAmount = 800 * ONE_USDC;
        uint256 aliceShares = _depositAs(alice, aliceDepositAmount);

        _scaleInnerVaultRate(8_000); // -20%
        assertEq(vault.maxDeposit(bob), 0, "closed during the shortfall");

        _scaleInnerVaultRate(15_000); // x1.5: the vault is worth 1.2x the deposit, 200 above par
        assertEq(vault.depositorShortfall(), 0, "depositors should be whole");
        assertEq(vault.maxDeposit(bob), type(uint256).max, "reopened");
        assertEq(vault.previewRedeem(aliceShares), aliceDepositAmount, "alice should be exactly whole");

        uint256 bobShares = _depositAs(bob, bobDepositAmount);

        // Both are exactly worth their deposits, and everything above par is the owner's.
        assertEq(vault.previewRedeem(aliceShares), aliceDepositAmount, "alice lost principal to bob");
        assertEq(vault.previewRedeem(bobShares), bobDepositAmount, "bob's value");
        // Alice's credit loss (up to 2 wei) grows by 1.2x and Bob's adds up to 2 wei, then one floor.
        assertApproxEqAbs(vault.unharvestedYield(), 200 * ONE_USDC, 2 * ROUNDING_TOLERANCE_PER_DEPOSITOR, "owner");
    }

    /// @notice The dust the inner vault's rounding leaves behind does not close the vault: right after a
    ///         deposit with no yield, depositors are 1 to 2 wei short, and deposits and mints still work.
    function test_Deposit_staysOpenForADustShortfall() public {
        _depositAs(alice, 1_000 * ONE_USDC);

        uint256 shortfall = vault.depositorShortfall();
        assertGt(shortfall, 0, "setup: the inner vault's rounding should leave a dust shortfall");
        assertLe(shortfall, MAX_INNER_DUST_PER_ACTION, "dust is at most 2 wei per action");

        assertEq(vault.maxDeposit(bob), type(uint256).max, "maxDeposit is unchanged");
        assertEq(vault.maxMint(bob), type(uint256).max, "maxMint is unchanged");
        _depositAs(bob, 500 * ONE_USDC);
        vm.prank(carol);
        assertEq(vault.mint(100 * ONE_USDC, carol), 100 * ONE_USDC, "the mint should work, at par");
    }

    /// @notice The gate is exactly `MAX_SHORTFALL_FOR_DEPOSITS`: a shortfall of that many wei, or one wei
    ///         less, leaves deposits open, and one wei more closes them.
    /// @dev The shortfalls are built with `_setInnerVaultRateForShortfall`, which picks the inner vault's rate so
    ///      the vault's holdings are worth exactly `totalSupply() - shortfall`. The mock's rate has 27 decimals and
    ///      the vault holds fewer than 1e9 inner shares, so one step of the rate moves the holdings' value by far
    ///      less than a wei and the floor lands exactly. The helper asserts the shortfall it produced.
    function test_Deposit_gateIsExactlyAtMaxShortfall() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 maxShortfall = vault.MAX_SHORTFALL_FOR_DEPOSITS();
        uint256 bobDepositAmount = 100 * ONE_USDC;

        _setInnerVaultRateForShortfall(maxShortfall + 1);
        assertEq(vault.maxDeposit(bob), 0, "one wei above the gate should be closed");
        assertEq(vault.maxMint(bob), 0, "one wei above the gate should be closed for mints");
        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, bob, bobDepositAmount, 0));
        vault.deposit(bobDepositAmount, bob);

        _setInnerVaultRateForShortfall(maxShortfall - 1);
        assertEq(vault.maxDeposit(bob), type(uint256).max, "one wei below the gate should be open");

        _setInnerVaultRateForShortfall(maxShortfall);
        assertEq(vault.maxDeposit(bob), type(uint256).max, "exactly at the gate should be open");
        assertEq(vault.maxMint(bob), type(uint256).max, "exactly at the gate should be open for mints");
        assertEq(_depositAs(bob, bobDepositAmount), bobDepositAmount, "a deposit exactly at the gate should work");
    }

    /// @notice A deposit exactly at the allowance still mints at par, and an immediate full redeem returns the
    ///         deposit minus only Bob's share of the shortfall, which is far below the allowance.
    function test_Deposit_exactlyAtTheAllowance_thenAnImmediateRedeemLosesOnlyItsShareOfTheShortfall() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 maxShortfall = vault.MAX_SHORTFALL_FOR_DEPOSITS();
        _setInnerVaultRateForShortfall(maxShortfall);
        assertEq(vault.maxDeposit(bob), type(uint256).max, "exactly at the allowance is open");

        // What the inner vault credits Bob's deposit, read from the mock alone: whatever it loses is the dust
        // Bob adds to the shortfall. At this rate and amount it is exactly 1 wei.
        uint256 bobDepositAmount = 100 * ONE_USDC;
        uint256 bobInnerShares = innerVault.previewDeposit(bobDepositAmount);
        uint256 bobCreditLoss = bobDepositAmount - innerVault.previewRedeem(bobInnerShares);
        assertEq(bobCreditLoss, 1, "setup: the deposit should lose exactly 1 wei at this rate");

        uint256 bobShares = _depositAs(bob, bobDepositAmount);
        assertEq(vault.depositorShortfall(), maxShortfall + bobCreditLoss, "the shortfall grows by Bob's dust");

        // The outcome is deterministic: that wei pushes the shortfall to 1,001, one past the allowance, so the
        // vault closes to new deposits until yield refills it. The allowance is a hard line.
        assertEq(vault.maxDeposit(carol), 0, "the vault should close once Bob's wei pushes it past the allowance");
        assertEq(vault.maxMint(carol), 0, "and for mints");

        // Bob's immediate full redeem is never blocked. It pays his pro-rata value: below his deposit by his
        // share of the shortfall (1,001 over a supply of 1.1e9 shares, for his 100e6 shares: 91 wei), and
        // nowhere near the whole allowance.
        assertEq(bobShares, bobDepositAmount, "bob held one share per asset unit");
        uint256 bobAssetsReceived = _redeemAllAs(bob);
        assertLt(bobAssetsReceived, bobDepositAmount, "bob should hold a paper loss");
        assertGe(bobAssetsReceived, bobDepositAmount - maxShortfall, "bob lost more than the allowance");
        assertEq(bobAssetsReceived, 99_999_909, "bob's pro-rata value");
    }

    /// @notice Many small redeems during a shortfall. A one-share redeem pays nothing and forfeits the share. Each
    ///         later exit is paid at most its pro-rata value plus under a wei, and the holder who stays loses only
    ///         the dust of those exits.
    function test_RepeatedSmallRedeemsDuringAShortfall_payAtMostProRataAndTheHolderWhoStaysLosesOnlyDust() public {
        _depositAs(alice, 30_000 * ONE_USDC);
        _depositAs(bob, 20_000 * ONE_USDC);
        _scaleInnerVaultRate(8_000); // -20%: about 10,000 USDC short, so shares are worth about 0.8

        // One share is worth under one wei, so redeeming it pays nothing and the share is forfeited. The inner
        // call is skipped: the mock reverts on a zero-amount withdrawal, so success shows it.
        uint256 aliceSharesBefore = vault.balanceOf(alice);
        uint256 aliceUsdcBefore = usdc.balanceOf(alice);
        assertEq(vault.previewRedeem(1), 0, "one share is worth under a wei");
        vm.prank(alice);
        assertEq(vault.redeem(1, alice, alice), 0, "a one-share redeem pays nothing");
        assertEq(usdc.balanceOf(alice), aliceUsdcBefore, "alice was paid");
        assertEq(vault.balanceOf(alice), aliceSharesBefore - 1, "the share was not forfeited");
        _assertInvariants();

        uint256 exitCount = 20;
        uint256 sharesPerExit = 1_000 * ONE_USDC;
        uint256 bobValueBefore = vault.previewRedeem(vault.balanceOf(bob));
        for (uint256 exitIndex = 0; exitIndex < exitCount; exitIndex++) {
            // Pro-rata is `chunk * totalAssets / totalSupply`. OZ's `+ 1` terms make the payout
            // `chunk * (totalAssets + 1) / (totalSupply + 1)`, which exceeds pro-rata by
            // `chunk * (totalSupply - totalAssets) / (totalSupply * (totalSupply + 1))`: under 1 wei, as the
            // chunk is no more than the supply. So the payout is at most the floored pro-rata value plus 1.
            uint256 proRataPayoutCeiling = Math.mulDiv(sharesPerExit, vault.totalAssets(), vault.totalSupply()) + 1;
            vm.prank(alice);
            uint256 payout = vault.redeem(sharesPerExit, alice, alice);
            assertLe(payout, proRataPayoutCeiling, "an exit was paid more than pro-rata plus a wei");
            _assertInvariants();
        }

        // Each exit costs Bob under 3 wei: the leaver's excess from the `+ 1` terms is under 1 wei, and the
        // inner vault burns up to 2 wei more than the leaver was paid (a rounded-up share).
        assertGe(
            vault.previewRedeem(vault.balanceOf(bob)),
            bobValueBefore - exitCount * 3,
            "the holder who stays lost more than the dust of the exits"
        );
    }

    /// @notice Shares can be transferred during a shortfall. The two halves are together worth what the whole
    ///         was, give or take the wei lost to flooring twice, and each half redeems for exactly its preview.
    function test_ShareTransferDuringAShortfall_splitsTheValueAndBothHalvesRedeemAtTheirPreview() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(8_000); // -20%
        assertGt(vault.depositorShortfall(), 0, "setup: depositors should be short");
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);

        vm.prank(alice);
        assertTrue(vault.transfer(carol, aliceShares / 2), "share transfer failed");
        _assertInvariants();

        uint256 aliceHalfShares = vault.balanceOf(alice);
        uint256 carolHalfShares = vault.balanceOf(carol);
        uint256 aliceHalfValue = vault.previewRedeem(aliceHalfShares);
        uint256 carolHalfValue = vault.previewRedeem(carolHalfShares);
        assertEq(aliceHalfShares + carolHalfShares, aliceShares, "the transfer moved shares, not value");
        // Two floors instead of one lose up to a wei.
        assertApproxEqAbs(aliceHalfValue + carolHalfValue, aliceValueBefore, 1, "the halves add up to the whole");

        vm.prank(alice);
        assertEq(vault.redeem(aliceHalfShares, alice, alice), aliceHalfValue, "alice's half differs from its preview");
        _assertInvariants();

        // Alice's exit moves Carol's value by the dust of the exit, so her preview is taken again.
        uint256 carolPreviewAfterAliceExits = vault.previewRedeem(carolHalfShares);
        vm.prank(carol);
        assertEq(
            vault.redeem(carolHalfShares, carol, carol),
            carolPreviewAfterAliceExits,
            "carol's half differs from its preview"
        );
        _assertInvariants();
    }

    /// @notice A total wipeout: the inner vault is worth zero. A redeem pays nothing and skips the inner call,
    ///         and once the value returns the remaining holders are whole. Shares are pinned at par, so the
    ///         shares burned for nothing are forfeited to the owner, not to the holders who stay.
    /// @dev A rate of 1 (1e-27 assets per share) makes every inner share redeem for zero while keeping the
    ///      mock's divisions defined. Restoring the rate mints the mock its USDC back.
    function test_TotalWipeout_redeemPaysNothingAndRemainingHoldersRecover() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        uint256 rateBefore = innerVault.rate();

        innerVault.setRate(1);
        assertEq(vault.innerVaultValue(), 0, "setup: the vault should be worth zero");
        assertEq(vault.totalAssets(), 0, "setup: depositors should be worth zero");
        assertEq(vault.depositorShortfall(), aliceShares, "everything is missing");
        assertEq(vault.maxDeposit(bob), 0, "deposits are closed");

        // The inner call is skipped: the mock reverts on a zero-amount withdrawal, so success shows it.
        uint256 sharesToRedeem = aliceShares / 2;
        uint256 aliceBalanceBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        assertEq(vault.redeem(sharesToRedeem, alice, alice), 0, "alice should get nothing");
        assertEq(usdc.balanceOf(alice), aliceBalanceBefore, "alice was paid");
        uint256 remainingShares = vault.balanceOf(alice);
        assertEq(remainingShares, aliceShares - sharesToRedeem, "alice's shares were not burned");

        // Recovery: the remaining shares are whole again at par. The half burned for nothing is the
        // owner's now, less the dust of the original deposit.
        innerVault.setRate(rateBefore);
        _assertInvariants();
        assertEq(vault.depositorShortfall(), 0, "the remaining holders should be whole");
        assertEq(vault.previewRedeem(remainingShares), remainingShares, "they redeem 1:1");
        assertApproxEqAbs(vault.unharvestedYield(), sharesToRedeem, MAX_INNER_DUST_PER_ACTION, "the forfeited half");
        assertEq(vault.maxDeposit(bob), type(uint256).max, "deposits reopen");
    }

    // --------------------------------------------------------------------------------------
    // donations, other depositors' rounding, harvest
    // --------------------------------------------------------------------------------------

    /// @notice Inner shares sent to the vault become the unharvested yield and cannot move the share price.
    ///         The 1% gain first lifts depositors to par; a donation that arrives while they are below it
    ///         refills them first (see `test_DonationDuringShortfall_refillsDepositorsFirst`).
    function test_DonationOfInnerShares_goesToOwnerAndLeavesSharePriceUnchanged() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(10_100);
        uint256 aliceShares = vault.balanceOf(alice);
        uint256 oneShareAssetsBefore = vault.convertToAssets(ONE_USDC);
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        uint256 unharvestedYieldBefore = vault.unharvestedYield();

        // The attacker buys inner shares directly and sends them to the vault.
        uint256 donatedAssets = 100 * ONE_USDC;
        _donateInnerSharesWorth(donatedAssets);

        assertEq(vault.convertToAssets(ONE_USDC), oneShareAssetsBefore, "the donation moved the share price");
        assertEq(oneShareAssetsBefore, ONE_USDC, "one share is one asset unit");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "the donation moved alice's value");
        // The inner vault credited the donor's deposit with up to 2 wei less than it was worth.
        assertApproxEqAbs(
            vault.unharvestedYield() - unharvestedYieldBefore,
            donatedAssets,
            MAX_INNER_DUST_PER_ACTION,
            "the donation should become unharvested yield"
        );

        // The owner can collect it on top of its earlier yield (one floor in the harvest), and Alice
        // still gets exactly her principal.
        assertApproxEqAbs(
            _harvestAsOwner(),
            unharvestedYieldBefore + donatedAssets,
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "harvest did not collect the donation"
        );
        assertEq(_redeemAllAs(alice), 1_000 * ONE_USDC, "alice should get exactly her principal");
    }

    /// @notice A donation that arrives while depositors are below par refills them first, and only the part
    ///         beyond par becomes the unharvested yield.
    function test_DonationDuringShortfall_refillsDepositorsFirst() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(8_000); // -20%: about 200 USDC short, and no unharvested yield to absorb it
        uint256 aliceShares = vault.balanceOf(alice);
        uint256 shortfallBefore = vault.depositorShortfall();
        assertApproxEqAbs(shortfallBefore, 200 * ONE_USDC, ROUNDING_TOLERANCE_PER_DEPOSITOR, "setup shortfall");
        assertEq(vault.unharvestedYield(), 0, "no yield during a shortfall");

        // A donation worth about 50 USDC: all of it goes to Alice.
        uint256 smallDonationInnerShares = _donateInnerSharesWorth(50 * ONE_USDC);
        assertGt(smallDonationInnerShares, 0, "nothing was donated");
        assertEq(vault.unharvestedYield(), 0, "the owner should get nothing while depositors are short");
        assertApproxEqAbs(
            shortfallBefore - vault.depositorShortfall(),
            50 * ONE_USDC,
            MAX_INNER_DUST_PER_ACTION,
            "the donation should refill depositors"
        );
        assertApproxEqAbs(
            vault.previewRedeem(aliceShares),
            800 * ONE_USDC + 50 * ONE_USDC,
            ROUNDING_TOLERANCE_PER_DEPOSITOR,
            "alice's value rose by the donation"
        );

        // A donation worth about 300 USDC more: the first ~150 refills the rest of the shortfall, and
        // the remaining ~150 is the owner's.
        _donateInnerSharesWorth(300 * ONE_USDC);
        assertEq(vault.depositorShortfall(), 0, "depositors should be whole");
        assertApproxEqAbs(
            vault.unharvestedYield(), 150 * ONE_USDC, 2 * ROUNDING_TOLERANCE_PER_DEPOSITOR, "only the excess is yield"
        );
        assertEq(vault.previewRedeem(aliceShares), 1_000 * ONE_USDC, "alice is exactly at par");
    }

    /// @notice While there is unharvested yield, the inner vault's rounding on other depositors' round trips
    ///         comes out of that yield and never out of an existing depositor, and each round trip returns
    ///         exactly what went in.
    function test_RoundTripsByOthers_dustComesOutOfUnharvestedYield() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);

        uint256 aliceShares = vault.balanceOf(alice);
        uint256 oneShareAssetsBefore = vault.convertToAssets(ONE_USDC);
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        uint256 unharvestedYieldBefore = vault.unharvestedYield();

        // Odd amounts so the inner vault's rounding really happens.
        uint256 roundTripCount = 50;
        for (uint256 roundTripIndex = 0; roundTripIndex < roundTripCount; roundTripIndex++) {
            uint256 bobDepositAmount = 100 * ONE_USDC + 12_345 + roundTripIndex * 7;
            _depositAs(bob, bobDepositAmount);
            assertEq(_redeemAllAs(bob), bobDepositAmount, "a round trip must return exactly the deposit");
        }

        assertEq(vault.convertToAssets(ONE_USDC), oneShareAssetsBefore, "the share price moved");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value moved");

        // Each round trip is two inner-vault actions, each costing at most `MAX_INNER_DUST_PER_ACTION`.
        uint256 unharvestedYieldAfter = vault.unharvestedYield();
        assertLt(
            unharvestedYieldAfter, unharvestedYieldBefore, "the dust should have come out of the unharvested yield"
        );
        assertGe(
            unharvestedYieldAfter,
            unharvestedYieldBefore - roundTripCount * 2 * MAX_INNER_DUST_PER_ACTION,
            "more dust than the inner vault can lose"
        );
    }

    /// @notice A harvest only touches the unharvested yield: depositors are exactly unchanged, the vault
    ///         still holds at least `totalSupply()`, and only a wei or two of yield is left behind.
    function test_Harvest_leavesDepositorsUntouched() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _depositAs(bob, 3_000 * ONE_USDC);
        _scaleInnerVaultRate(10_700);

        uint256 aliceShares = vault.balanceOf(alice);
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        uint256 oneShareAssetsBefore = vault.convertToAssets(ONE_USDC);
        uint256 totalAssetsBefore = vault.totalAssets();
        uint256 shortfallBefore = vault.depositorShortfall();

        assertGt(_harvestAsOwner(), 0, "nothing was harvested");

        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value changed");
        assertEq(vault.convertToAssets(ONE_USDC), oneShareAssetsBefore, "share price changed");
        assertEq(vault.totalAssets(), totalAssetsBefore, "total assets changed");
        assertEq(vault.depositorShortfall(), shortfallBefore, "shortfall changed");
        assertGe(vault.innerVaultValue(), vault.totalSupply(), "harvest took depositors' backing");
        // The redeemed shares are `floor(yield / rate)`, so less than one share's worth of yield is left:
        // under `rate / RAY + 1` wei, which is at most 2 while the rate is below 2.0.
        assertLe(vault.unharvestedYield(), MAX_INNER_DUST_PER_ACTION, "a full harvest should leave a wei or two");
    }

    /// @notice `harvest` announces who was paid, with how many inner shares, and how much.
    function test_Harvest_emitsYieldHarvested() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);

        uint256 expectedInnerShares = innerVault.convertToShares(vault.unharvestedYield());
        uint256 expectedAssetsHarvested = innerVault.previewRedeem(expectedInnerShares);

        vm.expectEmit(true, false, false, true, address(vault));
        emit YieldCaptureVault.YieldHarvested(owner, expectedInnerShares, expectedAssetsHarvested);
        _harvestAsOwner();
    }

    /// @notice Only the owner can harvest, and the assets go to the owner.
    function test_Harvest_onlyOwner_andPaysTheOwner() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, attacker));
        vault.harvest();

        uint256 ownerBalanceBefore = usdc.balanceOf(owner);
        uint256 assetsHarvested = _harvestAsOwner();
        assertGt(assetsHarvested, 0, "nothing was harvested");
        assertEq(usdc.balanceOf(owner) - ownerBalanceBefore, assetsHarvested, "the owner was not paid");
        assertEq(usdc.balanceOf(attacker), 100_000_000 * ONE_USDC, "the caller who was refused was paid");
    }

    /// @notice A harvest is partial when the inner vault has little liquidity, and the yield it does not
    ///         redeem stays with the owner for later.
    function test_Harvest_isPartialWhenLiquidityIsCapped() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000); // about 100 USDC for the owner

        uint256 liquidityCap = 20 * ONE_USDC;
        innerVault.setAvailableLiquidity(liquidityCap);

        uint256 assetsHarvested = _harvestAsOwner();
        // `maxRedeem` converts the cap to shares rounding down, and the redeem rounds down again.
        assertApproxEqAbs(assetsHarvested, liquidityCap, 2, "harvest should be capped by liquidity");
        assertLe(assetsHarvested, liquidityCap, "harvest exceeded liquidity");
        // This is the test that runs the capped branch of `harvest`: fewer shares than the yield is worth
        // are redeemed, and the vault must still hold at least `totalSupply()`.
        assertGe(vault.innerVaultValue(), vault.totalSupply(), "a partial harvest took the backing");
        // The credit loss on the deposit (up to 2 wei) came out of the yield, and the capped harvest took
        // up to 2 wei under the cap.
        assertApproxEqAbs(
            vault.unharvestedYield(), 80 * ONE_USDC, ROUNDING_TOLERANCE_PER_DEPOSITOR, "about 80 USDC is still waiting"
        );

        // Once liquidity returns, the remainder can be harvested.
        innerVault.setAvailableLiquidity(type(uint256).max);
        uint256 assetsHarvestedLater = _harvestAsOwner();
        assertApproxEqAbs(assetsHarvestedLater, 80 * ONE_USDC, ROUNDING_TOLERANCE_PER_DEPOSITOR, "remainder");
        assertGe(vault.innerVaultValue(), vault.totalSupply(), "the full harvest took the backing");
    }

    /// @notice When the Aave reserve is paused (or inactive), StataTokenV2's `maxRedeem` reports 0.
    ///         `harvest` then returns 0 instead of reverting, and the yield stays with the owner.
    function test_Harvest_returnsZeroWhenReserveIsPaused() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 unharvestedYieldBefore = vault.unharvestedYield();

        innerVault.setReservePaused(true);

        uint256 ownerBalanceBefore = usdc.balanceOf(owner);
        uint256 assetsHarvested = _harvestAsOwner();

        assertEq(assetsHarvested, 0, "paused harvest should return 0");
        assertEq(usdc.balanceOf(owner), ownerBalanceBefore, "owner was paid while paused");
        assertEq(vault.unharvestedYield(), unharvestedYieldBefore, "yield is still the owner's");
    }

    /// @notice After `transferOwnership`, the new owner collects the yield that built up and the old owner can
    ///         no longer harvest.
    function test_Harvest_paysTheNewOwnerAfterTransferOwnership() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);

        address newOwner = makeAddr("newOwner");
        vm.prank(owner);
        vault.transferOwnership(newOwner);

        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, owner));
        vault.harvest();

        vm.prank(newOwner);
        uint256 assetsHarvested = vault.harvest();
        assertGt(assetsHarvested, 0, "nothing was harvested");
        assertEq(usdc.balanceOf(newOwner), assetsHarvested, "the new owner was not paid");
        assertEq(usdc.balanceOf(owner), 0, "the old owner was paid");
    }

    // --------------------------------------------------------------------------------------
    // fuzzing: preview parity and harvest
    // --------------------------------------------------------------------------------------

    /// @notice Every preview equals what the matching action then really does, measured by balance
    ///         changes rather than by the action's return value. The rate moves from -20% to +200%, so
    ///         the steps run with unharvested yield and, for a fall, while depositors are in a shortfall. When
    ///         the vault is closed to deposits, the deposit and the mint must revert instead.
    function testFuzz_previewsMatchExecutedResults(uint256 fuzzedDepositAssets, uint256 fuzzedRateScaleBps) public {
        uint256 depositAssets = bound(fuzzedDepositAssets, 1 * ONE_USDC, 10_000_000 * ONE_USDC);
        uint256 rateScaleBps = bound(fuzzedRateScaleBps, 8_000, 30_000);

        // Alice and Bob enter before the rate moves, so Alice has something to withdraw after a fall.
        _depositAs(bob, 5_000 * ONE_USDC);
        _depositAs(alice, depositAssets);
        _scaleInnerVaultRate(rateScaleBps);

        // deposit
        uint256 aliceUsdcBefore = usdc.balanceOf(alice);
        uint256 aliceSharesBefore = vault.balanceOf(alice);
        if (vault.maxDeposit(alice) == 0) {
            bytes memory closedError =
                abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, alice, depositAssets, 0);
            vm.prank(alice);
            vm.expectRevert(closedError);
            vault.deposit(depositAssets, alice);
        } else {
            uint256 previewedDepositShares = vault.previewDeposit(depositAssets);
            _depositAs(alice, depositAssets);
            assertEq(vault.balanceOf(alice) - aliceSharesBefore, previewedDepositShares, "previewDeposit shares");
            assertEq(aliceUsdcBefore - usdc.balanceOf(alice), depositAssets, "deposit assets pulled");
        }

        // mint
        _scaleInnerVaultRate(10_100);
        uint256 sharesToMint = vault.previewDeposit(depositAssets) / 3;
        aliceUsdcBefore = usdc.balanceOf(alice);
        aliceSharesBefore = vault.balanceOf(alice);
        if (vault.maxMint(alice) == 0) {
            vm.prank(alice);
            vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxMint.selector, alice, sharesToMint, 0));
            vault.mint(sharesToMint, alice);
        } else {
            uint256 previewedMintAssets = vault.previewMint(sharesToMint);
            vm.prank(alice);
            vault.mint(sharesToMint, alice);
            _assertInvariants();
            assertEq(vault.balanceOf(alice) - aliceSharesBefore, sharesToMint, "mint shares");
            assertEq(aliceUsdcBefore - usdc.balanceOf(alice), previewedMintAssets, "previewMint assets");
        }

        // withdraw
        _scaleInnerVaultRate(10_100);
        uint256 assetsToWithdraw = vault.maxWithdraw(alice) / 3;
        uint256 previewedWithdrawShares = vault.previewWithdraw(assetsToWithdraw);
        aliceUsdcBefore = usdc.balanceOf(alice);
        aliceSharesBefore = vault.balanceOf(alice);
        vm.prank(alice);
        vault.withdraw(assetsToWithdraw, alice, alice);
        _assertInvariants();
        assertEq(aliceSharesBefore - vault.balanceOf(alice), previewedWithdrawShares, "previewWithdraw shares");
        assertEq(usdc.balanceOf(alice) - aliceUsdcBefore, assetsToWithdraw, "withdraw assets paid");

        // redeem
        _scaleInnerVaultRate(10_100);
        uint256 sharesToRedeem = vault.balanceOf(alice) / 2;
        uint256 previewedRedeemAssets = vault.previewRedeem(sharesToRedeem);
        aliceUsdcBefore = usdc.balanceOf(alice);
        aliceSharesBefore = vault.balanceOf(alice);
        vm.prank(alice);
        vault.redeem(sharesToRedeem, alice, alice);
        _assertInvariants();
        assertEq(aliceSharesBefore - vault.balanceOf(alice), sharesToRedeem, "redeem shares burned");
        assertEq(usdc.balanceOf(alice) - aliceUsdcBefore, previewedRedeemAssets, "previewRedeem assets");
    }

    /// @notice Whatever the deposits, the rate move (a fall or a rise) and the liquidity cap, a harvest never
    ///         touches the depositors. `_harvestAsOwner` asserts that no holder's value moves, that a whole
    ///         vault still holds `totalSupply()` afterwards, and that the owner is paid no more than its
    ///         yield. Harvesting again once the cap is lifted must also leave under `rate / RAY + 1` wei.
    function testFuzz_harvestNeverTouchesDepositors(
        uint256 fuzzedDepositorCount,
        uint256 fuzzedFirstAmount,
        uint256 fuzzedSecondAmount,
        uint256 fuzzedThirdAmount,
        uint256 fuzzedRateScaleBps,
        uint256 fuzzedLiquidityCap
    ) public {
        uint256 depositorCount = bound(fuzzedDepositorCount, 1, 3);
        uint256[3] memory depositAmounts = [
            bound(fuzzedFirstAmount, 1 * ONE_USDC, 1_000_000 * ONE_USDC),
            bound(fuzzedSecondAmount, 1 * ONE_USDC, 1_000_000 * ONE_USDC),
            bound(fuzzedThirdAmount, 1 * ONE_USDC, 1_000_000 * ONE_USDC)
        ];
        address[3] memory depositors = [alice, bob, carol];

        uint256 totalDeposited;
        for (uint256 depositorIndex = 0; depositorIndex < depositorCount; depositorIndex++) {
            _depositAs(depositors[depositorIndex], depositAmounts[depositorIndex]);
            totalDeposited += depositAmounts[depositorIndex];
        }

        // A fall of up to 20% or a rise of up to 200%.
        _scaleInnerVaultRate(bound(fuzzedRateScaleBps, 8_000, 30_000));

        // Half of the runs also cap the liquidity, anywhere from 1 wei to twice the deposits.
        bool capLiquidity = fuzzedLiquidityCap % 2 == 0;
        if (capLiquidity) {
            innerVault.setAvailableLiquidity(bound(fuzzedLiquidityCap, 1, 2 * totalDeposited));
        }

        _harvestAsOwner();

        innerVault.setAvailableLiquidity(type(uint256).max);
        _harvestAsOwner();
        assertLe(vault.unharvestedYield(), innerVault.rate() / innerVault.RAY() + 1, "a full harvest left too much");
    }

    // --------------------------------------------------------------------------------------
    // liquidity, supply cap, pauses and freezes
    // --------------------------------------------------------------------------------------

    /// @notice `maxWithdraw` and `maxRedeem` follow the inner vault's available liquidity, and a
    ///         redeem of exactly `maxRedeem` succeeds while one share more is refused.
    function test_MaxWithdrawAndMaxRedeem_areCappedByInnerLiquidity() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        _depositAs(alice, aliceDepositAmount);
        uint256 aliceShares = vault.balanceOf(alice);

        // Her whole claim fits within the inner vault's liquidity, so all of it is redeemable, and
        // `maxWithdraw` is exactly what those shares are worth.
        assertEq(vault.maxRedeem(alice), aliceShares, "maxRedeem without a cap");
        assertEq(vault.maxWithdraw(alice), vault.previewRedeem(aliceShares), "maxWithdraw without a cap");

        uint256 liquidityCap = 100 * ONE_USDC;
        innerVault.setAvailableLiquidity(liquidityCap);

        // Budget for how far below the cap a payout can land: the inner vault rounds its own limit
        // down (up to 2 wei at a rate near 1.1), and converting that limit to vault shares and back
        // costs the vault one more wei.
        uint256 roundingBelowCap = 3;

        uint256 maxAssetsWithdrawable = vault.maxWithdraw(alice);
        assertLe(maxAssetsWithdrawable, innerVault.maxWithdraw(address(vault)), "maxWithdraw above the inner limit");
        assertApproxEqAbs(maxAssetsWithdrawable, liquidityCap, roundingBelowCap, "maxWithdraw should be about the cap");
        assertLe(maxAssetsWithdrawable, liquidityCap, "maxWithdraw above the cap");

        uint256 maxSharesRedeemable = vault.maxRedeem(alice);
        assertLt(maxSharesRedeemable, aliceShares, "maxRedeem should be capped");
        assertApproxEqAbs(vault.previewRedeem(maxSharesRedeemable), liquidityCap, roundingBelowCap, "maxRedeem value");

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                ERC4626.ERC4626ExceededMaxRedeem.selector, alice, maxSharesRedeemable + 1, maxSharesRedeemable
            )
        );
        vault.redeem(maxSharesRedeemable + 1, alice, alice);

        vm.prank(alice);
        uint256 aliceAssetsReceived = vault.redeem(maxSharesRedeemable, alice, alice);
        _assertInvariants();
        assertLe(aliceAssetsReceived, liquidityCap, "redeemed more than the liquidity");
        assertApproxEqAbs(aliceAssetsReceived, liquidityCap, roundingBelowCap, "redeemed about the cap");
    }

    /// @notice `withdraw` of exactly `maxWithdraw` succeeds while the inner vault's liquidity is capped,
    ///         and pays that amount to the wei.
    function test_Withdraw_maxWithdrawSucceedsUnderLiquidityCap() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        innerVault.setAvailableLiquidity(100 * ONE_USDC);

        uint256 maxAssetsWithdrawable = vault.maxWithdraw(alice);
        assertGt(maxAssetsWithdrawable, 0, "nothing is withdrawable");

        uint256 aliceBalanceBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.withdraw(maxAssetsWithdrawable, alice, alice);
        _assertInvariants();

        assertEq(usdc.balanceOf(alice) - aliceBalanceBefore, maxAssetsWithdrawable, "withdraw paid a different amount");
    }

    /// @notice Right after a deposit, with no yield, the whole balance is redeemable. The inner vault
    ///         reports its holdings rounded down, which would leave a holder one vault share short if
    ///         `maxRedeem` only converted that limit to shares.
    function test_RedeemWholeBalance_worksRightAfterDeposit() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        uint256 aliceShares = _depositAs(alice, aliceDepositAmount);

        assertEq(vault.maxRedeem(alice), aliceShares, "maxRedeem should be the whole balance");

        uint256 previewedAssets = vault.previewRedeem(aliceShares);
        uint256 aliceAssetsReceived = _redeemAllAs(alice);

        assertEq(vault.balanceOf(alice), 0, "alice still holds shares");
        assertEq(aliceAssetsReceived, previewedAssets, "redeem differs from the preview");
        // The inner vault credited her deposit with up to 2 wei less, and with no yield to absorb it she
        // is that much below par. Nothing else is lost: the vault's own pricing costs her no extra wei.
        assertApproxEqAbs(aliceAssetsReceived, aliceDepositAmount, MAX_INNER_DUST_PER_ACTION, "alice's principal");
    }

    /// @notice While the Aave reserve is paused, StataTokenV2's max functions report 0, so every limit
    ///         here is 0 even though Alice holds shares, and a deposit is refused up front.
    function test_ReservePaused_limitsAreZeroWhileHoldingShares() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        assertGt(vault.balanceOf(alice), 0, "alice should hold shares");
        assertEq(vault.maxDeposit(alice), type(uint256).max, "maxDeposit before pause");
        assertEq(vault.maxMint(alice), type(uint256).max, "maxMint before pause");
        assertGt(vault.maxRedeem(alice), 0, "maxRedeem before pause");
        assertGt(vault.maxWithdraw(alice), 0, "maxWithdraw before pause");

        innerVault.setReservePaused(true);

        assertEq(vault.maxDeposit(alice), 0, "maxDeposit while paused");
        assertEq(vault.maxMint(alice), 0, "maxMint while paused");
        assertEq(vault.maxRedeem(alice), 0, "maxRedeem while paused");
        assertEq(vault.maxWithdraw(alice), 0, "maxWithdraw while paused");

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, alice, 1e6, 0));
        vault.deposit(1e6, alice);
    }

    /// @notice A frozen Aave reserve blocks supply, not withdrawal: deposits close, but `maxRedeem` is
    ///         unchanged and both `harvest` and `redeem` keep working.
    function test_ReserveFrozen_blocksDepositsButNotWithdrawalsOrHarvest() public {
        uint256 aliceDepositAmount = 1_000 * ONE_USDC;
        uint256 aliceShares = _depositAs(alice, aliceDepositAmount);
        _scaleInnerVaultRate(11_000);

        innerVault.setReserveFrozen(true);

        assertEq(vault.maxDeposit(bob), 0, "maxDeposit while frozen");
        assertEq(vault.maxMint(bob), 0, "maxMint while frozen");
        assertEq(vault.maxRedeem(alice), aliceShares, "maxRedeem is unchanged by a freeze");
        assertGt(vault.maxWithdraw(alice), 0, "maxWithdraw is unchanged by a freeze");

        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, bob, 1e6, 0));
        vault.deposit(1e6, bob);

        assertGt(_harvestAsOwner(), 0, "harvest should work while frozen");
        assertEq(_redeemAllAs(alice), aliceDepositAmount, "alice should be able to exit while frozen");
    }

    /// @notice StataTokenV2's own pause is not visible in its max functions, so this vault's limits do not
    ///         change. Deposits, redemptions and `harvest` revert until it is lifted, then work again.
    function test_TokenPaused_actionsRevertUntilLifted() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 aliceShares = vault.balanceOf(alice);

        innerVault.setTokenPaused(true);

        // The limits cannot see this pause.
        assertEq(vault.maxDeposit(bob), type(uint256).max, "maxDeposit is unchanged");
        assertEq(vault.maxRedeem(alice), aliceShares, "maxRedeem is unchanged");
        assertGt(vault.maxWithdraw(alice), 0, "maxWithdraw is unchanged");

        bytes memory tokenPausedError = abi.encodeWithSelector(MockStataVault.TokenPaused.selector);

        vm.prank(owner);
        vm.expectRevert(tokenPausedError);
        vault.harvest();

        vm.prank(bob);
        vm.expectRevert(tokenPausedError);
        vault.deposit(100 * ONE_USDC, bob);

        vm.prank(alice);
        vm.expectRevert(tokenPausedError);
        vault.redeem(aliceShares, alice, alice);

        // Nothing was lost while it was set, and everything works once it is lifted.
        innerVault.setTokenPaused(false);

        assertGt(_harvestAsOwner(), 0, "harvest after the pause");
        assertGt(_depositAs(bob, 100 * ONE_USDC), 0, "deposit after the pause");
        assertGt(_redeemAllAs(alice), 0, "redeem after the pause");
    }

    /// @notice With a finite supply cap, `maxMint` equals `maxDeposit` (shares are 1:1), minting that many
    ///         pulls exactly the cap, and one asset more than the cap is refused.
    function test_SupplyCap_maxMintEqualsMaxDepositAndDepositAboveCapReverts() public {
        _depositAs(bob, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);

        uint256 supplyCapRemaining = 5_000 * ONE_USDC;
        innerVault.setSupplyCapRemaining(supplyCapRemaining);
        assertEq(vault.maxDeposit(alice), supplyCapRemaining, "maxDeposit is the remaining supply cap");
        assertEq(vault.maxMint(alice), supplyCapRemaining, "maxMint is the same, one share per asset unit");
        assertEq(vault.previewMint(vault.maxMint(alice)), vault.maxDeposit(alice), "maxMint fits the cap exactly");

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                ERC4626.ERC4626ExceededMaxDeposit.selector, alice, supplyCapRemaining + 1, supplyCapRemaining
            )
        );
        vault.deposit(supplyCapRemaining + 1, alice);

        uint256 aliceUsdcBefore = usdc.balanceOf(alice);
        uint256 maxSharesMintable = vault.maxMint(alice);
        vm.prank(alice);
        uint256 assetsPulled = vault.mint(maxSharesMintable, alice);
        _assertInvariants();
        assertEq(vault.balanceOf(alice), maxSharesMintable, "alice did not get the shares");
        assertEq(assetsPulled, supplyCapRemaining, "mint should pull exactly the cap");
        assertEq(aliceUsdcBefore - usdc.balanceOf(alice), supplyCapRemaining, "alice paid a different amount");
    }

    // --------------------------------------------------------------------------------------
    // full exit and allowances
    // --------------------------------------------------------------------------------------

    /// @notice When the last holder leaves, nobody is owed anything and what remains is the owner's, and the
    ///         next depositor starts clean.
    function test_FullExit_leavesEverythingToTheOwnerAndLaterDepositorWorksNormally() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(10_100);
        _redeemAllAs(alice);

        assertEq(vault.totalSupply(), 0, "supply after the full exit");
        assertEq(vault.totalAssets(), 0, "nobody is owed anything");
        assertEq(vault.unharvestedYield(), vault.innerVaultValue(), "everything left is the owner's");
        assertGt(vault.unharvestedYield(), 0, "the owner should have the yield");

        // The next depositor gets exactly one share per asset unit and is worth their deposit to the wei,
        // and the credit loss on their deposit comes out of the unharvested yield.
        uint256 bobDepositAmount = 500 * ONE_USDC;
        uint256 unharvestedYieldBefore = vault.unharvestedYield();
        uint256 bobShares = _depositAs(bob, bobDepositAmount);
        assertEq(vault.totalSupply(), bobDepositAmount, "supply is just bob's deposit");
        assertEq(vault.previewRedeem(bobShares), bobDepositAmount, "bob's value right after depositing");
        assertLe(vault.unharvestedYield(), unharvestedYieldBefore, "a deposit cannot add unharvested yield");
        assertGe(
            vault.unharvestedYield(),
            unharvestedYieldBefore - MAX_INNER_DUST_PER_ACTION,
            "bob's deposit cost the owner more than the inner vault's credit loss"
        );

        _scaleInnerVaultRate(10_100);
        assertEq(_redeemAllAs(bob), bobDepositAmount, "bob should get exactly his principal");
    }

    /// @notice A holder can redeem on behalf of another only with an allowance, and it is spent.
    function test_RedeemOnBehalf_spendsAllowance() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSignature("ERC20InsufficientAllowance(address,uint256,uint256)", bob, 0, aliceShares)
        );
        vault.redeem(aliceShares, bob, alice);

        vm.prank(alice);
        vault.approve(bob, aliceShares);

        uint256 bobBalanceBefore = usdc.balanceOf(bob);
        vm.prank(bob);
        uint256 bobAssetsReceived = vault.redeem(aliceShares, bob, alice);
        assertGt(bobAssetsReceived, 0, "nothing was redeemed");
        assertEq(usdc.balanceOf(bob) - bobBalanceBefore, bobAssetsReceived, "bob was not paid");
        assertEq(vault.allowance(alice, bob), 0, "allowance was not spent");
        assertEq(vault.balanceOf(alice), 0, "alice's shares were not burned");
    }

    /// @notice The `withdraw` path also needs an allowance, and spends exactly the shares it burns.
    function test_WithdrawOnBehalf_spendsAllowance() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 assetsToWithdraw = 400 * ONE_USDC;
        uint256 sharesToBurn = vault.previewWithdraw(assetsToWithdraw);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSignature("ERC20InsufficientAllowance(address,uint256,uint256)", bob, 0, sharesToBurn)
        );
        vault.withdraw(assetsToWithdraw, bob, alice);

        vm.prank(alice);
        vault.approve(bob, sharesToBurn + 5);

        uint256 aliceSharesBefore = vault.balanceOf(alice);
        uint256 bobBalanceBefore = usdc.balanceOf(bob);
        vm.prank(bob);
        vault.withdraw(assetsToWithdraw, bob, alice);
        _assertInvariants();

        assertEq(usdc.balanceOf(bob) - bobBalanceBefore, assetsToWithdraw, "bob was not paid");
        assertEq(aliceSharesBefore - vault.balanceOf(alice), sharesToBurn, "alice's shares burned");
        assertEq(vault.allowance(alice, bob), 5, "allowance should drop by exactly the shares burned");
    }

    // --------------------------------------------------------------------------------------
    // the owner's pause
    // --------------------------------------------------------------------------------------

    /// @notice Only the owner can pause and unpause, and each emits its event.
    function test_Pause_onlyOwnerCanPauseAndUnpause_andEmitsEvents() public {
        bytes memory notOwnerError = abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, attacker);

        vm.prank(attacker);
        vm.expectRevert(notOwnerError);
        vault.pause();

        vm.expectEmit(true, true, true, true, address(vault));
        emit Pausable.Paused(owner);
        vm.prank(owner);
        vault.pause();
        assertTrue(vault.paused(), "should be paused");

        vm.prank(attacker);
        vm.expectRevert(notOwnerError);
        vault.unpause();
        assertTrue(vault.paused(), "a stranger must not unpause");

        vm.expectEmit(true, true, true, true, address(vault));
        emit Pausable.Unpaused(owner);
        vm.prank(owner);
        vault.unpause();
        assertFalse(vault.paused(), "should be unpaused");
    }

    /// @notice While paused, every limit is 0, deposits, mints, withdrawals, redeems and share transfers revert,
    ///         and pausing changes no balance or value. `harvest` and `execute` still work.
    function test_Pause_closesDepositsWithdrawalsAndTransfers_butNotHarvestOrExecute() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        _depositAs(bob, 500 * ONE_USDC);
        _scaleInnerVaultRate(11_000); // +10%: about 150 USDC of unharvested yield

        uint256 totalAssetsBefore = vault.totalAssets();
        uint256 unharvestedYieldBefore = vault.unharvestedYield();
        uint256 shortfallBefore = vault.depositorShortfall();
        uint256 totalSupplyBefore = vault.totalSupply();
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);

        vm.prank(owner);
        vault.pause();

        // Pausing moves nothing.
        assertEq(vault.balanceOf(alice), aliceShares, "alice's balance changed");
        assertEq(vault.totalSupply(), totalSupplyBefore, "supply changed");
        assertEq(vault.totalAssets(), totalAssetsBefore, "totalAssets changed");
        assertEq(vault.unharvestedYield(), unharvestedYieldBefore, "unharvestedYield changed");
        assertEq(vault.depositorShortfall(), shortfallBefore, "depositorShortfall changed");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value changed");

        // Every limit is 0.
        assertEq(vault.maxDeposit(alice), 0, "maxDeposit while paused");
        assertEq(vault.maxMint(alice), 0, "maxMint while paused");
        assertEq(vault.maxWithdraw(alice), 0, "maxWithdraw while paused");
        assertEq(vault.maxRedeem(alice), 0, "maxRedeem while paused");

        // The max checks run first, so the ERC-4626 limit errors are what a caller sees.
        uint256 amount = 100 * ONE_USDC;
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxDeposit.selector, alice, amount, 0));
        vault.deposit(amount, alice);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxMint.selector, alice, amount, 0));
        vault.mint(amount, alice);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxWithdraw.selector, alice, amount, 0));
        vault.withdraw(amount, alice, alice);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ERC4626.ERC4626ExceededMaxRedeem.selector, alice, amount, 0));
        vault.redeem(amount, alice, alice);

        // Share transfers revert too, both `transfer` and `transferFrom`.
        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        // forge-lint: disable-next-line(erc20-unchecked-transfer)
        vault.transfer(carol, 1);
        vm.prank(alice);
        vault.approve(bob, 1);
        vm.prank(bob);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        // forge-lint: disable-next-line(erc20-unchecked-transfer)
        vault.transferFrom(alice, carol, 1);
        assertEq(vault.balanceOf(carol), 0, "no share moved");

        // `harvest` still works and pays the owner; it leaves depositors untouched (checked by the helper).
        assertGt(_harvestAsOwner(), 0, "harvest should work while paused");

        // So does `execute`: a stray token is recovered with a plain transfer.
        MockUSDC strayToken = new MockUSDC();
        strayToken.mint(address(vault), 5 * ONE_USDC);
        _executeAsOwner(_batchOf(_call(address(strayToken), abi.encodeCall(IERC20.transfer, (owner, 5 * ONE_USDC)))));
        assertEq(strayToken.balanceOf(owner), 5 * ONE_USDC, "execute should work while paused");
    }

    /// @notice After `unpause`, deposits, redeems and transfers work again and the values are as they were.
    function test_Unpause_restoresDepositsWithdrawalsAndTransfers() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 totalAssetsBefore = vault.totalAssets();
        uint256 unharvestedYieldBefore = vault.unharvestedYield();
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);

        vm.prank(owner);
        vault.pause();
        vm.prank(owner);
        vault.unpause();

        assertEq(vault.totalAssets(), totalAssetsBefore, "totalAssets changed");
        assertEq(vault.unharvestedYield(), unharvestedYieldBefore, "unharvestedYield changed");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value changed");
        assertEq(vault.maxDeposit(bob), type(uint256).max, "deposits reopen");
        assertEq(vault.maxRedeem(alice), aliceShares, "redeems reopen");

        assertEq(_depositAs(bob, 100 * ONE_USDC), 100 * ONE_USDC, "deposit after unpause");
        vm.prank(alice);
        assertTrue(vault.transfer(carol, aliceShares / 4), "transfer after unpause");
        assertEq(vault.balanceOf(carol), aliceShares / 4, "carol received the shares");
        _assertInvariants();
        assertEq(_redeemAllAs(alice), (aliceShares - aliceShares / 4), "redeem after unpause pays par");
    }

    // --------------------------------------------------------------------------------------
    // the owner's guarded execute
    // --------------------------------------------------------------------------------------

    /// @notice Only the owner can execute, and only plain call batches: other call types and try-mode are refused.
    function test_Execute_onlyOwner_andRejectsUnsupportedModes() public {
        bytes memory noCalls = ERC7579Utils.encodeBatch(new Execution[](0));

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, attacker));
        vault.execute(_batchMode(), noCalls);

        bytes32 singleCallMode = Mode.unwrap(
            ERC7579Utils.encodeMode(
                ERC7579Utils.CALLTYPE_SINGLE, ERC7579Utils.EXECTYPE_DEFAULT, ModeSelector.wrap(0), ModePayload.wrap(0)
            )
        );
        bytes32 tryMode = Mode.unwrap(
            ERC7579Utils.encodeMode(
                ERC7579Utils.CALLTYPE_BATCH, ERC7579Utils.EXECTYPE_TRY, ModeSelector.wrap(0), ModePayload.wrap(0)
            )
        );
        assertTrue(vault.supportsExecutionMode(_batchMode()), "the batch mode is supported");
        assertFalse(vault.supportsExecutionMode(singleCallMode), "a single call is not");
        assertFalse(vault.supportsExecutionMode(tryMode), "try-mode is not: it would swallow failures");

        vm.prank(owner);
        vm.expectRevert(ERC7821.UnsupportedExecutionMode.selector);
        vault.execute(singleCallMode, noCalls);
        vm.prank(owner);
        vm.expectRevert(ERC7821.UnsupportedExecutionMode.selector);
        vault.execute(tryMode, noCalls);

        vm.prank(owner);
        vault.execute(_batchMode(), noCalls);
    }

    /// @notice The owner claims incentives that accrue to the vault as the stata holder. Only the holder can
    ///         claim, so a direct call from the owner gets nothing. The claim moves no inner shares and no value.
    function test_Execute_claimsRewardsAsTheVault() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        MockUSDC rewardToken = new MockUSDC();
        uint256 rewardAmount = 50 * ONE_USDC;
        innerVault.creditRewards(address(vault), address(rewardToken), rewardAmount);
        address[] memory rewards = new address[](1);
        rewards[0] = address(rewardToken);

        vm.prank(owner);
        innerVault.claimRewards(owner, rewards);
        assertEq(rewardToken.balanceOf(owner), 0, "only the holder can claim its rewards");

        uint256 vaultInnerSharesBefore = innerVault.balanceOf(address(vault));
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        uint256 unharvestedYieldBefore = vault.unharvestedYield();

        _executeAsOwner(
            _batchOf(_call(address(innerVault), abi.encodeCall(MockStataVault.claimRewards, (owner, rewards))))
        );

        assertEq(rewardToken.balanceOf(owner), rewardAmount, "the owner should receive the rewards");
        assertEq(innerVault.claimableRewards(address(vault), address(rewardToken)), 0, "the rewards were claimed");
        assertEq(innerVault.balanceOf(address(vault)), vaultInnerSharesBefore, "inner shares moved");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value moved");
        assertEq(vault.unharvestedYield(), unharvestedYieldBefore, "the yield moved");
    }

    /// @notice Calls run in order, and a failing call reverts the whole batch with its own error.
    function test_Execute_runsCallsInOrder_andAFailingCallRevertsTheBatch() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        MockUSDC rewardToken = new MockUSDC();
        uint256 rewardAmount = 50 * ONE_USDC;
        innerVault.creditRewards(address(vault), address(rewardToken), rewardAmount);
        address[] memory rewards = new address[](1);
        rewards[0] = address(rewardToken);
        Execution memory claimToTheVault =
            _call(address(innerVault), abi.encodeCall(MockStataVault.claimRewards, (address(vault), rewards)));

        // The second call forwards what the first one claimed, so it only works if they run in order.
        _executeAsOwner(
            _batchOf(
                claimToTheVault, _call(address(rewardToken), abi.encodeCall(IERC20.transfer, (owner, rewardAmount)))
            )
        );
        assertEq(rewardToken.balanceOf(owner), rewardAmount, "the owner should have the rewards");
        assertEq(rewardToken.balanceOf(address(vault)), 0, "nothing should be left in the vault");

        // Now the second call asks for one wei more than exists: the first call's claim is undone as well.
        innerVault.creditRewards(address(vault), address(rewardToken), rewardAmount);
        Execution memory overspend =
            _call(address(rewardToken), abi.encodeCall(IERC20.transfer, (owner, rewardAmount + 1)));
        bytes memory executionData = ERC7579Utils.encodeBatch(_batchOf(claimToTheVault, overspend));
        vm.prank(owner);
        vm.expectRevert(
            abi.encodeWithSignature(
                "ERC20InsufficientBalance(address,uint256,uint256)", address(vault), rewardAmount, rewardAmount + 1
            )
        );
        vault.execute(_batchMode(), executionData);
        assertEq(
            innerVault.claimableRewards(address(vault), address(rewardToken)), rewardAmount, "the claim was not undone"
        );
        assertEq(rewardToken.balanceOf(owner), rewardAmount, "the owner got more than the first batch paid");
    }

    /// @notice A batch that transfers the inner-vault shares out is refused.
    function test_Execute_cannotTransferInnerSharesOut() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 innerShares = innerVault.balanceOf(address(vault));

        _assertGuardRefuses(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC20.transfer, (owner, innerShares)))),
            abi.encodeWithSelector(YieldCaptureVault.InnerVaultSharesDecreased.selector, innerShares, 0)
        );
    }

    /// @notice `transferFrom` is refused outright, wherever the allowance came from. A batch cannot create the
    ///         allowance (approve is refused), so the test plants a self-allowance as the vault to show it makes no
    ///         difference.
    function test_Execute_cannotMoveInnerSharesWithTransferFrom() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 innerShares = innerVault.balanceOf(address(vault));
        vm.prank(address(vault));
        assertTrue(innerVault.approve(address(vault), innerShares), "planting the allowance failed");

        _assertGuardRefuses(
            _batchOf(
                _call(address(innerVault), abi.encodeCall(IERC20.transferFrom, (address(vault), owner, innerShares)))
            ),
            abi.encodeWithSelector(YieldCaptureVault.TransferFromForbidden.selector)
        );
    }

    /// @notice Redeeming the inner-vault shares for the vault's assets is refused.
    function test_Execute_cannotRedeemInnerShares() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 innerShares = innerVault.balanceOf(address(vault));

        _assertGuardRefuses(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC4626.redeem, (innerShares, owner, address(vault))))),
            abi.encodeWithSelector(YieldCaptureVault.InnerVaultSharesDecreased.selector, innerShares, 0)
        );
    }

    /// @notice Withdrawing assets from the inner vault is refused, even a small amount.
    function test_Execute_cannotWithdrawFromTheInnerVault() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        uint256 innerShares = innerVault.balanceOf(address(vault));
        uint256 assetsToWithdraw = 10 * ONE_USDC;
        uint256 innerSharesBurned = innerVault.previewWithdraw(assetsToWithdraw);

        _assertGuardRefuses(
            _batchOf(
                _call(address(innerVault), abi.encodeCall(IERC4626.withdraw, (assetsToWithdraw, owner, address(vault))))
            ),
            abi.encodeWithSelector(
                YieldCaptureVault.InnerVaultSharesDecreased.selector, innerShares, innerShares - innerSharesBurned
            )
        );
    }

    /// @notice Even the unharvested yield cannot leave through `execute`: it leaves only through `harvest`.
    function test_Execute_cannotRedeemJustTheUnharvestedYield() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 innerShares = innerVault.balanceOf(address(vault));
        uint256 yieldInnerShares = innerVault.convertToShares(vault.unharvestedYield());
        assertGt(yieldInnerShares, 0, "setup: there should be yield to try to take");

        _assertGuardRefuses(
            _batchOf(
                _call(address(innerVault), abi.encodeCall(IERC4626.redeem, (yieldInnerShares, owner, address(vault))))
            ),
            abi.encodeWithSelector(
                YieldCaptureVault.InnerVaultSharesDecreased.selector, innerShares, innerShares - yieldInnerShares
            )
        );
    }

    /// @notice Approving the inner-vault shares is refused for any amount, including 0, wherever it sits in the batch.
    function test_Execute_cannotApproveInnerShares() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        bytes memory approveRefused = abi.encodeWithSelector(YieldCaptureVault.CannotApproveInnerVaultShares.selector);
        Execution memory harmlessCall = _call(address(innerVault), abi.encodeCall(IERC20.balanceOf, (address(vault))));

        _assertGuardRefuses(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC20.approve, (owner, ONE_USDC)))), approveRefused
        );
        _assertGuardRefuses(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC20.approve, (owner, 0)))), approveRefused
        );
        _assertGuardRefuses(
            _batchOf(harmlessCall, _call(address(innerVault), abi.encodeCall(IERC20.approve, (owner, ONE_USDC)))),
            approveRefused
        );
        assertEq(innerVault.allowance(address(vault), owner), 0, "no allowance should exist");
    }

    /// @notice The asset cannot be called from a batch at all: not `approve`, `transfer`, `transferFrom` or a view.
    ///         That also stops the owner revoking the vault's approval to the inner vault, which deposits need.
    function test_Execute_cannotCallTheAsset() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        bytes memory assetForbidden =
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(usdc));

        _assertGuardRefuses(
            _batchOf(_call(address(usdc), abi.encodeCall(IERC20.approve, (alice, ONE_USDC)))), assetForbidden
        );
        _assertGuardRefuses(
            _batchOf(_call(address(usdc), abi.encodeCall(IERC20.approve, (address(innerVault), 0)))), assetForbidden
        );
        Execution memory transferUsdc = _call(address(usdc), abi.encodeCall(IERC20.transfer, (owner, 1)));
        _assertGuardRefuses(_batchOf(transferUsdc), assetForbidden);
        _assertGuardRefuses(
            _batchOf(_call(address(usdc), abi.encodeCall(IERC20.transferFrom, (alice, owner, 1)))), assetForbidden
        );
        _assertGuardRefuses(
            _batchOf(_call(address(usdc), abi.encodeCall(IERC20.balanceOf, (address(vault))))), assetForbidden
        );
        assertEq(usdc.allowance(address(vault), address(innerVault)), type(uint256).max, "the approval was revoked");
    }

    /// @notice Other tokens, such as a stray reward token, can still be approved and transferred by a batch.
    function test_Execute_allowsApprovingAndTransferringOtherTokens() public {
        MockUSDC strayToken = new MockUSDC();
        strayToken.mint(address(vault), 5 * ONE_USDC);

        _executeAsOwner(
            _batchOf(
                _call(address(strayToken), abi.encodeCall(IERC20.approve, (alice, ONE_USDC))),
                _call(address(strayToken), abi.encodeCall(IERC20.transfer, (owner, 5 * ONE_USDC)))
            )
        );

        assertEq(strayToken.allowance(address(vault), alice), ONE_USDC, "the approval should go through");
        assertEq(strayToken.balanceOf(owner), 5 * ONE_USDC, "the transfer should go through");
    }

    /// @notice Regression for the critical finding. Depositors approve the vault for their USDC, usually without
    ///         limit, and inside a batch the vault is the caller, so `transferFrom` on the asset would drain their
    ///         wallets. Calling the asset is refused.
    function test_Execute_cannotPullDepositorUsdcThroughTheirApproval() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        assertEq(usdc.allowance(alice, address(vault)), type(uint256).max, "setup: alice approved the vault");
        uint256 aliceWalletBalance = usdc.balanceOf(alice);
        assertGt(aliceWalletBalance, 0, "setup: alice should keep USDC in her wallet");

        _assertGuardRefuses(
            _batchOf(_call(address(usdc), abi.encodeCall(IERC20.transferFrom, (alice, owner, aliceWalletBalance)))),
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(usdc))
        );
        assertEq(usdc.balanceOf(alice), aliceWalletBalance, "alice's wallet was drained");
    }

    /// @notice `transferFrom` is refused on any other token too, since as the vault it could only spend allowances
    ///         that people gave the vault: here a stray token, and the inner-vault shares of a user who approved it.
    function test_Execute_cannotCallTransferFromOnAnyToken() public {
        MockUSDC strayToken = new MockUSDC();
        strayToken.mint(alice, 5 * ONE_USDC);
        vm.startPrank(alice);
        assertTrue(strayToken.approve(address(vault), type(uint256).max), "approving the stray token failed");
        assertTrue(usdc.approve(address(innerVault), type(uint256).max), "approving the inner vault failed");
        uint256 aliceInnerShares = innerVault.deposit(100 * ONE_USDC, alice);
        assertTrue(innerVault.approve(address(vault), type(uint256).max), "approving the vault failed");
        vm.stopPrank();
        bytes memory transferFromRefused = abi.encodeWithSelector(YieldCaptureVault.TransferFromForbidden.selector);

        _assertGuardRefuses(
            _batchOf(_call(address(strayToken), abi.encodeCall(IERC20.transferFrom, (alice, owner, 5 * ONE_USDC)))),
            transferFromRefused
        );
        assertEq(strayToken.balanceOf(alice), 5 * ONE_USDC, "alice's stray tokens moved");

        _assertGuardRefuses(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC20.transferFrom, (alice, owner, aliceInnerShares)))),
            transferFromRefused
        );
        assertEq(innerVault.balanceOf(alice), aliceInnerShares, "alice's inner shares moved");
    }

    /// @notice Accepted behaviour: shares may leave and return within one batch. The owner can move them for a
    ///         moment, but must hand them back before the batch ends.
    function test_Execute_allowsMovingInnerSharesOutAndBackWithinOneBatch() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        uint256 innerShares = innerVault.balanceOf(address(vault));
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        MockShareBorrower borrower = new MockShareBorrower();

        _executeAsOwner(
            _batchOf(
                _call(address(innerVault), abi.encodeCall(IERC20.transfer, (address(borrower), innerShares))),
                _call(
                    address(borrower),
                    abi.encodeCall(MockShareBorrower.giveBack, (address(innerVault), address(vault), innerShares))
                )
            )
        );

        assertEq(innerVault.balanceOf(address(vault)), innerShares, "the shares should be back");
        assertEq(innerVault.balanceOf(address(borrower)), 0, "the borrower kept nothing");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value moved");
    }

    /// @notice A batch that raises the inner-vault share balance passes: here donated USDC is put to work.
    function test_Execute_allowsDepositingDonatedAssetIntoTheInnerVault() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 donatedAmount = 10 * ONE_USDC;
        vm.prank(bob);
        assertTrue(usdc.transfer(address(vault), donatedAmount), "donation transfer failed");

        uint256 innerSharesBefore = innerVault.balanceOf(address(vault));
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares);
        uint256 unharvestedYieldBefore = vault.unharvestedYield();

        _executeAsOwner(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC4626.deposit, (donatedAmount, address(vault)))))
        );

        assertGt(innerVault.balanceOf(address(vault)), innerSharesBefore, "the balance should have grown");
        assertEq(usdc.balanceOf(address(vault)), 0, "the donation should be in the inner vault");
        assertEq(vault.previewRedeem(aliceShares), aliceValueBefore, "alice's value moved");
        // The donation became unharvested yield, less the inner vault's credit loss (up to 2 wei).
        assertApproxEqAbs(
            vault.unharvestedYield() - unharvestedYieldBefore, donatedAmount, MAX_INNER_DUST_PER_ACTION, "the yield"
        );
    }

    /// @notice A batch cannot call the vault itself, by its address or by the zero address, which ERC-7579 batches
    ///         treat as an alias for the vault. The vault never needs to.
    function test_Execute_callingTheVaultItselfIsForbidden() public {
        _depositAs(alice, 1_000 * ONE_USDC);

        _assertGuardRefuses(
            _batchOf(_call(address(vault), abi.encodeCall(YieldCaptureVault.pause, ()))),
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(vault))
        );
        assertFalse(vault.paused(), "the vault must not pause itself");

        _assertGuardRefuses(
            _batchOf(_call(address(0), abi.encodeCall(YieldCaptureVault.harvest, ()))),
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(0))
        );
        _assertGuardRefuses(
            _batchOf(_call(address(vault), abi.encodeCall(Ownable.transferOwnership, (attacker)))),
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(vault))
        );
        assertEq(vault.owner(), owner, "the owner must be unchanged");
    }

    /// @notice Regression for the medium finding. A user who approved the vault for its own share token cannot have
    ///         those shares pulled by a self-call, by address or by the zero alias.
    function test_Execute_cannotPullVaultSharesThroughASelfCall() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        vm.prank(alice);
        assertTrue(vault.approve(address(vault), aliceShares), "approving the vault failed");
        bytes memory pullShares = abi.encodeCall(IERC20.transferFrom, (alice, owner, aliceShares));

        _assertGuardRefuses(
            _batchOf(_call(address(vault), pullShares)),
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(vault))
        );
        _assertGuardRefuses(
            _batchOf(_call(address(0), pullShares)),
            abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, address(0))
        );
        assertEq(vault.balanceOf(alice), aliceShares, "alice's shares moved");
        assertEq(vault.balanceOf(owner), 0, "the owner got shares");
    }

    /// @notice Shares sent to the vault by mistake are recovered with a self `transfer`, by address or by the zero
    ///         alias. The shares stay in `totalSupply()`, so no other holder's value moves.
    function test_Execute_recoversVaultSharesSentToTheVault() public {
        uint256 aliceShares = _depositAs(alice, 1_000 * ONE_USDC);
        uint256 bobShares = _depositAs(bob, 500 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 mistakenShares = 100 * ONE_USDC;
        vm.prank(alice);
        assertTrue(vault.transfer(address(vault), mistakenShares), "the mistaken transfer failed");
        assertEq(vault.balanceOf(address(vault)), mistakenShares, "setup: the vault holds shares");

        uint256 totalSupplyBefore = vault.totalSupply();
        uint256 aliceValueBefore = vault.previewRedeem(aliceShares - mistakenShares);
        uint256 bobValueBefore = vault.previewRedeem(bobShares);

        // First 60 by the vault's address, then the remaining 40 by the zero alias.
        uint256 firstAmount = 60 * ONE_USDC;
        uint256 secondAmount = mistakenShares - firstAmount;
        _executeAsOwner(_batchOf(_call(address(vault), abi.encodeCall(IERC20.transfer, (owner, firstAmount)))));
        assertEq(vault.balanceOf(owner), firstAmount, "the owner should get the first shares");
        _executeAsOwner(_batchOf(_call(address(0), abi.encodeCall(IERC20.transfer, (owner, secondAmount)))));

        assertEq(vault.balanceOf(owner), mistakenShares, "the owner should get all the shares");
        assertEq(vault.balanceOf(address(vault)), 0, "the vault should hold none");
        assertEq(vault.totalSupply(), totalSupplyBefore, "supply moved");
        assertEq(vault.previewRedeem(aliceShares - mistakenShares), aliceValueBefore, "alice's value moved");
        assertEq(vault.previewRedeem(bobShares), bobValueBefore, "bob's value moved");
    }

    /// @notice Calls to the vault itself are allowed for `transfer` only. Every other selector, and empty calldata,
    ///         is refused with `ExecuteTargetForbidden`, by address and by the zero alias. That includes
    ///         `transferFrom`, which hits this rule before the `transferFrom` rule.
    function test_Execute_selfCallsOtherThanTransferAreForbidden() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        bytes[7] memory forbiddenCalls = [
            abi.encodeCall(IERC20.approve, (owner, 1)),
            abi.encodeCall(IERC20.transferFrom, (alice, owner, 1)),
            abi.encodeCall(IERC4626.redeem, (1, owner, alice)),
            abi.encodeCall(IERC4626.deposit, (1, owner)),
            abi.encodeCall(YieldCaptureVault.pause, ()),
            abi.encodeCall(YieldCaptureVault.harvest, ()),
            bytes("")
        ];
        address[2] memory selfTargets = [address(vault), address(0)];

        for (uint256 targetIndex = 0; targetIndex < selfTargets.length; targetIndex++) {
            for (uint256 callIndex = 0; callIndex < forbiddenCalls.length; callIndex++) {
                _assertGuardRefuses(
                    _batchOf(_call(selfTargets[targetIndex], forbiddenCalls[callIndex])),
                    abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, selfTargets[targetIndex])
                );
            }
        }
        assertFalse(vault.paused(), "the vault must not pause itself");
    }

    /// @notice A self `transfer` is still a share transfer, so the pause stops it. It works again once unpaused.
    function test_Execute_selfTransferWhilePausedRevertsEnforcedPause() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        vm.prank(alice);
        assertTrue(vault.transfer(address(vault), 10 * ONE_USDC), "the mistaken transfer failed");
        Execution[] memory recovery =
            _batchOf(_call(address(vault), abi.encodeCall(IERC20.transfer, (owner, 10 * ONE_USDC))));

        vm.prank(owner);
        vault.pause();
        _assertGuardRefuses(recovery, abi.encodeWithSelector(Pausable.EnforcedPause.selector));

        vm.prank(owner);
        vault.unpause();
        _executeAsOwner(recovery);
        assertEq(vault.balanceOf(owner), 10 * ONE_USDC, "the shares should be recovered after unpausing");
    }

    /// @notice The full recovery path for stray USDC: put it into the inner vault with `execute`, then `harvest`
    ///         pays it to the owner, less the inner vault's credit loss and the harvest's floor.
    function test_Execute_strayUsdcIsRecoveredByDepositingThenHarvesting() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        _scaleInnerVaultRate(11_000);
        uint256 strayAmount = 10 * ONE_USDC;
        vm.prank(bob);
        assertTrue(usdc.transfer(address(vault), strayAmount), "donation transfer failed");
        uint256 unharvestedYieldBefore = vault.unharvestedYield();

        _executeAsOwner(
            _batchOf(_call(address(innerVault), abi.encodeCall(IERC4626.deposit, (strayAmount, address(vault)))))
        );
        uint256 ownerBalanceBefore = usdc.balanceOf(owner);
        uint256 assetsHarvested = _harvestAsOwner();

        assertEq(usdc.balanceOf(owner) - ownerBalanceBefore, assetsHarvested, "the owner was not paid");
        assertEq(usdc.balanceOf(address(vault)), 0, "no USDC should be left in the vault");
        // The owner gets the earlier yield plus the stray USDC. The deposit loses up to 2 wei and the harvest leaves
        // up to 2 wei of yield behind.
        assertApproxEqAbs(
            assetsHarvested,
            unharvestedYieldBefore + strayAmount,
            2 * MAX_INNER_DUST_PER_ACTION,
            "the owner should recover the stray USDC"
        );
    }

    /// @notice `execute` keeps working while the vault is paused, like `harvest`.
    function test_Execute_worksWhilePaused() public {
        _depositAs(alice, 1_000 * ONE_USDC);
        MockUSDC rewardToken = new MockUSDC();
        innerVault.creditRewards(address(vault), address(rewardToken), 5 * ONE_USDC);
        address[] memory rewards = new address[](1);
        rewards[0] = address(rewardToken);

        vm.prank(owner);
        vault.pause();
        _executeAsOwner(
            _batchOf(_call(address(innerVault), abi.encodeCall(MockStataVault.claimRewards, (owner, rewards))))
        );

        assertEq(rewardToken.balanceOf(owner), 5 * ONE_USDC, "the claim should work while paused");
    }

    // --------------------------------------------------------------------------------------
    // admin and validation
    // --------------------------------------------------------------------------------------

    function test_Constructor_setsConfiguration() public {
        YieldCaptureVault configuredVault = _deployVault();

        assertEq(configuredVault.innerVault(), address(innerVault), "innerVault");
        assertEq(configuredVault.asset(), address(usdc), "asset comes from the inner vault");
        assertEq(configuredVault.owner(), owner, "owner");
        assertFalse(configuredVault.paused(), "starts unpaused");
        assertEq(configuredVault.decimals(), usdc.decimals(), "shares have the asset's decimals");
        assertEq(configuredVault.totalSupply(), 0, "no shares at the start");
        assertEq(usdc.allowance(address(configuredVault), address(innerVault)), type(uint256).max, "approval");
    }

    function test_Constructor_rejectsZeroOwner() public {
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableInvalidOwner.selector, address(0)));
        new YieldCaptureVault(address(innerVault), "Yield Capture USDC", "ycUSDC", address(0));
    }

    /// @notice Ownership can be handed over but never renounced, whoever asks: a renounced owner could not
    ///         unpause or collect yield.
    function test_RenounceOwnership_alwaysReverts() public {
        vm.prank(owner);
        vm.expectRevert(YieldCaptureVault.OwnershipCannotBeRenounced.selector);
        vault.renounceOwnership();
        assertEq(vault.owner(), owner, "the owner must be unchanged");

        // The override has no `onlyOwner`, so a stranger gets the same error.
        vm.prank(attacker);
        vm.expectRevert(YieldCaptureVault.OwnershipCannotBeRenounced.selector);
        vault.renounceOwnership();
        assertEq(vault.owner(), owner, "the owner must be unchanged");
    }

    /// @notice A deposit too small to mint one inner share is refused with a clear error. At the
    ///         starting rate of 1.1129, 1 wei is worth less than one inner share. A zero-asset deposit or
    ///         mint is the smallest case of the same thing.
    function test_Deposit_revertsWhenTooSmallForInnerVault() public {
        vm.prank(alice);
        vm.expectRevert(YieldCaptureVault.DepositTooSmallForInnerVault.selector);
        vault.deposit(1, alice);

        vm.prank(alice);
        vm.expectRevert(YieldCaptureVault.DepositTooSmallForInnerVault.selector);
        vault.deposit(0, alice);

        vm.prank(alice);
        vm.expectRevert(YieldCaptureVault.DepositTooSmallForInnerVault.selector);
        vault.mint(1, alice);

        vm.prank(alice);
        vm.expectRevert(YieldCaptureVault.DepositTooSmallForInnerVault.selector);
        vault.mint(0, alice);
    }
}
