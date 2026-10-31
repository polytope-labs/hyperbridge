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

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC4626} from "@openzeppelin/contracts/interfaces/IERC4626.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ERC7821} from "@openzeppelin/contracts/account/extensions/draft-ERC7821.sol";
import {ERC7579Utils} from "@openzeppelin/contracts/account/utils/draft-ERC7579Utils.sol";
import {Execution} from "@openzeppelin/contracts/interfaces/draft-IERC7579.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

/**
 * @title YieldCaptureVault
 * @author Polytope Labs (hello@polytope.technology)
 * @notice An ERC-4626 vault that wraps another ERC-4626 vault (Aave's stataUSDC in production). Depositors
 * get their principal back and all of the yield goes to the owner, paid by `harvest`.
 *
 * @dev Shares are 1:1 with the asset, so `totalSupply()` is what depositors are owed and anything the vault
 * holds above it is owner yield. A loss first eats the unharvested yield; beyond that, redemptions are
 * pro-rata and deposits close until yield refills the shortfall. The owner can pause deposits, withdrawals
 * and share transfers, and ownership cannot be renounced. The owner can also `execute` batched calls as the
 * vault (e.g. to claim rewards), guarded so it can neither take depositors' principal nor spend the approvals
 * the vault asks for (the asset and its own shares). `harvest` and `execute` still work while paused, though
 * share transfers don't. The inner vault and the asset are trusted: no fees, no rebasing, no reentrancy.
 */
