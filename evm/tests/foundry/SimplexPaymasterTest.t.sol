// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../../src/utils/SimplexPaymaster.sol";

contract MockHost {
    bytes public hyperbridgeId;
    address public uniswapV2Router;

    constructor(bytes memory _hyperbridgeId) {
        hyperbridgeId = _hyperbridgeId;
    }

    function hyperbridge() external view returns (bytes memory) {
        return hyperbridgeId;
    }

    function setUniswapV2Router(address router) external {
        uniswapV2Router = router;
    }
}

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

/// @dev V2-style router paying out a preset amount of native for any token input.
contract MockV2Router {
    address private immutable _weth;
    uint256 public nextAmountOut;

    constructor(address weth_) {
        _weth = weth_;
    }

    function WETH() external view returns (address) {
        return _weth;
    }

    function setNextAmountOut(uint256 amountOut) external {
        nextAmountOut = amountOut;
    }

    function swapExactTokensForETH(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256
    ) external returns (uint256[] memory amounts) {
        require(path.length == 2 && path[1] == _weth, "INVALID_PATH");
        ERC20(path[0]).transferFrom(msg.sender, address(this), amountIn);

        uint256 amountOut = nextAmountOut;
        require(amountOut >= amountOutMin, "INSUFFICIENT_OUTPUT_AMOUNT");
        (bool sent, ) = to.call{value: amountOut}("");
        require(sent, "ETH_TRANSFER_FAILED");

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
    }

    receive() external payable {}
}

