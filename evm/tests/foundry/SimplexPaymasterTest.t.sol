// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {Vm} from "forge-std/Vm.sol";
import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IPaymaster} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {PaymasterCore} from "@openzeppelin/community-contracts/contracts/account/paymaster/PaymasterCore.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";

import {SimplexPaymaster, AggregatorV3Interface, IStakeManager} from "../../src/utils/SimplexPaymaster.sol";

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

contract MockToken is ERC20, ERC20Permit {
    uint8 private immutable _decimals;

    constructor(string memory name, uint8 decimals_) ERC20(name, name) ERC20Permit(name) {
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

    function swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] calldata path, address to, uint256)
        external
        returns (uint256[] memory amounts)
    {
        require(path.length == 2 && path[1] == _weth, "INVALID_PATH");
        ERC20(path[0]).transferFrom(msg.sender, address(this), amountIn);

        uint256 amountOut = nextAmountOut;
        require(amountOut >= amountOutMin, "INSUFFICIENT_OUTPUT_AMOUNT");
        (bool sent,) = to.call{value: amountOut}("");
        require(sent, "ETH_TRANSFER_FAILED");

        amounts = new uint256[](2);
        amounts[0] = amountIn;
        amounts[1] = amountOut;
    }

    receive() external payable {}
}

/// @dev The EntryPoint's StakeManager, with v0.8's revert strings.
contract MockEntryPoint {
    mapping(address => IStakeManager.DepositInfo) private _deposits;

    function getDepositInfo(address account) external view returns (IStakeManager.DepositInfo memory) {
        return _deposits[account];
    }

    function balanceOf(address account) external view returns (uint256) {
        return _deposits[account].deposit;
    }

    function stakeOf(address account) external view returns (uint256) {
        return _deposits[account].stake;
    }

    function depositTo(address account) external payable {
        _deposits[account].deposit += msg.value;
    }

    function withdrawTo(address payable to, uint256 amount) external {
        IStakeManager.DepositInfo storage info = _deposits[msg.sender];
        require(amount <= info.deposit, "Withdraw amount too large");
        info.deposit -= amount;
        (bool ok,) = to.call{value: amount}("");
        require(ok, "failed to withdraw");
    }

    function addStake(uint32 unstakeDelaySec) external payable {
        IStakeManager.DepositInfo storage info = _deposits[msg.sender];
        require(unstakeDelaySec > 0, "must specify unstake delay");
        require(unstakeDelaySec >= info.unstakeDelaySec, "cannot decrease unstake time");
        uint256 stake = info.stake + msg.value;
        require(stake > 0, "no stake specified");
        info.staked = true;
        info.stake = uint112(stake);
        info.unstakeDelaySec = unstakeDelaySec;
        info.withdrawTime = 0;
    }

    function unlockStake() external {
        IStakeManager.DepositInfo storage info = _deposits[msg.sender];
        require(info.unstakeDelaySec != 0, "not staked");
        require(info.staked, "already unstaking");
        info.withdrawTime = uint48(block.timestamp) + info.unstakeDelaySec;
        info.staked = false;
    }

    function withdrawStake(address payable to) external {
        IStakeManager.DepositInfo storage info = _deposits[msg.sender];
        uint256 stake = info.stake;
        require(stake > 0, "No stake to withdraw");
        require(info.withdrawTime > 0, "must call unlockStake() first");
        require(info.withdrawTime <= block.timestamp, "Stake withdrawal is not due");
        info.unstakeDelaySec = 0;
        info.withdrawTime = 0;
        info.stake = 0;
        (bool ok,) = to.call{value: stake}("");
        require(ok, "failed to withdraw stake");
    }
}

