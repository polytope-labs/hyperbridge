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
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC4626} from "@openzeppelin/contracts/interfaces/IERC4626.sol";
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";
import {
    ERC7579Utils,
    Mode,
    ModeSelector,
    ModePayload
} from "@openzeppelin/contracts/account/utils/draft-ERC7579Utils.sol";

import {YieldCaptureVault} from "../contracts/vaults/YieldCaptureVault.sol";

/// @dev The part of StataTokenV2 used by the `execute` tests. Only the holder can claim its own rewards.
interface IStataRewards {
    function claimRewards(address receiver, address[] calldata rewards) external;
}

/// @notice Runs {YieldCaptureVault} over the real Aave stataUSDC ("Wrapped Aave Base USDC", `waBasUSDC`)
///         on Base. It checks the accounting against real interest, which the mock in
///         `YieldCaptureVault.t.sol` can only imitate.
///
///         Set `BASE_RPC_URL` to a Base RPC endpoint to run it, for example
///         `BASE_RPC_URL=https://mainnet.base.org forge test --match-path test/YieldCaptureVaultBaseFork.t.sol`.
///         Without it every test is skipped. The fork is taken at the latest block unless
///         `BASE_FORK_BLOCK` pins one. The tests are also skipped while Aave's supply cap for USDC
///         leaves no room for their deposits.
contract YieldCaptureVaultBaseForkTest is Test {
    /// @dev Aave's StataTokenV2 for USDC on Base: 6 decimals, wraps the aToken as a non-rebasing share.
    address internal constant STATA_USDC = 0xC768c589647798a6EE01A91FdE98EF2ed046DBD6;
    address internal constant BASE_USDC = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;

    /// @dev stataUSDC's registered reward: aBasUSDC. Its campaign has ended, so nothing accrues any more.
    address internal constant A_BAS_USDC = 0x4e65fE4DbA92790696d040ac24Aa414708F5c0AB;

    uint256 internal constant ONE_USDC = 1e6;

    /// @dev The largest deposit any test makes. `setUp` skips the run if the supply cap cannot take it.
    uint256 internal constant LARGEST_TEST_DEPOSIT = 10_000 * ONE_USDC;

    /// @dev Rounding budget for the interest comparison. stataUSDC burns rounded-up shares on Alice's
    ///      withdrawal (up to 2 wei over what she is paid, at a rate near 1.1), which comes out of the
    ///      owner's interest, and the harvest floors once more (1 wei).
    uint256 internal constant ROUNDING_TOLERANCE = 3;

    /// @dev The most a deposit can lose to stataUSDC's rounding. It credits a rounded-down number of shares,
    ///      worth less than `rate / RAY + 1` wei under the deposit, so at most 2 wei at a rate near 1.1.
    uint256 internal constant MAX_STATA_CREDIT_LOSS = 2;

    IERC4626 internal stataUsdc = IERC4626(STATA_USDC);
    IERC20 internal usdc = IERC20(BASE_USDC);
    YieldCaptureVault internal vault;

    address internal owner = makeAddr("owner");
    address internal alice = makeAddr("alice");

    function setUp() public {
        string memory rpcUrl = vm.envOr("BASE_RPC_URL", string(""));
        if (bytes(rpcUrl).length == 0) {
            vm.skip(true);
        }

        // Fork at the latest block by default. Public RPC endpoints prune old state, so a hardcoded
        // block would stop working; set `BASE_FORK_BLOCK` to pin one on an archive endpoint.
        uint256 forkBlock = vm.envOr("BASE_FORK_BLOCK", uint256(0));
        if (forkBlock == 0) {
            vm.createSelectFork(rpcUrl);
        } else {
            vm.createSelectFork(rpcUrl, forkBlock);
        }

        vault = new YieldCaptureVault(STATA_USDC, "Yield Capture USDC", "ycUSDC", owner);

        // A full Aave supply cap (or a paused reserve) would make the deposits revert. That is not a
        // bug in the vault, so skip instead of failing.
        if (stataUsdc.maxDeposit(address(vault)) < LARGEST_TEST_DEPOSIT) {
            vm.skip(true);
        }

        deal(BASE_USDC, alice, 1_000_000 * ONE_USDC);
        vm.prank(alice);
        usdc.approve(address(vault), type(uint256).max);
    }

    // --------------------------------------------------------------------------------------
    // helpers
    // --------------------------------------------------------------------------------------

    function _depositAs(address depositor, uint256 assets) internal returns (uint256 shares) {
        vm.prank(depositor);
        shares = vault.deposit(assets, depositor);
    }

    function _harvestAsOwner() internal returns (uint256 assetsSentToOwner) {
        vm.prank(owner);
        assetsSentToOwner = vault.harvest();
    }

    function _batchMode() internal pure returns (bytes32) {
        return Mode.unwrap(
            ERC7579Utils.encodeMode(
                ERC7579Utils.CALLTYPE_BATCH, ERC7579Utils.EXECTYPE_DEFAULT, ModeSelector.wrap(0), ModePayload.wrap(0)
            )
        );
    }

    function _singleCallBatch(address target, bytes memory callData) internal pure returns (bytes memory) {
        Execution[] memory batch = new Execution[](1);
        batch[0] = Execution({target: target, value: 0, callData: callData});
        return ERC7579Utils.encodeBatch(batch);
    }

    /// @dev Aave's liquidity index grows with `block.timestamp`, so moving the clock accrues interest.
    function _accrueInterestForThirtyDays() internal {
        vm.warp(block.timestamp + 30 days);
    }

    // --------------------------------------------------------------------------------------
    // tests
    // --------------------------------------------------------------------------------------

    /// @notice Shares have the asset's 6 decimals, and 10,000 USDC buys exactly 10,000 shares.
    function test_Shares_areOneToOneWithUsdc() public {
        assertEq(vault.decimals(), 6, "shares should have USDC's 6 decimals");
        assertEq(_depositAs(alice, LARGEST_TEST_DEPOSIT), LARGEST_TEST_DEPOSIT, "10,000 USDC should buy 10,000 shares");
        assertEq(vault.balanceOf(alice), LARGEST_TEST_DEPOSIT, "balance");
        assertEq(vault.totalSupply(), LARGEST_TEST_DEPOSIT, "supply");
    }

    /// @notice Alice gets exactly her principal back and the owner gets the interest that stataUSDC earned
    ///         on her deposit.
    function test_OwnerTakesAllYield_aliceGetsPrincipalOwnerGetsInterest() public {
        uint256 aliceDepositAmount = 10_000 * ONE_USDC;
        _depositAs(alice, aliceDepositAmount);
        uint256 innerSharesHeldByVault = stataUsdc.balanceOf(address(vault));

        _accrueInterestForThirtyDays();

        // The interest the position earned, valued by stataUSDC itself rather than by the vault.
        uint256 expectedInterest = stataUsdc.previewRedeem(innerSharesHeldByVault) - aliceDepositAmount;
        assertGt(expectedInterest, 0, "no interest accrued in 30 days");

        uint256 aliceShares = vault.balanceOf(alice);
        uint256 aliceBalanceBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.redeem(aliceShares, alice, alice);
        uint256 aliceAssetsReceived = usdc.balanceOf(alice) - aliceBalanceBefore;

        // While depositors are whole, shares redeem 1:1, so Alice gets her deposit back to the wei and the
        // interest stays with the owner.
        assertEq(aliceAssetsReceived, aliceDepositAmount, "alice should get exactly her principal");

        uint256 ownerBalanceBefore = usdc.balanceOf(owner);
        uint256 assetsHarvested = _harvestAsOwner();
        assertGt(assetsHarvested, 0, "nothing harvested");
        assertEq(usdc.balanceOf(owner) - ownerBalanceBefore, assetsHarvested, "owner was not paid");
        assertApproxEqAbs(assetsHarvested, expectedInterest, ROUNDING_TOLERANCE, "owner should get the interest");
        assertLe(vault.unharvestedYield(), 2, "a full harvest should leave a wei or two");
    }

    /// @notice Pausing against the real stataUSDC: `maxRedeem` and `maxDeposit` are 0 while paused and `harvest`
    ///         still pays the owner. After `unpause`, Alice redeems exactly her principal.
    function test_PauseThenUnpause_aliceRedeemsExactlyHerPrincipal() public {
        uint256 aliceDepositAmount = LARGEST_TEST_DEPOSIT;
        _depositAs(alice, aliceDepositAmount);
        _accrueInterestForThirtyDays();
        uint256 aliceShares = vault.balanceOf(alice);

        vm.prank(owner);
        vault.pause();
        assertEq(vault.maxRedeem(alice), 0, "maxRedeem while paused");
        assertEq(vault.maxDeposit(alice), 0, "maxDeposit while paused");
        assertGt(_harvestAsOwner(), 0, "harvest should work while paused");

        vm.prank(owner);
        vault.unpause();
        assertEq(vault.maxRedeem(alice), aliceShares, "maxRedeem after unpause");

        uint256 aliceBalanceBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.redeem(aliceShares, alice, alice);
        assertEq(usdc.balanceOf(alice) - aliceBalanceBefore, aliceDepositAmount, "alice should get her principal");
    }

    /// @notice The owner claims stataUSDC's rewards as the vault, the stata holder. The reward campaign has ended, so
    ///         nothing is paid, but the real contract accepts the call and the vault's stata shares are unchanged.
    function test_Execute_claimsStataRewardsAsTheVault() public {
        _depositAs(alice, LARGEST_TEST_DEPOSIT);
        _accrueInterestForThirtyDays();
        uint256 stataSharesBefore = stataUsdc.balanceOf(address(vault));
        uint256 ownerRewardsBefore = IERC20(A_BAS_USDC).balanceOf(owner);

        address[] memory rewards = new address[](1);
        rewards[0] = A_BAS_USDC;
        bytes memory executionData =
            _singleCallBatch(STATA_USDC, abi.encodeCall(IStataRewards.claimRewards, (owner, rewards)));
        vm.prank(owner);
        vault.execute(_batchMode(), executionData);

        assertEq(stataUsdc.balanceOf(address(vault)), stataSharesBefore, "the claim moved stata shares");
        assertGe(IERC20(A_BAS_USDC).balanceOf(owner), ownerRewardsBefore, "the owner's rewards fell");
    }

    /// @notice Moving the vault's stata shares out through `execute` is refused on the real stataUSDC.
    function test_Execute_cannotTransferStataSharesOut() public {
        _depositAs(alice, LARGEST_TEST_DEPOSIT);
        uint256 stataShares = stataUsdc.balanceOf(address(vault));

        bytes memory transferOut = abi.encodeCall(IERC20.transfer, (owner, stataShares));
        bytes memory executionData = _singleCallBatch(STATA_USDC, transferOut);
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(YieldCaptureVault.InnerVaultSharesDecreased.selector, stataShares, 0));
        vault.execute(_batchMode(), executionData);

        assertEq(stataUsdc.balanceOf(address(vault)), stataShares, "the stata shares moved");
    }

    /// @notice Against the real Base USDC: Alice approved the vault without limit and keeps most of her USDC in her
    ///         wallet. The owner's batch cannot spend that approval, because calling the asset is refused.
    function test_Execute_cannotPullDepositorUsdcThroughTheirApproval() public {
        _depositAs(alice, LARGEST_TEST_DEPOSIT);
        uint256 aliceWalletBalance = usdc.balanceOf(alice);
        assertGt(aliceWalletBalance, 0, "setup: alice should keep USDC in her wallet");
        // Real USDC spends even a maximum allowance down, so check that it covers her whole wallet.
        assertGe(usdc.allowance(alice, address(vault)), aliceWalletBalance, "setup: alice approved the vault");

        bytes memory pullUsdc = abi.encodeCall(IERC20.transferFrom, (alice, owner, aliceWalletBalance));
        bytes memory executionData = _singleCallBatch(BASE_USDC, pullUsdc);
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(YieldCaptureVault.ExecuteTargetForbidden.selector, BASE_USDC));
        vault.execute(_batchMode(), executionData);

        assertEq(usdc.balanceOf(alice), aliceWalletBalance, "alice's wallet was drained");
        assertEq(usdc.balanceOf(owner), 0, "the owner got USDC");
    }

    /// @notice `previewRedeem` equals what a real redeem pays, for a partial and then a full redeem.
    function test_PreviewRedeem_matchesExecutedRedeem() public {
        _depositAs(alice, 10_000 * ONE_USDC);
        _accrueInterestForThirtyDays();

        uint256 sharesToRedeemFirst = vault.balanceOf(alice) / 3;
        uint256 previewedFirst = vault.previewRedeem(sharesToRedeemFirst);
        uint256 balanceBeforeFirst = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.redeem(sharesToRedeemFirst, alice, alice);
        assertEq(usdc.balanceOf(alice) - balanceBeforeFirst, previewedFirst, "first redeem differs from preview");

        uint256 sharesToRedeemNext = vault.balanceOf(alice);
        uint256 previewedNext = vault.previewRedeem(sharesToRedeemNext);
        uint256 balanceBeforeNext = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.redeem(sharesToRedeemNext, alice, alice);
        assertEq(usdc.balanceOf(alice) - balanceBeforeNext, previewedNext, "full redeem differs from preview");
    }

    /// @notice The vault accepts exactly what stataUSDC accepts (its supply cap and pause state), also right
    ///         after a deposit, when stataUSDC's rounding leaves depositors a wei or two short. That dust is
    ///         far below the vault's `MAX_SHORTFALL_FOR_DEPOSITS`, so it does not close the vault.
    function test_MaxDeposit_equalsStataUsdcMaxDeposit() public {
        assertEq(vault.maxDeposit(alice), stataUsdc.maxDeposit(address(vault)), "maxDeposit");

        _depositAs(alice, LARGEST_TEST_DEPOSIT);
        assertLe(vault.depositorShortfall(), MAX_STATA_CREDIT_LOSS, "the deposit left more than dust");
        assertEq(vault.maxDeposit(alice), stataUsdc.maxDeposit(address(vault)), "maxDeposit after a deposit");
    }

    /// @notice Shares are 1:1 with USDC, so `maxMint` is stataUSDC's deposit limit in USDC (not stataUSDC's own
    ///         `maxMint`, which counts its non-1:1 shares), and minting it asks for exactly that limit.
    function test_MaxMint_equalsStataUsdcDepositLimit() public view {
        uint256 stataDepositLimit = stataUsdc.maxDeposit(address(vault));
        assertEq(vault.maxMint(alice), stataDepositLimit, "maxMint");
        assertEq(vault.previewMint(vault.maxMint(alice)), stataDepositLimit, "minting maxMint costs the limit");
    }

    /// @notice A holder can redeem their whole balance in the same block as the deposit, with no yield to
    ///         absorb the inner vault's rounding. She gets back the principal minus exactly the shortfall
    ///         stataUSDC's rounding left, which is at most `MAX_STATA_CREDIT_LOSS`.
    function test_RedeemWholeBalance_worksInTheSameBlockAsTheDeposit() public {
        uint256 aliceDepositAmount = LARGEST_TEST_DEPOSIT;
        uint256 aliceShares = _depositAs(alice, aliceDepositAmount);

        assertEq(vault.maxRedeem(alice), aliceShares, "maxRedeem should be the whole balance");
        uint256 shortfall = vault.depositorShortfall();
        assertLe(shortfall, MAX_STATA_CREDIT_LOSS, "stataUSDC lost more than its rounding on the deposit");

        uint256 aliceBalanceBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.redeem(aliceShares, alice, alice);
        uint256 aliceAssetsReceived = usdc.balanceOf(alice) - aliceBalanceBefore;

        assertEq(vault.balanceOf(alice), 0, "alice still holds shares");
        // With `S` shares and the vault worth `S - shortfall`, redeeming all `S` pays
        // `S * (S - shortfall + 1) / (S + 1) = S - shortfall + shortfall / (S + 1)`. The last term is under 1,
        // so the floor is exactly `S - shortfall`: the vault's own pricing costs her nothing more.
        assertEq(aliceAssetsReceived, aliceDepositAmount - shortfall, "alice should lose exactly the shortfall");
    }
}
