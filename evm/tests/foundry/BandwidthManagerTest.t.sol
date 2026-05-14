// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {Vm} from "forge-std/Vm.sol";

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

import {TestConsensusClient} from "./TestConsensusClient.sol";
import {TestHost} from "./TestHost.sol";
import {FeeToken} from "./FeeToken.sol";
import {HandlerV1} from "../../src/core/HandlerV1.sol";
import {HostParams, PerByteFee} from "../../src/core/EvmHost.sol";
import {HostManager, HostManagerParams} from "../../src/core/HostManager.sol";

import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";

import {BandwidthManager, BandwidthPurchaseMsg, Tier, Withdrawal} from "../../src/apps/BandwidthManager.sol";

/// 6-decimal stablecoin used by the multi-decimal scaling test.
contract Stable6d is ERC20 {
    constructor(string memory name_, string memory symbol_) ERC20(name_, symbol_) {}

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract BandwidthManagerTest is Test {
    bytes internal constant PALLET_TO = bytes("BWMARKET");
    bytes internal constant APP = hex"a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
    bytes internal constant APP_CHAIN = bytes("EVM-8453");

    address internal constant BUYER = address(0xB0B);
    address internal constant TREASURY = address(0xCAFE);

    uint256 internal constant PARA_ID = 2000;
    uint256 internal constant TIER1 = 1;
    uint256 internal constant TIER1_PRICE_18D = 1e18;

    /// Signature hash of `EvmHost.PostRequestEvent` — used to find the
    /// dispatch log without hand-rolling a full ABI for `Vm.Log`.
    bytes32 internal constant POST_REQUEST_SIG = keccak256(
        "PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)"
    );

    TestHost internal host;
    FeeToken internal feeToken;
    BandwidthManager internal manager;

    function setUp() public {
        // Pin chain id so `host.host()` is stable across runs.
        vm.chainId(1337);

        feeToken = new FeeToken(address(this), "HyperUSD", "USD.h");
        feeToken.grantMinterRole(address(this));

        host = _deployHost(address(feeToken));
        manager = new BandwidthManager(address(this));
        manager.setHost(address(host));

        _setTier(manager, host, TIER1, TIER1_PRICE_18D);
    }

    // ---------- happy path ----------

    /// End-to-end: buyer is debited, manager is credited, the real
    /// host stores the commitment, and its `PostRequestEvent` carries
    /// the round-tripping `BandwidthPurchaseMsg`.
    function testPurchaseDispatchesViaRealHost() public {
        feeToken.mint(BUYER, TIER1_PRICE_18D);
        vm.startPrank(BUYER);
        feeToken.approve(address(manager), TIER1_PRICE_18D);

        vm.recordLogs();
        bytes32 commitment = manager.purchase(APP, TIER1, 1, APP_CHAIN);
        vm.stopPrank();

        assertEq(feeToken.balanceOf(BUYER), 0, "buyer not debited");
        assertEq(feeToken.balanceOf(address(manager)), TIER1_PRICE_18D, "manager not credited");
        assertEq(host.requestCommitments(commitment).sender, address(manager), "host did not record commitment");

        BandwidthPurchaseMsg memory body = _findDispatchedBody(vm.getRecordedLogs(), address(host));
        assertEq(body.app, APP);
        assertEq(body.tier, TIER1);
        assertEq(body.months, 1);
        assertEq(body.chain, APP_CHAIN);
    }

    function testBulkPurchaseChargesProportionally() public {
        uint256 months = 6;
        uint256 expected = TIER1_PRICE_18D * months;
        feeToken.mint(BUYER, expected);

        vm.startPrank(BUYER);
        feeToken.approve(address(manager), expected);
        vm.recordLogs();
        manager.purchase(APP, TIER1, months, APP_CHAIN);
        vm.stopPrank();

        assertEq(feeToken.balanceOf(address(manager)), expected, "bulk price not multiplied");
        BandwidthPurchaseMsg memory body = _findDispatchedBody(vm.getRecordedLogs(), address(host));
        assertEq(body.months, months, "months not propagated to pallet");
    }

    // ---------- multi-decimal scaling ----------

    /// Verifies the `% scale` guard against a real 6-decimal token.
    /// $1 in 18-d → 1e6 raw on a 6-d token, scaled cleanly.
    function testScalesAgainstSixDecimalStablecoin() public {
        Stable6d usd = new Stable6d("USD Coin", "USDC");
        TestHost usdHost = _deployHost(address(usd));
        BandwidthManager usdMgr = new BandwidthManager(address(this));
        usdMgr.setHost(address(usdHost));
        _setTier(usdMgr, usdHost, TIER1, TIER1_PRICE_18D);

        uint256 expected = 1_000_000;
        usd.mint(BUYER, expected);
        vm.startPrank(BUYER);
        usd.approve(address(usdMgr), expected);
        usdMgr.purchase(APP, TIER1, 1, APP_CHAIN);
        vm.stopPrank();

        assertEq(usd.balanceOf(address(usdMgr)), expected);
    }

    /// 1e11 in 18-d is sub-microcent on a 6-d token — would scale to
    /// 0 raw, so the manager must reject before silently undercharging.
    function testRejectsNonRepresentablePrice() public {
        Stable6d usd = new Stable6d("USD Coin", "USDC");
        TestHost usdHost = _deployHost(address(usd));
        BandwidthManager usdMgr = new BandwidthManager(address(this));
        usdMgr.setHost(address(usdHost));
        _setTier(usdMgr, usdHost, 2, 1e11);

        vm.expectRevert(BandwidthManager.PriceNotRepresentable.selector);
        vm.prank(BUYER);
        usdMgr.purchase(APP, 2, 1, APP_CHAIN);
    }

    // ---------- rejection paths ----------

    function testRejectsUnknownTier() public {
        vm.expectRevert(BandwidthManager.UnknownTier.selector);
        vm.prank(BUYER);
        manager.purchase(APP, 99, 1, APP_CHAIN);
    }

    function testRejectsEmptyApp() public {
        vm.expectRevert(BandwidthManager.InvalidPurchase.selector);
        vm.prank(BUYER);
        manager.purchase(hex"", TIER1, 1, APP_CHAIN);
    }

    function testRejectsEmptyAppChain() public {
        vm.expectRevert(BandwidthManager.InvalidPurchase.selector);
        vm.prank(BUYER);
        manager.purchase(APP, TIER1, 1, hex"");
    }

    function testRejectsZeroMonths() public {
        vm.expectRevert(BandwidthManager.InvalidPurchase.selector);
        vm.prank(BUYER);
        manager.purchase(APP, TIER1, 0, APP_CHAIN);
    }

    // ---------- governance ----------

    function testOnAcceptRejectsNonHostCaller() public {
        IncomingPostRequest memory req =
            _governanceRequestFor(host, _setTiersBody(TIER1, TIER1_PRICE_18D));
        vm.expectRevert();
        manager.onAccept(req);
    }

    /// Defence-in-depth: even if the local host delivered it, only
    /// hyperbridge-source messages may govern the manager.
    function testOnAcceptRejectsNonHyperbridgeSource() public {
        IncomingPostRequest memory inc =
            _governanceRequestFor(host, _setTiersBody(TIER1, TIER1_PRICE_18D));
        inc.request.source = bytes("some-other-chain");
        vm.prank(address(host));
        vm.expectRevert(BandwidthManager.UnauthorizedAction.selector);
        manager.onAccept(inc);
    }

    function testGovernanceCanSetMultipleTiers() public {
        Tier[] memory updates = new Tier[](3);
        updates[0] = Tier({tier: 1, price: 5e18});
        updates[1] = Tier({tier: 2, price: 50e18});
        updates[2] = Tier({tier: 3, price: 500e18});

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.SetTiers)), abi.encode(updates)
        );
        IncomingPostRequest memory req = _governanceRequestFor(host, body);
        vm.prank(address(host));
        manager.onAccept(req);

        assertEq(manager.tierPrice(1), 5e18);
        assertEq(manager.tierPrice(2), 50e18);
        assertEq(manager.tierPrice(3), 500e18);
    }

    /// Withdraw names the token explicitly in the body, so it
    /// recovers both the live fee token and any stale balance left
    /// behind by a host-side fee-token swap.
    function testGovernanceCanWithdrawAcrossTokenSwap() public {
        feeToken.mint(BUYER, TIER1_PRICE_18D);
        vm.startPrank(BUYER);
        feeToken.approve(address(manager), TIER1_PRICE_18D);
        manager.purchase(APP, TIER1, 1, APP_CHAIN);
        vm.stopPrank();

        // Stand-in for "old fee token after a swap" — the manager
        // shouldn't care which ERC20 it's asked to ship out.
        FeeToken stale = new FeeToken(address(this), "OldUSD", "OUSD");
        stale.grantMinterRole(address(this));
        stale.mint(address(manager), 999e18);

        IncomingPostRequest memory withdrawLive = _governanceRequestFor(
            host,
            bytes.concat(
                bytes1(uint8(BandwidthManager.OnAcceptActions.Withdraw)),
                abi.encode(Withdrawal({token: address(feeToken), beneficiary: TREASURY, amount: TIER1_PRICE_18D}))
            )
        );
        vm.prank(address(host));
        manager.onAccept(withdrawLive);
        assertEq(feeToken.balanceOf(TREASURY), TIER1_PRICE_18D, "live fee token not withdrawn");

        IncomingPostRequest memory withdrawStale = _governanceRequestFor(
            host,
            bytes.concat(
                bytes1(uint8(BandwidthManager.OnAcceptActions.Withdraw)),
                abi.encode(Withdrawal({token: address(stale), beneficiary: TREASURY, amount: 999e18}))
            )
        );
        vm.prank(address(host));
        manager.onAccept(withdrawStale);
        assertEq(stale.balanceOf(TREASURY), 999e18, "stale token must still be recoverable");
    }

    // ---------- helpers ----------

    function _deployHost(address feeTokenAddr) internal returns (TestHost h) {
        TestConsensusClient cc = new TestConsensusClient();
        HandlerV1 handler = new HandlerV1();
        HostManager hm = new HostManager(HostManagerParams({admin: address(this), host: address(0)}));

        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = PARA_ID;
        PerByteFee[] memory pbf = new PerByteFee[](0);

        // `defaultPerByteFee = 0` so the manager doesn't need a
        // treasury balance to cover dispatch fees on top of the
        // tier price — purchase chains are expected to subsidise
        // their own dispatch.
        HostParams memory params = HostParams({
            uniswapV2: address(0),
            perByteFees: pbf,
            admin: address(this),
            hostManager: address(hm),
            handler: address(handler),
            defaultTimeout: 0,
            unStakingPeriod: 21 * (60 * 60 * 24),
            challengePeriod: 0,
            consensusClient: address(cc),
            defaultPerByteFee: 0,
            stateCommitmentFee: 0,
            feeToken: feeTokenAddr,
            hyperbridge: StateMachine.kusama(PARA_ID),
            stateMachines: stateMachines
        });
        h = new TestHost(params);
        hm.setIsmpHost(address(h));
    }

    function _setTier(BandwidthManager m, TestHost h, uint256 tier, uint256 price18d) internal {
        IncomingPostRequest memory req = _governanceRequestFor(h, _setTiersBody(tier, price18d));
        vm.prank(address(h));
        m.onAccept(req);
    }

    function _setTiersBody(uint256 tier, uint256 price18d) internal pure returns (bytes memory) {
        Tier[] memory updates = new Tier[](1);
        updates[0] = Tier({tier: tier, price: price18d});
        return bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.SetTiers)), abi.encode(updates)
        );
    }

    function _governanceRequestFor(TestHost h, bytes memory body)
        internal
        view
        returns (IncomingPostRequest memory)
    {
        return IncomingPostRequest({
            request: PostRequest({
                source: h.hyperbridge(),
                dest: h.host(),
                nonce: 0,
                from: bytes("hyperbridge-governance"),
                to: PALLET_TO,
                timeoutTimestamp: 0,
                body: body
            }),
            relayer: address(0)
        });
    }

    /// Walks the recorded logs for the real host's `PostRequestEvent`
    /// and decodes the dispatched body. Reverts if no matching log
    /// is found, so callers can trust the return value.
    function _findDispatchedBody(Vm.Log[] memory logs, address emitter)
        internal
        pure
        returns (BandwidthPurchaseMsg memory)
    {
        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].emitter != emitter) continue;
            if (logs[i].topics.length == 0 || logs[i].topics[0] != POST_REQUEST_SIG) continue;
            (,, bytes memory to,,, bytes memory body,) =
                abi.decode(logs[i].data, (string, string, bytes, uint256, uint256, bytes, uint256));
            require(keccak256(to) == keccak256(PALLET_TO), "unexpected dispatch destination");
            return abi.decode(body, (BandwidthPurchaseMsg));
        }
        revert("PostRequestEvent not emitted by host");
    }
}
