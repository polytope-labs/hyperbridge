// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;
import "forge-std/Test.sol";
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

contract RateFillMathHarness is IntentsBase {
    constructor() EIP712("IntentGateway", "2") {}

    function quote(uint256 e, uint256 q, uint256 p, uint256 t, uint256 o)
        external
        pure
        returns (uint256, uint256, uint256)
    {
        return _quoteRateFill(e, q, p, t, o);
    }
}

contract RateFillMathTest is Test {
    RateFillMathHarness internal harness = new RateFillMathHarness();

    function testFuzz_RateBounds(uint64 e, uint64 q, uint64 p, uint64 t, uint64 surplus) public {
        e = uint64(bound(e, 1, type(uint64).max));
        q = uint64(bound(q, 1, type(uint64).max));
        p = uint64(bound(p, 0, q - 1));
        t = uint64(bound(t, 1, type(uint64).max));
        uint256 offered = Math.mulDiv(t, q, e, Math.Rounding.Ceil) + surplus;
        uint256 expectedCredit = Math.min(Math.mulDiv(t, q, e), q - p);
        uint256 expectedRelease = Math.mulDiv(e, p + expectedCredit, q) - Math.mulDiv(e, p, q);
        if (expectedCredit == 0 || expectedRelease == 0) {
            vm.expectRevert(IntentsBase.RateFillTooSmall.selector);
            harness.quote(e, q, p, t, offered);
            return;
        }
        (uint256 credit, uint256 release, uint256 delivered) = harness.quote(e, q, p, t, offered);
        assertGt(credit, 0);
        assertLe(p + credit, q);
        assertLe(release, t);
        assertGe(delivered, credit);
        assertLe(delivered, offered);
        assertGe(delivered * t, offered * release);
        assertEq(Math.mulDiv(e, p, q) + release + (e - Math.mulDiv(e, p + credit, q)), e);
    }

    function testRate_MaximumAmountsUseFullPrecision() public view {
        (uint256 c, uint256 r, uint256 d) =
            harness.quote(type(uint256).max, type(uint256).max, 0, type(uint256).max, type(uint256).max);
        assertEq(c, type(uint256).max);
        assertEq(r, type(uint256).max);
        assertEq(d, type(uint256).max);
    }
}
