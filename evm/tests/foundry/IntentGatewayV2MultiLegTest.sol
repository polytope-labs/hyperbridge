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

import "forge-std/Test.sol";
import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {FeeOnTransferToken} from "./IntentGatewayV2SameChainTest.sol";
import {
    IntentGatewayV2,
    Order,
    Params,
    InitParams,
    TokenInfo,
    PaymentInfo,
    DispatchInfo,
    FillOptions,
    CancelOptions,
    WithdrawalRequest
} from "../../src/apps/IntentGatewayV2.sol";
import {deployIntentGatewayImpl} from "./IntentGatewayDeploy.sol";
import {IntentsBase} from "../../src/apps/intentsv2/IntentsBase.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Call} from "@hyperbridge/core/interfaces/ICallDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {GetRequest, GetResponse} from "@hyperbridge/core/libraries/Message.sol";
import {StorageValue} from "@polytope-labs/solidity-merkle-trees/src/trie/Node.sol";

/**
 * @title IntentGatewayV2MultiLegTest
 * @notice Orders whose legs repeat a token, e.g. one pair offered at several prices. Escrow, fill
 * progress and protocol fees are keyed by (commitment, leg index), so each leg settles on its own.
 * Several tests pin SRLabs findings that token-keyed accounting made possible for such orders:
 * S3-2 (a completing leg released the whole token balance), S2-15 (a repeated output token shared
 * one fill counter) and the predispatch sweep of S2-12/S2-13.
 */
