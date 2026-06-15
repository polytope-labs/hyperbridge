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

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @title StreamingYieldVault
/// @author Polytope Labs (hello@polytope.technology)
/// @notice An ERC-4626 vault whose yield is supplied by the owner via periodic transfers
///         (`addYield`) and recognized linearly over a fixed window (`VEST`). Because yield
///         is streamed rather than recognized instantly, no single block can be sandwiched
///         around a yield event ("yield sniping"): a same-block deposit/withdraw sees an
///         unchanged share price and captures nothing.
///
/// @dev The exchange rate is `(balanceOf(this) - lockedYield) / totalSupply`. Yield that has
///      not yet vested is masked out of `totalAssets`, so it cannot be claimed early.
///
///      Deposits and mints are disabled while a tranche is vesting. New capital may only enter
///      in the window after a tranche fully vests and before the next `addYield`, so no one can
///      join mid-tranche and capture yield meant for the holders present when it began. `addYield`
///      must wait `MIN_WINDOW` past the vest end, guaranteeing that window exists every cycle
///      regardless of keeper timing.
contract StreamingYieldVault is ERC4626, Ownable {
    using SafeERC20 for IERC20;

    /// @notice Window over which each yield tranche is linearly recognized. Deposits and mints are
    ///         disabled while a tranche vests (`maxDeposit`/`maxMint` report 0), so `VEST` must end
    ///         before the next `addYield` to leave a window for new capital to enter. With
    ///         `MIN_WINDOW = 2h`, a 22h vest yields a 24h minimum cadence and a guaranteed 2h
    ///         deposit window each cycle.
    uint256 public constant VEST = 22 hours;

    /// @notice Minimum time `addYield` must wait after the current tranche finishes vesting before
    ///         it may start the next one. Because `addYield` cannot fire during this stretch, every
    ///         cycle has a guaranteed deposit window of at least `MIN_WINDOW`, independent of how
    ///         eagerly the keeper runs. Minimum cadence is therefore `VEST + MIN_WINDOW`.
    uint256 public constant MIN_WINDOW = 2 hours;

    /// @dev Virtual-share offset hardening the first-depositor inflation attack. Shares carry
    ///      `assetDecimals + DECIMALS_OFFSET` decimals.
    uint8 private constant DECIMALS_OFFSET = 6;

    /// @dev The size of the yield tranche currently being recognized.
    uint256 private _vestingAmount;

    /// @dev The timestamp at which the current tranche started vesting. Zero means no tranche
    ///      has ever been added.
    uint256 private _vestingStart;

    /// @notice Thrown when `addYield` is called before the previous tranche has fully vested.
    error YieldStillVesting(uint256 vestedAt);

    /// @notice Thrown when `addYield` is called with a zero amount.
    error ZeroAmount();

    /// @notice Thrown when `addYield` is called during the guaranteed deposit window, before
    ///         `MIN_WINDOW` has elapsed past the end of the previous tranche's vesting.
    error DepositWindowOpen(uint256 closesAt);

    /// @notice Emitted when a new yield tranche begins vesting.
    event YieldAdded(uint256 amount, uint256 vestingStart);

    constructor(IERC20 asset_, string memory name_, string memory symbol_, address owner_)
        ERC20(name_, symbol_)
        ERC4626(asset_)
        Ownable(owner_)
    {}

    /// @inheritdoc ERC4626
    /// @notice Total assets backing shares, net of any not-yet-vested yield.
    function totalAssets() public view override returns (uint256) {
        return IERC20(asset()).balanceOf(address(this)) - _lockedYield();
    }
    
    /// @inheritdoc ERC4626
    function _decimalsOffset() internal pure override returns (uint8) {
        return DECIMALS_OFFSET;
    }

    /// @notice The portion of the current tranche that has not yet been recognized.
    function lockedYield() external view returns (uint256) {
        return _lockedYield();
    }

    /// @notice The timestamp at which the current tranche finishes vesting. Deposits open at this
    ///         time; `addYield` may only be called from `nextYieldAt()` onward.
    function vestedAt() external view returns (uint256) {
        return _vestingStart + VEST;
    }

    /// @notice The earliest time the next `addYield` may be called. The interval
    ///         `[vestedAt(), nextYieldAt()]` is the guaranteed deposit window for each cycle.
    function nextYieldAt() external view returns (uint256) {
        return _vestingStart + VEST + MIN_WINDOW;
    }

    /// @dev True while the current tranche is still vesting, i.e. deposits are locked. Returns
    ///      false before the first tranche has ever been added (`_vestingStart == 0`).
    function _isVesting() internal view returns (bool) {
        uint256 start = _vestingStart;
        return start != 0 && block.timestamp < start + VEST;
    }

    /// @inheritdoc ERC4626
    /// @dev Zero while a tranche is vesting so deposits are closed (and integrators can detect it);
    ///      unbounded otherwise. This is the single lock that keeps new capital from joining
    ///      mid-tranche: `deposit` reverts at its `maxDeposit` check with `ERC4626ExceededMaxDeposit`.
    function maxDeposit(address) public view override returns (uint256) {
        return _isVesting() ? 0 : type(uint256).max;
    }

    /// @inheritdoc ERC4626
    /// @dev Zero while a tranche is vesting so integrators see mints are closed; unbounded otherwise.
    function maxMint(address) public view override returns (uint256) {
        return _isVesting() ? 0 : type(uint256).max;
    }

    /// @dev Linear unlock of the current tranche, keyed on `block.timestamp` so that a deposit
    ///      and withdrawal within the same block observe an identical, unchanged share price.
    function _lockedYield() internal view returns (uint256) {
        uint256 start = _vestingStart;
        uint256 elapsed = block.timestamp - start;
        if (elapsed >= VEST) return 0;
        return (_vestingAmount * (VEST - elapsed)) / VEST;
    }
    
    /// @notice Add a new yield tranche, pulled from the caller. Reverts unless the previous
    ///         tranche has fully vested, which guarantees tranches never overlap and no yield
    ///         is ever left permanently locked.
    /// @param amount The amount of `asset` to stream in over `VEST`.
    function addYield(uint256 amount) external onlyOwner {
        if (amount == 0) revert ZeroAmount();

        uint256 start = _vestingStart;
        if (start != 0) {
            if (block.timestamp < start + VEST) revert YieldStillVesting(start + VEST);
            // Hold off until the guaranteed deposit window has elapsed, so new capital always has
            // a chance to enter between tranches regardless of how promptly the keeper runs.
            if (block.timestamp < start + VEST + MIN_WINDOW) revert DepositWindowOpen(start + VEST + MIN_WINDOW);
        }

        // Pull the funds first so `balanceOf` already reflects `amount` before it is marked
        // locked; otherwise `totalAssets` would transiently underflow when a tranche exceeds
        // the current backing (e.g. the very first `addYield` on a near-empty vault).
        IERC20(asset()).safeTransferFrom(msg.sender, address(this), amount);

        _vestingAmount = amount;
        _vestingStart = block.timestamp;

        emit YieldAdded(amount, block.timestamp);
    }
}