contract YieldCaptureVault is ERC4626, Ownable, Pausable, ERC7821 {
    using SafeERC20 for IERC20;

    /// @notice Shortfall (asset wei) above which deposits close; absorbs rounding dust
    uint256 public constant MAX_SHORTFALL_FOR_DEPOSITS = 1_000;

    /// @notice The wrapped ERC-4626 vault that holds all deposits
    address public immutable innerVault;

    /// @notice Thrown when the inner vault would mint no shares for a deposit, including a zero deposit
    error DepositTooSmallForInnerVault();

    /// @notice Thrown on any attempt to renounce ownership
    error OwnershipCannotBeRenounced();

    /// @notice Thrown when an `execute` batch calls the vault itself or the asset
    error ExecuteTargetForbidden(address target);

    /// @notice Thrown when an `execute` batch calls `transferFrom`, which could only spend others' approvals
    error TransferFromForbidden();

    /// @notice Thrown when an `execute` batch approves the inner-vault shares
    error CannotApproveInnerVaultShares();

    /// @notice Thrown when an `execute` batch leaves the vault with fewer inner-vault shares
    error InnerVaultSharesDecreased(uint256 innerSharesBefore, uint256 innerSharesAfter);

    /// @notice Emitted when the unharvested yield is redeemed and paid out
    event YieldHarvested(address indexed recipient, uint256 innerShares, uint256 assets);

    /**
     * @notice Deploys the vault over `innerVault_` and approves it to pull the asset
     * @param initialOwner Owns the vault and receives the yield; cannot be zero
     */
    constructor(address innerVault_, string memory name_, string memory symbol_, address initialOwner)
        ERC20(name_, symbol_)
        ERC4626(IERC20(IERC4626(innerVault_).asset()))
        Ownable(initialOwner)
    {
        innerVault = innerVault_;

        // The inner vault is fixed and trusted, so an unlimited allowance is safe.
        IERC20(IERC4626(innerVault_).asset()).forceApprove(innerVault_, type(uint256).max);
    }

    /**
     * @notice What the vault's inner-vault shares would redeem for now
     */
    function innerVaultValue() public view returns (uint256) {
        return IERC4626(innerVault).previewRedeem(IERC4626(innerVault).balanceOf(address(this)));
    }

    /**
     * @notice Depositors' value: one asset per share, capped at what the vault holds
     * @dev Capping at `totalSupply()` keeps the owner's yield and donations out of the share price.
     */
    function totalAssets() public view override returns (uint256) {
        return Math.min(innerVaultValue(), totalSupply());
    }

    /**
     * @notice Value held in the inner vault above `totalSupply()`; the owner's to harvest
     */
    function unharvestedYield() public view returns (uint256) {
        uint256 currentInnerVaultValue = innerVaultValue();
        uint256 assetsOwedToDepositors = totalSupply();
        return currentInnerVaultValue > assetsOwedToDepositors ? currentInnerVaultValue - assetsOwedToDepositors : 0;
    }

    /**
     * @notice How far the inner vault's value is below `totalSupply()`
     */
    function depositorShortfall() public view returns (uint256) {
        uint256 currentInnerVaultValue = innerVaultValue();
        uint256 assetsOwedToDepositors = totalSupply();
        return assetsOwedToDepositors > currentInnerVaultValue ? assetsOwedToDepositors - currentInnerVaultValue : 0;
    }

    /**
     * @notice Always 1:1, so nobody can buy shares below par
     */
    function previewDeposit(uint256 assets) public pure override returns (uint256) {
        return assets;
    }

    /**
     * @notice Always 1:1, so nobody can buy shares below par
     */
    function previewMint(uint256 shares) public pure override returns (uint256) {
        return shares;
    }

    /**
     * @notice Sends the assets straight into the inner vault
     * @dev The inner vault may credit up to 2 wei less than `assets`; that comes out of unharvested yield, or is a
     * dust shortfall if there is none.
     */
    function _deposit(address caller, address receiver, uint256 assets, uint256 shares) internal override {
        // The stataToken reverts on zero shares; fail with a clear error instead.
        if (IERC4626(innerVault).previewDeposit(assets) == 0) revert DepositTooSmallForInnerVault();

        IERC20(asset()).safeTransferFrom(caller, address(this), assets);
        IERC4626(innerVault).deposit(assets, address(this));
        _mint(receiver, shares);

        emit Deposit(caller, receiver, assets, shares);
    }

    /**
     * @notice Pays from the inner vault straight to `receiver`
     * @dev Burning the shares lowers `totalSupply()`, which is what depositors are owed, so nothing else
     * needs updating.
     */
    function _withdraw(address caller, address receiver, address holder, uint256 assets, uint256 shares)
        internal
        override
    {
        if (caller != holder) _spendAllowance(holder, caller, shares);
        _burn(holder, shares);

        // Aave reverts on zero amounts.
        if (assets != 0) IERC4626(innerVault).withdraw(assets, receiver, address(this));

        emit Withdraw(caller, receiver, holder, assets, shares);
    }

    /**
     * @notice Reverts every mint, burn and transfer of shares while paused
     * @dev One hook covers deposits, withdrawals and share transfers.
     */
    function _update(address from, address to, uint256 value) internal override whenNotPaused {
        super._update(from, to, value);
    }

    /**
     * @notice Zero while paused or while depositors are more than `MAX_SHORTFALL_FOR_DEPOSITS` short; otherwise
     * the inner vault's limit
     */
    function maxDeposit(address) public view override returns (uint256) {
        if (paused() || depositorShortfall() > MAX_SHORTFALL_FOR_DEPOSITS) return 0;
        return IERC4626(innerVault).maxDeposit(address(this));
    }

    /**
     * @notice Same as `maxDeposit`, since shares are 1:1
     */
    function maxMint(address receiver) public view override returns (uint256) {
        return maxDeposit(receiver);
    }

    /**
     * @notice The holder's balance, capped by what the inner vault can pay out now; zero while paused
     * @dev Checks the full balance first; converting the limit to shares rounds down and could leave the
     * holder a share short.
     */
    function maxRedeem(address holder) public view override returns (uint256) {
        if (paused()) return 0;

        uint256 holderShares = balanceOf(holder);
        uint256 innerVaultWithdrawLimit = IERC4626(innerVault).maxWithdraw(address(this));

        if (previewRedeem(holderShares) <= innerVaultWithdrawLimit) return holderShares;

        // Round down so the result is always payable.
        return Math.min(holderShares, _convertToShares(innerVaultWithdrawLimit, Math.Rounding.Floor));
    }

    /**
     * @notice Sends the unharvested yield to whoever is the owner when it is called, up to the inner vault's
     * available liquidity
     * @dev Returns 0 when the inner vault has nothing redeemable (no liquidity, or a paused reserve). Works
     * while paused: it never touches depositors' backing.
     */
    function harvest() external onlyOwner returns (uint256 assetsSentToOwner) {
        // Round down so the vault always keeps at least totalSupply() in value.
        uint256 innerSharesToRedeem = Math.min(
            IERC4626(innerVault).convertToShares(unharvestedYield()), IERC4626(innerVault).maxRedeem(address(this))
        );
        if (innerSharesToRedeem == 0 || IERC4626(innerVault).previewRedeem(innerSharesToRedeem) == 0) return 0;

        address recipient = owner();
        assetsSentToOwner = IERC4626(innerVault).redeem(innerSharesToRedeem, recipient, address(this));

        emit YieldHarvested(recipient, innerSharesToRedeem, assetsSentToOwner);
    }

    /**
     * @notice Owner-only batch of calls made as the vault, e.g. to claim Aave incentives
     * @dev Refuses the asset as a target, any `transferFrom`, `approve` on the inner vault, and calls to the vault
     * itself except `transfer` (to recover shares sent to it), and reverts if the vault ends with fewer inner-vault
     * shares. So it can't take depositors' principal or spend the approvals the vault asks for (the asset and its
     * own shares). Allowances users grant the vault for any other token are not protected, so they shouldn't grant
     * any. Stray asset: deposit it into the inner vault, then `harvest`. Works while paused, but share transfers don't.
     */
    function execute(bytes32 mode, bytes calldata executionData) public payable override onlyOwner {
        Execution[] calldata executions = ERC7579Utils.decodeBatch(executionData);
        for (uint256 executionIndex = 0; executionIndex < executions.length; executionIndex++) {
            address target = executions[executionIndex].target;
            bytes calldata callData = executions[executionIndex].callData;

            bytes4 selector = callData.length >= 4 ? bytes4(callData[:4]) : bytes4(0);

            // The zero address is an alias for the vault in ERC-7579 batches. As the vault, `transfer` moves only
            // its own shares; any other call to itself could reach other people's.
            bool isSelfCall = target == address(0) || target == address(this);
            if (target == asset() || (isSelfCall && selector != IERC20.transfer.selector)) {
                revert ExecuteTargetForbidden(target);
            }

            // As the vault, `transferFrom` could only spend allowances other people gave it.
            if (selector == IERC20.transferFrom.selector) revert TransferFromForbidden();
            // An allowance would let someone pull the shares after the batch, which the check below can't see.
            if (target == innerVault && selector == IERC20.approve.selector) revert CannotApproveInnerVaultShares();
        }

        uint256 preBalance = IERC4626(innerVault).balanceOf(address(this));
        super.execute(mode, executionData);
        uint256 postBalance = IERC4626(innerVault).balanceOf(address(this));

        if (postBalance < preBalance) {
            revert InnerVaultSharesDecreased(preBalance, postBalance);
        }
    }

    /**
     * @notice Pauses deposits, withdrawals and share transfers
     * @dev `harvest` and `execute` keep working. A paused vault freezes exits, so the owner should be a multisig.
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @notice Resumes deposits, withdrawals and share transfers
     */
    function unpause() external onlyOwner {
        _unpause();
    }

    /**
     * @notice Always reverts: a renounced owner could never unpause, or collect yield
     */
    function renounceOwnership() public pure override {
        revert OwnershipCannotBeRenounced();
    }

    /**
     * @notice Only the owner may call `execute`
     */
    function _erc7821AuthorizedExecutor(address caller, bytes32, bytes calldata) internal view override returns (bool) {
        return caller == owner();
    }
}
