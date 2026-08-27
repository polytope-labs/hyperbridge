// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

import {SimplexPaymaster, AggregatorV3Interface, ISignatureTransfer} from "../../src/utils/SimplexPaymaster.sol";
import {SimplexPaymasterHarness} from "./SimplexPaymasterTest.t.sol";

interface IPermit2Test {
    function DOMAIN_SEPARATOR() external view returns (bytes32);

    function nonceBitmap(address owner, uint256 word) external view returns (uint256);
}

interface IEntryPointTest {
    function handleOps(PackedUserOperation[] calldata ops, address payable beneficiary) external;

    function getUserOpHash(PackedUserOperation calldata userOp) external view returns (bytes32);

    function depositTo(address account) external payable;

    function balanceOf(address account) external view returns (uint256);
}

/// @notice Exercises PERMIT2 mode against the real Permit2, real stablecoins,
///         real Chainlink feeds and the real EntryPoint v0.8 on a mainnet fork.
///         Contract correctness only: ERC-7562 bundler acceptance is a bundler
///         policy question a fork cannot answer.
abstract contract SimplexPaymasterPermit2ForkTest is Test {
    bytes32 constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
    bytes32 constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );

    bytes4 constant INVALID_NONCE = 0x756688fe; // InvalidNonce()
    bytes4 constant SIGNATURE_EXPIRED = 0xcd21db4f; // SignatureExpired(uint256)
    bytes4 constant INVALID_SIGNER = 0x815e1d64; // InvalidSigner()
    bytes4 constant INVALID_CONTRACT_SIGNATURE = 0xb0669cbc; // InvalidContractSignature()

    ISignatureTransfer constant PERMIT2 = ISignatureTransfer(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    IEntryPointTest constant ENTRY_POINT = IEntryPointTest(address(ERC4337Utils.ENTRYPOINT_V08));

    // Per-chain fixtures supplied by the concrete test.
    address host;
    address nativeOracle;
    address stable; // a registered token WITHOUT EIP-2612 on this chain
    address stableOracle;
    address solverAccountImpl; // live SolverAccount deployment, etched onto the solver EOA
    uint256 stableUnit;

    address solver;
    uint256 solverKey;
    address treasury = makeAddr("treasury");
    address beneficiary = makeAddr("beneficiary");

    SimplexPaymasterHarness paymaster;

    function _forkUrlEnv() internal pure virtual returns (string memory);

    function _configure() internal virtual;

    function setUp() public {
        string memory url = vm.envOr(_forkUrlEnv(), string(""));
        if (bytes(url).length == 0) return;
        vm.selectFork(vm.createFork(url));
        _configure();

        (solver, solverKey) = makeAddrAndKey("simplex-permit2-fork-solver");
        // Deterministic test addresses can carry a live EIP-7702 delegation on mainnet;
        // the EOA-path tests need a code-less sender.
        vm.etch(solver, "");

        SimplexPaymasterHarness implementation = new SimplexPaymasterHarness();
        address[] memory tokens = new address[](1);
        tokens[0] = stable;
        AggregatorV3Interface[] memory oracles = new AggregatorV3Interface[](1);
        oracles[0] = AggregatorV3Interface(stableOracle);
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (
                host,
                SimplexPaymaster.Params({
                    nativeOracle: AggregatorV3Interface(nativeOracle),
                    markupBps: 200,
                    treasury: treasury,
                    maxOracleAge: 7 days,
                    swapSlippageBps: 200
                }),
                tokens,
                oracles
            )
        );
        paymaster = SimplexPaymasterHarness(payable(address(new ERC1967Proxy(address(implementation), initData))));

        deal(stable, solver, 1_000 * stableUnit);
        _approvePermit2(type(uint256).max);
    }

    modifier onFork() {
        vm.skip(address(paymaster) == address(0));
        _;
    }

    // ── EOA sender (Permit2 ecrecover path) ──────────────────────────

    function testPermit2ModePullsPrefundAndConsumesNonce() public onFork {
        uint256 nonce = 1;
        uint256 deadline = block.timestamp + 1 hours;
        PackedUserOperation memory op = _permit2Op(100 * stableUnit, nonce, deadline, solverKey);

        uint256 solverBefore = IERC20(stable).balanceOf(solver);
        uint256 gasBefore = gasleft();
        (bytes memory context, uint256 validationData) = paymaster.validate(op, _maxCost(op));
        emit log_named_uint("validate gas (EOA sender)", gasBefore - gasleft());

        uint256 pulled = solverBefore - IERC20(stable).balanceOf(solver);
        assertGt(pulled, 0);
        assertEq(IERC20(stable).balanceOf(address(paymaster)), pulled);
        assertEq(IPermit2Test(address(PERMIT2)).nonceBitmap(solver, 0) & (1 << nonce), 1 << nonce);
        assertEq(validationData, ERC4337Utils.packValidationData(true, 0, uint48(deadline)));
        // context = userOpHash(32) || token(20) || tokenPrice(32) || prefundAmount(32) || prefunder(20)
        assertEq(context.length, 0x88);
        assertEq(address(bytes20(_slice(context, 0x74, 0x88))), solver);
        assertEq(uint256(bytes32(_slice(context, 0x54, 0x74))), pulled);
        // no residual allowance to the paymaster, unlike permit mode
        assertEq(IERC20(stable).allowance(solver, address(paymaster)), 0);
    }

    function testPermit2ModeNonceReuseRejected() public onFork {
        uint256 deadline = block.timestamp + 1 hours;
        paymaster.validate(_permit2Op(100 * stableUnit, 2, deadline, solverKey), 1e14);

        PackedUserOperation memory again = _permit2Op(100 * stableUnit, 2, deadline, solverKey);
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.Permit2Failed.selector, stable, abi.encodePacked(INVALID_NONCE))
        );
        paymaster.validate(again, 1e14);
    }

    function testPermit2ModeExpiredDeadlineRejected() public onFork {
        uint256 deadline = block.timestamp + 1 hours;
        PackedUserOperation memory op = _permit2Op(100 * stableUnit, 3, deadline, solverKey);
        vm.warp(deadline + 1);
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.Permit2Failed.selector, stable, abi.encodeWithSelector(SIGNATURE_EXPIRED, deadline)
            )
        );
        paymaster.validate(op, 1e14);
    }

    function testPermit2ModeWrongSignerRejected() public onFork {
        (, uint256 otherKey) = makeAddrAndKey("other");
        PackedUserOperation memory op = _permit2Op(100 * stableUnit, 4, block.timestamp + 1 hours, otherKey);
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.Permit2Failed.selector, stable, abi.encodePacked(INVALID_SIGNER))
        );
        paymaster.validate(op, 1e14);
    }

    /// A signature naming another spender is useless to this paymaster.
    function testPermit2ModeWrongSpenderRejected() public onFork {
        uint256 deadline = block.timestamp + 1 hours;
        bytes memory sig = _signPermit2(solverKey, makeAddr("someone-else"), 100 * stableUnit, 5, deadline);
        PackedUserOperation memory op = _opWithData(_permit2Data(100 * stableUnit, 5, deadline, sig));
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.Permit2Failed.selector, stable, abi.encodePacked(INVALID_SIGNER))
        );
        paymaster.validate(op, 1e14);
    }

    function testPermit2ModeWithoutPermit2AllowanceRejected() public onFork {
        _approvePermit2(0);
        PackedUserOperation memory op = _permit2Op(100 * stableUnit, 6, block.timestamp + 1 hours, solverKey);
        vm.expectRevert();
        paymaster.validate(op, 1e14);
    }

    /// permitAmount == prefund is accepted; one unit less is rejected before Permit2 is called.
    function testPermit2ModePermitAmountBoundary() public onFork {
        uint256 deadline = block.timestamp + 1 hours;
        PackedUserOperation memory probe = _permit2Op(100 * stableUnit, 7, deadline, solverKey);
        uint256 maxCost = 1e14;
        (,, uint256 tokenPrice) = paymaster.fetchDetails(probe);
        uint256 required = ((maxCost + 30_000 * _maxFeePerGas(probe)) * tokenPrice) / 1e18;

        PackedUserOperation memory low = _permit2Op(required - 1, 7, deadline, solverKey);
        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.InsufficientPermitAmount.selector, required - 1, required)
        );
        paymaster.validate(low, maxCost);
        assertEq(IPermit2Test(address(PERMIT2)).nonceBitmap(solver, 0) & (1 << 7), 0);

        PackedUserOperation memory exact = _permit2Op(required, 7, deadline, solverKey);
        paymaster.validate(exact, maxCost);
        assertEq(IERC20(stable).balanceOf(address(paymaster)), required);
    }

    /// The charged token is the one _fetchDetails validated, so a permit signed over a
    /// different token can never be pulled: Permit2 recovers a mismatched digest and reverts.
    function testPermit2ModeChargesOnlyTheValidatedToken() public onFork {
        uint256 deadline = block.timestamp + 1 hours;
        // A permit the solver signed over some OTHER token, but submitted under the
        // registered token's mode byte.
        address otherToken = address(0xDEAD);
        bytes memory sig = _signToken(solverKey, otherToken, address(paymaster), 100 * stableUnit, 11, deadline);
        bytes memory data = abi.encodePacked(uint8(2), stable, uint256(100 * stableUnit), uint256(11), deadline, sig);
        PackedUserOperation memory op = _opWithData(data);

        vm.expectRevert(
            abi.encodeWithSelector(SimplexPaymaster.Permit2Failed.selector, stable, abi.encodePacked(INVALID_SIGNER))
        );
        paymaster.validate(op, 1e14);
    }

    // ── Delegated sender (Permit2 ERC-1271 path through SolverAccount) ──

    function testPermit2ModeDelegatedSenderValidates() public onFork {
        vm.etch(solver, solverAccountImpl.code);
        PackedUserOperation memory op = _permit2Op(100 * stableUnit, 8, block.timestamp + 1 hours, solverKey);

        uint256 gasBefore = gasleft();
        paymaster.validate(op, _maxCost(op));
        emit log_named_uint("validate gas (delegated sender)", gasBefore - gasleft());
        assertGt(IERC20(stable).balanceOf(address(paymaster)), 0);
    }

    function testPermit2ModeDelegatedSenderWrongSignerRejected() public onFork {
        vm.etch(solver, solverAccountImpl.code);
        (, uint256 otherKey) = makeAddrAndKey("other");
        PackedUserOperation memory op = _permit2Op(100 * stableUnit, 9, block.timestamp + 1 hours, otherKey);
        vm.expectRevert(
            abi.encodeWithSelector(
                SimplexPaymaster.Permit2Failed.selector, stable, abi.encodePacked(INVALID_CONTRACT_SIGNATURE)
            )
        );
        paymaster.validate(op, 1e14);
    }

    /// Full EntryPoint round trip: validation pulls the prefund through Permit2,
    /// the no-op executes, postOp refunds the unused part to the sender.
    function testHandleOpsPermit2ModeSponsorsAndRefunds() public onFork {
        vm.etch(solver, solverAccountImpl.code);
        vm.deal(address(this), 10 ether);
        ENTRY_POINT.depositTo{value: 1 ether}(address(paymaster));

        PackedUserOperation memory op = _permit2Op(100 * stableUnit, 10, block.timestamp + 1 hours, solverKey);
        op.signature = _sign(solverKey, ENTRY_POINT.getUserOpHash(op));

        uint256 solverBefore = IERC20(stable).balanceOf(solver);
        uint256 depositBefore = ENTRY_POINT.balanceOf(address(paymaster));

        PackedUserOperation[] memory ops = new PackedUserOperation[](1);
        ops[0] = op;
        ENTRY_POINT.handleOps(ops, payable(beneficiary));

        uint256 charged = solverBefore - IERC20(stable).balanceOf(solver);
        uint256 nativeSpent = depositBefore - ENTRY_POINT.balanceOf(address(paymaster));
        assertGt(charged, 0);
        assertLe(charged, 100 * stableUnit);
        assertEq(IERC20(stable).balanceOf(address(paymaster)), charged);
        assertGt(nativeSpent, 0);
        // The refund happened: what stayed with the paymaster is less than the prefund it pulled.
        (,, uint256 tokenPrice) = paymaster.fetchDetails(op);
        uint256 prefund = ((_maxCost(op) + 30_000 * _maxFeePerGas(op)) * tokenPrice) / 1e18;
        assertLt(charged, prefund);
        assertEq(IPermit2Test(address(PERMIT2)).nonceBitmap(solver, 0) & (1 << 10), 1 << 10);
    }

    // ── Helpers ──────────────────────────────────────────────────────

    /// @dev Raw call: Tether's approve returns no bool.
    function _approvePermit2(uint256 amount) internal {
        vm.prank(solver);
        (bool ok,) = stable.call(abi.encodeWithSelector(IERC20.approve.selector, address(PERMIT2), amount));
        require(ok, "approve failed");
    }

    function _permit2Op(uint256 permitAmount, uint256 nonce, uint256 deadline, uint256 signerKey)
        internal
        view
        returns (PackedUserOperation memory)
    {
        bytes memory sig = _signPermit2(signerKey, address(paymaster), permitAmount, nonce, deadline);
        return _opWithData(_permit2Data(permitAmount, nonce, deadline, sig));
    }

    function _permit2Data(uint256 permitAmount, uint256 nonce, uint256 deadline, bytes memory sig)
        internal
        view
        returns (bytes memory)
    {
        return abi.encodePacked(uint8(2), stable, permitAmount, nonce, deadline, sig);
    }

    function _signPermit2(uint256 key, address spender, uint256 amount, uint256 nonce, uint256 deadline)
        internal
        view
        returns (bytes memory)
    {
        return _signToken(key, stable, spender, amount, nonce, deadline);
    }

    function _signToken(uint256 key, address token, address spender, uint256 amount, uint256 nonce, uint256 deadline)
        internal
        view
        returns (bytes memory)
    {
        bytes32 structHash = keccak256(
            abi.encode(
                PERMIT_TRANSFER_FROM_TYPEHASH,
                keccak256(abi.encode(TOKEN_PERMISSIONS_TYPEHASH, token, amount)),
                spender,
                nonce,
                deadline
            )
        );
        bytes32 digest =
            keccak256(abi.encodePacked("\x19\x01", IPermit2Test(address(PERMIT2)).DOMAIN_SEPARATOR(), structHash));
        // mode 0x02 lays the signature out as v ‖ r ‖ s (mirroring the EIP-2612 mode 0x00 fields).
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(key, digest);
        return abi.encodePacked(v, r, s);
    }

    /// Canonical r ‖ s ‖ v — the shape ECDSA.recover expects for the account signature.
    function _sign(uint256 key, bytes32 digest) internal pure returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(key, digest);
        return abi.encodePacked(r, s, v);
    }

    /// @dev Fees pinned just above the forked block's basefee so maxCost stays small.
    function _opWithData(bytes memory data) internal view returns (PackedUserOperation memory op) {
        uint256 maxFee = block.basefee + 1 gwei;
        op.sender = solver;
        op.callData = "";
        op.accountGasLimits = bytes32((uint256(200_000) << 128) | uint256(50_000));
        op.preVerificationGas = 60_000;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | maxFee);
        op.paymasterAndData = abi.encodePacked(address(paymaster), uint128(300_000), uint128(40_000), data);
    }

    function _maxFeePerGas(PackedUserOperation memory op) internal pure returns (uint256) {
        return uint256(op.gasFees) & type(uint128).max;
    }

    /// @dev Mirrors EntryPoint's requiredPrefund for the limits used in _opWithData.
    function _maxCost(PackedUserOperation memory op) internal pure returns (uint256) {
        uint256 gas = 200_000 + 50_000 + 60_000 + 300_000 + 40_000;
        return gas * _maxFeePerGas(op);
    }

    function _slice(bytes memory data, uint256 start, uint256 end) internal pure returns (bytes memory out) {
        out = new bytes(end - start);
        for (uint256 i = start; i < end; i++) {
            out[i - start] = data[i];
        }
    }
}

contract SimplexPaymasterPermit2EthereumForkTest is SimplexPaymasterPermit2ForkTest {
    function _forkUrlEnv() internal pure override returns (string memory) {
        return "MAINNET_FORK_URL";
    }

    function _configure() internal override {
        host = 0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3;
        nativeOracle = 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419;
        stable = 0xdAC17F958D2ee523a2206206994597C13D831ec7; // USDT, no EIP-2612
        stableOracle = 0x3E7d1eAB13ad0104d2750B8863b489D65364e32D;
        stableUnit = 1e6;
        solverAccountImpl = 0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3;
    }
}

contract SimplexPaymasterPermit2BscForkTest is SimplexPaymasterPermit2ForkTest {
    function _forkUrlEnv() internal pure override returns (string memory) {
        return "BSC_FORK_URL";
    }

    function _configure() internal override {
        host = 0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3;
        nativeOracle = 0x0567F2323251f0Aab15c8dFb1967E4e8A7D42aeE;
        stable = 0x55d398326f99059fF775485246999027B3197955; // Binance-Peg USDT, no EIP-2612
        stableOracle = 0xB97Ad0E74fa7d920791E90258A6E2085088b4320;
        stableUnit = 1e18;
        solverAccountImpl = 0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3;
    }
}
