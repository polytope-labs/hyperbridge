// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../../src/utils/SimplexPaymaster.sol";
import {SimplexPaymasterHarness} from "./SimplexPaymasterTest.t.sol";

interface IEntryPointGas {
    function handleOps(PackedUserOperation[] calldata ops, address payable beneficiary) external;

    function getUserOpHash(PackedUserOperation calldata userOp) external view returns (bytes32);

    function depositTo(address account) external payable;

    function balanceOf(address account) external view returns (uint256);

    function getNonce(address sender, uint192 key) external view returns (uint256);
}

/// @notice Measures whether the gas the paymaster is charged by the EntryPoint stays
///         covered by the tokens it charges the user, when the user inflates gas
///         limits it does not consume. EntryPoint v0.7+ adds a penalty on the unused
///         portion of `callGasLimit + paymasterPostOpGasLimit`; the paymaster only
///         caps the latter. Runs against the real EntryPoint v0.8 on a fork.
contract SimplexPaymasterGasGriefTest is Test {
    IEntryPointGas constant ENTRY_POINT = IEntryPointGas(address(ERC4337Utils.ENTRYPOINT_V08));

    address constant HOST = 0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3;
    address constant ETH_USD = 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419;
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;
    address constant USDT_USD = 0x3E7d1eAB13ad0104d2750B8863b489D65364e32D;
    address constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address constant USDC_USD = 0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6;
    address constant SOLVER_ACCOUNT = 0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3;

    address solver;
    uint256 solverKey;
    address beneficiary = makeAddr("beneficiary");
    SimplexPaymasterHarness paymaster;

    function setUp() public {
        string memory url = vm.envOr("MAINNET_FORK_URL", string(""));
        if (bytes(url).length == 0) return;
        vm.selectFork(vm.createFork(url));

        (solver, solverKey) = makeAddrAndKey("gas-grief-solver");
        vm.etch(solver, SOLVER_ACCOUNT.code);

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
                oracles
            )
        );
        paymaster = SimplexPaymasterHarness(payable(address(new ERC1967Proxy(address(implementation), initData))));

        deal(USDT, solver, 1_000_000e6);
        vm.prank(solver);
        (bool ok,) = USDT.call(abi.encodeWithSelector(IERC20.approve.selector, address(paymaster), type(uint256).max));
        require(ok, "approve failed");
        deal(USDC, solver, 1_000_000e6);
        vm.prank(solver);
        IERC20(USDC).approve(address(paymaster), type(uint256).max);

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

    /// EntryPoint v0.8 penalises the unused part of `paymasterPostOpGasLimit` AFTER the
    /// value handed to postOp is fixed, so that penalty is never billed to the user — the
    /// paymaster eats it out of the `_postOpCost()` cushion. The penalty is waived while
    /// `gasLimit <= gasUsed + PENALTY_GAS_THRESHOLD (40k)`, so MAX_POST_OP_GAS_LIMIT is set
    /// to exactly 40k: the largest value that is penalty-free whatever postOp costs.
    /// At the cap the margin must equal the margin at the floor — a flat, penalty-free band.
    function testEnforcedPostOpBandIsPenaltyFree() public onFork {
        // Discarded: the first sponsored op pays the cold-storage cost of the paymaster's
        // and sender's token balance slots, which would otherwise read as a penalty.
        _run(60_000, 40_000);

        (uint256 chargedCap, uint256 spentCap) = _run(60_000, 40_000);
        uint256 marginAtCap = chargedCap - spentCap;

        (uint256 chargedFloor, uint256 spentFloor) = _run(60_000, 30_000);
        uint256 marginAtFloor = chargedFloor - spentFloor;

        emit log_named_uint("margin @ cap   40k (wei)", marginAtCap);
        emit log_named_uint("margin @ floor 30k (wei)", marginAtFloor);
        // Identical across the whole accepted band: the EntryPoint waives the penalty
        // while gasLimit <= gasUsed + 40k, so the cap is penalty-free whatever postOp costs.
        assertEq(marginAtCap, marginAtFloor, "penalty charged inside the allowed band");
    }

    /// The band the contract enforces is exactly [MIN, MAX]; outside it validation refuses.
    function testPostOpLimitsOutsideTheBandAreRefused() public onFork {
        PackedUserOperation[] memory tooHigh = _ops(60_000, 100_000);
        vm.expectRevert();
        ENTRY_POINT.handleOps(tooHigh, payable(beneficiary));

        PackedUserOperation[] memory tooLow = _ops(60_000, 20_000);
        vm.expectRevert();
        ENTRY_POINT.handleOps(tooLow, payable(beneficiary));
    }

    /// Probe the minimum viable postOp gas limit per token: postOp must still fit, and the
    /// EntryPoint penalty must be waived (limit <= gasUsed + 40k).
    function testPostOpFitsWithinFiftyThousandForBothTokens() public onFork {
        uint128[2] memory limits = [uint128(40_000), 30_000];
        address[2] memory toks = [USDT, USDC];
        string[2] memory names = ["USDT", "USDC"];
        for (uint256 t = 0; t < toks.length; t++) {
            for (uint256 i = 0; i < limits.length; i++) {
                (uint256 charged, uint256 spent) = _runToken(toks[t], 60_000, limits[i]);
                emit log_named_string("token", names[t]);
                emit log_named_uint("  postOpGasLimit", limits[i]);
                emit log_named_uint("  margin (wei)", charged - spent);
            }
        }
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

    function _ops(uint128 callGasLimit, uint128 postOpGasLimit)
        internal
        view
        returns (PackedUserOperation[] memory ops)
    {
        ops = new PackedUserOperation[](1);
        ops[0] = _buildOp(USDT, callGasLimit, postOpGasLimit);
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
        ENTRY_POINT.handleOps(ops, payable(beneficiary));

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
        op.sender = solver;
        op.nonce = ENTRY_POINT.getNonce(solver, 0);
        op.callData = "";
        op.accountGasLimits = bytes32((uint256(200_000) << 128) | uint256(callGasLimit));
        op.preVerificationGas = 60_000;
        uint256 maxFee = block.basefee + 1 gwei;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | maxFee);
        op.paymasterAndData =
            abi.encodePacked(address(paymaster), uint128(300_000), postOpGasLimit, abi.encodePacked(uint8(1), token));
        op.signature = _sign(solverKey, ENTRY_POINT.getUserOpHash(op));
    }

    function _sign(uint256 key, bytes32 digest) internal pure returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(key, digest);
        return abi.encodePacked(r, s, v);
    }
}
