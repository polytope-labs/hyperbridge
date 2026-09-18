// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {IntentGatewayV2SameChainTest} from "./IntentGatewayV2SameChainTest.sol";
import {
    Order,
    FillOptions,
    TokenInfo,
    CancelOptions,
    PaymentInfo,
    DispatchInfo
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {Call} from "@hyperbridge/core/interfaces/ICallDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract IntentGatewayRateFillTest is IntentGatewayV2SameChainTest {
    function _rateFill(Order memory order, uint256 take, uint256 offered) internal {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(order.inputs[0].token, take);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, offered);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), offered);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, inputs));
        vm.stopPrank();
    }

    function testRate_PartialSurplusAndCappedCompletion() public {
        Order memory order = _placeSameChainOrder(1000, 1000, 0);
        uint256 userBefore = dai.balanceOf(user);
        uint256 solverBefore = usdc.balanceOf(solver);
        _rateFill(order, 800, 880);
        bytes32 commitment = keccak256(abi.encode(order));
        assertEq(intentGateway._partialFills(commitment, 0), 800);
        assertEq(usdc.balanceOf(solver) - solverBefore, 800);
        assertEq(dai.balanceOf(user) - userBefore, 840);
        uint256 outputBefore = dai.balanceOf(solver);
        _rateFill(order, 500, 550);
        assertEq(outputBefore - dai.balanceOf(solver), 220);
        assertEq(usdc.balanceOf(solver) - solverBefore, 1000);
        assertEq(dai.balanceOf(user) - userBefore, 1050);
        assertEq(dai.balanceOf(address(intentGateway)), 50);
        assertEq(intentGateway._orders(commitment, 0), 0);
    }

    function testRate_QuantizedInputRelease() public {
        Order memory order = _placeSameChainOrder(10, 3, 0);
        uint256 before = usdc.balanceOf(solver);
        _rateFill(order, 4, 2);
        assertEq(usdc.balanceOf(solver) - before, 3);
        assertEq(intentGateway._partialFills(keccak256(abi.encode(order)), 0), 1);
        _rateFill(order, 10, 3);
        assertEq(usdc.balanceOf(solver) - before, 10);
    }

    function testRate_UniformPaymentTransfersAndCancelConserveEscrow() public {
        Order memory order = _placeSameChainOrder(10, 3, 0);
        bytes32 commitment = keccak256(abi.encode(order));
        uint256 solverInputBefore = usdc.balanceOf(solver);
        uint256 solverOutputBefore = dai.balanceOf(solver);
        uint256 beneficiaryBefore = dai.balanceOf(user);
        uint256 protocolBefore = dai.balanceOf(address(intentGateway));

        _rateFill(order, 4, 4);

        assertEq(usdc.balanceOf(solver) - solverInputBefore, 3);
        assertEq(solverOutputBefore - dai.balanceOf(solver), 3);
        assertEq(intentGateway._partialFills(commitment, 0), 1);
        // Surplus is two; the configured 50/50 split pays one to each side.
        assertEq(dai.balanceOf(user) - beneficiaryBefore, 2);
        assertEq(dai.balanceOf(address(intentGateway)) - protocolBefore, 1);

        uint256 userInputBefore = usdc.balanceOf(user);
        vm.prank(user);
        intentGateway.cancelOrder(order, CancelOptions(0, 0));
        assertEq(usdc.balanceOf(user) - userInputBefore, 7);
        assertEq(intentGateway._orders(commitment, 0), 0);
    }

    function testRate_DifferentSolversAndRatesConserveEscrowOnCancellation() public {
        Order memory order = _placeSameChainOrder(1000, 100, 0);
        uint256 firstBefore = usdc.balanceOf(solver);
        _rateFill(order, 300, 36);
        uint256 secondBefore = usdc.balanceOf(otherUser);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 400);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 60);
        deal(address(dai), otherUser, 60);
        vm.startPrank(otherUser);
        dai.approve(address(intentGateway), 60);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        vm.stopPrank();
        bytes32 commitment = keccak256(abi.encode(order));
        uint256 released = usdc.balanceOf(solver) - firstBefore + usdc.balanceOf(otherUser) - secondBefore;
        assertEq(released, 700);
        assertEq(intentGateway._partialFills(commitment, 0), 70);
        assertEq(released + intentGateway._orders(commitment, 0), 1000);
        uint256 userBefore = usdc.balanceOf(user);
        vm.prank(user);
        intentGateway.cancelOrder(order, CancelOptions(0, 0));
        assertEq(released + usdc.balanceOf(user) - userBefore, 1000);
        assertEq(intentGateway._orders(commitment, 0), 0);
        assertEq(dai.balanceOf(address(intentGateway)), 13, "surplus retained on both rates");
    }

    function testRate_DustQuoteRejectedWithoutProgress() public {
        Order memory order = _placeSameChainOrder(1000, 100, 0);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 1);
        vm.prank(solver);
        vm.expectRevert(IntentsBase.RateFillTooSmall.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        bytes32 commitment = keccak256(abi.encode(order));
        assertEq(intentGateway._partialFills(commitment, 0), 0);
        assertEq(intentGateway._orders(commitment, 0), 1000);
    }

    function testRate_BelowFloorRejected() public {
        Order memory order = _placeSameChainOrder(1000, 1000, 0);
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(order.inputs[0].token, 400);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 399);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 399);
        vm.expectRevert(IntentsBase.RateBelowOrder.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, inputs));
        vm.stopPrank();
    }

    function testRate_EqualPriceFullFill() public {
        Order memory order = _placeSameChainOrder(100, 100, 0);
        uint256 before = usdc.balanceOf(solver);
        _rateFill(order, 100, 100);
        assertEq(usdc.balanceOf(solver) - before, 100);
        assertEq(intentGateway._orders(keccak256(abi.encode(order)), 0), 0);
    }

    function testRate_EmptyInputsRejected() public {
        Order memory order = _placeSameChainOrder(100, 100, 0);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 100);
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, order.output.assets, new TokenInfo[](0)));
        vm.stopPrank();
        assertEq(intentGateway._orders(keccak256(abi.encode(order)), 0), 100);
    }

    function testRate_NativeInputReleaseAndCancellation() public {
        Order memory order = _placeNativeOrder(user, user, 1000, block.number + 100);
        uint256 before = solver.balance;
        _rateFill(order, 400, order.output.assets[0].amount);
        assertEq(solver.balance - before, 400);
        uint256 userBefore = user.balance;
        vm.prank(user);
        intentGateway.cancelOrder(order, CancelOptions(0, 0));
        assertEq(user.balance - userBefore, 600);
    }

    function testRate_PausedAndExpiredQuoteRejected() public {
        Order memory order = _placeSameChainOrder(100, 100, 0);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 100);
        intentGateway.pause();
        vm.expectRevert(bytes4(keccak256("EnforcedPause()")));
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, order.output.assets, takes));
        intentGateway.unpause();
        vm.expectRevert(bytes4(keccak256("FillExpired()")));
        intentGateway.fillOrder(order, FillOptions(0, 0, block.number - 1, order.output.assets, takes));
    }

    function testRate_CappedNativeOutputRefundsUnusedValue() public {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(bytes32(uint256(uint160(address(usdc)))), 1000);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(bytes32(0), 1000);
        Order memory order = Order(
            bytes32(uint256(uint160(user))),
            host.host(),
            host.host(),
            block.number + 100,
            0,
            0,
            address(0),
            DispatchInfo(new TokenInfo[](0), ""),
            inputs,
            PaymentInfo(bytes32(uint256(uint160(user))), outputs, "")
        );
        vm.startPrank(user);
        usdc.approve(address(intentGateway), 1000);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(inputs[0].token, 800);
        TokenInfo[] memory offered = new TokenInfo[](1);
        offered[0] = TokenInfo(bytes32(0), 880);
        uint256 before = solver.balance;
        vm.prank(solver);
        intentGateway.fillOrder{value: 1000}(order, FillOptions(0, 0, 0, offered, takes));
        assertEq(before - solver.balance, 880);
        takes[0].amount = 500;
        offered[0].amount = 550;
        vm.prank(solver);
        intentGateway.fillOrder{value: 550}(order, FillOptions(0, 0, 0, offered, takes));
        assertEq(before - solver.balance, 1100);
        assertEq(address(intentGateway).balance, 50);
    }

    function testRate_UncappedNativeOutputDebitsDerivedPaymentAndRefundsBudget() public {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(bytes32(uint256(uint160(address(usdc)))), 10);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(bytes32(0), 3);
        Order memory order = Order(
            bytes32(uint256(uint160(user))),
            host.host(),
            host.host(),
            block.number + 100,
            0,
            0,
            address(0),
            DispatchInfo(new TokenInfo[](0), ""),
            inputs,
            PaymentInfo(bytes32(uint256(uint160(user))), outputs, "")
        );
        vm.startPrank(user);
        usdc.approve(address(intentGateway), 10);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(inputs[0].token, 4);
        TokenInfo[] memory offered = new TokenInfo[](1);
        offered[0] = TokenInfo(bytes32(0), 4);
        uint256 solverBefore = solver.balance;
        vm.prank(solver);
        intentGateway.fillOrder{value: 4}(order, FillOptions(0, 0, 0, offered, takes));

        assertEq(solverBefore - solver.balance, 3);
        assertEq(address(intentGateway).balance, 1);
        assertEq(intentGateway._partialFills(keccak256(abi.encode(order)), 0), 1);
    }

    function testRate_OversizedOutputCallReceivesCreditAndRetainsDerivedSurplus() public {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(bytes32(uint256(uint160(address(usdc)))), 10);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(bytes32(uint256(uint160(address(dai)))), 3);
        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(dai), value: 0, data: abi.encodeWithSelector(IERC20.approve.selector, address(intentGateway), 3)
        });
        Order memory order = Order(
            bytes32(uint256(uint160(user))),
            host.host(),
            host.host(),
            block.number + 100,
            0,
            0,
            address(0),
            DispatchInfo(new TokenInfo[](0), ""),
            inputs,
            PaymentInfo(bytes32(uint256(uint160(user))), outputs, abi.encode(calls))
        );
        vm.startPrank(user);
        usdc.approve(address(intentGateway), 10);
        intentGateway.placeOrder(order, bytes32(0));
        vm.stopPrank();

        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(inputs[0].token, 11);
        TokenInfo[] memory offered = new TokenInfo[](1);
        offered[0] = TokenInfo(outputs[0].token, 11);
        uint256 solverBefore = dai.balanceOf(solver);
        uint256 userBefore = dai.balanceOf(user);
        uint256 protocolBefore = dai.balanceOf(address(intentGateway));
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 11);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, offered, takes));
        vm.stopPrank();

        assertEq(solverBefore - dai.balanceOf(solver), 10);
        assertEq(dai.balanceOf(user) - userBefore, 3);
        assertEq(dai.balanceOf(address(intentGateway)) - protocolBefore, 7);
        assertEq(usdc.balanceOf(solver), 100000 * 1e6 + 10);
    }

    function testRate_RejectsInvalidLegsAndNoProgress() public {
        Order memory order = _placeSameChainOrder(1000, 1000, 0);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 100);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 110);
        vm.startPrank(solver);
        takes[0].token = order.output.assets[0].token;
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        takes[0].token = order.inputs[0].token;
        takes[0].amount = 0;
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        outputs[0].amount = 0;
        vm.expectRevert(IntentsBase.RateFillTooSmall.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, takes));
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs, new TokenInfo[](2)));
        vm.stopPrank();
        assertEq(intentGateway._partialFills(keccak256(abi.encode(order)), 0), 0);
        assertEq(intentGateway._orders(keccak256(abi.encode(order)), 0), 1000);
    }
}
