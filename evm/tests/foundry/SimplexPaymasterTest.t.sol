// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../../src/utils/SimplexPaymaster.sol";

contract MockOracle {
    int256 public answer;
    uint8 public immutable decimals;
    uint256 public updatedAt;

    constructor(int256 _answer, uint8 _decimals) {
        answer = _answer;
        decimals = _decimals;
        updatedAt = block.timestamp;
    }

    function setAnswer(int256 _answer) external {
        answer = _answer;
        updatedAt = block.timestamp;
    }

    function setUpdatedAt(uint256 _updatedAt) external {
        updatedAt = _updatedAt;
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        return (1, answer, updatedAt, updatedAt, 1);
    }
}

contract MockToken is ERC20 {
    uint8 private immutable _decimals;

    constructor(string memory name, uint8 decimals_) ERC20(name, name) {
        _decimals = decimals_;
    }

    function decimals() public view override returns (uint8) {
        return _decimals;
    }
}

/// @dev Exposes internal hooks for direct testing of paymasterData parsing and prefunding.
contract SimplexPaymasterHarness is SimplexPaymaster {
    function fetchDetails(
        PackedUserOperation calldata userOp
    ) external view returns (uint256 validationData, IERC20 token, uint256 tokenPrice) {
        return _fetchDetails(userOp, bytes32(0));
    }

    function validate(
        PackedUserOperation calldata userOp,
        uint256 maxCost
    ) external returns (bytes memory context, uint256 validationData) {
        return _validatePaymasterUserOp(userOp, bytes32(0), maxCost);
    }
}

