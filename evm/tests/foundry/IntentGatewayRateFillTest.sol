// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;
import {IntentGatewayV2SameChainTest} from "./IntentGatewayV2SameChainTest.sol";
import {
    Order,
    FillOptions,
    TokenInfo,
    CancelOptions,
    PaymentInfo,
    DispatchInfo
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

interface RateGateway {
    function fillOrderAtRate(Order calldata, FillOptions calldata, TokenInfo[] calldata) external payable;
}

contract IntentGatewayRateFillTest is IntentGatewayV2SameChainTest {
    function _rateFill(Order memory order, uint256 take, uint256 offered) internal {
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(order.inputs[0].token, take);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, offered);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), offered);
        RateGateway(address(intentGateway)).fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), inputs);
        vm.stopPrank();
    }

    function testRate_PartialSurplusAndCappedCompletion() public {
        Order memory order = _placeSameChainOrder(1000, 1000, 0);
        uint256 userBefore = dai.balanceOf(user);
        uint256 solverBefore = usdc.balanceOf(solver);
        _rateFill(order, 800, 880);
        bytes32 commitment = keccak256(abi.encode(order));
        assertEq(intentGateway._partialFills(commitment, order.output.assets[0].token), 800);
        assertEq(usdc.balanceOf(solver) - solverBefore, 800);
        assertEq(dai.balanceOf(user) - userBefore, 840);
        uint256 outputBefore = dai.balanceOf(solver);
        _rateFill(order, 500, 550);
        assertEq(outputBefore - dai.balanceOf(solver), 220);
        assertEq(usdc.balanceOf(solver) - solverBefore, 1000);
        assertEq(dai.balanceOf(user) - userBefore, 1050);
        assertEq(dai.balanceOf(address(intentGateway)), 50);
        assertEq(intentGateway._orders(commitment, address(usdc)), 0);
    }

    function testRate_QuantizedInputRelease() public {
        Order memory order = _placeSameChainOrder(10, 3, 0);
        uint256 before = usdc.balanceOf(solver);
        _rateFill(order, 4, 2);
        assertEq(usdc.balanceOf(solver) - before, 3);
        assertEq(intentGateway._partialFills(keccak256(abi.encode(order)), order.output.assets[0].token), 1);
        _rateFill(order, 10, 3);
        assertEq(usdc.balanceOf(solver) - before, 10);
    }

    function testRate_BelowFloorRejected() public {
        Order memory order = _placeSameChainOrder(1000, 1000, 0);
        TokenInfo[] memory inputs = new TokenInfo[](1);
        inputs[0] = TokenInfo(order.inputs[0].token, 400);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 399);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 399);
        vm.expectRevert(bytes4(keccak256("RateBelowOrder()")));
        RateGateway(address(intentGateway)).fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), inputs);
        vm.stopPrank();
    }

    function testRate_LegacyAndRateUseCumulativeRounding() public {
        Order memory order = _placeSameChainOrder(10, 3, 0);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 1);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 1);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs));
        vm.stopPrank();
        uint256 before = usdc.balanceOf(solver);
        _rateFill(order, 4, 2);
        assertEq(usdc.balanceOf(solver) - before, 3);
        outputs[0].amount = 1;
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 1);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs));
        vm.stopPrank();
        assertEq(usdc.balanceOf(solver) - before, 7);
        assertEq(intentGateway._orders(keccak256(abi.encode(order)), address(usdc)), 0);
    }

    function testRate_LegacyRoundingDebtRejectsRateButLegacyCompletes() public {
        Order memory order = _placeSameChainOrder(10, 6, 0);
        bytes32 commitment = keccak256(abi.encode(order));
        // State after two old one-output slices: each released floor(10/6)=1.
        // Cumulative accounting would have released floor(20/6)=3.
        bytes32 escrowSlot = keccak256(abi.encode(address(usdc), keccak256(abi.encode(commitment, uint256(9)))));
        bytes32 progressSlot =
            keccak256(abi.encode(order.output.assets[0].token, keccak256(abi.encode(commitment, uint256(11)))));
        vm.store(address(intentGateway), escrowSlot, bytes32(uint256(8)));
        vm.store(address(intentGateway), progressSlot, bytes32(uint256(2)));
        vm.prank(address(intentGateway));
        usdc.transfer(solver, 2);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 4);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 6);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 4);
        vm.expectRevert(bytes4(keccak256("LegacyRateAccounting()")));
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), takes);
        uint256 before = usdc.balanceOf(solver);
        intentGateway.fillOrder(order, FillOptions(0, 0, 0, outputs));
        vm.stopPrank();
        assertEq(usdc.balanceOf(solver) - before, 8);
        assertEq(intentGateway._orders(commitment, address(usdc)), 0);
    }

    function testRate_EmptyInputsPreserveLegacySelectorBehavior() public {
        Order memory order = _placeSameChainOrder(100, 100, 0);
        vm.startPrank(solver);
        dai.approve(address(intentGateway), 100);
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, order.output.assets), new TokenInfo[](0));
        vm.stopPrank();
        assertEq(intentGateway._orders(keccak256(abi.encode(order)), address(usdc)), 0);
        assertEq(intentGateway.fillOrder.selector, bytes4(0xa5470064));
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
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, order.output.assets), takes);
        intentGateway.unpause();
        vm.expectRevert(bytes4(keccak256("FillExpired()")));
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, block.number - 1, order.output.assets), takes);
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
        intentGateway.fillOrderAtRate{value: 1000}(order, FillOptions(0, 0, 0, offered), takes);
        assertEq(before - solver.balance, 880);
        takes[0].amount = 500;
        offered[0].amount = 550;
        vm.prank(solver);
        intentGateway.fillOrderAtRate{value: 550}(order, FillOptions(0, 0, 0, offered), takes);
        assertEq(before - solver.balance, 1100);
        assertEq(address(intentGateway).balance, 50);
    }

    function testRate_RejectsInvalidLegsAndNoProgress() public {
        Order memory order = _placeSameChainOrder(1000, 1000, 0);
        TokenInfo[] memory takes = new TokenInfo[](1);
        takes[0] = TokenInfo(order.inputs[0].token, 100);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        outputs[0] = TokenInfo(order.output.assets[0].token, 110);
        vm.startPrank(solver);
        takes[0].token = order.output.assets[0].token;
        vm.expectRevert(bytes4(keccak256("InvalidInput()")));
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), takes);
        takes[0].token = order.inputs[0].token;
        takes[0].amount = 0;
        vm.expectRevert(bytes4(keccak256("InvalidInput()")));
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), takes);
        outputs[0].amount = 0;
        vm.expectRevert(bytes4(keccak256("RateFillTooSmall()")));
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), takes);
        vm.expectRevert(bytes4(keccak256("InvalidInput()")));
        intentGateway.fillOrderAtRate(order, FillOptions(0, 0, 0, outputs), new TokenInfo[](2));
        vm.stopPrank();
        assertEq(intentGateway._partialFills(keccak256(abi.encode(order)), order.output.assets[0].token), 0);
        assertEq(intentGateway._orders(keccak256(abi.encode(order)), address(usdc)), 1000);
    }
}
