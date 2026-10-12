// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IEntryPoint} from "@account-abstraction/contracts/interfaces/IEntryPoint.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../../src/utils/SimplexPaymaster.sol";
import {SolverAccount} from "../../src/apps/intentsv2/SolverAccount.sol";
import {SimplexPaymasterHarness} from "./SimplexPaymasterTest.t.sol";
import {ISignatureTransfer} from "@uniswap/permit2/src/interfaces/ISignatureTransfer.sol";
import {toEntryPointOp, toEntryPointOps} from "./EntryPointOps.sol";

/// @notice Measures whether the gas the paymaster is charged by the EntryPoint stays
///         covered by the tokens it charges the user, when the user inflates gas
///         limits it does not consume. EntryPoint v0.7+ adds a penalty on the unused
///         portion of `callGasLimit + paymasterPostOpGasLimit`; the paymaster only
///         caps the latter. Runs against the real EntryPoint v0.9 on a fork.
contract SimplexPaymasterGasGriefTest is Test {
    IEntryPoint constant ENTRY_POINT = IEntryPoint(0x433709009B8330FDa32311DF1C2AFA402eD8D009);
    ISignatureTransfer constant PERMIT2 = ISignatureTransfer(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    bytes32 constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
    bytes32 constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );

    address constant HOST = 0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3;
    address constant ETH_USD = 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419;
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;
    address constant USDT_USD = 0x3E7d1eAB13ad0104d2750B8863b489D65364e32D;
    address constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address constant USDC_USD = 0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6;
    address constant INTENT_GATEWAY = 0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716;
    // mode(1) + token(20) + permitAmount(32) + nonce(32) + deadline(32) + signature(65)
    uint256 constant PERMIT2_DATA_LENGTH = 182;
    bytes8 constant PAYMASTER_SIG_MAGIC = 0x22e325a297439656;

    address solver;
    uint256 solverKey;
    address beneficiary = makeAddr("beneficiary");
    address bundler = makeAddr("bundler");
    SimplexPaymasterHarness paymaster;

    function setUp() public {
        string memory url = vm.envOr("MAINNET_FORK_URL", string(""));
        if (bytes(url).length == 0) return;
        vm.selectFork(vm.createFork(url));

        (solver, solverKey) = makeAddrAndKey("gas-grief-solver");
        // The solver EOA runs the local SolverAccount build, which serves v0.9.
        vm.etch(solver, address(new SolverAccount(INTENT_GATEWAY)).code);

        address[] memory tokens = new address[](2);
        tokens[0] = USDT;
        tokens[1] = USDC;
        AggregatorV3Interface[] memory oracles = new AggregatorV3Interface[](2);
        oracles[0] = AggregatorV3Interface(USDT_USD);
        oracles[1] = AggregatorV3Interface(USDC_USD);

        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (
                HOST,
                SimplexPaymaster.Params({
                    nativeOracle: AggregatorV3Interface(ETH_USD),
                    markupBps: 200,
                    treasury: makeAddr("treasury"),
                    maxOracleAge: 7 days,
                    swapSlippageBps: 200
                }),
                tokens,
                oracles,
                address(0)
            )
        );
        paymaster = SimplexPaymasterHarness(payable(address(new ERC1967Proxy(address(implementation), initData))));

        // Raw call: Tether's approve returns no bool.
        deal(USDT, solver, 1_000_000e6);
        vm.prank(solver);
        (bool ok,) = USDT.call(abi.encodeWithSelector(IERC20.approve.selector, address(PERMIT2), type(uint256).max));
        require(ok, "approve failed");
        deal(USDC, solver, 1_000_000e6);
        vm.prank(solver);
        IERC20(USDC).approve(address(PERMIT2), type(uint256).max);

        vm.deal(address(this), 100 ether);
        ENTRY_POINT.depositTo{value: 10 ether}(address(paymaster));
    }

    modifier onFork() {
        vm.skip(address(paymaster) == address(0));
        _;
    }

    /// Honest op: a modest callGasLimit that the (empty) call mostly uses.
    function testHonestOpIsProfitableForPaymaster() public onFork {
        (uint256 weiCharged, uint256 nativeSpent) = _run(60_000);
        emit log_named_uint("honest: wei-equivalent charged to user", weiCharged);
        emit log_named_uint("honest: native debited from paymaster", nativeSpent);
        assertGe(weiCharged, nativeSpent, "paymaster must not subsidise an honest op");
    }

    /// Griefing op: identical work, but callGasLimit inflated. The unused portion is
    /// penalised by the EntryPoint; the question is who pays that penalty.
    function testInflatedCallGasLimitDoesNotDrainPaymaster() public onFork {
        (uint256 weiCharged, uint256 nativeSpent) = _run(10_000_000);
        emit log_named_uint("inflated: wei-equivalent charged to user", weiCharged);
        emit log_named_uint("inflated: native debited from paymaster", nativeSpent);
        if (nativeSpent > weiCharged) {
            emit log_named_uint("PAYMASTER SUBSIDY (wei) per op", nativeSpent - weiCharged);
        }
        assertGe(weiCharged, nativeSpent, "paymaster subsidised an inflated-gas op");
    }

    /// EntryPoint v0.9 penalises the unused part of `paymasterPostOpGasLimit` AFTER the value
    /// handed to postOp is fixed, so that penalty is never billed to the user — the paymaster
    /// eats it out of the `_postOpCost()` cushion. The penalty is waived while
    /// `gasLimit <= gasUsed + PENALTY_GAS_THRESHOLD (40k)`, so the SDK sends 40k: the largest
    /// penalty-free value. The margin at 40k must equal the margin at the 30k floor — flat and
    /// penalty-free — even though the contract ceiling stays at 100k for backward compatibility.
    function testSdkPostOpValueIsPenaltyFree() public onFork {
        // Discarded: the first sponsored op pays the cold-storage cost of the paymaster's
        // and sender's token balance slots, which would otherwise read as a penalty.
        _run(60_000, 40_000);

        (uint256 chargedSdk, uint256 spentSdk) = _run(60_000, 40_000);
        uint256 marginAtSdk = chargedSdk - spentSdk;

        (uint256 chargedFloor, uint256 spentFloor) = _run(60_000, 30_000);
        uint256 marginAtFloor = chargedFloor - spentFloor;

        emit log_named_uint("margin @ sdk   40k (wei)", marginAtSdk);
        emit log_named_uint("margin @ floor 30k (wei)", marginAtFloor);
        // Identical across the SDK's [30k, 40k] range: the EntryPoint waives the penalty
        // while gasLimit <= gasUsed + 40k, so 40k is penalty-free whatever postOp costs.
        assertEq(marginAtSdk, marginAtFloor, "penalty charged inside the SDK band");
    }

    /// The band the contract enforces is exactly [MIN, MAX]; outside it validation refuses.
    /// The contract band is [MIN, MAX] = [30k, 100k]. Above/below it, validation refuses with
    /// the exact InvalidPostOpGasLimit(supplied, min, max) — a bare expectRevert would also pass
    /// on an unrelated AA revert and prove nothing.
    function testPostOpLimitsOutsideTheBandAreRefused() public onFork {
        // Build the op before expectRevert — _buildOp itself makes an external call (getUserOpHash).
        PackedUserOperation memory tooHigh = _buildOp(USDT, 60_000, 100_001);
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.InvalidPostOpGasLimit.selector, uint256(100_001), uint256(30_000), uint256(100_000)
            )
        );
        paymaster.validate(tooHigh, 1e14);

        PackedUserOperation memory tooLow = _buildOp(USDT, 60_000, 20_000);
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.InvalidPostOpGasLimit.selector, uint256(20_000), uint256(30_000), uint256(100_000)
            )
        );
        paymaster.validate(tooLow, 1e14);
    }

    /// Across the contract's whole accepted band — the SDK's penalty-free [30k, 40k] and
    /// the 100k ceiling kept for older clients, where the EntryPoint's unused-gas penalty
    /// DOES apply — both fee tokens stay profitable for the paymaster: the penalty at 100k
    /// must fit inside the _postOpCost cushion the user already pays. This is the case the
    /// restored ceiling's safety argument rests on.
    function testPostOpProfitableForBothTokensAcrossTheBand() public onFork {
        uint128[3] memory limits = [uint128(100_000), 40_000, 30_000];
        address[2] memory toks = [USDT, USDC];
        string[2] memory names = ["USDT", "USDC"];
        for (uint256 t = 0; t < toks.length; t++) {
            for (uint256 i = 0; i < limits.length; i++) {
                (uint256 charged, uint256 spent) = _runToken(toks[t], 60_000, limits[i]);
                emit log_named_string("token", names[t]);
                emit log_named_uint("  postOpGasLimit", limits[i]);
                emit log_named_uint("  margin (wei)", charged - spent);
                assertGe(charged, spent, "paymaster subsidised an op inside the band");
            }
        }
    }

    /// Through the real EntryPoint, only a listed bundler's handleOps reaches the sponsored op;
    /// any other origin is refused with the paymaster's own reason.
    function testBundlerAllowlistGatesHandleOps() public onFork {
        address outsider = makeAddr("outsider");
        address[] memory bundlers = new address[](1);
        bundlers[0] = bundler;

        IncomingPostRequest memory incoming;
        incoming.request.source = IDispatcher(HOST).hyperbridge();
        incoming.request.body =
            bytes.concat(bytes1(uint8(SimplexPaymaster.RequestKind.SetBundlers)), abi.encode(bundlers, true));
        vm.prank(HOST);
        paymaster.onAccept(incoming);

        PackedUserOperation[] memory ops = new PackedUserOperation[](1);
        ops[0] = _buildOp(USDT, 60_000, 40_000);
        uint256 nonce = ops[0].nonce;

        vm.prank(outsider, outsider);
        vm.expectRevert(
            abi.encodeWithSelector(
                IEntryPoint.FailedOpWithRevert.selector,
                uint256(0),
                "AA33 reverted",
                abi.encodeWithSelector(SimplexPaymaster.UnauthorizedBundler.selector, outsider)
            )
        );
        ENTRY_POINT.handleOps(toEntryPointOps(ops), payable(beneficiary));

        vm.prank(bundler, bundler);
        ENTRY_POINT.handleOps(toEntryPointOps(ops), payable(beneficiary));
        assertEq(ENTRY_POINT.getNonce(solver, 0), nonce + 1);
    }

    /// EntryPoint v0.9 accepts a `paymasterSignature ‖ uint16 length ‖ magic` suffix on
    /// `paymasterAndData`, and `paymasterData()` hands it to the paymaster unstripped, so the
    /// PERMIT2 exact-length parse refuses the op during validation.
    function testPaymasterSignatureSuffixIsRefused() public onFork {
        (, uint256 paymasterSignerKey) = makeAddrAndKey("paymaster-signer");
        PackedUserOperation[] memory ops = new PackedUserOperation[](1);
        ops[0] = _buildOp(USDT, 60_000, 40_000, paymasterSignerKey);
        uint256 dataLength = PERMIT2_DATA_LENGTH + 65 + 2 + PAYMASTER_SIG_MAGIC.length;
        assertEq(ops[0].paymasterAndData.length, 52 + dataLength);

        vm.prank(bundler, bundler);
        vm.expectRevert(
            abi.encodeWithSelector(
                IEntryPoint.FailedOpWithRevert.selector,
                uint256(0),
                "AA33 reverted",
                abi.encodeWithSelector(SimplexPaymaster.InvalidPaymasterData.selector, dataLength)
            )
        );
        ENTRY_POINT.handleOps(toEntryPointOps(ops), payable(beneficiary));
    }

    function _run(uint128 callGasLimit) internal returns (uint256 weiCharged, uint256 nativeSpent) {
        return _run(callGasLimit, 40_000);
    }

    /// @dev Runs one sponsored no-op and returns
    ///      (wei-equivalent of tokens taken from the user, native debited from the paymaster).
    function _run(uint128 callGasLimit, uint128 postOpGasLimit)
        internal
        returns (uint256 weiCharged, uint256 nativeSpent)
    {
        return _runToken(USDT, callGasLimit, postOpGasLimit);
    }

    function _runToken(address token, uint128 callGasLimit, uint128 postOpGasLimit)
        internal
        returns (uint256 weiCharged, uint256 nativeSpent)
    {
        PackedUserOperation memory op = _buildOp(token, callGasLimit, postOpGasLimit);

        uint256 tokensBefore = IERC20(token).balanceOf(solver);
        uint256 depositBefore = ENTRY_POINT.balanceOf(address(paymaster));

        PackedUserOperation[] memory ops = new PackedUserOperation[](1);
        ops[0] = op;
        vm.prank(bundler, bundler);
        ENTRY_POINT.handleOps(toEntryPointOps(ops), payable(beneficiary));

        uint256 tokensCharged = tokensBefore - IERC20(token).balanceOf(solver);
        nativeSpent = depositBefore - ENTRY_POINT.balanceOf(address(paymaster));
        // Convert the token charge back to wei at the paymaster's own quoted price.
        weiCharged = (tokensCharged * 1e18) / paymaster.getTokenPrice(token);
    }

    function _buildOp(address token, uint128 callGasLimit, uint128 postOpGasLimit)
        internal
        view
        returns (PackedUserOperation memory op)
    {
        return _buildOp(token, callGasLimit, postOpGasLimit, 0);
    }

    /// @dev A non-zero `paymasterSignerKey` appends a v0.9 `paymasterSignature` suffix signed by it.
    function _buildOp(address token, uint128 callGasLimit, uint128 postOpGasLimit, uint256 paymasterSignerKey)
        internal
        view
        returns (PackedUserOperation memory op)
    {
        op.sender = solver;
        op.nonce = ENTRY_POINT.getNonce(solver, 0);
        op.callData = "";
        op.accountGasLimits = bytes32((uint256(200_000) << 128) | uint256(callGasLimit));
        op.preVerificationGas = 60_000;
        uint256 maxFee = block.basefee + 1 gwei;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | maxFee);
        // Permit2 mode. The account nonce doubles as the Permit2 nonce: unique per executed op and
        // sequential, so the bitmap word stays warm across the runs the margin assertions compare.
        uint256 deadline = block.timestamp + 1 hours;
        bytes memory permitSig = _signPermit2(token, 1_000e6, op.nonce, deadline);
        op.paymasterAndData = abi.encodePacked(
            address(paymaster),
            uint128(300_000),
            postOpGasLimit,
            abi.encodePacked(uint8(2), token, uint256(1_000e6), op.nonce, deadline, permitSig)
        );
        if (paymasterSignerKey != 0) {
            bytes memory paymasterSig = _sign(paymasterSignerKey, ENTRY_POINT.getUserOpHash(toEntryPointOp(op)));
            op.paymasterAndData =
                abi.encodePacked(op.paymasterAndData, paymasterSig, uint16(paymasterSig.length), PAYMASTER_SIG_MAGIC);
        }
        op.signature = _sign(solverKey, ENTRY_POINT.getUserOpHash(toEntryPointOp(op)));
    }

    /// @dev v ‖ r ‖ s, the layout mode 0x02 expects.
    function _signPermit2(address token, uint256 amount, uint256 nonce, uint256 deadline)
        internal
        view
        returns (bytes memory)
    {
        bytes32 structHash = keccak256(
            abi.encode(
                PERMIT_TRANSFER_FROM_TYPEHASH,
                keccak256(abi.encode(TOKEN_PERMISSIONS_TYPEHASH, token, amount)),
                address(paymaster),
                nonce,
                deadline
            )
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", PERMIT2.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverKey, digest);
        return abi.encodePacked(v, r, s);
    }

    function _sign(uint256 key, bytes32 digest) internal pure returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(key, digest);
        return abi.encodePacked(r, s, v);
    }
}
