// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

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

/// @dev Exposes _fetchDetails for direct testing of paymasterData parsing.
contract SimplexPaymasterHarness is SimplexPaymaster {
    constructor(
        AggregatorV3Interface _nativeOracle,
        uint256 _markupBps,
        address _treasury,
        address _owner
    ) SimplexPaymaster(_nativeOracle, _markupBps, _treasury, _owner) {}

    function fetchDetails(
        PackedUserOperation calldata userOp
    ) external view returns (uint256 validationData, IERC20 token, uint256 tokenPrice) {
        return _fetchDetails(userOp, bytes32(0));
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

        paymaster = new SimplexPaymasterHarness(
            AggregatorV3Interface(address(nativeOracle)),
            0, // no markup for the base pricing assertions
            treasury,
            owner
        );

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

    // ── Admin ────────────────────────────────────────────────────────

    function testMarkupCapEnforced() public {
        vm.prank(owner);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        paymaster.setMarkup(5_001);

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        new SimplexPaymasterHarness(AggregatorV3Interface(address(nativeOracle)), 5_001, treasury, owner);
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

    function testWithdrawTokenToTreasury() public {
        deal(address(usdc6), address(paymaster), 1_000_000);
        vm.prank(owner);
        paymaster.withdrawTokenToTreasury(IERC20(address(usdc6)), 1_000_000);
        assertEq(usdc6.balanceOf(treasury), 1_000_000);
    }

    // ── Helpers ──────────────────────────────────────────────────────

    /// @dev paymasterAndData = paymaster(20) || verificationGasLimit(16) || postOpGasLimit(16) || data
    function _userOpWithPaymasterData(bytes memory data) internal view returns (PackedUserOperation memory op) {
        op.sender = address(0xBEEF);
        op.paymasterAndData = abi.encodePacked(address(paymaster), uint128(150_000), uint128(100_000), data);
    }
}
