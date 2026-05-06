// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {DispatchPost} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";

import {BandwidthManager, BandwidthPurchaseMsg} from "../../src/apps/BandwidthManager.sol";

/// Stablecoin mock with configurable decimals.
contract MockStable is ERC20 {
    uint8 private immutable _dec;

    constructor(string memory name_, string memory symbol_, uint8 dec_) ERC20(name_, symbol_) {
        _dec = dec_;
    }

    function decimals() public view override returns (uint8) {
        return _dec;
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// Stand-in for `EvmHost` that records the last dispatched payload and
/// exposes a mutable `feeToken()` so tests can simulate a host-side
/// fee-token swap.
contract MockHost {
    bytes public hyperbridgeId;
    address public feeTokenAddr;
    DispatchPost public lastPost;
    bool public dispatched;
    bytes32 public lastCommitment;

    constructor(bytes memory hb, address feeToken_) {
        hyperbridgeId = hb;
        feeTokenAddr = feeToken_;
    }

    function hyperbridge() external view returns (bytes memory) {
        return hyperbridgeId;
    }

    function feeToken() external view returns (address) {
        return feeTokenAddr;
    }

    function setFeeToken(address t) external {
        feeTokenAddr = t;
    }

    function dispatch(DispatchPost memory post) external returns (bytes32) {
        lastPost = post;
        dispatched = true;
        lastCommitment = keccak256(abi.encode(post));
        return lastCommitment;
    }
}

contract BandwidthManagerTest is Test {
    BandwidthManager internal manager;
    MockHost internal host;
    MockStable internal stable;

    bytes internal constant APP = hex"a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
    bytes internal constant APP_CHAIN = bytes("EVM-8453"); // Base. Opaque to the contract; pallet parses via `StateMachine::FromStr`.
    address internal constant BUYER = address(0xB0B);
    address internal constant TREASURY = address(0xCAFE);
    bytes internal constant HYPERBRIDGE_ID = bytes("hb-test");

    /// $1 expressed in canonical 18-decimal units — used as tier 1's price.
    uint256 internal constant TIER1_PRICE_18D = 1e18;
    uint256 internal constant TIER1 = 1;

    function setUp() public {
        stable = new MockStable("USD Coin", "USDC", 6);
        host = new MockHost(HYPERBRIDGE_ID, address(stable));
        manager = new BandwidthManager(address(host));

        _setTier(TIER1, TIER1_PRICE_18D);
    }

    function testConstructorState() public view {
        assertEq(manager.host_(), address(host));
    }

    function testRejectsUnknownTier() public {
        vm.startPrank(BUYER);
        vm.expectRevert(BandwidthManager.UnknownTier.selector);
        manager.purchase(APP, 99, APP_CHAIN);
        vm.stopPrank();
    }

    function testRejectsEmptyApp() public {
        vm.startPrank(BUYER);
        vm.expectRevert(BandwidthManager.InvalidPurchase.selector);
        manager.purchase(hex"", TIER1, APP_CHAIN);
        vm.stopPrank();
    }

    function testRejectsEmptyAppChain() public {
        vm.startPrank(BUYER);
        vm.expectRevert(BandwidthManager.InvalidPurchase.selector);
        manager.purchase(APP, TIER1, hex"");
        vm.stopPrank();
    }

    /// $1 USDC (6d) = 1e6 raw — tier price scales down cleanly.
    function testPurchaseScalesSixDecimalToken() public {
        uint256 expectedAmount = 1_000_000;
        stable.mint(BUYER, expectedAmount);

        vm.startPrank(BUYER);
        stable.approve(address(manager), expectedAmount);
        bytes32 commit = manager.purchase(APP, TIER1, APP_CHAIN);
        vm.stopPrank();

        assertTrue(host.dispatched(), "dispatch not invoked");
        assertEq(commit, host.lastCommitment(), "commitment not returned to caller");
        assertEq(stable.balanceOf(address(manager)), expectedAmount, "manager did not pull funds");

        DispatchPost memory post = _readPost();
        BandwidthPurchaseMsg memory body = abi.decode(post.body, (BandwidthPurchaseMsg));
        assertEq(body.app, APP);
        assertEq(body.tier, TIER1);
        assertEq(body.appChain, APP_CHAIN);
        assertEq(post.fee, 0, "purchase carries no relayer fee");
        assertEq(post.timeout, 0, "purchase has no timeout");
        assertEq(post.payer, address(manager));
    }

    /// 18-decimal feeToken — same tier price, no rescaling.
    function testPurchaseEighteenDecimalToken() public {
        MockStable bsc = new MockStable("USDC.bsc", "USDC", 18);
        MockHost bscHost = new MockHost(HYPERBRIDGE_ID, address(bsc));
        BandwidthManager bscMgr = new BandwidthManager(address(bscHost));
        _setTierOn(bscMgr, bscHost, TIER1, TIER1_PRICE_18D);

        bsc.mint(BUYER, TIER1_PRICE_18D);

        vm.startPrank(BUYER);
        bsc.approve(address(bscMgr), TIER1_PRICE_18D);
        bscMgr.purchase(APP, TIER1, APP_CHAIN);
        vm.stopPrank();

        assertEq(bsc.balanceOf(address(bscMgr)), TIER1_PRICE_18D);
    }

    /// Tier price not divisible by `10**(18-dec)` is rejected so a
    /// silent floor-rounding can't drain or undercharge the buyer.
    function testRejectsNonRepresentablePrice() public {
        // 1e11 in 18-d is sub-microcent on a 6-d token (would scale to 0).
        _setTier(2, 1e11);

        vm.startPrank(BUYER);
        vm.expectRevert(BandwidthManager.PriceNotRepresentable.selector);
        manager.purchase(APP, 2, APP_CHAIN);
        vm.stopPrank();
    }

    function testRecurringPurchasesPullFundsEachTime() public {
        uint256 amount = 1_000_000;
        stable.mint(BUYER, amount * 2);

        vm.startPrank(BUYER);
        stable.approve(address(manager), amount * 2);
        manager.purchase(APP, TIER1, APP_CHAIN);
        manager.purchase(APP, TIER1, APP_CHAIN);
        vm.stopPrank();

        assertEq(stable.balanceOf(address(manager)), amount * 2);
    }

    /// A host-side feeToken swap must be picked up by the manager
    /// without redeployment.
    function testHostFeeTokenSwapTakesEffect() public {
        MockStable next = new MockStable("USD Coin", "USDC", 6);
        host.setFeeToken(address(next));

        next.mint(BUYER, 1_000_000);

        vm.startPrank(BUYER);
        next.approve(address(manager), 1_000_000);
        manager.purchase(APP, TIER1, APP_CHAIN);
        vm.stopPrank();

        assertEq(next.balanceOf(address(manager)), 1_000_000, "manager pulled from new feeToken");
        assertEq(stable.balanceOf(address(manager)), 0, "old feeToken untouched");
    }

    function testOnAcceptRejectsNonHostCaller() public {
        IncomingPostRequest memory inc = _governanceRequest(_setTiersBody(TIER1, TIER1_PRICE_18D));
        vm.expectRevert();
        manager.onAccept(inc);
    }

    /// Defence-in-depth: even if the local host delivered it, only
    /// hyperbridge-source messages may govern the manager.
    function testOnAcceptRejectsNonHyperbridgeSource() public {
        IncomingPostRequest memory inc = _governanceRequest(_setTiersBody(TIER1, TIER1_PRICE_18D));
        inc.request.source = bytes("some-other-chain");
        vm.prank(address(host));
        vm.expectRevert(BandwidthManager.UnauthorizedAction.selector);
        manager.onAccept(inc);
    }

    function testGovernanceCanSetMultipleTiers() public {
        uint256[] memory tiers = new uint256[](3);
        uint256[] memory prices = new uint256[](3);
        tiers[0] = 1;
        tiers[1] = 2;
        tiers[2] = 3;
        prices[0] = 5e18;
        prices[1] = 50e18;
        prices[2] = 500e18;

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.SetTiers)),
            abi.encode(tiers, prices)
        );
        vm.prank(address(host));
        manager.onAccept(_governanceRequest(body));

        assertEq(manager.tierPrice(1), 5e18);
        assertEq(manager.tierPrice(2), 50e18);
        assertEq(manager.tierPrice(3), 500e18);
    }

    function testGovernanceCanWithdraw() public {
        uint256 amount = 1_000_000;
        stable.mint(BUYER, amount);
        vm.startPrank(BUYER);
        stable.approve(address(manager), amount);
        manager.purchase(APP, TIER1, APP_CHAIN);
        vm.stopPrank();

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.Withdraw)),
            abi.encode(address(stable), TREASURY, amount)
        );
        vm.prank(address(host));
        manager.onAccept(_governanceRequest(body));

        assertEq(stable.balanceOf(TREASURY), amount, "withdrawal did not credit beneficiary");
        assertEq(stable.balanceOf(address(manager)), 0, "manager still holds funds");
    }

    /// Withdraw can recover balances of a token that is no longer the
    /// host's `feeToken()` — the explicit token address makes feeToken
    /// swaps non-destructive.
    function testGovernanceCanWithdrawStaleToken() public {
        uint256 amount = 1_000_000;
        stable.mint(BUYER, amount);
        vm.startPrank(BUYER);
        stable.approve(address(manager), amount);
        manager.purchase(APP, TIER1, APP_CHAIN);
        vm.stopPrank();

        // Host swaps to a brand-new feeToken; old USDC is now "stale".
        MockStable next = new MockStable("USD Coin", "USDC", 6);
        host.setFeeToken(address(next));

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.Withdraw)),
            abi.encode(address(stable), TREASURY, amount)
        );
        vm.prank(address(host));
        manager.onAccept(_governanceRequest(body));

        assertEq(stable.balanceOf(TREASURY), amount, "stale token must still be recoverable");
    }

    // ----- helpers -----

    function _setTier(uint256 tier, uint256 price18d) internal {
        _setTierOn(manager, host, tier, price18d);
    }

    function _setTierOn(BandwidthManager m, MockHost h, uint256 tier, uint256 price18d) internal {
        uint256[] memory tiers = new uint256[](1);
        uint256[] memory prices = new uint256[](1);
        tiers[0] = tier;
        prices[0] = price18d;

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.SetTiers)),
            abi.encode(tiers, prices)
        );
        vm.prank(address(h));
        m.onAccept(_governanceRequest(body));
    }

    function _setTiersBody(uint256 tier, uint256 price18d) internal pure returns (bytes memory) {
        uint256[] memory tiers = new uint256[](1);
        uint256[] memory prices = new uint256[](1);
        tiers[0] = tier;
        prices[0] = price18d;
        return bytes.concat(
            bytes1(uint8(BandwidthManager.OnAcceptActions.SetTiers)), abi.encode(tiers, prices)
        );
    }

    function _governanceRequest(bytes memory body) internal pure returns (IncomingPostRequest memory) {
        return IncomingPostRequest({
            request: PostRequest({
                source: HYPERBRIDGE_ID,
                dest: bytes("local"),
                nonce: 0,
                from: bytes("hyperbridge-governance"),
                to: bytes("BWMARKET"),
                timeoutTimestamp: 0,
                body: body
            }),
            relayer: address(0)
        });
    }

    function _readPost() internal view returns (DispatchPost memory) {
        return _readPost(host);
    }

    /// Re-shape `MockHost.lastPost()` (which the auto-getter returns as a
    /// tuple) back into a `DispatchPost` for readable call sites.
    function _readPost(MockHost h) internal view returns (DispatchPost memory post) {
        (
            bytes memory dest,
            bytes memory to,
            bytes memory body,
            uint64 timeout,
            uint256 fee,
            address payer
        ) = h.lastPost();
        post = DispatchPost({dest: dest, to: to, body: body, timeout: timeout, fee: fee, payer: payer});
    }
}