contract MockEntryPoint {
    mapping(address => uint256) public balanceOf;

    function depositTo(address account) external payable {
        balanceOf[account] += msg.value;
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

    bytes constant HYPERBRIDGE_ID = bytes("POLKADOT-3367");

    address treasury = makeAddr("treasury");
    address sender = address(0xBEEF);

    MockHost hyperbridgeHost;
    MockOracle nativeOracle;
    MockOracle usdcOracle;
    MockToken usdc6; // 6-decimal USDC (Base-style)
    MockToken usdc18; // 18-decimal USDC (BSC-style)
    MockV2Router router;
    MockEntryPoint entryPoint;
    SimplexPaymasterHarness paymaster;

    function setUp() public {
        vm.warp(1_700_000_000);

        hyperbridgeHost = new MockHost(HYPERBRIDGE_ID);
        nativeOracle = new MockOracle(NATIVE_USD, 8);
        usdcOracle = new MockOracle(TOKEN_USD, 8);
        usdc6 = new MockToken("USDC6", 6);
        usdc18 = new MockToken("USDC18", 18);

        router = new MockV2Router(address(0xE7E7));
        vm.deal(address(router), 100 ether);
        hyperbridgeHost.setUniswapV2Router(address(router));

        // The paymaster deposits to the canonical v0.8 EntryPoint address.
        vm.etch(address(ERC4337Utils.ENTRYPOINT_V08), address(new MockEntryPoint()).code);
        entryPoint = MockEntryPoint(address(ERC4337Utils.ENTRYPOINT_V08));

        paymaster = _deployPaymaster(0); // no markup for the base pricing assertions
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
        _setMarkup(200); // 2%
        assertEq(paymaster.getTokenPrice(address(usdc6)), (6e8 * 10_200) / 10_000);
    }

    function testTokenPriceNormalizesOracleDecimals() public {
        // An 18-decimal token/USD feed must price identically to an 8-decimal one.
        MockOracle oracle18 = new MockOracle(1e18, 18);
        MockToken token = new MockToken("T", 6);
        _govern(SimplexPaymaster.RequestKind.RegisterToken, abi.encode(address(token), address(oracle18)));

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

    function testUpdateParamsTightensOracleAge() public {
        nativeOracle.setUpdatedAt(block.timestamp - 100);
        _updateParams(address(nativeOracle), 0, treasury, 50);
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
        _govern(SimplexPaymaster.RequestKind.DeactivateToken, abi.encode(address(usdc6)));

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
        _govern(SimplexPaymaster.RequestKind.RegisterToken, abi.encode(address(usdc6), address(usdcOracle)));
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

    // ── Permit-mode validation guards ────────────────────────────────

    /// Permit mode (0x00) must reject an unregistered token before the permit
    /// call, so the validation-phase external call can never target an
    /// attacker-chosen address.
    function testPermitModeUnregisteredTokenRejectedBeforePermit() public {
        address rogue = makeAddr("rogue");
        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(rogue));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotRegistered.selector, rogue));
        paymaster.validate(op, 1e15);
    }

    /// Permit mode must also reject a deactivated token before the permit call.
    function testPermitModeDeactivatedTokenRejectedBeforePermit() public {
        _govern(SimplexPaymaster.RequestKind.DeactivateToken, abi.encode(address(usdc6)));

        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(address(usdc6)));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotActive.selector, address(usdc6)));
        paymaster.validate(op, 1e15);
    }

    /// A permit-mode header shorter than mode+token reverts as malformed.
    function testPermitModeShortDataReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(abi.encodePacked(uint8(0), uint8(0xAB)));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(2)));
        paymaster.validate(op, 1e15);
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
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        paymaster.initialize(address(hyperbridgeHost), params, tokens, oracles);
    }

    function testImplementationCannotBeInitialized() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        implementation.initialize(address(hyperbridgeHost), params, tokens, oracles);
    }

    function testInitializeRejectsNonContractHost() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (makeAddr("eoa"), params, tokens, oracles)
        );
        vm.expectRevert(SimplexPaymaster.InvalidHost.selector);
        new ERC1967Proxy(address(implementation), initData);
    }

    function testInitializeRejectsLengthMismatch() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, ) = _initArgs(0);
        AggregatorV3Interface[] memory oracles = new AggregatorV3Interface[](1);
        oracles[0] = AggregatorV3Interface(address(usdcOracle));
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (address(hyperbridgeHost), params, tokens, oracles)
        );
        vm.expectRevert(SimplexPaymaster.LengthMismatch.selector);
        new ERC1967Proxy(address(implementation), initData);
    }

    // ── Governance ───────────────────────────────────────────────────

    function testOnAcceptOnlyHost() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, abi.encode(newImpl, bytes(""))));
    }

    function testOnAcceptRejectsNonHyperbridgeSource() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.onAccept(_request(bytes("EVM-1"), SimplexPaymaster.RequestKind.UpgradeContract, abi.encode(newImpl, bytes(""))));
    }

    function testGovernanceUpgradePreservesState() public {
        address newImpl = address(new SimplexPaymasterHarness());
        _setMarkup(200);

        _govern(SimplexPaymaster.RequestKind.UpgradeContract, abi.encode(newImpl, bytes("")));

        bytes32 implSlot = vm.load(address(paymaster), ERC1967Utils.IMPLEMENTATION_SLOT);
        assertEq(address(uint160(uint256(implSlot))), newImpl);

        assertEq(paymaster.markupBps(), 200);
        assertEq(paymaster.getRegisteredTokens().length, 2);
        assertEq(paymaster.getTokenPrice(address(usdc6)), (6e8 * 10_200) / 10_000);
    }

    function testUpdateParamsValidation() public {
        vm.startPrank(address(hyperbridgeHost));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        paymaster.onAccept(
            _request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpdateParams, _paramsPayload(address(nativeOracle), 5_001, treasury, 86_400))
        );

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidOracleAge.selector, uint256(8 days)));
        paymaster.onAccept(
            _request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpdateParams, _paramsPayload(address(nativeOracle), 0, treasury, 8 days))
        );

        vm.expectRevert(SimplexPaymaster.ZeroAddress.selector);
        paymaster.onAccept(
            _request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpdateParams, _paramsPayload(address(nativeOracle), 0, address(0), 86_400))
        );
        vm.stopPrank();
    }

    function testUpdateParamsReplacesNativeOracle() public {
        MockOracle newNative = new MockOracle(1_200e18, 18);
        _updateParams(address(newNative), 0, treasury, 86_400);

        assertEq(paymaster.nativeOracleDecimals(), 18);
        // $1200 native normalized from the 18-decimal feed: 1200e8 * 1e6 / 1e8 token units per 1e18 wei.
        assertEq(paymaster.getTokenPrice(address(usdc6)), 12e8);
    }

    function testGovernanceWithdrawsSurplusToTreasury() public {
        deal(address(usdc6), address(paymaster), 1_000_000);
        _govern(SimplexPaymaster.RequestKind.WithdrawAssets, abi.encode(address(usdc6), uint256(1_000_000)));
        assertEq(usdc6.balanceOf(treasury), 1_000_000);
    }

    // ── postOp gas limit cap ─────────────────────────────────────────

    function testPostOpGasLimitAboveCapReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(
            abi.encodePacked(uint8(1), address(usdc6)),
            uint128(100_001)
        );
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.InvalidPostOpGasLimit.selector, uint256(100_001), uint256(100_000))
        );
        paymaster.validate(op, 1e15);
    }

    function testPostOpGasLimitAtCapAccepted() public {
        _fundAndApprove(1_000e6);
        PackedUserOperation memory op = _userOpWithPaymasterData(
            abi.encodePacked(uint8(1), address(usdc6)),
            uint128(100_000)
        );
        (, uint256 validationData) = paymaster.validate(op, 1e15);
        assertEq(validationData, 0);
    }

    // ── Fee recycling ────────────────────────────────────────────────

    function testSwapAndDepositFullBalance() public {
        // $600 of the $1 token at $600 native = 1 native; 200 bps slippage → min 0.98.
        deal(address(usdc6), address(paymaster), 600e6);
        router.setNextAmountOut(1 ether);

        vm.expectEmit(true, false, false, true, address(paymaster));
        emit SimplexPaymaster.FeesRecycled(address(usdc6), 600e6, 1 ether, 1 ether);
        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 0);

        assertEq(usdc6.balanceOf(address(paymaster)), 0);
        assertEq(usdc6.balanceOf(address(router)), 600e6);
        assertEq(entryPoint.balanceOf(address(paymaster)), 1 ether);
    }

    function testSwapAndDepositPartialAndClamped() public {
        deal(address(usdc6), address(paymaster), 600e6);
        router.setNextAmountOut(0.5 ether);

        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 300e6);
        assertEq(usdc6.balanceOf(address(paymaster)), 300e6);

        // More than the remaining balance clamps to the balance.
        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 1_000e6);
        assertEq(usdc6.balanceOf(address(paymaster)), 0);
        assertEq(entryPoint.balanceOf(address(paymaster)), 1 ether);
    }

    function testSwapAndDepositSweepsStrayNative() public {
        deal(address(usdc6), address(paymaster), 600e6);
        vm.deal(address(paymaster), 0.5 ether);
        router.setNextAmountOut(1 ether);

        vm.expectEmit(true, false, false, true, address(paymaster));
        emit SimplexPaymaster.FeesRecycled(address(usdc6), 600e6, 1 ether, 1.5 ether);
        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 0);

        assertEq(entryPoint.balanceOf(address(paymaster)), 1.5 ether);
    }

    function testSwapAndDepositRejectsNonTreasury() public {
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.swapAndDeposit(address(usdc6), 0);
    }

    function testSwapAndDepositRouterUnsetReverts() public {
        hyperbridgeHost.setUniswapV2Router(address(0));

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidRouter.selector, address(0)));
        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 0);
    }

    function testSwapAndDepositUnregisteredTokenReverts() public {
        address unknown = makeAddr("unknown");
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotRegistered.selector, unknown));
        vm.prank(treasury);
        paymaster.swapAndDeposit(unknown, 0);
    }

    function testSwapAndDepositAllowsDeactivatedToken() public {
        _govern(SimplexPaymaster.RequestKind.DeactivateToken, abi.encode(address(usdc6)));
        deal(address(usdc6), address(paymaster), 600e6);
        router.setNextAmountOut(1 ether);

        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 0);
        assertEq(entryPoint.balanceOf(address(paymaster)), 1 ether);
    }

    function testSwapAndDepositEnforcesOracleSlippageBound() public {
        deal(address(usdc6), address(paymaster), 600e6);

        // $300 in expects 0.5 native; 200 bps tolerance → 0.49 minimum, which passes...
        router.setNextAmountOut(0.49 ether);
        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 300e6);

        // ...one wei below it reverts.
        router.setNextAmountOut(0.49 ether - 1);
        vm.expectRevert("INSUFFICIENT_OUTPUT_AMOUNT");
        vm.prank(treasury);
        paymaster.swapAndDeposit(address(usdc6), 300e6);
    }

    function testSwapParamsValidation() public {
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidSlippage.selector, uint256(1_001)));
        paymaster.onAccept(
            _request(
                HYPERBRIDGE_ID,
                SimplexPaymaster.RequestKind.UpdateParams,
                _paramsPayloadFull(address(nativeOracle), 0, treasury, 86_400, 1_001)
            )
        );
    }

    function testInheritedWithdrawEntryPointsDisabled() public {
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.withdraw(payable(treasury), 0);

        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.withdrawTokens(IERC20(address(usdc6)), treasury, 1);

        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.withdrawStake(payable(treasury));
    }

    function testMarkupCapEnforced() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(5_001);
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (address(hyperbridgeHost), params, tokens, oracles)
        );
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        new ERC1967Proxy(address(implementation), initData);
    }

    // ── Helpers ──────────────────────────────────────────────────────

    function _initArgs(
        uint256 markupBps
    )
        internal
        view
        returns (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles)
    {
        params = SimplexPaymaster.Params({
            nativeOracle: AggregatorV3Interface(address(nativeOracle)),
            markupBps: markupBps,
            treasury: treasury,
            maxOracleAge: 86_400,
            swapSlippageBps: 200
        });
        tokens = new address[](2);
        tokens[0] = address(usdc6);
        tokens[1] = address(usdc18);
        oracles = new AggregatorV3Interface[](2);
        oracles[0] = AggregatorV3Interface(address(usdcOracle));
        oracles[1] = AggregatorV3Interface(address(usdcOracle));
    }

    function _deployPaymaster(uint256 markupBps) internal returns (SimplexPaymasterHarness) {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(markupBps);
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (address(hyperbridgeHost), params, tokens, oracles)
        );
        return SimplexPaymasterHarness(payable(address(new ERC1967Proxy(address(implementation), initData))));
    }

    function _request(
        bytes memory source,
        SimplexPaymaster.RequestKind kind,
        bytes memory payload
    ) internal pure returns (IncomingPostRequest memory req) {
        req.request.source = source;
        req.request.body = bytes.concat(bytes1(uint8(kind)), payload);
    }

    /// @dev Delivers a governance request as the host with Hyperbridge as the source.
    function _govern(SimplexPaymaster.RequestKind kind, bytes memory payload) internal {
        vm.prank(address(hyperbridgeHost));
        paymaster.onAccept(_request(HYPERBRIDGE_ID, kind, payload));
    }

    function _paramsPayload(
        address oracle,
        uint256 markupBps,
        address treasury_,
        uint256 maxOracleAge
    ) internal pure returns (bytes memory) {
        return _paramsPayloadFull(oracle, markupBps, treasury_, maxOracleAge, 200);
    }

    function _paramsPayloadFull(
        address oracle,
        uint256 markupBps,
        address treasury_,
        uint256 maxOracleAge,
        uint256 swapSlippageBps
    ) internal pure returns (bytes memory) {
        return abi.encode(
            SimplexPaymaster.Params({
                nativeOracle: AggregatorV3Interface(oracle),
                markupBps: markupBps,
                treasury: treasury_,
                maxOracleAge: maxOracleAge,
                swapSlippageBps: swapSlippageBps
            })
        );
    }

    function _updateParams(address oracle, uint256 markupBps, address treasury_, uint256 maxOracleAge) internal {
        _govern(SimplexPaymaster.RequestKind.UpdateParams, _paramsPayload(oracle, markupBps, treasury_, maxOracleAge));
    }

    function _setMarkup(uint256 markupBps) internal {
        _updateParams(address(nativeOracle), markupBps, treasury, 86_400);
    }

    function _fundAndApprove(uint256 amount) internal {
        deal(address(usdc6), sender, amount);
        vm.prank(sender);
        usdc6.approve(address(paymaster), amount);
    }

    /// @dev A well-formed 150-byte permit-mode payload (mode 0x00) for `token`.
    ///      Signature fields are zero — the registration guard fires before the
    ///      permit is ever executed, so no valid signature is needed.
    function _permitData(address token) internal pure returns (bytes memory) {
        return abi.encodePacked(
            uint8(0),
            token,
            uint256(0), // permitAmount
            uint256(0), // deadline
            uint8(0), // v
            bytes32(0), // r
            bytes32(0) // s
        );
    }

    /// @dev paymasterAndData = paymaster(20) || verificationGasLimit(16) || postOpGasLimit(16) || data
    ///      gasFees = maxPriorityFeePerGas(16) || maxFeePerGas(16), both 1 gwei
    function _userOpWithPaymasterData(bytes memory data) internal view returns (PackedUserOperation memory op) {
        return _userOpWithPaymasterData(data, 100_000);
    }

    function _userOpWithPaymasterData(
        bytes memory data,
        uint128 postOpGasLimit
    ) internal view returns (PackedUserOperation memory op) {
        op.sender = sender;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | uint256(1 gwei));
        op.paymasterAndData = abi.encodePacked(address(paymaster), uint128(150_000), postOpGasLimit, data);
    }
}
