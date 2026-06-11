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
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

/// @title StreamingYieldVault
/// @author Polytope Labs (hello@polytope.technology)
/// @notice An ERC-4626 vault whose yield is supplied by the owner via periodic transfers
///         (`addYield`) and recognized linearly over a fixed window (`VEST`). Because yield
///         is streamed rather than recognized instantly, no single block can be sandwiched
///         around a yield event ("yield sniping"): a same-block deposit/withdraw sees an
///         unchanged share price and captures nothing.
///
/// @dev Design notes:
///      - The exchange rate is `(balanceOf(this) - lockedYield) / totalSupply`. Yield that has
///        not yet vested is masked out of `totalAssets`, so it cannot be claimed early.
///      - The owner may `pause()` to freeze all share movement (transfers, deposits, withdrawals)
///        in an emergency. Yield may still be added while paused; it simply cannot be withdrawn.
///      - The first-depositor inflation/donation attack is mitigated by OpenZeppelin's virtual
///        shares/assets via a non-zero `_decimalsOffset()`. Seed-and-burn at deployment as well.
///      - No reentrancy guard is used: the only external calls are `transfer`/`transferFrom` on
///        the asset, and OpenZeppelin's ERC4626 orders effects before interactions. This assumes
///        a standard, non-rebasing, non-fee-on-transfer, non-hooked (no ERC-777 callbacks) asset.
contract StreamingYieldVault is ERC4626, Pausable, Ownable {
    using SafeERC20 for IERC20;

    /// @notice Window over which each yield tranche is linearly recognized.
    /// @dev Keep `VEST <= the cadence at which `addYield` is called`. For ~24h cadence, 23h
    ///      maximizes anti-snipe protection while leaving margin for keeper jitter.
    uint256 public constant VEST = 23 hours;

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

    /// @notice The portion of the current tranche that has not yet been recognized.
    function lockedYield() external view returns (uint256) {
        return _lockedYield();
    }

    /// @notice The timestamp at which the current tranche finishes vesting, which is also the
    ///         earliest time `addYield` may be called again.
    function vestedAt() external view returns (uint256) {
        return _vestingStart + VEST;
    }

    /// @notice Add a new yield tranche, pulled from the caller. Reverts unless the previous
    ///         tranche has fully vested, which guarantees tranches never overlap and no yield
    ///         is ever left permanently locked.
    /// @param amount The amount of `asset` to stream in over `VEST`.
    function addYield(uint256 amount) external onlyOwner {
        if (amount == 0) revert ZeroAmount();

        uint256 start = _vestingStart;
        if (start != 0 && block.timestamp < start + VEST) {
            revert YieldStillVesting(start + VEST);
        }

        // Pull the funds first so `balanceOf` already reflects `amount` before it is marked
        // locked; otherwise `totalAssets` would transiently underflow when a tranche exceeds
        // the current backing (e.g. the very first `addYield` on a near-empty vault).
        IERC20(asset()).safeTransferFrom(msg.sender, address(this), amount);

        _vestingAmount = amount;
        _vestingStart = block.timestamp;

        emit YieldAdded(amount, block.timestamp);
    }

    /// @notice Freeze all share movement (transfers, deposits, withdrawals) in an emergency.
    function pause() external onlyOwner {
        _pause();
    }

    /// @notice Resume share movement.
    function unpause() external onlyOwner {
        _unpause();
    }

    /// @dev Linear unlock of the current tranche, keyed on `block.timestamp` so that a deposit
    ///      and withdrawal within the same block observe an identical, unchanged share price.
    function _lockedYield() internal view returns (uint256) {
        uint256 start = _vestingStart;
        uint256 elapsed = block.timestamp - start;
        if (elapsed >= VEST) return 0;
        return (_vestingAmount * (VEST - elapsed)) / VEST;
    }

    /// @inheritdoc ERC4626
    function _decimalsOffset() internal pure override returns (uint8) {
        return DECIMALS_OFFSET;
    }

    /// @dev Single chokepoint for all share movement (mint, burn, transfer); gated by the pause.
    function _update(address from, address to, uint256 value) internal override whenNotPaused {
        super._update(from, to, value);
    }
}
