// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {deployIntentGatewayImpl} from "./IntentGatewayDeploy.sol";
import {IntentGatewayV2} from "../../src/apps/IntentGatewayV2.sol";
import {ExtrinsicIntents} from "../../src/apps/intentsv2/ExtrinsicIntents.sol";
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {
    Order,
    Params,
    ParamsUpdate,
    DestinationFee,
    TokenInfo,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    CancelOptions,
    WithdrawalRequest,
    SweepDust
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest, GetRequest, GetResponse, StorageValue} from "@hyperbridge/core/libraries/Message.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Vm} from "forge-std/Vm.sol";

contract TaxedProtocolFeeToken is ERC20 {
    constructor(address owner) ERC20("Taxed", "TAX") {
        _mint(owner, 10_000);
    }

    function _update(address from, address to, uint256 amount) internal override {
        if (from != address(0) && to != address(0)) {
            uint256 tax = amount / 10;
            super._update(from, address(0), tax);
            amount -= tax;
        }
        super._update(from, to, amount);
    }
}

contract IntentGatewayProtocolFeesTest is MainnetForkBaseTest {
    IntentGatewayV2 internal gateway;
    address internal user;
    address internal solver;
    bytes internal localChain;
    bytes internal governanceChain;
    bytes32 constant DUST = keccak256("DustCollected(address,uint256)");
    bytes32 constant REFUND = keccak256("ProtocolFeeRefunded(bytes32,address,uint256)");

    function setUp() public override {
        super.setUp();
        localChain = host.host();
        governanceChain = host.hyperbridge();
        user = makeCleanAddr("feeUser");
        solver = makeCleanAddr("feeSolver");
        gateway = IntentGatewayV2(payable(address(new ERC1967Proxy(address(deployIntentGatewayImpl()), ""))));
        bytes[] memory peers = new bytes[](1);
        peers[0] = bytes("DEST_CHAIN");
        gateway.initialize(
            Params(address(host), address(dispatcher), false, 0, 1000, address(0)), peers, address(0), address(this)
        );
        deal(address(usdc), user, 1_000_000);
        deal(address(dai), solver, 1_000_000);
        vm.deal(user, 10 ether);
    }

    function _tokens(address token, uint256 amount) internal pure returns (TokenInfo[] memory tokens) {
        tokens = new TokenInfo[](1);
        tokens[0] = TokenInfo(bytes32(uint256(uint160(token))), amount);
    }

    function _place(address token, uint256 gross, uint256 net, bool cross) internal returns (Order memory order) {
        order = Order({
            user: bytes32(uint256(uint160(user))),
            source: host.host(),
            destination: cross ? bytes("DEST_CHAIN") : host.host(),
            deadline: block.number + 1000,
            nonce: gateway._nonce(),
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo(new TokenInfo[](0), ""),
            inputs: _tokens(token, gross),
            output: PaymentInfo(bytes32(uint256(uint160(user))), _tokens(address(dai), 100), "")
        });
        vm.startPrank(user);
        if (token != address(0)) IERC20(token).approve(address(gateway), gross);
        gateway.placeOrder{value: token == address(0) ? gross : 0}(order, bytes32(0));
        vm.stopPrank();
        order.inputs[0].amount = net;
        assertEq(gateway._orders(keccak256(abi.encode(order)), token), net, "post-fee commitment");
    }

    function _cancel(Order memory order) internal {
        vm.prank(user);
        gateway.cancelOrder(order, CancelOptions(0, 0));
    }

    function _fill(Order memory order, uint256 output) internal {
        vm.startPrank(solver);
        dai.approve(address(gateway), output);
        gateway.fillOrder(order, FillOptions(0, 0, 0, _tokens(address(dai), output)));
        vm.stopPrank();
    }

    function _post(IntentsBase.RequestKind kind, bytes memory body, bool governance) internal {
        PostRequest memory request = PostRequest({
            source: governance ? governanceChain : bytes("DEST_CHAIN"),
            dest: localChain,
            nonce: 0,
            from: abi.encodePacked(address(gateway)),
            to: abi.encodePacked(address(gateway)),
            body: bytes.concat(bytes1(uint8(kind)), body),
            timeoutTimestamp: 0
        });
        vm.prank(address(host));
        gateway.onAccept(IncomingPostRequest({relayer: address(this), request: request}));
    }

    function _redeem(Order memory order, uint256 principal, bool finalizes) internal {
        _post(
            finalizes ? IntentsBase.RequestKind.RedeemEscrow : IntentsBase.RequestKind.RedeemEscrowPartial,
            abi.encode(
                WithdrawalRequest({
                    commitment: keccak256(abi.encode(order)),
                    tokens: _tokens(address(uint160(uint256(order.inputs[0].token))), principal),
                    beneficiary: bytes32(uint256(uint160(solver)))
                })
            ),
            false
        );
    }

    function _proof(Order memory order, uint8 filled) internal {
        bytes[] memory keys = new bytes[](1);
        bytes32 commitment = keccak256(abi.encode(order));
        keys[0] = abi.encodePacked(
            keccak256(abi.encode(order.output.assets[0].token, keccak256(abi.encode(commitment, uint256(11)))))
        );
        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue(keys[0], filled == 0 ? abi.encodePacked(uint8(128)) : abi.encodePacked(filled));
        uint256[] memory totals = new uint256[](1);
        totals[0] = 100;
        GetRequest memory request = GetRequest({
            source: host.host(),
            dest: bytes("DEST_CHAIN"),
            nonce: 0,
            from: abi.encodePacked(address(gateway)),
            keys: keys,
            height: 0,
            timeoutTimestamp: 0,
            context: abi.encode(commitment, order.user, order.inputs, totals)
        });
        vm.prank(address(host));
        gateway.onGetResponse(IncomingGetResponse(GetResponse(request, values), address(this)));
    }

    function _rate(uint256 bps) internal {
        Params memory p = gateway.params();
        p.protocolFeeBps = bps;
        _post(IntentsBase.RequestKind.UpdateParams, abi.encode(ParamsUpdate(p, new DestinationFee[](0))), true);
    }

    function _sweep(TokenInfo[] memory tokens) internal {
        _post(IntentsBase.RequestKind.SweepDust, abi.encode(SweepDust(solver, tokens)), true);
    }

    function _assertEvents(Vm.Log[] memory logs, bytes32 commitment, address token, uint256 refund, uint256 earned)
        internal
        view
    {
        uint256 refundTotal;
        uint256 earnedTotal;
        uint256 refundCount;
        uint256 earnedCount;
        for (uint256 i; i < logs.length; i++) {
            if (logs[i].emitter != address(gateway)) continue;
            if (logs[i].topics[0] == REFUND) {
                assertEq(logs[i].topics[1], commitment);
                assertEq(logs[i].topics[2], bytes32(uint256(uint160(token))));
                refundTotal += abi.decode(logs[i].data, (uint256));
                refundCount++;
            }
            if (logs[i].topics[0] == DUST) {
                (address actualToken, uint256 amount) = abi.decode(logs[i].data, (address, uint256));
                assertEq(actualToken, token);
                earnedTotal += amount;
                earnedCount++;
            }
        }
        assertEq(refundTotal, refund, "fee refund event");
        assertEq(earnedTotal, earned, "earned fee event");
        assertEq(refundCount, refund == 0 ? 0 : 1, "one positive refund event");
        assertEq(earnedCount, earned == 0 ? 0 : 1, "one positive revenue event");
    }

    function testPlacementDoesNotRecognizePendingFeeAsRevenue() public {
        vm.recordLogs();
        Order memory order = _place(address(usdc), 1000, 900, false);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 0);
        (uint256 fee, uint256 committed) = gateway._protocolFees(keccak256(abi.encode(order)), address(usdc));
        assertEq(fee, 100);
        assertEq(committed, 900);
    }

    function testUnfilledEarlyCancellationRefundsEntireFee() public {
        Order memory order = _place(address(usdc), 1000, 900, false);
        uint256 before = usdc.balanceOf(user);
        vm.recordLogs();
        _cancel(order);
        assertEq(usdc.balanceOf(user) - before, 1000, "principal and fee returned");
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 100, 0);
        (uint256 fee, uint256 committed) = gateway._protocolFees(keccak256(abi.encode(order)), address(usdc));
        assertEq(fee, 0);
        assertEq(committed, 0);
    }

    function testUnfilledExpiredNativeCancellationRefundsEntireFee() public {
        Order memory order = _place(address(0), 1000, 900, false);
        vm.roll(order.deadline + 1);
        uint256 before = user.balance;
        _cancel(order);
        assertEq(user.balance - before, 1000);
    }

    function testPartialCancellationUsesOriginalDenominatorAfterRateChange() public {
        Order memory order = _place(address(usdc), 1000, 900, false);
        _fill(order, 40);
        _rate(5000);
        uint256 before = usdc.balanceOf(user);
        vm.recordLogs();
        _cancel(order);
        assertEq(usdc.balanceOf(user) - before, 600);
        assertEq(usdc.balanceOf(solver), 360);
        assertEq(usdc.balanceOf(address(gateway)), 40);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 60, 40);
    }

    function testPartialExpiredCancellationRefundsProportionalFee() public {
        Order memory order = _place(address(usdc), 1000, 900, false);
        _fill(order, 40);
        vm.roll(order.deadline + 1);
        uint256 before = usdc.balanceOf(user);
        _cancel(order);
        assertEq(usdc.balanceOf(user) - before, 600);
    }

    function testFullFillRecognizesFeeOnlyAtFinalFill() public {
        Order memory order = _place(address(usdc), 1000, 900, false);
        vm.recordLogs();
        _fill(order, 40);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 0);
        vm.recordLogs();
        _fill(order, 60);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 100);
        assertEq(usdc.balanceOf(solver), 900);
    }

    function testCrossChainPartialRedemptionThenProofRefundsOriginalFraction() public {
        Order memory order = _place(address(usdc), 1000, 900, true);
        _redeem(order, 360, false);
        uint256 before = usdc.balanceOf(user);
        vm.recordLogs();
        _proof(order, 40);
        assertEq(usdc.balanceOf(user) - before, 600);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 60, 40);
    }

    function testCrossChainCancellationBeforeDelayedRedemptionSettlesOnce() public {
        Order memory order = _place(address(usdc), 1000, 900, true);
        uint256 before = usdc.balanceOf(user);
        _proof(order, 40);
        assertEq(usdc.balanceOf(user) - before, 600);
        vm.recordLogs();
        _redeem(order, 360, false);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 0);
        assertEq(usdc.balanceOf(solver), 360);
        assertEq(usdc.balanceOf(address(gateway)), 40);
    }

    function testFullyFilledProofRetainsFeeUntilFinalRedeem() public {
        Order memory order = _place(address(usdc), 1000, 900, true);
        vm.recordLogs();
        _proof(order, 100);
        bytes32 commitment = keccak256(abi.encode(order));
        (uint256 fee, uint256 committed) = gateway._protocolFees(commitment, address(usdc));
        assertEq(fee, 100, "fully-filled proof must retain the fee");
        assertEq(committed, 900, "original principal remains until final redemption");
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 0);
        vm.recordLogs();
        _redeem(order, 900, true);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 100);
        (fee, committed) = gateway._protocolFees(commitment, address(usdc));
        assertEq(fee, 0);
        assertEq(committed, 0);
    }

    function testFinalRedeemBeforeDelayedPartialRedeemRecognizesFeeOnce() public {
        Order memory order = _place(address(usdc), 1000, 900, true);
        vm.recordLogs();
        _redeem(order, 540, true);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 100);
        vm.recordLogs();
        _redeem(order, 360, false);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 0);
    }

    function testZeroPrincipalFinalRedeemStillRecognizesFee() public {
        Order memory order = _place(address(usdc), 1000, 900, true);
        _redeem(order, 900, false);
        vm.recordLogs();
        _redeem(order, 0, true);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 100);
    }

    function testRefundEscrowMessageIncludesFeeRefund() public {
        Order memory order = _place(address(usdc), 1000, 900, true);
        uint256 before = usdc.balanceOf(user);
        _post(
            IntentsBase.RequestKind.RefundEscrow,
            abi.encode(
                WithdrawalRequest({
                    commitment: keccak256(abi.encode(order)), tokens: order.inputs, beneficiary: order.user
                })
            ),
            false
        );
        assertEq(usdc.balanceOf(user) - before, 1000);
    }

    function testFeeRefundRoundsDownUsingRefundablePrincipal() public {
        Order memory order = _place(address(usdc), 101, 91, false);
        _fill(order, 50); // 45 principal released, 46 remains, refund floor(10*46/91)=5.
        uint256 before = usdc.balanceOf(user);
        vm.recordLogs();
        _cancel(order);
        assertEq(usdc.balanceOf(user) - before, 51);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 5, 5);
    }

    function testTinyPrincipalRoundingCanRefundEntireFeeAfterPartialOutput() public {
        _rate(5000);
        Order memory order = _place(address(usdc), 2, 1, false);
        _fill(order, 99); // Output paid but floor(1*99/100)=0 principal released.
        uint256 before = usdc.balanceOf(user);
        _cancel(order);
        assertEq(usdc.balanceOf(user) - before, 2);
    }

    function testFeeOnTransferUsesActualReceivedAmount() public {
        TaxedProtocolFeeToken token = new TaxedProtocolFeeToken(user);
        Order memory order = _place(address(token), 1000, 810, false); // receive 900, hold 90 fee.
        uint256 before = token.balanceOf(user);
        vm.recordLogs();
        _cancel(order);
        assertEq(token.balanceOf(user) - before, 810, "outbound transfer tax applies to combined refund");
        assertEq(token.balanceOf(address(gateway)), 0);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(token), 90, 0);
    }

    function testZeroFeeCancellationHasNoFeeEvents() public {
        _rate(0);
        Order memory order = _place(address(usdc), 1000, 1000, false);
        vm.recordLogs();
        _cancel(order);
        _assertEvents(vm.getRecordedLogs(), keccak256(abi.encode(order)), address(usdc), 0, 0);
        assertEq(usdc.balanceOf(user), 1_000_000);
    }

    function testSweepEarnedDustKeepsTwoOtherOrdersRefundable() public {
        Order memory first = _place(address(usdc), 1000, 900, false);
        Order memory second = _place(address(usdc), 1000, 900, false);
        Order memory earned = _place(address(usdc), 1000, 900, false);
        _fill(earned, 100);
        _sweep(_tokens(address(usdc), 100));
        _cancel(first);
        _cancel(second);
        assertEq(usdc.balanceOf(address(gateway)), 0);
        assertEq(usdc.balanceOf(user), 999_000);
    }

    function testVersionThreeUpgradePreservesLegacyEscrowWithoutRetroactiveRefund() public {
        Order memory order = _place(address(usdc), 1000, 900, false);
        bytes32 commitment = keccak256(abi.encode(order));
        // Reconstruct the pre-accounting v3 snapshot: historical net escrow and fee balance,
        // with the newly appended mapping still empty. Historical fees may already be swept.
        bytes32 feeSlot = keccak256(abi.encode(address(usdc), keccak256(abi.encode(commitment, uint256(14)))));
        vm.store(address(gateway), feeSlot, bytes32(0));
        vm.store(address(gateway), bytes32(uint256(feeSlot) + 1), bytes32(0));
        _sweep(_tokens(address(usdc), 100));
        _post(
            IntentsBase.RequestKind.Execute,
            abi.encodeCall(ExtrinsicIntents.upgradeToAndCall, (address(deployIntentGatewayImpl()), bytes(""))),
            true
        );
        assertEq(gateway.version(), 4);
        assertEq(gateway._orders(commitment, address(usdc)), 900);
        uint256 before = usdc.balanceOf(user);
        vm.recordLogs();
        _cancel(order);
        assertEq(usdc.balanceOf(user) - before, 900, "legacy principal only");
        _assertEvents(vm.getRecordedLogs(), commitment, address(usdc), 0, 0);
        assertEq(usdc.balanceOf(address(gateway)), 0);
    }
}
