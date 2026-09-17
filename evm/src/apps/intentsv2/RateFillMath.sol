// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

/// @dev No fixed-point rates: the solver signs input/output raw amounts.
library RateFillMath {
    error RateBelowOrder();
    error RateFillTooSmall();

    function quote(uint256 escrow, uint256 required, uint256 filled, uint256 take, uint256 offered)
        internal
        pure
        returns (uint256 credit, uint256 release, uint256 delivered)
    {
        if (escrow == 0 || required == 0 || take == 0 || offered == 0) revert RateFillTooSmall();
        if (Math.mulDiv(take, required, escrow, Math.Rounding.Ceil) > offered) revert RateBelowOrder();
        uint256 uncapped = Math.mulDiv(take, required, escrow);
        uint256 remaining = required - filled;
        credit = Math.min(uncapped, remaining);
        release = Math.mulDiv(escrow, filled + credit, required) - Math.mulDiv(escrow, filled, required);
        if (credit == 0 || release == 0) revert RateFillTooSmall();
        delivered =
            uncapped > remaining ? Math.max(credit, Math.mulDiv(offered, release, take, Math.Rounding.Ceil)) : offered;
    }
}
