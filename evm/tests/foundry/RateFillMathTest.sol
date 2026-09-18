// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;
import "forge-std/Test.sol";
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

contract RateFillMathHarness is IntentsBase {
    constructor() EIP712("IntentGateway", "2") {}

    function quote(
        uint256 escrowInput,
        uint256 requiredOutput,
        uint256 previousCredit,
        uint256 quotedInput,
        uint256 offeredOutput
    ) external pure returns (uint256, uint256, uint256) {
        return _quoteRateFill(escrowInput, requiredOutput, previousCredit, quotedInput, offeredOutput);
    }
}

contract RateFillMathTest is Test {
    RateFillMathHarness internal harness = new RateFillMathHarness();

    function testFuzz_RateBounds(
        uint64 escrowInput,
        uint64 requiredOutput,
        uint64 previousCredit,
        uint64 quotedInput,
        uint64 surplus
    ) public {
        escrowInput = uint64(bound(escrowInput, 1, type(uint64).max));
        requiredOutput = uint64(bound(requiredOutput, 1, type(uint64).max));
        previousCredit = uint64(bound(previousCredit, 0, requiredOutput - 1));
        quotedInput = uint64(bound(quotedInput, 1, type(uint64).max));
        uint256 offeredOutput = Math.mulDiv(quotedInput, requiredOutput, escrowInput, Math.Rounding.Ceil) + surplus;
        uint256 expectedCredit =
            Math.min(Math.mulDiv(quotedInput, requiredOutput, escrowInput), requiredOutput - previousCredit);
        uint256 expectedRelease = Math.mulDiv(escrowInput, previousCredit + expectedCredit, requiredOutput)
            - Math.mulDiv(escrowInput, previousCredit, requiredOutput);
        if (expectedCredit == 0 || expectedRelease == 0) {
            vm.expectRevert(IntentsBase.RateFillTooSmall.selector);
            harness.quote(escrowInput, requiredOutput, previousCredit, quotedInput, offeredOutput);
            return;
        }
        (uint256 creditedOutput, uint256 releasedInput, uint256 deliveredOutput) =
            harness.quote(escrowInput, requiredOutput, previousCredit, quotedInput, offeredOutput);
        assertGt(creditedOutput, 0);
        assertLe(previousCredit + creditedOutput, requiredOutput);
        assertLe(releasedInput, quotedInput);
        assertGe(deliveredOutput, creditedOutput);
        assertLe(deliveredOutput, offeredOutput);
        assertGe(deliveredOutput * quotedInput, offeredOutput * releasedInput);
        assertEq(
            Math.mulDiv(escrowInput, previousCredit, requiredOutput) + releasedInput
                + (escrowInput - Math.mulDiv(escrowInput, previousCredit + creditedOutput, requiredOutput)),
            escrowInput
        );
    }

    function testRate_CappedQuotePaysLaterSurplus() public view {
        (uint256 credit, uint256 released, uint256 paid) = harness.quote(1000, 100, 60, 1000, 120);
        assertEq(credit, 40);
        assertEq(released, 400);
        assertEq(paid, 48);
        assertEq(paid - credit, 8);
    }

    function testRate_UncappedPaymentUsesReleasedInput() public view {
        (uint256 credit, uint256 released, uint256 paid) = harness.quote(10, 3, 0, 4, 4);
        assertEq(credit, 1);
        assertEq(released, 3);
        assertEq(paid, 3);
    }

    function testRate_PaymentCoversCreditedOutputAfterQuantization() public view {
        (uint256 credit, uint256 released, uint256 paid) = harness.quote(3, 10, 0, 2, 7);
        assertEq(credit, 6);
        assertEq(released, 1);
        assertEq(paid, 6);
    }

    function testRate_OversizedCapacityPaysOnlyForReleasedInput() public view {
        (uint256 credit, uint256 released, uint256 paid) = harness.quote(10, 3, 0, 11, 11);
        assertEq(credit, 3);
        assertEq(released, 10);
        assertEq(paid, 10);
    }

    function testRate_EqualRationalRatesSettleIdenticallyAcrossCapacityBoundary() public view {
        (uint256 exactCredit, uint256 exactRelease, uint256 exactPayment) = harness.quote(10, 3, 0, 11, 11);
        (uint256 oversizedCredit, uint256 oversizedRelease, uint256 oversizedPayment) = harness.quote(10, 3, 0, 20, 20);
        assertEq(exactCredit, 3);
        assertEq(oversizedCredit, exactCredit);
        assertEq(exactRelease, 10);
        assertEq(oversizedRelease, exactRelease);
        assertEq(exactPayment, 10);
        assertEq(oversizedPayment, exactPayment);
    }

    function testRate_MaximumAmountsCappedCompletion() public view {
        (uint256 credit, uint256 released, uint256 paid) = harness.quote(
            type(uint256).max, type(uint256).max - 1, type(uint256).max - 2, type(uint256).max, type(uint256).max
        );
        assertEq(credit, 1);
        assertEq(released, 2);
        assertEq(paid, 2);
    }

    function testRate_MaximumAmountsUseFullPrecision() public view {
        (uint256 creditedOutput, uint256 releasedInput, uint256 deliveredOutput) =
            harness.quote(type(uint256).max, type(uint256).max, 0, type(uint256).max, type(uint256).max);
        assertEq(creditedOutput, type(uint256).max);
        assertEq(releasedInput, type(uint256).max);
        assertEq(deliveredOutput, type(uint256).max);
    }
}
