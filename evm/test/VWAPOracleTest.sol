// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {VWAPOracle} from "../src/utils/VWAPOracle.sol";
import {IIntentPriceOracle} from "@hyperbridge/core/apps/IntentPriceOracle.sol";
import {TokenInfo} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

// Mock ERC20 with configurable decimals
contract MockERC20 is ERC20 {
    uint8 private _decimals;

    constructor(string memory name, string memory symbol, uint8 decimals_) ERC20(name, symbol) {
        _decimals = decimals_;
    }

    function decimals() public view override returns (uint8) {
        return _decimals;
    }
}

contract VWAPOracleTest is Test {
    VWAPOracle public oracle;
    address public admin;
    address public host;
    address public user;
    MockERC20 public usdc;
    MockERC20 public dai;
    MockERC20 public wbtc;
    bytes public sourceChain = abi.encodePacked("ethereum");
    bytes public hyperChain;

    event SpreadRecorded(bytes32 indexed commitment, address indexed destinationToken, int256 spreadBps);
    event TokenDecimalsUpdated(bytes sourceChain, address indexed token, uint8 decimals);

    function setUp() public {
        admin = makeAddr("admin");
        host = makeAddr("host");
        user = makeAddr("user");
        oracle = new VWAPOracle(admin);
        hyperChain = StateMachine.kusama(2000);
        usdc = new MockERC20("USDC", "USDC", 6);
        dai = new MockERC20("DAI", "DAI", 18);
        wbtc = new MockERC20("WBTC", "WBTC", 8);
        vm.chainId(137);
    }

    // ==================== Initialization Tests ====================

    function testInitialization() public view {
        assertEq(oracle.host(), address(0), "Host should be uninitialized");
    }

    function testInitUnauthorized() public {
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](0);
        vm.prank(user);
        vm.expectRevert(VWAPOracle.Unauthorized.selector);
        oracle.init(host, updates);
    }

    function testInitOnlyOnce() public {
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](0);
        vm.startPrank(admin);
        oracle.init(host, updates);
        vm.expectRevert(VWAPOracle.Unauthorized.selector);
        oracle.init(host, updates);
        vm.stopPrank();
    }

    function testInitInvalidDecimals() public {
        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(usdc), decimals: 0});
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});
        vm.prank(admin);
        vm.expectRevert(VWAPOracle.InvalidInput.selector);
        oracle.init(host, updates);
    }

    function testInitWithHighDecimals() public {
        // Test that decimals > 18 are supported (e.g., some tokens may have 24+ decimals)
        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(usdc), decimals: 24});
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});
        vm.prank(admin);
        oracle.init(host, updates);

        assertEq(oracle.decimals(sourceChain, address(usdc)), 24, "Should support high decimals");
    }

    function testHighDecimalsNormalization() public {
        // Test token with 24 decimals (some tokens like YAM have high decimals)
        MockERC20 highDecToken = new MockERC20("HighDec", "HDT", 24);

        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(highDecToken), decimals: 24});
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        vm.prank(admin);
        oracle.init(host, updates);

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);

        // Input: 1000 tokens with 24 decimals = 1000 * 1e24
        // Output: 1000 tokens with 24 decimals = 1000 * 1e24
        // Both should normalize to 1000 * 1e18
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(highDecToken)))), amount: 1000 * 1e24});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(highDecToken)))), amount: 1000 * 1e24});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(highDecToken));
        // Should be normalized to 18 decimals: 1000 * 1e24 / 1e6 = 1000 * 1e18
        assertEq(data.totalVolume, 1000 * 1e18, "High decimal token should normalize to 18 decimals");
        assertEq(data.weightedSpreadSum, 0, "Equal amounts should have 0 spread");
    }

    // ==================== Spread Recording Tests ====================

    function testRecordSpread() public {
        // Configure source chain decimals only
        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(dai), decimals: 18});
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        vm.prank(admin);
        oracle.init(host, updates);

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 995 * 1e18});

        vm.expectEmit(true, true, false, true);
        emit SpreadRecorded(commitment, address(dai), -50);
        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));
        assertEq(data.fillCount, 1);
        assertEq(data.totalVolume, 1000 * 1e18);
    }

    function testNativeToken() public {
        // Configure native token on source
        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(0), decimals: 18});
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        vm.prank(admin);
        oracle.init(host, updates);

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(0), amount: 1 ether});
        outputs[0] = TokenInfo({token: bytes32(0), amount: 0.995 ether});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(0));
        assertEq(data.fillCount, 1);
    }

    function testDifferentDecimals() public {
        // USDC on source has 6 decimals
        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(usdc), decimals: 6});
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        vm.prank(admin);
        oracle.init(host, updates);

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        // Input: 1000 USDC with 6 decimals on source
        // Output: 1000 USDC with 6 decimals on dest (read from contract)
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(usdc));
        // Both normalized to 18 decimals = 1000e18
        assertEq(data.totalVolume, 1000 * 1e18);
        assertEq(data.weightedSpreadSum, 0); // No spread
    }

    function testRecordSpread_SkipsUnconfiguredSourceDecimals() public {
        vm.prank(admin);
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](0);
        oracle.init(host, updates);

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 995 * 1e18});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));
        assertEq(data.fillCount, 0, "Should skip tokens without configured decimals");
    }

    function testMultipleFills() public {
        _initOracle();

        bytes32 commitment1 = keccak256("order1");
        bytes32 commitment2 = keccak256("order2");

        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 995 * 1e18});

        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2010 * 1e18});

        oracle.recordSpread(commitment1, sourceChain, inputs1, outputs1);
        oracle.recordSpread(commitment2, sourceChain, inputs2, outputs2);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));
        assertEq(data.fillCount, 2);
        assertEq(data.totalVolume, 3000 * 1e18);
    }

    function testMultipleTokens() public {
        _initOracle();

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](2);
        TokenInfo[] memory outputs = new TokenInfo[](2);

        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 995 * 1e18});

        inputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 2000 * 1e6});
        outputs[1] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 2010 * 1e6});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory dataDAI = oracle.spread(sourceChain, address(dai));
        IIntentPriceOracle.CumulativeSpreadData memory dataUSDC = oracle.spread(sourceChain, address(usdc));

        assertEq(dataDAI.fillCount, 1);
        assertEq(dataUSDC.fillCount, 1);
    }

    // ==================== VWAP Calculation Tests ====================

    function testVWAPCalculation() public {
        _initOracle();

        bytes32 commitment1 = keccak256("order1");
        bytes32 commitment2 = keccak256("order2");
        bytes32 commitment3 = keccak256("order3");

        // Fill 1: 1000 tokens, -50 bps spread
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 995 * 1e18});
        oracle.recordSpread(commitment1, sourceChain, inputs1, outputs1);

        // Fill 2: 2000 tokens, +100 bps spread
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2020 * 1e18});
        oracle.recordSpread(commitment2, sourceChain, inputs2, outputs2);

        // Fill 3: 3000 tokens, -30 bps spread
        TokenInfo[] memory inputs3 = new TokenInfo[](1);
        TokenInfo[] memory outputs3 = new TokenInfo[](1);
        inputs3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 3000 * 1e18});
        outputs3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2991 * 1e18});
        oracle.recordSpread(commitment3, sourceChain, inputs3, outputs3);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));

        assertEq(data.fillCount, 3);
        assertEq(data.totalVolume, 6000 * 1e18);
        // VWAP: (-50*1000 + 100*2000 - 30*3000) / 6000 = 10 bps
        int256 vwap = data.weightedSpreadSum / int256(data.totalVolume);
        assertEq(vwap, 10);
    }

    function testVWAPWithAllPositiveSpreads() public {
        _initOracle();

        // All fills have positive spreads (fillers giving more)
        // Fill 1: 1000 tokens, +20 bps
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1002 * 1e18});
        oracle.recordSpread(keccak256("order1"), sourceChain, inputs1, outputs1);

        // Fill 2: 2000 tokens, +50 bps
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2010 * 1e18});
        oracle.recordSpread(keccak256("order2"), sourceChain, inputs2, outputs2);

        // Fill 3: 1000 tokens, +30 bps
        TokenInfo[] memory inputs3 = new TokenInfo[](1);
        TokenInfo[] memory outputs3 = new TokenInfo[](1);
        inputs3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1003 * 1e18});
        oracle.recordSpread(keccak256("order3"), sourceChain, inputs3, outputs3);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));

        // VWAP: (20*1000 + 50*2000 + 30*1000) / 4000 = 150000 / 4000 = 37.5 bps
        int256 vwap = data.weightedSpreadSum / int256(data.totalVolume);
        assertEq(vwap, 37, "VWAP should be ~37 bps (rounded)");
        assertEq(data.fillCount, 3);
        assertEq(data.totalVolume, 4000 * 1e18);
    }

    function testVWAPWithAllNegativeSpreads() public {
        _initOracle();

        // All fills have negative spreads (fillers capturing value)
        // Fill 1: 1000 tokens, -100 bps
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 990 * 1e18});
        oracle.recordSpread(keccak256("order1"), sourceChain, inputs1, outputs1);

        // Fill 2: 3000 tokens, -50 bps
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 3000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2985 * 1e18});
        oracle.recordSpread(keccak256("order2"), sourceChain, inputs2, outputs2);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));

        // VWAP: (-100*1000 + -50*3000) / 4000 = -250000 / 4000 = -62.5 bps
        int256 vwap = data.weightedSpreadSum / int256(data.totalVolume);
        assertEq(vwap, -62, "VWAP should be ~-62 bps (rounded)");
        assertTrue(vwap < 0, "VWAP should be negative");
    }

    function testVWAPWithZeroSpreads() public {
        _initOracle();

        // Mix of zero and non-zero spreads
        // Fill 1: 1000 tokens, 0 bps (exact match)
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        oracle.recordSpread(keccak256("order1"), sourceChain, inputs1, outputs1);

        // Fill 2: 2000 tokens, +100 bps
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2020 * 1e18});
        oracle.recordSpread(keccak256("order2"), sourceChain, inputs2, outputs2);

        // Fill 3: 1000 tokens, 0 bps
        TokenInfo[] memory inputs3 = new TokenInfo[](1);
        TokenInfo[] memory outputs3 = new TokenInfo[](1);
        inputs3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs3[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        oracle.recordSpread(keccak256("order3"), sourceChain, inputs3, outputs3);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));

        // VWAP: (0*1000 + 100*2000 + 0*1000) / 4000 = 200000 / 4000 = 50 bps
        int256 vwap = data.weightedSpreadSum / int256(data.totalVolume);
        assertEq(vwap, 50, "VWAP should be 50 bps");
        assertEq(data.fillCount, 3);
    }

    function testVWAPWithDifferentDecimals() public {
        _initOracle();

        // Mix USDC (6 decimals) and DAI (18 decimals) to verify normalization in VWAP
        // Fill 1: 1000 USDC, -50 bps
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 995 * 1e6});
        oracle.recordSpread(keccak256("order1"), sourceChain, inputs1, outputs1);

        // Fill 2: 1000 DAI, +50 bps
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1005 * 1e18});
        oracle.recordSpread(keccak256("order2"), sourceChain, inputs2, outputs2);

        IIntentPriceOracle.CumulativeSpreadData memory dataUSDC = oracle.spread(sourceChain, address(usdc));
        IIntentPriceOracle.CumulativeSpreadData memory dataDAI = oracle.spread(sourceChain, address(dai));

        // Both should be normalized to 1000e18 volume
        assertEq(dataUSDC.totalVolume, 1000 * 1e18, "USDC volume should normalize to 18 decimals");
        assertEq(dataDAI.totalVolume, 1000 * 1e18, "DAI volume should be 18 decimals");

        // Verify spreads are tracked independently
        int256 vwapUSDC = dataUSDC.weightedSpreadSum / int256(dataUSDC.totalVolume);
        int256 vwapDAI = dataDAI.weightedSpreadSum / int256(dataDAI.totalVolume);
        assertEq(vwapUSDC, -50, "USDC VWAP should be -50 bps");
        assertEq(vwapDAI, 50, "DAI VWAP should be +50 bps");
    }

    function testVWAPVolumeWeighting() public {
        _initOracle();

        // Demonstrate importance of volume weighting
        // Small fill with large spread vs large fill with small spread

        // Fill 1: 100 tokens, +500 bps (5% positive)
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 100 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 105 * 1e18});
        oracle.recordSpread(keccak256("order1"), sourceChain, inputs1, outputs1);

        // Fill 2: 9900 tokens, -10 bps (0.1% negative)
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 9900 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 9890.1 * 1e18});
        oracle.recordSpread(keccak256("order2"), sourceChain, inputs2, outputs2);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));

        // VWAP should be dominated by the large fill
        // VWAP: (500*100 + -10*9900) / 10000 = (50000 - 99000) / 10000 = -4.9 bps
        int256 vwap = data.weightedSpreadSum / int256(data.totalVolume);

        // Despite small fill having +500 bps, VWAP is negative due to large volume at -10 bps
        assertTrue(vwap < 0, "VWAP should be negative despite one large positive spread");
        assertEq(data.totalVolume, 10000 * 1e18);
    }

    function testVWAPWithExtremeVolumeDifferences() public {
        _initOracle();

        // One tiny fill and one massive fill
        // Fill 1: 1 token, +1000 bps (10%)
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 11 * 1e17}); // 1.1 tokens
        oracle.recordSpread(keccak256("order1"), sourceChain, inputs1, outputs1);

        // Fill 2: 1 million tokens, -10 bps
        TokenInfo[] memory inputs2 = new TokenInfo[](1);
        TokenInfo[] memory outputs2 = new TokenInfo[](1);
        inputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1_000_000 * 1e18});
        outputs2[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 999_000 * 1e18});
        oracle.recordSpread(keccak256("order2"), sourceChain, inputs2, outputs2);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));

        int256 vwap = data.weightedSpreadSum / int256(data.totalVolume);

        // VWAP: (1000*1 + -10*1000000) / 1000001 = (1000 - 10000000) / 1000001 ≈ -9.99 bps
        // Tiny fill's huge spread (+1000 bps) is negligible compared to large volume at -10 bps
        assertTrue(vwap < 0, "VWAP should be dominated by large volume fill");
        assertEq(vwap, -9, "VWAP should be approximately -10 bps");
        assertEq(data.fillCount, 2);
    }

    function testVWAPSingleLargeFillVsManySmallFills() public {
        _initOracle();

        // Compare: one 10000 token fill vs ten 1000 token fills, all same spread
        // Part 1: Single large fill
        TokenInfo[] memory inputs1 = new TokenInfo[](1);
        TokenInfo[] memory outputs1 = new TokenInfo[](1);
        inputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 10000 * 1e18});
        outputs1[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 9950 * 1e18});
        oracle.recordSpread(keccak256("large"), sourceChain, inputs1, outputs1);

        IIntentPriceOracle.CumulativeSpreadData memory dataSingle = oracle.spread(sourceChain, address(dai));
        int256 vwapSingle = dataSingle.weightedSpreadSum / int256(dataSingle.totalVolume);

        // Part 2: Many small fills (use USDC to track separately)
        for (uint256 i = 0; i < 10; i++) {
            TokenInfo[] memory inputs = new TokenInfo[](1);
            TokenInfo[] memory outputs = new TokenInfo[](1);
            inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 1000 * 1e6});
            outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(usdc)))), amount: 995 * 1e6});
            oracle.recordSpread(keccak256(abi.encodePacked("small", i)), sourceChain, inputs, outputs);
        }

        IIntentPriceOracle.CumulativeSpreadData memory dataMany = oracle.spread(sourceChain, address(usdc));
        int256 vwapMany = dataMany.weightedSpreadSum / int256(dataMany.totalVolume);

        // Both should have same VWAP since spread and total volume are identical
        assertEq(vwapSingle, vwapMany, "VWAP should be identical regardless of fill count");
        assertEq(vwapSingle, -50, "Both should have -50 bps VWAP");
        assertEq(dataSingle.fillCount, 1);
        assertEq(dataMany.fillCount, 10);
    }

    // ==================== Governance Tests ====================

    function testGovernanceUpdateSourceTokenDecimals() public {
        _initOracle();

        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(wbtc), decimals: 8});

        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        bytes memory body = bytes.concat(hex"00", abi.encode(updates));

        IncomingPostRequest memory incoming = IncomingPostRequest({
            request: PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(0)),
                dest: new bytes(0),
                body: body,
                nonce: 0,
                source: hyperChain,
                timeoutTimestamp: 0
            }),
            relayer: address(0)
        });

        vm.mockCall(host, abi.encodeWithSignature("hyperbridge()"), abi.encode(hyperChain));

        vm.expectEmit(true, true, false, true);
        emit TokenDecimalsUpdated(sourceChain, address(wbtc), 8);

        vm.prank(host);
        oracle.onAccept(incoming);

        assertEq(oracle.decimals(sourceChain, address(wbtc)), 8);
    }

    function testGovernanceUnauthorizedSource() public {
        _initOracle();

        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](1);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(wbtc), decimals: 8});

        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        bytes memory body = bytes.concat(hex"00", abi.encode(updates));
        bytes memory wrongSource = abi.encodePacked("wrong-chain");

        IncomingPostRequest memory incoming = IncomingPostRequest({
            request: PostRequest({
                to: abi.encodePacked(address(0)),
                from: abi.encodePacked(address(0)),
                dest: new bytes(0),
                body: body,
                nonce: 0,
                source: wrongSource,
                timeoutTimestamp: 0
            }),
            relayer: address(0)
        });

        vm.mockCall(host, abi.encodeWithSignature("hyperbridge()"), abi.encode(hyperChain));

        vm.prank(host);
        vm.expectRevert(VWAPOracle.Unauthorized.selector);
        oracle.onAccept(incoming);
    }

    // ==================== Edge Case Tests ====================

    function testMaxSpread() public {
        _initOracle();

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);

        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 2000 * 1e18});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));
        assertEq(data.weightedSpreadSum, 10000 * int256(1000 * 1e18));
    }

    function testMinSpread() public {
        _initOracle();

        bytes32 commitment = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);

        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 0});

        oracle.recordSpread(commitment, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data = oracle.spread(sourceChain, address(dai));
        assertEq(data.weightedSpreadSum, -10000 * int256(1000 * 1e18));
    }

    function testTimestampUpdates() public {
        _initOracle();

        bytes32 commitment1 = keccak256("order1");
        TokenInfo[] memory inputs = new TokenInfo[](1);
        TokenInfo[] memory outputs = new TokenInfo[](1);
        inputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 1000 * 1e18});
        outputs[0] = TokenInfo({token: bytes32(uint256(uint160(address(dai)))), amount: 995 * 1e18});

        uint256 timestamp1 = block.timestamp;
        oracle.recordSpread(commitment1, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data1 = oracle.spread(sourceChain, address(dai));
        assertEq(data1.lastUpdate, timestamp1);

        vm.warp(block.timestamp + 1000);

        bytes32 commitment2 = keccak256("order2");
        oracle.recordSpread(commitment2, sourceChain, inputs, outputs);

        IIntentPriceOracle.CumulativeSpreadData memory data2 = oracle.spread(sourceChain, address(dai));
        assertEq(data2.lastUpdate, timestamp1 + 1000);
        assertGt(data2.lastUpdate, data1.lastUpdate);
    }

    // ==================== Helper Functions ====================

    function _initOracle() internal {
        VWAPOracle.TokenDecimal[] memory tokens = new VWAPOracle.TokenDecimal[](2);
        tokens[0] = VWAPOracle.TokenDecimal({token: address(dai), decimals: 18});
        tokens[1] = VWAPOracle.TokenDecimal({token: address(usdc), decimals: 6});

        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](1);
        updates[0] = VWAPOracle.TokenDecimalsUpdate({sourceChain: sourceChain, tokens: tokens});

        vm.prank(admin);
        oracle.init(host, updates);
    }
}