/// @dev Exposes internal hooks for direct testing of paymasterData parsing and prefunding.
contract SimplexPaymasterHarness is SimplexPaymaster {
    function fetchDetails(PackedUserOperation calldata userOp)
        external
        view
        returns (uint256 validationData, IERC20 token, uint256 tokenPrice)
    {
        return _fetchDetails(userOp, bytes32(0));
    }

    function validate(PackedUserOperation calldata userOp, uint256 maxCost)
        external
        returns (bytes memory context, uint256 validationData)
    {
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
    bytes32 constant PERMIT_TYPEHASH =
        keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)");
    bytes32 constant INITIALIZABLE_SLOT = 0xf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00;
    // The origin rundler simulates validation from; governance lists it like any bundler wallet.
    address constant RUNDLER_SIMULATION_ORIGIN = 0x0643866dA50efE0b055Cd15aF95191968c8411b5;
    uint256 constant BUNDLERS_SLOT = 9;
    address constant ENTRYPOINT_V09 = 0x433709009B8330FDa32311DF1C2AFA402eD8D009;
    bytes8 constant PAYMASTER_SIG_MAGIC = 0x22e325a297439656;
    uint32 constant UNSTAKE_DELAY = 1 days;

    event RelayerUpdated(address previous, address current);
    event PermitExecuted(address indexed token, address indexed owner, uint256 amount);

    struct StorageEntry {
        uint256 astId;
        string contract_;
        string label;
        uint256 offset;
        string slot;
        string type_;
    }

    address treasury = makeAddr("treasury");
    address relayerA = makeAddr("relayerA");
    address relayerB = makeAddr("relayerB");
    address bundlerA = makeAddr("bundlerA");
    address bundlerB = makeAddr("bundlerB");
    address sender;
    uint256 senderKey;

    MockHost hyperbridgeHost;
    MockOracle nativeOracle;
    MockOracle usdcOracle;
    MockToken usdc6; // 6-decimal USDC (Base-style)
    MockToken usdc18; // 18-decimal USDC (BSC-style)
    MockV2Router router;
    MockEntryPoint entryPoint;
    MockEntryPoint entryPointV08;
    SimplexPaymasterHarness paymaster;

    function setUp() public {
        vm.warp(1_700_000_000);
        (sender, senderKey) = makeAddrAndKey("sender");

        hyperbridgeHost = new MockHost(HYPERBRIDGE_ID);
        nativeOracle = new MockOracle(NATIVE_USD, 8);
        usdcOracle = new MockOracle(TOKEN_USD, 8);
        usdc6 = new MockToken("USDC6", 6);
        usdc18 = new MockToken("USDC18", 18);

        router = new MockV2Router(address(0xE7E7));
        vm.deal(address(router), 100 ether);
        hyperbridgeHost.setUniswapV2Router(address(router));

        // The paymaster serves the canonical v0.9 EntryPoint and migrates its funds out of v0.8.
        bytes memory entryPointCode = address(new MockEntryPoint()).code;
        vm.etch(ENTRYPOINT_V09, entryPointCode);
        vm.etch(address(ERC4337Utils.ENTRYPOINT_V08), entryPointCode);
        entryPoint = MockEntryPoint(ENTRYPOINT_V09);
        entryPointV08 = MockEntryPoint(address(ERC4337Utils.ENTRYPOINT_V08));

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

        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(address(usdc6)));
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

    /// The retired approve mode is refused even when a standing allowance would have funded it,
    /// so a legacy allowance to the paymaster can no longer be spent.
    function testApproveModeIsRefused() public {
        _fund(1_000e6);
        vm.prank(sender);
        usdc6.approve(address(paymaster), 1_000e6);
        PackedUserOperation memory op = _userOpWithPaymasterData(abi.encodePacked(uint8(1), address(usdc6)));

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMode.selector, uint8(1)));
        paymaster.fetchDetails(op);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMode.selector, uint8(1)));
        paymaster.validate(op, 1e15);
        assertEq(usdc6.balanceOf(address(paymaster)), 0);
    }

    function testFetchDetailsInvalidModeReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(abi.encodePacked(uint8(3), address(usdc6)));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMode.selector, uint8(3)));
        paymaster.fetchDetails(op);
    }

    function testFetchDetailsPermit2ModeReturnsDeadlineAsValidUntil() public view {
        uint256 deadline = block.timestamp + 1 hours;
        PackedUserOperation memory op = _userOpWithPaymasterData(_permit2Data(address(usdc6), 5e6, 7, deadline));
        (uint256 validationData, IERC20 token, uint256 tokenPrice) = paymaster.fetchDetails(op);
        assertEq(validationData, ERC4337Utils.packValidationData(true, 0, uint48(deadline)));
        assertEq(address(token), address(usdc6));
        assertEq(tokenPrice, 6e8);
    }

    function testFetchDetailsPermit2ModeMaxDeadlineHasNoExpiry() public view {
        PackedUserOperation memory op =
            _userOpWithPaymasterData(_permit2Data(address(usdc6), 5e6, 7, type(uint256).max));
        (uint256 validationData,,) = paymaster.fetchDetails(op);
        assertEq(validationData, 0);
    }

    function testFetchDetailsPermit2ModeWrongLengthReverts() public {
        bytes memory data = _permit2Data(address(usdc6), 5e6, 7, block.timestamp + 1 hours);

        PackedUserOperation memory op = _userOpWithPaymasterData(abi.encodePacked(data, uint8(0)));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(183)));
        paymaster.fetchDetails(op);

        bytes memory shorter = new bytes(181);
        for (uint256 i = 0; i < 181; i++) {
            shorter[i] = data[i];
        }
        op = _userOpWithPaymasterData(shorter);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(181)));
        paymaster.fetchDetails(op);

        op = _userOpWithPaymasterData(abi.encodePacked(uint8(2), address(usdc6)));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(21)));
        paymaster.fetchDetails(op);
    }

    // ── Permit2-mode validation guards ───────────────────────────────

    /// The token registry is checked before anything reaches Permit2.
    function testPermit2ModeUnregisteredTokenRejected() public {
        address rogue = makeAddr("rogue");
        PackedUserOperation memory op = _userOpWithPaymasterData(_permit2Data(rogue, 5e6, 7, block.timestamp + 1));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotRegistered.selector, rogue));
        paymaster.validate(op, 1e15);
    }

    function testPermit2ModeDeactivatedTokenRejected() public {
        _govern(SimplexPaymaster.RequestKind.DeactivateToken, abi.encode(address(usdc6)));
        PackedUserOperation memory op =
            _userOpWithPaymasterData(_permit2Data(address(usdc6), 5e6, 7, block.timestamp + 1));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.TokenNotActive.selector, address(usdc6)));
        paymaster.validate(op, 1e15);
    }

    /// A permit smaller than the prefund is rejected before any external call.
    function testPermit2ModePermitBelowPrefundReverts() public {
        // 0.001 native at $600 with the postOp cushion is ~0.62 USDC; permit only 0.5.
        PackedUserOperation memory op =
            _userOpWithPaymasterData(_permit2Data(address(usdc6), 5e5, 7, block.timestamp + 1));
        uint256 required = (1e15 + POST_OP_COST * 1 gwei) * 6e8 / 1e18;
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.InsufficientPermitAmount.selector, uint256(5e5), required)
        );
        paymaster.validate(op, 1e15);
    }

    /// The local chain has no Permit2; the failure must be diagnosable rather than
    /// an empty revert from Solidity's extcodesize check.
    function testPermit2ModeWithoutPermit2CodeReverts() public {
        PackedUserOperation memory op =
            _userOpWithPaymasterData(_permit2Data(address(usdc6), 5e6, 7, block.timestamp + 1));
        vm.expectRevert(SimplexPaymaster.Permit2NotDeployed.selector);
        paymaster.validate(op, 1e15);
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
        _fund(1_000e6);

        PackedUserOperation memory op = _permitOp(5e6, 40_000);
        vm.expectEmit(true, true, true, true, address(paymaster));
        emit PermitExecuted(address(usdc6), sender, 5e6);
        // 0.001 native at $600 costs about 0.62 USDC with the postOp cushion.
        (, uint256 validationData) = paymaster.validate(op, 1e15);
        assertEq(validationData, 0);
        uint256 pulled = usdc6.balanceOf(address(paymaster));
        assertGt(pulled, 0);
        assertEq(usdc6.allowance(sender, address(paymaster)), 5e6 - pulled);
    }

    // ── Initialization ───────────────────────────────────────────────

    function testInitializeOnlyOnce() public {
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        paymaster.initialize(address(hyperbridgeHost), params, tokens, oracles, address(0));
    }

    function testImplementationCannotBeInitialized() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        implementation.initialize(address(hyperbridgeHost), params, tokens, oracles, address(0));
    }

    function testInitializeRejectsNonContractHost() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        bytes memory initData =
            abi.encodeCall(SimplexPaymaster.initialize, (makeAddr("eoa"), params, tokens, oracles, address(0)));
        vm.expectRevert(SimplexPaymaster.InvalidHost.selector);
        new ERC1967Proxy(address(implementation), initData);
    }

    function testInitializeRejectsLengthMismatch() public {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens,) = _initArgs(0);
        AggregatorV3Interface[] memory oracles = new AggregatorV3Interface[](1);
        oracles[0] = AggregatorV3Interface(address(usdcOracle));
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize, (address(hyperbridgeHost), params, tokens, oracles, address(0))
        );
        vm.expectRevert(SimplexPaymaster.LengthMismatch.selector);
        new ERC1967Proxy(address(implementation), initData);
    }

    // ── Governance ───────────────────────────────────────────────────

    function testOnAcceptOnlyHost() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.onAccept(
            _request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, abi.encode(newImpl, bytes("")))
        );
    }

    function testOnAcceptRejectsNonHyperbridgeSource() public {
        address newImpl = address(new SimplexPaymasterHarness());
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.onAccept(
            _request(bytes("EVM-1"), SimplexPaymaster.RequestKind.UpgradeContract, abi.encode(newImpl, bytes("")))
        );
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
            _request(
                HYPERBRIDGE_ID,
                SimplexPaymaster.RequestKind.UpdateParams,
                _paramsPayload(address(nativeOracle), 5_001, treasury, 86_400)
            )
        );

        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidOracleAge.selector, uint256(8 days)));
        paymaster.onAccept(
            _request(
                HYPERBRIDGE_ID,
                SimplexPaymaster.RequestKind.UpdateParams,
                _paramsPayload(address(nativeOracle), 0, treasury, 8 days)
            )
        );

        vm.expectRevert(SimplexPaymaster.ZeroAddress.selector);
        paymaster.onAccept(
            _request(
                HYPERBRIDGE_ID,
                SimplexPaymaster.RequestKind.UpdateParams,
                _paramsPayload(address(nativeOracle), 0, address(0), 86_400)
            )
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

    // ── Relayer gate ─────────────────────────────────────────────────

    function testVersionTracksInitialization() public {
        assertEq(paymaster.version(), 3);
        assertEq(new SimplexPaymasterHarness().version(), type(uint64).max);
    }

    function testInitializeArmsRelayer() public {
        vm.expectEmit(true, true, true, true);
        emit RelayerUpdated(address(0), relayerA);
        SimplexPaymasterHarness armed = _deployPaymaster(0, relayerA);
        assertEq(armed.relayer(), relayerA);

        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.UnauthorizedRelayer.selector);
        armed.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UnlockStake, "", relayerB));
    }

    function testOpenGateAcceptsAnyRelayer() public {
        assertEq(paymaster.relayer(), address(0));
        _govern(
            SimplexPaymaster.RequestKind.UpdateParams,
            _paramsPayload(address(nativeOracle), 100, treasury, 86_400),
            makeAddr("anyone")
        );
        assertEq(paymaster.markupBps(), 100);
    }

    function testSetRelayerViaGovernanceAndGateRefusesOthers() public {
        vm.expectEmit(true, true, true, true, address(paymaster));
        emit RelayerUpdated(address(0), relayerA);
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        assertEq(paymaster.relayer(), relayerA);

        bytes memory payload = _paramsPayload(address(nativeOracle), 100, treasury, 86_400);
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.UnauthorizedRelayer.selector);
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpdateParams, payload, relayerB));
        assertEq(paymaster.markupBps(), 0);

        _govern(SimplexPaymaster.RequestKind.UpdateParams, payload, relayerA);
        assertEq(paymaster.markupBps(), 100);
    }

    function testSetRelayerRotates() public {
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        vm.expectEmit(true, true, true, true, address(paymaster));
        emit RelayerUpdated(relayerA, relayerB);
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerB), relayerA);
        assertEq(paymaster.relayer(), relayerB);

        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.UnauthorizedRelayer.selector);
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UnlockStake, "", relayerA));
        _govern(
            SimplexPaymaster.RequestKind.UpdateParams,
            _paramsPayload(address(nativeOracle), 100, treasury, 86_400),
            relayerB
        );
        assertEq(paymaster.markupBps(), 100);
    }

    /// Zero has no recovery value: whoever can deliver it could deliver a real key instead, and
    /// it would only reopen the gate to forged deliveries.
    function testSetRelayerRejectsZero() public {
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.ZeroAddress.selector);
        paymaster.onAccept(
            _request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.SetRelayer, abi.encode(address(0)), relayerA)
        );
        assertEq(paymaster.relayer(), relayerA);
    }

    /// Pins the wire format: one ABI word, as the pallet encodes it, never a packed address.
    function testSetRelayerRejectsPackedPayload() public {
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert();
        paymaster.onAccept(
            _request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.SetRelayer, abi.encodePacked(relayerA))
        );
        assertEq(paymaster.relayer(), address(0));
    }

    function testArmedGateStillChecksSource() public {
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.onAccept(_request(bytes("EVM-1"), SimplexPaymaster.RequestKind.UnlockStake, "", relayerA));
    }

    function testArmedGateRefusesForgedUpgrade() public {
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        address before = _implementation(address(paymaster));
        address newImpl = address(new SimplexPaymasterHarness());

        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.UnauthorizedRelayer.selector);
        paymaster.onAccept(
            _request(
                HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, ""), relayerB
            )
        );
        assertEq(_implementation(address(paymaster)), before);
    }

    /// `reinitializer(3)` alone would let anyone re-run `initialize` with their own host on a
    /// proxy an upgrade left at version 2; `onlyFresh` is what refuses it.
    function testInitializeRefusedOnProxyAtVersionTwo() public {
        _setVersion(paymaster, 2);
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(0);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        paymaster.initialize(address(hyperbridgeHost), params, tokens, oracles, relayerA);
    }

    // ── EntryPoint migration ─────────────────────────────────────────

    function testEntryPointIsV09AndV08IsRefused() public {
        assertEq(address(paymaster.entryPoint()), ENTRYPOINT_V09);
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 40_000);
        address v08 = address(entryPointV08);

        vm.prank(v08);
        vm.expectRevert(abi.encodeWithSelector(PaymasterCore.PaymasterUnauthorized.selector, v08));
        paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);

        vm.prank(v08);
        vm.expectRevert(abi.encodeWithSelector(PaymasterCore.PaymasterUnauthorized.selector, v08));
        paymaster.postOp(IPaymaster.PostOpMode.opSucceeded, "", 0, 0);

        _validateFrom(bundlerA, op);
    }

    function testMigrateMovesDepositAndStake() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        address newImpl = address(new SimplexPaymasterHarness());

        vm.expectEmit(true, true, true, true, address(paymaster));
        emit SimplexPaymaster.EntryPointMigrated(3 ether, 1 ether, UNSTAKE_DELAY, 2 ether);
        _govern(SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, _migrateCall()));

        assertEq(_implementation(address(paymaster)), newImpl);
        assertEq(paymaster.version(), 3);

        IStakeManager.DepositInfo memory v08 = entryPointV08.getDepositInfo(address(paymaster));
        assertEq(v08.deposit, 0);
        assertFalse(v08.staked);
        assertEq(v08.stake, 1 ether);
        assertEq(v08.withdrawTime, block.timestamp + UNSTAKE_DELAY);

        IStakeManager.DepositInfo memory v09 = entryPoint.getDepositInfo(address(paymaster));
        assertEq(v09.deposit, 2 ether);
        assertTrue(v09.staked);
        assertEq(v09.stake, 1 ether);
        assertEq(v09.unstakeDelaySec, UNSTAKE_DELAY);
        assertEq(address(paymaster).balance, 0);
    }

    function testMigrateWithoutStakeDepositsEverything() public {
        _seedV08(paymaster, 3 ether, 0);
        vm.deal(address(paymaster), 0.5 ether);
        address newImpl = address(new SimplexPaymasterHarness());

        vm.expectCall(address(entryPointV08), abi.encodeCall(MockEntryPoint.unlockStake, ()), 0);
        vm.expectCall(address(entryPoint), abi.encodeWithSelector(MockEntryPoint.addStake.selector), 0);
        vm.expectEmit(true, true, true, true, address(paymaster));
        emit SimplexPaymaster.EntryPointMigrated(3 ether, 0, 0, 3.5 ether);
        _govern(SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, _migrateCall()));

        assertEq(paymaster.version(), 3);
        assertEq(entryPointV08.balanceOf(address(paymaster)), 0);
        IStakeManager.DepositInfo memory v09 = entryPoint.getDepositInfo(address(paymaster));
        assertEq(v09.deposit, 3.5 ether);
        assertFalse(v09.staked);
        assertEq(v09.stake, 0);
        assertEq(address(paymaster).balance, 0);
    }

    function testMigrateCopiesStakeAlreadyUnlocking() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        vm.prank(address(paymaster));
        entryPointV08.unlockStake();
        uint256 withdrawTime = block.timestamp + UNSTAKE_DELAY;
        vm.warp(block.timestamp + 1 hours);

        _govern(
            SimplexPaymaster.RequestKind.UpgradeContract,
            _upgradePayload(address(new SimplexPaymasterHarness()), _migrateCall())
        );

        assertEq(paymaster.version(), 3);
        assertEq(entryPointV08.getDepositInfo(address(paymaster)).withdrawTime, withdrawTime);
        IStakeManager.DepositInfo memory v09 = entryPoint.getDepositInfo(address(paymaster));
        assertTrue(v09.staked);
        assertEq(v09.stake, 1 ether);
        assertEq(v09.unstakeDelaySec, UNSTAKE_DELAY);
        assertEq(v09.deposit, 2 ether);
    }

    /// Short of native for the v0.9 stake, nothing moves and the proxy stays at version 2, so
    /// governance can deliver the same upgrade again once the proxy is topped up.
    function testMigrateRevertsWhenStakeExceedsFunds() public {
        _seedV08(paymaster, 0.5 ether, 1 ether);
        address before = _implementation(address(paymaster));
        bytes memory payload = _upgradePayload(address(new SimplexPaymasterHarness()), _migrateCall());

        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InsufficientStakeFunds.selector, 0.5 ether, 1 ether));
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, payload));

        assertEq(paymaster.version(), 2);
        assertEq(_implementation(address(paymaster)), before);
        assertEq(entryPointV08.balanceOf(address(paymaster)), 0.5 ether);
        assertTrue(entryPointV08.getDepositInfo(address(paymaster)).staked);

        (bool sent,) = address(paymaster).call{value: 0.5 ether}("");
        assertTrue(sent);
        _govern(SimplexPaymaster.RequestKind.UpgradeContract, payload);
        assertEq(paymaster.version(), 3);
        assertEq(entryPoint.stakeOf(address(paymaster)), 1 ether);
        assertEq(entryPoint.balanceOf(address(paymaster)), 0);
    }

    function testMigratePreservesState() public {
        _setBundlers(_addresses(bundlerA), true);
        _setMarkup(200);
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        _seedV08(paymaster, 3 ether, 1 ether);

        _govern(
            SimplexPaymaster.RequestKind.UpgradeContract,
            _upgradePayload(address(new SimplexPaymasterHarness()), _migrateCall()),
            relayerA
        );

        assertEq(paymaster.version(), 3);
        assertEq(paymaster.relayer(), relayerA);
        assertEq(paymaster.treasury(), treasury);
        assertEq(paymaster.markupBps(), 200);
        assertEq(paymaster.getRegisteredTokens().length, 2);
        assertEq(paymaster.getTokenPrice(address(usdc6)), (6e8 * 10_200) / 10_000);
        assertEq(paymaster.getBundlers(), _addresses(bundlerA));
    }

    function testMigrateRejectsEveryoneButHost() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.migrate();
        vm.prank(treasury);
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.migrate();
        assertEq(paymaster.version(), 2);
        assertEq(entryPointV08.balanceOf(address(paymaster)), 3 ether);
    }

    function testMigrateRunsOnce() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        address newImpl = address(new SimplexPaymasterHarness());
        _govern(SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, _migrateCall()));

        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        paymaster.onAccept(
            _request(
                HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, _migrateCall())
            )
        );
        assertEq(paymaster.version(), 3);
    }

    /// Only a proxy one version behind migrates; an older one would skip a migration.
    function testMigrateOnlyFromVersionTwo() public {
        bytes memory payload = _upgradePayload(address(new SimplexPaymasterHarness()), _migrateCall());

        _setVersion(paymaster, 1);
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, payload));
        assertEq(paymaster.version(), 1);

        SimplexPaymasterHarness fresh = _deployPaymaster(0);
        assertEq(fresh.version(), 3);
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        fresh.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.UpgradeContract, payload));
    }

    function testUpgradeWithoutMigrateStaysMigratable() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        address newImpl = address(new SimplexPaymasterHarness());
        _govern(SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, ""));
        assertEq(paymaster.version(), 2);
        assertEq(entryPointV08.balanceOf(address(paymaster)), 3 ether);

        _govern(SimplexPaymaster.RequestKind.UpgradeContract, _upgradePayload(newImpl, _migrateCall()));
        assertEq(paymaster.version(), 3);
        assertEq(entryPointV08.balanceOf(address(paymaster)), 0);
        assertEq(entryPoint.stakeOf(address(paymaster)), 1 ether);
        assertEq(entryPoint.balanceOf(address(paymaster)), 2 ether);
    }

    function testWithdrawStakeV08RevertsBeforeDelay() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        vm.expectRevert("must call unlockStake() first");
        paymaster.withdrawStakeV08();

        _govern(
            SimplexPaymaster.RequestKind.UpgradeContract,
            _upgradePayload(address(new SimplexPaymasterHarness()), _migrateCall())
        );
        vm.warp(block.timestamp + UNSTAKE_DELAY - 1);
        vm.expectRevert("Stake withdrawal is not due");
        paymaster.withdrawStakeV08();
        assertEq(treasury.balance, 0);
    }

    function testWithdrawStakeV08PaysTreasuryAfterDelay() public {
        _seedV08(paymaster, 3 ether, 1 ether);
        _govern(
            SimplexPaymaster.RequestKind.UpgradeContract,
            _upgradePayload(address(new SimplexPaymasterHarness()), _migrateCall())
        );
        vm.warp(block.timestamp + UNSTAKE_DELAY);

        vm.prank(makeAddr("anyone"));
        paymaster.withdrawStakeV08();
        assertEq(treasury.balance, 1 ether);
        assertEq(entryPointV08.stakeOf(address(paymaster)), 0);
        assertEq(entryPoint.stakeOf(address(paymaster)), 1 ether);
        assertEq(entryPoint.balanceOf(address(paymaster)), 2 ether);

        vm.expectRevert("No stake to withdraw");
        paymaster.withdrawStakeV08();
    }

    /// EntryPoint v0.9 may append `paymasterSignature || uint16(len) || magic` to paymasterAndData;
    /// the exact-length permit layouts refuse it rather than reading past their fields.
    function testPermitDataWithPaymasterSignatureRejected() public {
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 40_000);
        bytes memory paymasterAndData = op.paymasterAndData;
        op.paymasterAndData = _withPaymasterSignature(paymasterAndData);

        vm.prank(address(entryPoint));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(225)));
        paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);
        assertEq(usdc6.nonces(sender), 0);

        op.paymasterAndData = paymasterAndData;
        _validateFrom(bundlerA, op);
    }

    function testPermit2DataWithPaymasterSignatureRejected() public {
        PackedUserOperation memory op =
            _userOpWithPaymasterData(_permit2Data(address(usdc6), 5e6, 7, block.timestamp + 1 hours));
        paymaster.fetchDetails(op);

        op.paymasterAndData = _withPaymasterSignature(op.paymasterAndData);
        vm.prank(address(entryPoint));
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, uint256(257)));
        paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);
    }

    // ── Bundler allowlist ────────────────────────────────────────────

    function testEmptyBundlerSetAcceptsAnyOrigin() public {
        assertEq(paymaster.getBundlers().length, 0);
        _fund(1_000e6);
        _validateFrom(makeAddr("anyBundler"), _permitOp(5e6, 40_000));
    }

    /// ERC-7562 bans ORIGIN during validation, so the `tx.origin` lookup must not run while the
    /// list is empty.
    function testEmptyBundlerSetSkipsOriginLookup() public {
        _fund(1_000e6);
        assertFalse(_validationLooksUpOrigin(bundlerA), "origin looked up with an empty list");

        _setBundlers(_addresses(bundlerA), true);
        assertTrue(_validationLooksUpOrigin(bundlerA), "origin lookup not detected with a non-empty list");
    }

    function testListedBundlerPasses() public {
        _setBundlers(_addresses(bundlerA, bundlerB), true);
        _fund(1_000e6);
        _validateFrom(bundlerB, _permitOp(5e6, 40_000));
        assertEq(usdc6.nonces(sender), 1);
    }

    function testUnlistedBundlerRevertsBeforePermit() public {
        _setBundlers(_addresses(bundlerA), true);
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 40_000);

        vm.prank(address(entryPoint), bundlerB);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.UnauthorizedBundler.selector, bundlerB));
        paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);
        assertEq(usdc6.nonces(sender), 0);
        assertEq(usdc6.allowance(sender, address(paymaster)), 0);

        _validateFrom(bundlerA, op);
        assertEq(usdc6.nonces(sender), 1);
    }

    /// The origin is checked before the postOp bounds and the permit, so an unlisted bundler
    /// never reaches a validation-phase external call.
    function testBundlerCheckRunsFirst() public {
        _setBundlers(_addresses(bundlerA), true);
        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(address(usdc6)), uint128(100_001));
        vm.prank(address(entryPoint), bundlerB);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.UnauthorizedBundler.selector, bundlerB));
        paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);
    }

    function testSimulationOriginPassesOnlyOnceListed() public {
        _setBundlers(_addresses(bundlerA), true);
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 40_000);
        _expectBundlerRefused(RUNDLER_SIMULATION_ORIGIN);

        _setBundlers(_addresses(RUNDLER_SIMULATION_ORIGIN), true);
        _validateFrom(RUNDLER_SIMULATION_ORIGIN, op);
    }

    function testRemovingEveryBundlerTurnsTheCheckOff() public {
        _setBundlers(_addresses(bundlerA, bundlerB), true);
        _setBundlers(_addresses(bundlerA), false);
        assertEq(paymaster.getBundlers(), _addresses(bundlerB));
        _expectBundlerRefused(bundlerA);

        _setBundlers(_addresses(bundlerB), false);
        assertEq(paymaster.getBundlers().length, 0);
        _fund(1_000e6);
        _validateFrom(bundlerA, _permitOp(5e6, 40_000));
    }

    function testReAddAndRemovingNonMemberAreSilentNoOps() public {
        _setBundlers(_addresses(bundlerA), true);

        vm.recordLogs();
        _setBundlers(_addresses(bundlerA), true);
        _setBundlers(_addresses(bundlerB), false);
        assertEq(vm.getRecordedLogs().length, 0);
        assertEq(paymaster.getBundlers(), _addresses(bundlerA));
    }

    function testBundlerUpdatedEmittedOnlyOnChange() public {
        address[] memory bundlers = new address[](3);
        bundlers[0] = bundlerA;
        bundlers[1] = bundlerB;
        bundlers[2] = bundlerA;

        vm.recordLogs();
        _setBundlers(bundlers, true);
        Vm.Log[] memory logs = vm.getRecordedLogs();
        assertEq(logs.length, 2);
        _assertBundlerLog(logs[0], bundlerA, true);
        _assertBundlerLog(logs[1], bundlerB, true);

        vm.recordLogs();
        _setBundlers(bundlers, false);
        logs = vm.getRecordedLogs();
        assertEq(logs.length, 2);
        _assertBundlerLog(logs[0], bundlerA, false);
        _assertBundlerLog(logs[1], bundlerB, false);
    }

    function testSetBundlersRejectsZero() public {
        bytes memory withZero = abi.encode(_addresses(bundlerA, address(0)), true);
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.ZeroAddress.selector);
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.SetBundlers, withZero));
        assertEq(paymaster.getBundlers().length, 0);
    }

    function testSetBundlersHonoursRelayerGate() public {
        _govern(SimplexPaymaster.RequestKind.SetRelayer, abi.encode(relayerA));
        bytes memory payload = abi.encode(_addresses(bundlerA), true);

        vm.prank(address(hyperbridgeHost));
        vm.expectRevert(SimplexPaymaster.UnauthorizedRelayer.selector);
        paymaster.onAccept(_request(HYPERBRIDGE_ID, SimplexPaymaster.RequestKind.SetBundlers, payload, relayerB));
        assertEq(paymaster.getBundlers().length, 0);

        _govern(SimplexPaymaster.RequestKind.SetBundlers, payload, relayerA);
        assertEq(paymaster.getBundlers(), _addresses(bundlerA));
    }

    /// Pins the wire format: bare ABI `(address[], bool)`, as the pallet encodes it, never packed.
    function testSetBundlersRejectsPackedPayload() public {
        vm.prank(address(hyperbridgeHost));
        vm.expectRevert();
        paymaster.onAccept(
            _request(
                HYPERBRIDGE_ID,
                SimplexPaymaster.RequestKind.SetBundlers,
                abi.encodePacked(_addresses(bundlerA, bundlerB), true)
            )
        );
        assertEq(paymaster.getBundlers().length, 0);
    }

    /// The exact body the pallet produces for this call; both sides pin the same bytes.
    function testSetBundlersWireFixture() public {
        bytes memory body = bytes.concat(
            hex"08",
            hex"0000000000000000000000000000000000000000000000000000000000000040",
            hex"0000000000000000000000000000000000000000000000000000000000000001",
            hex"0000000000000000000000000000000000000000000000000000000000000002",
            hex"0000000000000000000000001111111111111111111111111111111111111111",
            hex"0000000000000000000000002222222222222222222222222222222222222222"
        );
        assertEq(body.length, 161);
        assertEq(uint8(SimplexPaymaster.RequestKind.SetBundlers), 8);

        IncomingPostRequest memory incoming;
        incoming.request.source = HYPERBRIDGE_ID;
        incoming.request.body = body;
        vm.prank(address(hyperbridgeHost));
        paymaster.onAccept(incoming);

        assertEq(
            paymaster.getBundlers(),
            _addresses(0x1111111111111111111111111111111111111111, 0x2222222222222222222222222222222222222222)
        );
    }

    /// `_bundlers` takes two slots out of `__gap`, so every earlier field and the end of the
    /// layout stay where live proxies have them.
    function testStorageLayoutAppendsBundlersInsideTheGap() public view {
        string memory json = vm.readFile("out/SimplexPaymaster.sol/SimplexPaymaster.json");
        require(vm.keyExistsJson(json, ".storageLayout"), "build with extra_output = [\"storageLayout\"]");
        StorageEntry[] memory layout = abi.decode(vm.parseJson(json, ".storageLayout.storage"), (StorageEntry[]));

        string[12] memory labels = [
            "_hostAddr",
            "nativeOracle",
            "nativeOracleDecimals",
            "maxOracleAge",
            "markupBps",
            "treasury",
            "tokenConfigs",
            "registeredTokens",
            "swapSlippageBps",
            "_relayer",
            "_bundlers",
            "__gap"
        ];
        string[12] memory slots = ["0", "1", "1", "2", "3", "4", "5", "6", "7", "8", "9", "11"];
        assertEq(layout.length, labels.length);
        for (uint256 i; i < labels.length; i++) {
            assertEq(layout[i].label, labels[i]);
            assertEq(layout[i].slot, slots[i], labels[i]);
            assertEq(layout[i].offset, uint256(i == 2 ? 20 : 0), labels[i]);
        }
        assertEq(vm.indexOf(layout[10].type_, "t_struct(AddressSet)"), 0, "_bundlers type");
        assertEq(layout[11].type_, "t_array(t_uint256)46_storage", "__gap type");
        assertEq(vm.parseUint(layout[11].slot) + 46 - 1, 56, "last used slot");
    }

    // ── postOp gas limit cap ─────────────────────────────────────────

    function testPostOpGasLimitAboveCapReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(address(usdc6)), uint128(100_001));
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.InvalidPostOpGasLimit.selector, uint256(100_001), uint256(30_000), uint256(100_000)
            )
        );
        paymaster.validate(op, 1e15);
    }

    /// Below the floor the refund subtraction can underflow on `innerHandleOp` overhead
    /// that sits outside every gas limit, so a too-small limit is refused outright.
    function testPostOpGasLimitBelowFloorReverts() public {
        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(address(usdc6)), uint128(29_999));
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.InvalidPostOpGasLimit.selector, uint256(29_999), uint256(30_000), uint256(100_000)
            )
        );
        paymaster.validate(op, 1e15);
    }

    function testPostOpGasLimitAtCapAccepted() public {
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 100_000);
        (, uint256 validationData) = paymaster.validate(op, 1e15);
        assertEq(validationData, 0);
    }

    /// The SDK's penalty-free 40k value sits inside the accepted band.
    function testPostOpGasLimitAtSdkValueAccepted() public {
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 40_000);
        (, uint256 validationData) = paymaster.validate(op, 1e15);
        assertEq(validationData, 0);
    }

    function testPostOpGasLimitAtFloorAccepted() public {
        _fund(1_000e6);
        PackedUserOperation memory op = _permitOp(5e6, 30_000);
        (, uint256 validationData) = paymaster.validate(op, 1e15);
        assertEq(validationData, 0);
    }

    // ── Stake ────────────────────────────────────────────────────────

    /// An open `addStake` lets anyone stretch `unstakeDelaySec`, which the EntryPoint only
    /// ever grows — pinning the stake beyond the point governance could recover it.
    function testAddStakeRejectsNonTreasury() public {
        vm.deal(address(this), 1 ether);
        vm.expectRevert(HyperApp.UnauthorizedCall.selector);
        paymaster.addStake{value: 0.1 ether}(86_400);
    }

    function testTreasuryCanAddStakeAndGovernanceCanRecoverIt() public {
        vm.deal(treasury, 1 ether);
        vm.prank(treasury);
        paymaster.addStake{value: 0.1 ether}(86_400);
        assertEq(entryPoint.stakeOf(address(paymaster)), 0.1 ether);
        assertEq(entryPointV08.stakeOf(address(paymaster)), 0);

        _govern(SimplexPaymaster.RequestKind.UnlockStake, "");
        assertFalse(entryPoint.getDepositInfo(address(paymaster)).staked);

        vm.warp(block.timestamp + 86_400);
        _govern(SimplexPaymaster.RequestKind.WithdrawStake, "");
        assertEq(entryPoint.stakeOf(address(paymaster)), 0);
        assertEq(treasury.balance, 1 ether);
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
            SimplexPaymaster.initialize, (address(hyperbridgeHost), params, tokens, oracles, address(0))
        );
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.InvalidMarkup.selector, uint256(5_001)));
        new ERC1967Proxy(address(implementation), initData);
    }

    // ── Helpers ──────────────────────────────────────────────────────

    function _initArgs(uint256 markupBps)
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
        return _deployPaymaster(markupBps, address(0));
    }

    function _deployPaymaster(uint256 markupBps, address relayer_) internal returns (SimplexPaymasterHarness) {
        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        (SimplexPaymaster.Params memory params, address[] memory tokens, AggregatorV3Interface[] memory oracles) =
            _initArgs(markupBps);
        bytes memory initData =
            abi.encodeCall(SimplexPaymaster.initialize, (address(hyperbridgeHost), params, tokens, oracles, relayer_));
        return SimplexPaymasterHarness(payable(address(new ERC1967Proxy(address(implementation), initData))));
    }

    function _request(bytes memory source, SimplexPaymaster.RequestKind kind, bytes memory payload)
        internal
        pure
        returns (IncomingPostRequest memory req)
    {
        return _request(source, kind, payload, address(0));
    }

    function _request(bytes memory source, SimplexPaymaster.RequestKind kind, bytes memory payload, address relayer_)
        internal
        pure
        returns (IncomingPostRequest memory req)
    {
        req.request.source = source;
        req.request.body = bytes.concat(bytes1(uint8(kind)), payload);
        req.relayer = relayer_;
    }

    /// @dev Delivers a governance request as the host with Hyperbridge as the source.
    function _govern(SimplexPaymaster.RequestKind kind, bytes memory payload) internal {
        _govern(kind, payload, address(0));
    }

    function _govern(SimplexPaymaster.RequestKind kind, bytes memory payload, address relayer_) internal {
        _governOn(paymaster, kind, payload, relayer_);
    }

    function _governOn(
        SimplexPaymasterHarness target,
        SimplexPaymaster.RequestKind kind,
        bytes memory payload,
        address relayer_
    ) internal {
        vm.prank(address(hyperbridgeHost));
        target.onAccept(_request(HYPERBRIDGE_ID, kind, payload, relayer_));
    }

    function _upgradePayload(address newImpl, bytes memory initData) internal pure returns (bytes memory) {
        return abi.encode(newImpl, initData);
    }

    function _migrateCall() internal pure returns (bytes memory) {
        return abi.encodeCall(SimplexPaymaster.migrate, ());
    }

    /// @dev Puts `target` at version 2 with a v0.8 deposit and, when `stake` is non-zero, a v0.8
    ///      stake locked for `UNSTAKE_DELAY`, as live proxies hold them.
    function _seedV08(SimplexPaymasterHarness target, uint256 deposit, uint256 stake) internal {
        _setVersion(target, 2);
        vm.deal(address(this), address(this).balance + deposit);
        entryPointV08.depositTo{value: deposit}(address(target));
        if (stake > 0) {
            vm.deal(address(target), address(target).balance + stake);
            vm.prank(address(target));
            entryPointV08.addStake{value: stake}(UNSTAKE_DELAY);
        }
    }

    /// @dev Appends a 65-byte v0.9 `paymasterSignature` suffix to `paymasterAndData`.
    function _withPaymasterSignature(bytes memory paymasterAndData) internal pure returns (bytes memory) {
        bytes memory signature = new bytes(65);
        return abi.encodePacked(paymasterAndData, signature, uint16(signature.length), PAYMASTER_SIG_MAGIC);
    }

    /// @dev Rewinds the `Initializable` version to look like a proxy deployed by an earlier implementation.
    function _setVersion(SimplexPaymasterHarness target, uint64 version_) internal {
        vm.store(address(target), INITIALIZABLE_SLOT, bytes32(uint256(version_)));
        assertEq(target.version(), version_);
    }

    function _implementation(address proxy) internal view returns (address) {
        return address(uint160(uint256(vm.load(proxy, ERC1967Utils.IMPLEMENTATION_SLOT))));
    }

    function _paramsPayload(address oracle, uint256 markupBps, address treasury_, uint256 maxOracleAge)
        internal
        pure
        returns (bytes memory)
    {
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

    function _fund(uint256 amount) internal {
        deal(address(usdc6), sender, amount);
    }

    function _setBundlers(address[] memory bundlers, bool allowed) internal {
        _govern(SimplexPaymaster.RequestKind.SetBundlers, abi.encode(bundlers, allowed));
    }

    function _addresses(address a) internal pure returns (address[] memory list) {
        list = new address[](1);
        list[0] = a;
    }

    function _addresses(address a, address b) internal pure returns (address[] memory list) {
        list = new address[](2);
        list[0] = a;
        list[1] = b;
    }

    /// @dev Calls the public entry point as the EntryPoint, inside a transaction `origin` sent.
    function _validateFrom(address origin, PackedUserOperation memory op) internal {
        vm.prank(address(entryPoint), origin);
        (, uint256 validationData) = paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);
        assertEq(validationData, 0);
    }

    /// @dev Whether validation reads `_bundlers._positions[origin]`, i.e. evaluates
    ///      `_bundlers.contains(tx.origin)`. Debug trace cheatcodes need `-vvv`, so storage reads
    ///      stand in for the ORIGIN opcode.
    function _validationLooksUpOrigin(address origin) internal returns (bool) {
        PackedUserOperation memory op = _permitOp(5e6, 40_000);
        bytes32 positionSlot = keccak256(abi.encode(bytes32(uint256(uint160(origin))), BUNDLERS_SLOT + 1));

        vm.record();
        _validateFrom(origin, op);
        (bytes32[] memory reads,) = vm.accesses(address(paymaster));
        vm.stopRecord();

        for (uint256 i; i < reads.length; i++) {
            if (reads[i] == positionSlot) return true;
        }
        return false;
    }

    function _expectBundlerRefused(address origin) internal {
        PackedUserOperation memory op = _userOpWithPaymasterData(_permitData(address(usdc6)));
        vm.prank(address(entryPoint), origin);
        vm.expectRevert(abi.encodeWithSelector(SimplexPaymaster.UnauthorizedBundler.selector, origin));
        paymaster.validatePaymasterUserOp(op, bytes32(0), 1e15);
    }

    function _assertBundlerLog(Vm.Log memory log, address bundler, bool allowed) internal view {
        assertEq(log.emitter, address(paymaster));
        assertEq(log.topics[0], SimplexPaymaster.BundlerUpdated.selector);
        assertEq(log.topics[1], bytes32(uint256(uint160(bundler))));
        assertEq(log.data, abi.encode(allowed));
    }

    /// @dev A permit-mode op whose EIP-2612 signature the sender key really signed.
    function _permitOp(uint256 permitAmount, uint128 postOpGasLimit)
        internal
        view
        returns (PackedUserOperation memory)
    {
        uint256 deadline = type(uint256).max;
        bytes32 structHash = keccak256(
            abi.encode(PERMIT_TYPEHASH, sender, address(paymaster), permitAmount, usdc6.nonces(sender), deadline)
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", usdc6.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(senderKey, digest);
        return _userOpWithPaymasterData(
            abi.encodePacked(uint8(0), address(usdc6), permitAmount, deadline, v, r, s), postOpGasLimit
        );
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

    /// @dev A well-formed 182-byte Permit2-mode payload (mode 0x02) with a zero signature.
    function _permit2Data(address token, uint256 permitAmount, uint256 nonce, uint256 deadline)
        internal
        pure
        returns (bytes memory)
    {
        return abi.encodePacked(uint8(2), token, permitAmount, nonce, deadline, new bytes(65));
    }

    /// @dev paymasterAndData = paymaster(20) || verificationGasLimit(16) || postOpGasLimit(16) || data
    ///      gasFees = maxPriorityFeePerGas(16) || maxFeePerGas(16), both 1 gwei
    function _userOpWithPaymasterData(bytes memory data) internal view returns (PackedUserOperation memory op) {
        return _userOpWithPaymasterData(data, 40_000);
    }

    function _userOpWithPaymasterData(bytes memory data, uint128 postOpGasLimit)
        internal
        view
        returns (PackedUserOperation memory op)
    {
        op.sender = sender;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | uint256(1 gwei));
        op.paymasterAndData = abi.encodePacked(address(paymaster), uint128(150_000), postOpGasLimit, data);
    }
}
