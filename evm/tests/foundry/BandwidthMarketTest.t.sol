// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {DispatchPost} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";

import {BandwidthMarket, BandwidthPurchaseMsg} from "../../src/apps/BandwidthMarket.sol";

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

contract BandwidthMarketTest is Test {
    BandwidthMarket internal market;
    MockHost internal host;
    MockStable internal stable;

    address internal constant APP = address(0xA1);
    address internal constant BUYER = address(0xB0B);
    address internal constant TREASURY = address(0xCAFE);
    bytes internal constant HYPERBRIDGE_ID = bytes("hb-test");

    /// $0.001 per byte expressed in canonical 18-decimal units.
    uint256 internal constant PRICE_PER_BYTE_18D = 1e15;

    function setUp() public {
        stable = new MockStable("USD Coin", "USDC", 6);
        host = new MockHost(HYPERBRIDGE_ID, address(stable));
        market = new BandwidthMarket(address(host), PRICE_PER_BYTE_18D);
    }

    function testConstructorState() public view {
        assertEq(market.host_(), address(host));
        assertEq(market.pricePerByte(), PRICE_PER_BYTE_18D);
    }

    /// Tokens with > 18 decimals underflow the `18 - dec` scaling and
    /// revert with the EVM arithmetic panic.
    function testRejectsHighDecimalTokenAtPurchase() public {
        MockStable weird = new MockStable("Weird", "WRD", 24);
        host.setFeeToken(address(weird));
        weird.mint(BUYER, 1e24);

        vm.startPrank(BUYER);
        weird.approve(address(market), 1e24);
        vm.expectRevert(stdError.arithmeticError);
        market.purchase(APP, 1e24);
        vm.stopPrank();
    }

    /// $1 USDC (6d) = 1e6 raw → 1e18 scaled → /1e15 = 1000 bytes.
    function testPurchaseScalesSixDecimalToken() public {
        uint256 amount = 1_000_000; // $1 in USDC
        stable.mint(BUYER, amount);

        vm.startPrank(BUYER);
        stable.approve(address(market), amount);
        bytes32 commit = market.purchase(APP, amount);
        vm.stopPrank();

        assertTrue(host.dispatched(), "dispatch not invoked");
        assertEq(commit, host.lastCommitment(), "commitment not returned to caller");
        assertEq(stable.balanceOf(address(market)), amount, "market did not pull funds");

        DispatchPost memory post = _readPost();
        BandwidthPurchaseMsg memory body = abi.decode(post.body, (BandwidthPurchaseMsg));
        assertEq(body.app, APP);
        assertEq(body.bytesPurchased, 1000);
        assertEq(body.amountPaid, 1e18, "amount must be rescaled to 18 decimals");
        assertEq(post.fee, 0, "purchase carries no relayer fee");
        assertEq(post.timeout, 0, "purchase has no timeout");
        assertEq(post.payer, address(market));
    }

    /// Recharges accumulate on the pallet side; here we only check that
    /// each `purchase` call pulls funds (mock host has no nonce, so
    /// commitments collide).
    function testRecurringPurchasesPullFundsEachTime() public {
        uint256 amount = 1_000_000;
        stable.mint(BUYER, amount * 2);

        vm.startPrank(BUYER);
        stable.approve(address(market), amount * 2);
        market.purchase(APP, amount);
        market.purchase(APP, amount);
        vm.stopPrank();

        assertEq(stable.balanceOf(address(market)), amount * 2);
    }

    /// Same $1 in an 18-decimal stablecoin must yield the same byte count
    /// as the 6-decimal case — the scaling-logic invariant.
    function testPurchaseEighteenDecimalTokenMatchesSixDecimal() public {
        MockStable bsc = new MockStable("USDC.bsc", "USDC", 18);
        MockHost bscHost = new MockHost(HYPERBRIDGE_ID, address(bsc));
        BandwidthMarket bscMarket = new BandwidthMarket(address(bscHost), PRICE_PER_BYTE_18D);

        uint256 amount = 1e18; // $1 on an 18-decimal chain
        bsc.mint(BUYER, amount);

        vm.startPrank(BUYER);
        bsc.approve(address(bscMarket), amount);
        bscMarket.purchase(APP, amount);
        vm.stopPrank();

        DispatchPost memory post = _readPost(bscHost);
        BandwidthPurchaseMsg memory body = abi.decode(post.body, (BandwidthPurchaseMsg));
        assertEq(body.bytesPurchased, 1000, "decimals normalisation broken");
        assertEq(body.amountPaid, 1e18, "amount already at 18 decimals must pass through");
    }

    /// A host-side feeToken swap must be picked up by the market
    /// without redeployment.
    function testHostFeeTokenSwapTakesEffect() public {
        MockStable next = new MockStable("USD Coin", "USDC", 6);
        host.setFeeToken(address(next));

        uint256 amount = 1_000_000;
        next.mint(BUYER, amount);

        vm.startPrank(BUYER);
        next.approve(address(market), amount);
        market.purchase(APP, amount);
        vm.stopPrank();

        assertEq(next.balanceOf(address(market)), amount, "market pulled from new feeToken");
        assertEq(stable.balanceOf(address(market)), 0, "old feeToken untouched");
    }

    function testRejectsBelowMinimumPurchase() public {
        // 1 raw unit of a 6-decimal token == 1e12 in 18-d → < pricePerByte (1e15).
        uint256 dust = 1;
        stable.mint(BUYER, dust);

        vm.startPrank(BUYER);
        stable.approve(address(market), dust);
        vm.expectRevert(BandwidthMarket.BelowMinimum.selector);
        market.purchase(APP, dust);
        vm.stopPrank();
    }

    function testRejectsZeroAmount() public {
        vm.expectRevert(BandwidthMarket.InvalidPurchase.selector);
        market.purchase(APP, 0);
    }

    function testOnAcceptRejectsNonHostCaller() public {
        IncomingPostRequest memory inc = _governanceRequest(_setPriceBody(2e15));
        vm.expectRevert();
        market.onAccept(inc);
    }

    /// Defence-in-depth: even if the local host delivered it, only
    /// hyperbridge-source messages may govern the market.
    function testOnAcceptRejectsNonHyperbridgeSource() public {
        IncomingPostRequest memory inc = _governanceRequest(_setPriceBody(2e15));
        inc.request.source = bytes("some-other-chain");
        vm.prank(address(host));
        vm.expectRevert(BandwidthMarket.UnauthorizedAction.selector);
        market.onAccept(inc);
    }

    function testGovernanceCanUpdatePricePerByte() public {
        uint256 newPrice = 2e15;
        IncomingPostRequest memory inc = _governanceRequest(_setPriceBody(newPrice));

        vm.prank(address(host));
        market.onAccept(inc);

        assertEq(market.pricePerByte(), newPrice);
    }

    function testGovernanceCanWithdraw() public {
        uint256 amount = 5_000_000; // $5, seeded via a real purchase
        stable.mint(BUYER, amount);
        vm.startPrank(BUYER);
        stable.approve(address(market), amount);
        market.purchase(APP, amount);
        vm.stopPrank();

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthMarket.OnAcceptActions.Withdraw)),
            abi.encode(address(stable), TREASURY, amount)
        );
        IncomingPostRequest memory inc = _governanceRequest(body);

        vm.prank(address(host));
        market.onAccept(inc);

        assertEq(stable.balanceOf(TREASURY), amount, "withdrawal did not credit beneficiary");
        assertEq(stable.balanceOf(address(market)), 0, "market still holds funds");
    }

    /// Withdraw can recover balances of a token that is no longer the
    /// host's `feeToken()` — the explicit token address makes feeToken
    /// swaps non-destructive.
    function testGovernanceCanWithdrawStaleToken() public {
        uint256 amount = 1_000_000;
        stable.mint(BUYER, amount);
        vm.startPrank(BUYER);
        stable.approve(address(market), amount);
        market.purchase(APP, amount);
        vm.stopPrank();

        // Host swaps to a brand-new feeToken; old USDC is now "stale".
        MockStable next = new MockStable("USD Coin", "USDC", 6);
        host.setFeeToken(address(next));

        bytes memory body = bytes.concat(
            bytes1(uint8(BandwidthMarket.OnAcceptActions.Withdraw)),
            abi.encode(address(stable), TREASURY, amount)
        );
        IncomingPostRequest memory inc = _governanceRequest(body);

        vm.prank(address(host));
        market.onAccept(inc);

        assertEq(stable.balanceOf(TREASURY), amount, "stale token must still be recoverable");
    }

    // ----- helpers -----

    function _setPriceBody(uint256 newPrice) internal pure returns (bytes memory) {
        return bytes.concat(
            bytes1(uint8(BandwidthMarket.OnAcceptActions.SetPricePerByte)), abi.encode(newPrice)
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