contract SimplexPaymasterTest is Test {
    // BNB at $600, 8-decimal feed
    int256 constant NATIVE_USD = 600e8;
    // Stablecoin at $1, 8-decimal feed
    int256 constant TOKEN_USD = 1e8;
    // PaymasterERC20._postOpCost()
    uint256 constant POST_OP_COST = 30_000;

    address owner = makeAddr("owner");
    address treasury = makeAddr("treasury");
    address sender = address(0xBEEF);

    MockOracle nativeOracle;
    MockOracle usdcOracle;
    MockToken usdc6; // 6-decimal USDC (Base-style)
    MockToken usdc18; // 18-decimal USDC (BSC-style)
    SimplexPaymasterHarness paymaster;

    function setUp() public {
        vm.warp(1_700_000_000);

        nativeOracle = new MockOracle(NATIVE_USD, 8);
        usdcOracle = new MockOracle(TOKEN_USD, 8);
        usdc6 = new MockToken("USDC6", 6);
        usdc18 = new MockToken("USDC18", 18);

        paymaster = _deployPaymaster(0); // no markup for the base pricing assertions

        vm.startPrank(owner);
        paymaster.registerToken(address(usdc6), AggregatorV3Interface(address(usdcOracle)));
        paymaster.registerToken(address(usdc18), AggregatorV3Interface(address(usdcOracle)));
        vm.stopPrank();
    }

    // ── Pricing ──────────────────────────────────────────────────────

    function testTokenPriceSixDecimals() public view {
        // $600 native, $1 token, 6 decimals: 1 wei of gas costs 6e8 / 1e18 token units.
        // 0.001 BNB (1e15 wei) should cost 0.60 USDC (600_000 units).
        uint256 price = paymaster.getTokenPrice(address(usdc6));
        assertEq(price, 6e8);
        assertEq((1e15 * price) / 1e18, 600_000);
    }

    function testTokenPriceEighteenDecimals() public view {
        uint256 price = paymaster.getTokenPrice(address(usdc18));
        assertEq(price, 6e20);
        // 0.001 BNB should cost 0.6 tokens in 18-decimal units.
        assertEq((1e15 * price) / 1e18, 6e17);
    }

    function testTokenPriceWithMarkup() public {
        vm.prank(owner);
        paymaster.setMarkup(200); // 2%
        assertEq(paymaster.getTokenPrice(address(usdc6)), (6e8 * 10_200) / 10_000);
    }

    function testTokenPriceNormalizesOracleDecimals() public {
        // An 18-decimal token/USD feed must price identically to an 8-decimal one.
        MockOracle oracle18 = new MockOracle(1e18, 18);
        MockToken token = new MockToken("T", 6);
        vm.prank(owner);
        paymaster.registerToken(address(token), AggregatorV3Interface(address(oracle18)));

        assertEq(paymaster.getTokenPrice(address(token)), 6e8);
    }

    function testEstimateTokenCostMatchesErc20Cost() public view {
        uint256 gasAmount = 500_000;
        uint256 maxFeePerGas = 3 gwei;
        uint256 expected = ((gasAmount + POST_OP_COST) * maxFeePerGas * 6e8) / 1e18;
        assertEq(paymaster.estimateTokenCost(address(usdc6), gasAmount, maxFeePerGas), expected);
    }

    // ── Oracle safety ────────────────────────────────────────────────

    function testStaleOracleReverts() public {
        nativeOracle.setUpdatedAt(block.timestamp - paymaster.maxOracleAge() - 1);
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.StaleOraclePrice.selector,
                address(nativeOracle),
                block.timestamp - paymaster.maxOracleAge() - 1
            )
        );
        paymaster.getTokenPrice(address(usdc6));
    }

    function testNonPositiveOraclePriceReverts() public {
        usdcOracle.setAnswer(0);
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.InvalidOraclePrice.selector, address(usdcOracle), int256(0))
        );
        paymaster.getTokenPrice(address(usdc6));
    }

    function testSetMaxOracleAge() public {
        nativeOracle.setUpdatedAt(block.timestamp - 100);
        vm.prank(owner);
        paymaster.setMaxOracleAge(50);
        vm.expectRevert();
        paymaster.getTokenPrice(address(usdc6));
    }

    // ── Token registry ───────────────────────────────────────────────

    function testUnregisteredTokenReverts() public {
        address unknown = makeAddr("unknown");
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotRegistered.selector, unknown));
        paymaster.getTokenPrice(unknown);
    }

    function testDeactivatedTokenRejectedInFetchDetails() public {
        vm.prank(owner);
        paymaster.deactivateToken(address(usdc6));

        PackedUserOperation memory op = _userOpWithPaymasterData(
            abi.encodePacked(uint8(1), address(usdc6))
        );
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotActive.selector, address(usdc6)));
        paymaster.fetchDetails(op);
    }

    function testRegisteredTokensEnumeration() public view {
        address[] memory tokens = paymaster.getRegisteredTokens();
        assertEq(tokens.length, 2);
        assertEq(tokens[0], address(usdc6));
        assertEq(tokens[1], address(usdc18));
    }

    function testReRegisterDoesNotDuplicate() public {
        vm.prank(owner);
        paymaster.registerToken(address(usdc6), AggregatorV3Interface(address(usdcOracle)));
        assertEq(paymaster.getRegisteredTokens().length, 2);
    }

    // ── paymasterData parsing ────────────────────────────────────────

    function testFetchDetailsApproveMode() public view {
        PackedUserOperation memory op = _userOpWithPaymasterData(
            abi.encodePacked(uint8(1), address(usdc6))
        );
        (uint256 validationData, IERC20 token, uint256 tokenPrice) = paymaster.fetchDetails(op);
        assertEq(validationData, 0);
        assertEq(address(token), address(usdc6));
        assertEq(tokenPrice, 6e8);
    }

    function testFetchDetailsInvalidModeReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(
            abi.encodePacked(uint8(2), address(usdc6))
        );
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMode.selector, uint8(2)));
        paymaster.fetchDetails(op);
    }

    function testFetchDetailsShortDataReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(abi.encodePacked(uint8(1)));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(1)));
        paymaster.fetchDetails(op);
    }

    // ── Prefund ──────────────────────────────────────────────────────

    function testPrefundTransfersTokensFromSender() public {
        _fundAndApprove(1_000e6);

        PackedUserOperation memory op = _userOpWithPaymasterData(
            abi.encodePacked(uint8(1), address(usdc6))
        );
        // 0.001 native at $600 costs about 0.62 USDC with the postOp cushion.
        (, uint256 validationData) = paymaster.validate(op, 1e15);
        assertEq(validationData, 0);
        assertGt(usdc6.balanceOf(address(paymaster)), 0);
    }

    // ── Initialization ───────────────────────────────────────────────

    function testInitializeOnlyOnce() public {
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        paymaster.initialize(AggregatorV3Interface(address(nativeOracle)), 0, treasury, owner);
    }

    function testImplementationCannotBeInitialized() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        implementation.initialize(AggregatorV3Interface(address(nativeOracle)), 0, treasury, owner);
    }

    // ── Upgrades ─────────────────────────────────────────────────────

    function testScheduleUpgradeOnlyOwner() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.expectRevert();
        paymaster.scheduleUpgrade(newImpl);
    }

    function testUpgradeWithoutScheduleReverts() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.UpgradeNotScheduled.selector, newImpl));
        paymaster.upgradeToAndCall(newImpl, "");
    }

    function testUpgradeBeforeDelayReverts() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.startPrank(owner);
        paymaster.scheduleUpgrade(newImpl);

        uint256 executableAfter = paymaster.pendingUpgradeAfter();
        vm.warp(executableAfter - 1);
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.UpgradeDelayNotElapsed.selector, executableAfter)
        );
        paymaster.upgradeToAndCall(newImpl, "");
        vm.stopPrank();
    }

    function testUpgradeToDifferentImplReverts() public {
        address scheduled = address(new SimplexPaymasterHarness());
        address other = address(new SimplexPaymasterHarness());
        vm.startPrank(owner);
        paymaster.scheduleUpgrade(scheduled);
        vm.warp(block.timestamp + paymaster.UPGRADE_DELAY());

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.UpgradeNotScheduled.selector, other));
        paymaster.upgradeToAndCall(other, "");
        vm.stopPrank();
    }

    function testUpgradeAfterDelayPreservesState() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.startPrank(owner);
        paymaster.setMarkup(200);
        paymaster.scheduleUpgrade(newImpl);
        vm.warp(block.timestamp + paymaster.UPGRADE_DELAY());
        paymaster.upgradeToAndCall(newImpl, "");
        vm.stopPrank();

        // Refresh the feeds so pricing assertions do not hit the staleness guard after the warp.
        nativeOracle.setAnswer(NATIVE_USD);
        usdcOracle.setAnswer(TOKEN_USD);

        assertEq(paymaster.owner(), owner);
        assertEq(paymaster.markupBps(), 200);
        assertEq(paymaster.getRegisteredTokens().length, 2);
        assertEq(paymaster.getTokenPrice(address(usdc6)), (6e8 * 10_200) / 10_000);
        assertEq(paymaster.pendingImplementation(), address(0));
    }

    function testCancelUpgrade() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.startPrank(owner);
        paymaster.scheduleUpgrade(newImpl);
        paymaster.cancelUpgrade();
        vm.warp(block.timestamp + paymaster.UPGRADE_DELAY());

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.UpgradeNotScheduled.selector, newImpl));
        paymaster.upgradeToAndCall(newImpl, "");
        vm.stopPrank();
    }

    // ── Admin ────────────────────────────────────────────────────────

    function testMarkupCapEnforced() public {
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        paymaster.setMarkup(5_001);

        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (AggregatorV3Interface(address(nativeOracle)), uint256(5_001), treasury, owner)
        );
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        new ERC1967Proxy(address(implementation), initData);
    }

    function testAdminFunctionsOnlyOwner() public {
        vm.expectRevert();
        paymaster.setMarkup(100);
        vm.expectRevert();
        paymaster.setTreasury(address(this));
        vm.expectRevert();
        paymaster.registerToken(address(usdc6), AggregatorV3Interface(address(usdcOracle)));
        vm.expectRevert();
        paymaster.withdrawTokenToTreasury(IERC20(address(usdc6)), 1);
    }

    function testTwoStepOwnershipTransfer() public {
        address newOwner = makeAddr("newOwner");
        vm.prank(owner);
        paymaster.transferOwnership(newOwner);
        assertEq(paymaster.owner(), owner);
        assertEq(paymaster.pendingOwner(), newOwner);

        vm.prank(newOwner);
        paymaster.acceptOwnership();
        assertEq(paymaster.owner(), newOwner);
    }

    function testWithdrawTokenToTreasury() public {
        deal(address(usdc6), address(paymaster), 1_000_000);
        vm.prank(owner);
        paymaster.withdrawTokenToTreasury(IERC20(address(usdc6)), 1_000_000);
        assertEq(usdc6.balanceOf(treasury), 1_000_000);
    }

    // ── Helpers ──────────────────────────────────────────────────────

    function _deployPaymaster(uint256 markupBps) internal returns (SimplexPaymasterHarness) {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (AggregatorV3Interface(address(nativeOracle)), markupBps, treasury, owner)
        );
        return SimplexPaymasterHarness(address(new ERC1967Proxy(address(implementation), initData)));
    }

    function _fundAndApprove(uint256 amount) internal {
        deal(address(usdc6), sender, amount);
        vm.prank(sender);
        usdc6.approve(address(paymaster), amount);
    }

    /// @dev paymasterAndData = paymaster(20) || verificationGasLimit(16) || postOpGasLimit(16) || data
    ///      gasFees = maxPriorityFeePerGas(16) || maxFeePerGas(16), both 1 gwei
    function _userOpWithPaymasterData(bytes memory data) internal view returns (PackedUserOperation memory op) {
        op.sender = sender;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | uint256(1 gwei));
        op.paymasterAndData = abi.encodePacked(address(paymaster), uint128(150_000), uint128(100_000), data);
    }
}