contract IntentGatewayV2MultiLegTest is MainnetForkBaseTest {
    IntentGatewayV2 internal gateway;

    address internal user;
    address internal solverA;
    address internal solverB;
    address internal relayer;

    bytes32 internal usdcToken;
    bytes32 internal daiToken;

    bytes32 internal constant DUST_COLLECTED = keccak256("DustCollected(address,uint256)");
    bytes32 internal constant GET_REQUEST_EVENT =
        keccak256("GetRequestEvent(string,string,bytes,bytes[],uint256,uint256,uint256,bytes,uint256)");
    bytes32 internal constant POST_REQUEST_EVENT =
        keccak256("PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)");

    function setUp() public override {
        super.setUp();

        user = makeCleanAddr("user");
        solverA = makeCleanAddr("solverA");
        solverB = makeCleanAddr("solverB");
        relayer = makeCleanAddr("relayer");
        usdcToken = bytes32(uint256(uint160(address(usdc))));
        daiToken = bytes32(uint256(uint160(address(dai))));

        bytes[] memory peers = new bytes[](3);
        peers[0] = host.host();
        peers[1] = bytes("SOURCE_CHAIN");
        peers[2] = bytes("DEST_CHAIN");
        gateway = _gateway(0, peers, relayer);

        vm.deal(user, 10 ether);
        deal(address(usdc), user, 100_000 * 1e6);
        deal(address(dai), user, 100_000 * 1e18);
        address[2] memory solvers = [solverA, solverB];
        for (uint256 i; i < solvers.length; i++) {
            vm.deal(solvers[i], 10 ether);
            deal(address(usdc), solvers[i], 100_000 * 1e6);
            deal(address(dai), solvers[i], 100_000 * 1e18);
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    function _gateway(uint256 protocolFeeBps, bytes[] memory peers, address relayer_)
        internal
        returns (IntentGatewayV2 gw)
    {
        gw = IntentGatewayV2(payable(address(new ERC1967Proxy(address(deployIntentGatewayImpl()), ""))));
        gw.initialize(
            InitParams({
                params: Params({
                    host: address(host),
                    dispatcher: address(dispatcher),
                    solverSelection: false,
                    surplusShareBps: 5000,
                    protocolFeeBps: protocolFeeBps,
                    priceOracle: address(0)
                }),
                peerChains: peers,
                relayer: relayer_,
                owner: address(this)
            })
        );
        address[3] memory accounts = [user, solverA, solverB];
        for (uint256 i; i < accounts.length; i++) {
            vm.startPrank(accounts[i]);
            usdc.approve(address(gw), type(uint256).max);
            dai.approve(address(gw), type(uint256).max);
            vm.stopPrank();
        }
    }

    function _legs(bytes32[2] memory tokens, uint256[2] memory amounts)
        internal
        pure
        returns (TokenInfo[] memory legs)
    {
        legs = new TokenInfo[](2);
        legs[0] = TokenInfo({token: tokens[0], amount: amounts[0]});
        legs[1] = TokenInfo({token: tokens[1], amount: amounts[1]});
    }

    function _order(
        bytes memory source,
        bytes memory destination,
        TokenInfo[] memory inputs,
        TokenInfo[] memory outputs
    ) internal view returns (Order memory) {
        return Order({
            user: bytes32(uint256(uint160(user))),
            source: source,
            destination: destination,
            deadline: block.number + 100,
            nonce: 0,
            fees: 0,
            session: address(0),
            predispatch: DispatchInfo({assets: new TokenInfo[](0), call: ""}),
            inputs: inputs,
            output: PaymentInfo({beneficiary: bytes32(uint256(uint160(user))), assets: outputs, call: ""})
        });
    }

    /// @dev One pair at two prices: leg 0 sells 1200 USDC for 1200 DAI, leg 1 sells 1000 USDC for 990 DAI.
    function _ladder(bytes memory source, bytes memory destination) internal view returns (Order memory) {
        return _order(
            source,
            destination,
            _legs([usdcToken, usdcToken], [uint256(1200 * 1e6), 1000 * 1e6]),
            _legs([daiToken, daiToken], [uint256(1200 * 1e18), 990 * 1e18])
        );
    }

    /// @dev Places `order` as `user` on `gw` and returns it as stamped, with its commitment.
    function _place(IntentGatewayV2 gw, Order memory order) internal returns (Order memory, bytes32) {
        uint256 nonce = gw._nonce();
        vm.prank(user);
        gw.placeOrder(order, bytes32(0));
        order.source = host.host();
        order.nonce = nonce;
        return (order, keccak256(abi.encode(order)));
    }

    function _fill(address solver, Order memory order, uint256 leg0, uint256 leg1) internal {
        TokenInfo[] memory outputs = _legs([order.output.assets[0].token, order.output.assets[1].token], [leg0, leg1]);
        vm.prank(solver);
        gateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, validUntil: 0, outputs: outputs}));
    }

    function _partialFillSlot(bytes32 commitment, uint256 leg) internal pure returns (bytes32) {
        return keccak256(abi.encode(leg, keccak256(abi.encode(commitment, uint256(11)))));
    }

    function _countDust(Vm.Log[] memory logs, address emitter, address token)
        internal
        pure
        returns (uint256 count, uint256 amount)
    {
        for (uint256 i; i < logs.length; i++) {
            if (logs[i].emitter != emitter || logs[i].topics[0] != DUST_COLLECTED) continue;
            (address dustToken, uint256 dustAmount) = abi.decode(logs[i].data, (address, uint256));
            if (dustToken != token) continue;
            count++;
            amount += dustAmount;
        }
    }

    function _redeem(IntentsBase.RequestKind kind, bytes32 commitment, TokenInfo[] memory tokens, address solver)
        internal
    {
        IncomingPostRequest memory incoming = _redeemRequest(kind, commitment, tokens, solver);
        vm.prank(address(host));
        gateway.onAccept(incoming);
    }

    /// @dev A withdrawal message from the destination gateway, as delivered by the authorised relayer.
    function _redeemRequest(IntentsBase.RequestKind kind, bytes32 commitment, TokenInfo[] memory tokens, address solver)
        internal
        view
        returns (IncomingPostRequest memory)
    {
        bytes memory body = bytes.concat(
            bytes1(uint8(kind)),
            abi.encode(
                WithdrawalRequest({
                    commitment: commitment, tokens: tokens, beneficiary: bytes32(uint256(uint160(solver)))
                })
            )
        );
        PostRequest memory request = PostRequest({
            source: bytes("DEST_CHAIN"),
            dest: host.host(),
            nonce: 0,
            from: abi.encodePacked(address(gateway)),
            to: abi.encodePacked(address(gateway)),
            body: body,
            timeoutTimestamp: 0
        });
        return IncomingPostRequest({relayer: relayer, request: request});
    }

    // ── same-chain ────────────────────────────────────────────────────────────

    /// @notice S3-2 and S2-15: completing one leg releases exactly that leg's escrow and leaves the other
    /// leg's escrow and progress untouched, so the order stays open until every leg is paid.
    function testSameChain_CompletingOneLegReleasesOnlyItsOwnEscrow() public {
        (Order memory order, bytes32 commitment) = _place(gateway, _ladder("", host.host()));

        uint256 before = usdc.balanceOf(solverA);
        _fill(solverA, order, 1200 * 1e18, 0);
        assertEq(usdc.balanceOf(solverA) - before, 1200 * 1e6, "leg 0 releases exactly its own escrow");
        assertEq(gateway._orders(commitment, 0), 0, "leg 0 settled");
        assertEq(gateway._orders(commitment, 1), 1000 * 1e6, "leg 1 escrow untouched");
        assertEq(gateway._partialFills(commitment, 0), 1200 * 1e18, "leg 0 progress");
        assertEq(gateway._partialFills(commitment, 1), 0, "leg 1 progress is its own");
        assertEq(gateway._filled(commitment), address(0), "an unpaid leg keeps the order open");
        assertEq(
            uint256(vm.load(address(gateway), keccak256(abi.encode(uint256(1), keccak256(abi.encode(commitment, 9)))))),
            1000 * 1e6,
            "_orders[commitment][1] lives at the index-keyed slot of mapping 9"
        );

        before = usdc.balanceOf(solverB);
        _fill(solverB, order, 0, 990 * 1e18);
        assertEq(usdc.balanceOf(solverB) - before, 1000 * 1e6, "leg 1 releases its own escrow");
        assertEq(gateway._filled(commitment), solverB, "the order finalizes once every leg is paid");
        assertEq(usdc.balanceOf(address(gateway)), 0, "no escrow left behind");
    }

    /// @notice S2-15 with distinct inputs: two legs asking for the same (native) output token keep separate
    /// fill counters, and the native token's zero address never aliases a leg index.
    function testSameChain_RepeatedNativeOutputTracksLegsSeparately() public {
        bytes32 nativeToken = bytes32(0);
        Order memory order = _order(
            "",
            host.host(),
            _legs([usdcToken, daiToken], [uint256(1000 * 1e6), 1000 * 1e18]),
            _legs([nativeToken, nativeToken], [uint256(0.3 ether), 0.25 ether])
        );
        bytes32 commitment;
        (order, commitment) = _place(gateway, order);

        TokenInfo[] memory outputs = _legs([nativeToken, nativeToken], [uint256(0.3 ether), 0]);
        vm.prank(solverA);
        gateway.fillOrder{value: 0.3 ether}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, validUntil: 0, outputs: outputs})
        );
        assertEq(gateway._partialFills(commitment, 1), 0, "leg 1 is not complete because leg 0 is");
        assertEq(gateway._filled(commitment), address(0), "order stays open");
        assertEq(dai.balanceOf(solverA), 100_000 * 1e18, "leg 1's DAI stays escrowed");

        uint256 userEth = user.balance;
        outputs = _legs([nativeToken, nativeToken], [uint256(0), 0.25 ether]);
        vm.prank(solverB);
        gateway.fillOrder{value: 0.25 ether}(
            order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, validUntil: 0, outputs: outputs})
        );
        assertEq(user.balance - userEth, 0.25 ether, "leg 1 paid in full");
        assertEq(dai.balanceOf(solverB), 101_000 * 1e18, "leg 1's DAI released to its solver");
        assertEq(gateway._filled(commitment), solverB, "order finalized");
    }

    /// @notice Partial fills at two price levels, then a cancel: each leg refunds its own remainder. With a
    /// shared token balance the second leg's refund used to find it already drained and revert.
    function testSameChain_CancelAfterPartialFillsRefundsEachLeg() public {
        (Order memory order, bytes32 commitment) = _place(gateway, _ladder("", host.host()));

        _fill(solverA, order, 600 * 1e18, 0); // half of leg 0 -> 600 USDC
        _fill(solverB, order, 0, 297 * 1e18); // 30% of leg 1 -> 300 USDC
        assertEq(gateway._orders(commitment, 0), 600 * 1e6, "leg 0 remainder");
        assertEq(gateway._orders(commitment, 1), 700 * 1e6, "leg 1 remainder");

        uint256 before = usdc.balanceOf(user);
        vm.expectEmit(true, false, false, true, address(gateway));
        emit IntentsBase.EscrowRefunded(commitment, _legs([usdcToken, usdcToken], [uint256(600 * 1e6), 700 * 1e6]));
        vm.prank(user);
        gateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: 0}));

        assertEq(usdc.balanceOf(user) - before, 1300 * 1e6, "both remainders refunded");
        assertEq(gateway._orders(commitment, 0), 0, "leg 0 cleared");
        assertEq(gateway._orders(commitment, 1), 0, "leg 1 cleared");
        assertEq(usdc.balanceOf(address(gateway)), 0, "no escrow left behind");
    }

    /// @notice An output-calldata order whose legs repeat an output token sweeps the dispatcher once per
    /// token. A second sweep of the same balance used to fail the whole fill.
    function testSameChain_CalldataOrderSweepsRepeatedOutputTokenOnce() public {
        Order memory order = _ladder("", host.host());
        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(dai), value: 0, data: abi.encodeWithSelector(IERC20.approve.selector, address(gateway), 1)
        });
        order.output.call = abi.encode(calls);
        (order,) = _place(gateway, order);

        deal(address(dai), address(dispatcher), 5 * 1e18); // residue the fill must sweep
        uint256 gatewayDai = dai.balanceOf(address(gateway));

        vm.recordLogs();
        _fill(solverA, order, 1200 * 1e18, 990 * 1e18);
        (uint256 count, uint256 amount) = _countDust(vm.getRecordedLogs(), address(gateway), address(dai));

        assertEq(count, 1, "DAI dust reported once");
        assertEq(amount, 5 * 1e18, "the residue, not twice it");
        assertEq(dai.balanceOf(address(gateway)) - gatewayDai, 5 * 1e18, "residue swept once");
        assertEq(dai.balanceOf(address(dispatcher)), 0, "dispatcher emptied");
    }

    /// @notice A leg's protocol fee is held and settled per leg: a cancel after one leg completed refunds
    /// the open leg's fee with its principal and keeps the completed leg's fee as revenue.
    function testSameChain_ProtocolFeesHeldAndSettledPerLeg() public {
        IntentGatewayV2 feeGateway = _gateway(30, new bytes[](0), address(0));
        Order memory order = _order(
            "",
            host.host(),
            _legs([usdcToken, usdcToken], [uint256(1000 * 1e6), 1000 * 1e6]),
            _legs([daiToken, daiToken], [uint256(1000 * 1e18), 990 * 1e18])
        );
        vm.prank(user);
        feeGateway.placeOrder(order, bytes32(0));
        order.source = host.host();
        order.inputs[0].amount = 997 * 1e6; // 30 bps held back per leg
        order.inputs[1].amount = 997 * 1e6;
        bytes32 commitment = keccak256(abi.encode(order));

        (uint256 fee0, uint256 committed0) = feeGateway._protocolFees(commitment, 0);
        (uint256 fee1, uint256 committed1) = feeGateway._protocolFees(commitment, 1);
        assertEq(fee0, 3 * 1e6, "leg 0 fee");
        assertEq(committed0, 997 * 1e6, "leg 0 principal");
        assertEq(fee1, 3 * 1e6, "leg 1 fee held separately");
        assertEq(committed1, 997 * 1e6, "leg 1 principal");

        TokenInfo[] memory outputs = _legs([daiToken, daiToken], [uint256(1000 * 1e18), 0]);
        vm.prank(solverA);
        feeGateway.fillOrder(order, FillOptions({relayerFee: 0, nativeDispatchFee: 0, validUntil: 0, outputs: outputs}));

        uint256 before = usdc.balanceOf(user);
        vm.expectEmit(true, true, false, true, address(feeGateway));
        emit IntentsBase.ProtocolFeeRefunded(commitment, address(usdc), 3 * 1e6);
        vm.prank(user);
        feeGateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: 0}));

        assertEq(usdc.balanceOf(user) - before, 1000 * 1e6, "leg 1 principal and its fee refunded");
        (fee0,) = feeGateway._protocolFees(commitment, 0);
        (fee1,) = feeGateway._protocolFees(commitment, 1);
        assertEq(fee0 + fee1, 0, "both legs settled");
        assertEq(usdc.balanceOf(address(feeGateway)), 3 * 1e6, "leg 0's fee kept as revenue");
    }

    // ── predispatch ───────────────────────────────────────────────────────────

    /// @notice Predispatch legs that repeat an input token are swept once, checked against their total, and
    /// excess is reported once. Per-leg sweeps of the same balance used to fail the placement.
    function testPlaceOrder_PredispatchSweepsRepeatedInputTokenOnce() public {
        Order memory order = _ladder("", host.host());
        TokenInfo[] memory predispatch = new TokenInfo[](1);
        predispatch[0] = TokenInfo({token: usdcToken, amount: 2100 * 1e6});
        order.predispatch = DispatchInfo({assets: predispatch, call: abi.encode(new Call[](0))});

        // 2100 covers each leg alone but not both together.
        vm.prank(user);
        vm.expectRevert(IntentsBase.InvalidInput.selector);
        gateway.placeOrder(order, bytes32(0));

        order.predispatch.assets[0].amount = 2300 * 1e6;
        vm.recordLogs();
        bytes32 commitment;
        (order, commitment) = _place(gateway, order);
        (uint256 count, uint256 amount) = _countDust(vm.getRecordedLogs(), address(gateway), address(usdc));

        assertEq(gateway._orders(commitment, 0), 1200 * 1e6, "leg 0 escrow");
        assertEq(gateway._orders(commitment, 1), 1000 * 1e6, "leg 1 escrow");
        assertEq(count, 1, "excess reported once");
        assertEq(amount, 100 * 1e6, "excess over both legs");
        assertEq(usdc.balanceOf(address(gateway)), 2300 * 1e6, "one sweep of the whole balance");
    }

    /// @notice S2-12 through predispatch: a fee-on-transfer shortfall is shared across the legs of that
    /// token pro rata, with the rounding remainder on the first leg, so the legs never escrow more than
    /// arrived.
    function testPlaceOrder_PredispatchFeeOnTransferShortfallSharedAcrossLegs() public {
        FeeOnTransferToken fot = new FeeOnTransferToken(100); // 1% per transfer
        fot.mint(user, 10_000 * 1e18);
        vm.prank(user);
        fot.approve(address(gateway), type(uint256).max);
        bytes32 fotToken = bytes32(uint256(uint160(address(fot))));

        uint256 sent = 1000 * 1e18;
        uint256 atDispatcher = sent - sent / 100; // 990
        uint256 arrived = atDispatcher - atDispatcher / 100; // 980.1

        Order memory order = _order(
            "",
            host.host(),
            _legs([fotToken, fotToken], [uint256(100 * 1e18 + 1), 890 * 1e18 - 1]),
            _legs([daiToken, daiToken], [uint256(100 * 1e18), 890 * 1e18])
        );
        TokenInfo[] memory predispatch = new TokenInfo[](1);
        predispatch[0] = TokenInfo({token: fotToken, amount: sent});
        order.predispatch = DispatchInfo({assets: predispatch, call: abi.encode(new Call[](0))});

        uint256 leg0 = (order.inputs[0].amount * arrived) / atDispatcher;
        uint256 leg1 = (order.inputs[1].amount * arrived) / atDispatcher;
        uint256 remainder = arrived - leg0 - leg1;
        assertEq(remainder, 1, "amounts chosen to exercise the rounding remainder");
        leg0 += remainder;

        vm.prank(user);
        gateway.placeOrder(order, bytes32(0));
        order.source = host.host();
        order.inputs[0].amount = leg0;
        order.inputs[1].amount = leg1;
        bytes32 commitment = keccak256(abi.encode(order));

        assertEq(gateway._orders(commitment, 0), leg0, "leg 0 share plus remainder");
        assertEq(gateway._orders(commitment, 1), leg1, "leg 1 share");
        assertEq(leg0 + leg1, arrived, "legs escrow exactly what arrived");
        assertEq(fot.balanceOf(address(gateway)), arrived, "gateway holds what arrived");
    }

    // ── cross-chain ───────────────────────────────────────────────────────────

    /// @notice Source-side cancel proves each leg's own `_partialFills` slot, so legs repeating an output
    /// token get distinct proof keys and are refunded independently.
    function testCrossChainCancelFromSource_ProvesEachLegSeparately() public {
        (Order memory order, bytes32 commitment) = _place(gateway, _ladder("", bytes("DEST_CHAIN")));

        vm.recordLogs();
        vm.prank(user);
        gateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: uint64(order.deadline + 1)}));
        Vm.Log[] memory logs = vm.getRecordedLogs();
        bytes[] memory keys;
        for (uint256 i; i < logs.length; i++) {
            if (logs[i].emitter != address(host) || logs[i].topics[0] != GET_REQUEST_EVENT) continue;
            (,,, keys,,,,,) =
                abi.decode(logs[i].data, (string, string, bytes, bytes[], uint256, uint256, uint256, bytes, uint256));
        }
        assertEq(keys.length, 2, "one key per leg");
        assertEq(
            keys[0], bytes.concat(abi.encodePacked(address(gateway)), _partialFillSlot(commitment, 0)), "leg 0 key"
        );
        assertEq(
            keys[1], bytes.concat(abi.encodePacked(address(gateway)), _partialFillSlot(commitment, 1)), "leg 1 key"
        );

        // Destination filled half of leg 0 and none of leg 1. Values arrive sorted by key, not by leg.
        uint256[] memory totals = new uint256[](2);
        totals[0] = 1200 * 1e18;
        totals[1] = 990 * 1e18;
        StorageValue[] memory values = new StorageValue[](2);
        values[0] = StorageValue({key: keys[1], value: ""});
        values[1] = StorageValue({key: keys[0], value: abi.encodePacked(bytes1(0x89), bytes9(uint72(600 * 1e18)))});
        GetRequest memory request = GetRequest({
            source: host.host(),
            dest: bytes("DEST_CHAIN"),
            nonce: 0,
            from: abi.encodePacked(address(gateway)),
            keys: keys,
            height: uint64(order.deadline + 1),
            timeoutTimestamp: 0,
            context: abi.encode(commitment, order.user, order.inputs, totals)
        });

        uint256 before = usdc.balanceOf(user);
        vm.prank(address(host));
        gateway.onGetResponse(
            IncomingGetResponse({response: GetResponse({request: request, values: values}), relayer: relayer})
        );

        assertEq(usdc.balanceOf(user) - before, 600 * 1e6 + 1000 * 1e6, "leg 0 unfilled half and all of leg 1");
        assertEq(gateway._orders(commitment, 0), 600 * 1e6, "leg 0 keeps the half its solver will redeem");
        assertEq(gateway._orders(commitment, 1), 0, "leg 1 fully refunded");
    }

    /// @notice On the destination, legs repeating an output token track progress separately, and a
    /// destination cancel refunds each leg's unredeemed fraction.
    function testCrossChainFillAndCancelFromDest_TrackLegsSeparately() public {
        Order memory order = _ladder(bytes("SOURCE_CHAIN"), host.host());
        bytes32 commitment = keccak256(abi.encode(order));

        vm.expectEmit(true, false, false, true, address(gateway));
        emit IntentsBase.PartialFill(
            commitment,
            solverA,
            _legs([daiToken, daiToken], [uint256(1200 * 1e18), 0]),
            _legs([usdcToken, usdcToken], [uint256(1200 * 1e6), 0])
        );
        _fill(solverA, order, 1200 * 1e18, 0);
        _fill(solverB, order, 0, 495 * 1e18); // half of leg 1
        assertEq(gateway._partialFills(commitment, 0), 1200 * 1e18, "leg 0 complete");
        assertEq(gateway._partialFills(commitment, 1), 495 * 1e18, "leg 1 half");
        assertEq(gateway._filled(commitment), address(0), "order open while leg 1 is unpaid");

        vm.recordLogs();
        vm.prank(user);
        gateway.cancelOrder(order, CancelOptions({relayerFee: 0, height: 0}));
        Vm.Log[] memory logs = vm.getRecordedLogs();
        bytes memory body;
        for (uint256 i; i < logs.length; i++) {
            if (logs[i].emitter != address(host) || logs[i].topics[0] != POST_REQUEST_EVENT) continue;
            (,,,,, body,) = abi.decode(logs[i].data, (string, string, bytes, uint256, uint256, bytes, uint256));
        }
        assertEq(uint8(body[0]), uint8(IntentsBase.RequestKind.RefundEscrow), "refund dispatched");
        bytes memory encoded = new bytes(body.length - 1);
        for (uint256 i; i < encoded.length; i++) {
            encoded[i] = body[i + 1];
        }
        WithdrawalRequest memory refund = abi.decode(encoded, (WithdrawalRequest));
        assertEq(refund.tokens.length, 2, "one entry per leg");
        assertEq(refund.tokens[0].amount, 0, "leg 0 fully redeemed by its solver");
        assertEq(refund.tokens[1].amount, 500 * 1e6, "leg 1 unredeemed half");
    }

    /// @notice On the source chain a redeem draws only on its own leg: leg 1 cannot take more than its escrow
    /// even when the gateway holds enough of the same token for other legs and orders.
    function testCrossChainRedeem_DrawsOnlyOnItsOwnLeg() public {
        (, bytes32 commitment) = _place(gateway, _ladder("", bytes("DEST_CHAIN")));
        deal(address(usdc), address(gateway), usdc.balanceOf(address(gateway)) + 5000 * 1e6); // other orders

        uint256 before = usdc.balanceOf(solverA);
        _redeem(
            IntentsBase.RequestKind.RedeemEscrowPartial,
            commitment,
            _legs([usdcToken, usdcToken], [uint256(1200 * 1e6), 0]),
            solverA
        );
        assertEq(usdc.balanceOf(solverA) - before, 1200 * 1e6, "leg 0 redeemed");
        assertEq(gateway._orders(commitment, 0), 0, "leg 0 drained");
        assertEq(gateway._orders(commitment, 1), 1000 * 1e6, "leg 1 untouched");

        IncomingPostRequest memory tooMuch = _redeemRequest(
            IntentsBase.RequestKind.RedeemEscrowPartial,
            commitment,
            _legs([usdcToken, usdcToken], [uint256(0), 1000 * 1e6 + 1]),
            solverB
        );
        vm.prank(address(host));
        vm.expectRevert(stdError.arithmeticError);
        gateway.onAccept(tooMuch);

        _redeem(
            IntentsBase.RequestKind.RedeemEscrow,
            commitment,
            _legs([usdcToken, usdcToken], [uint256(0), 1000 * 1e6]),
            solverB
        );
        assertEq(gateway._orders(commitment, 1), 0, "leg 1 redeemed");
        assertEq(gateway._filled(commitment), solverB, "completing redeem finalizes");
    }
}
