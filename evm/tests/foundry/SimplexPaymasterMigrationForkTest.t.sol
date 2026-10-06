// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";

import {SimplexPaymaster, AggregatorV3Interface, IStakeManager} from "../../src/utils/SimplexPaymaster.sol";
import {SolverAccount} from "../../src/apps/intentsv2/SolverAccount.sol";

interface IEntryPointFork {
    function handleOps(PackedUserOperation[] calldata ops, address payable beneficiary) external;

    function getUserOpHash(PackedUserOperation calldata userOp) external view returns (bytes32);

    function getNonce(address sender, uint192 key) external view returns (uint256);

    function getDepositInfo(address account) external view returns (IStakeManager.DepositInfo memory info);
}

interface IPermit2Domain {
    function DOMAIN_SEPARATOR() external view returns (bytes32);
}

/// @notice Delivers the EntryPoint v0.9 upgrade to the LIVE paymaster proxies the way governance
///         will: the host hands `onAccept` an `UpgradeContract` request from Hyperbridge, submitted
///         by the authorised relayer, whose init data runs `migrate`. Each case skips once its
///         proxy has left version 2.
abstract contract SimplexPaymasterMigrationForkTest is Test {
    using SafeERC20 for IERC20;

    IEntryPointFork constant ENTRY_POINT_V08 = IEntryPointFork(0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108);
    IEntryPointFork constant ENTRY_POINT_V09 = IEntryPointFork(0x433709009B8330FDa32311DF1C2AFA402eD8D009);
    address constant PERMIT2 = 0x000000000022D473030F116dDEE9F6B43aC78BA3;
    address constant INTENT_GATEWAY = 0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716;
    bytes32 constant PERMIT_TYPEHASH =
        keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)");
    bytes32 constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
    bytes32 constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );

    SimplexPaymaster paymaster;
    address host;
    address relayer;
    address treasury;
    IStakeManager.DepositInfo v08Before;
    IStakeManager.DepositInfo v09Before;
    uint256 balanceBefore;

    function _forkUrlEnv() internal pure virtual returns (string memory);

    function _proxy() internal pure virtual returns (address);

    /// @dev A token the live proxy registers that has no EIP-2612, sponsored in PERMIT2 mode.
    function _permit2Token() internal pure virtual returns (address);

    /// @dev A token the live proxy registers that has EIP-2612, sponsored in PERMIT mode; zero
    ///      where the chain has none.
    function _permitToken() internal pure virtual returns (address);

    function setUp() public {
        string memory url = vm.envOr(_forkUrlEnv(), string(""));
        if (bytes(url).length == 0) return;
        vm.selectFork(vm.createFork(url));
        // The live chains' hardfork: the BSC treasury that receives the stake is an EIP-7702-delegated EOA.
        vm.setEvmVersion("prague");

        SimplexPaymaster live = SimplexPaymaster(payable(_proxy()));
        if (live.version() != 2) return;
        paymaster = live;
        host = live.host();
        relayer = live.relayer();
        treasury = live.treasury();
        v08Before = ENTRY_POINT_V08.getDepositInfo(address(live));
        v09Before = ENTRY_POINT_V09.getDepositInfo(address(live));
        balanceBefore = address(live).balance;
    }

    modifier onFork() {
        vm.skip(address(paymaster) == address(0));
        _;
    }

    function testMigrateMovesDepositAndStakeToV09() public onFork {
        address[] memory tokens = paymaster.getRegisteredTokens();
        SimplexPaymaster.TokenConfig[] memory configs = _tokenConfigs(tokens);
        AggregatorV3Interface nativeOracle = paymaster.nativeOracle();
        uint256 markupBps = paymaster.markupBps();
        uint256 maxOracleAge = paymaster.maxOracleAge();
        uint256 swapSlippageBps = paymaster.swapSlippageBps();
        uint256 deposited = v08Before.deposit + balanceBefore - v08Before.stake;

        address implementation = address(new SimplexPaymaster());
        vm.expectEmit(true, true, true, true, address(paymaster));
        emit SimplexPaymaster.EntryPointMigrated(
            v08Before.deposit, v08Before.stake, v08Before.unstakeDelaySec, deposited
        );
        _migrate(implementation);

        assertEq(paymaster.version(), 3);
        assertEq(address(paymaster.entryPoint()), address(ENTRY_POINT_V09));
        assertEq(
            address(uint160(uint256(vm.load(address(paymaster), ERC1967Utils.IMPLEMENTATION_SLOT)))), implementation
        );

        IStakeManager.DepositInfo memory v08 = ENTRY_POINT_V08.getDepositInfo(address(paymaster));
        assertEq(v08.deposit, 0, "v0.8 deposit drained");
        assertFalse(v08.staked, "v0.8 stake unlocked");
        assertEq(v08.stake, v08Before.stake, "v0.8 stake waits out its delay");
        if (v08Before.staked) assertEq(v08.withdrawTime, block.timestamp + v08Before.unstakeDelaySec);

        IStakeManager.DepositInfo memory v09 = ENTRY_POINT_V09.getDepositInfo(address(paymaster));
        assertEq(v09.staked, v08Before.stake > 0);
        assertEq(v09.stake, v08Before.stake, "v0.9 stake copies v0.8");
        assertEq(v09.unstakeDelaySec, v08Before.unstakeDelaySec, "v0.9 delay copies v0.8");
        assertEq(v09.deposit, v09Before.deposit + deposited);
        assertEq(v09.stake + v09.deposit, v09Before.deposit + v08Before.deposit + balanceBefore, "no native lost");
        assertEq(address(paymaster).balance, 0);

        assertEq(paymaster.host(), host);
        assertEq(paymaster.relayer(), relayer);
        assertEq(paymaster.treasury(), treasury);
        assertEq(address(paymaster.nativeOracle()), address(nativeOracle));
        assertEq(paymaster.markupBps(), markupBps);
        assertEq(paymaster.maxOracleAge(), maxOracleAge);
        assertEq(paymaster.swapSlippageBps(), swapSlippageBps);
        assertEq(paymaster.getRegisteredTokens(), tokens);
        SimplexPaymaster.TokenConfig[] memory configsAfter = _tokenConfigs(tokens);
        for (uint256 i = 0; i < tokens.length; i++) {
            assertEq(address(configsAfter[i].tokenOracle), address(configs[i].tokenOracle));
            assertEq(configsAfter[i].tokenOracleDecimals, configs[i].tokenOracleDecimals);
            assertEq(configsAfter[i].tokenDecimals, configs[i].tokenDecimals);
            assertEq(configsAfter[i].active, configs[i].active);
        }
        assertEq(paymaster.getBundlers().length, 0, "the allowlist starts empty");
    }

    /// Anyone sweeps the unlocked v0.8 stake, and only to the treasury, once its delay has passed.
    function testWithdrawStakeV08PaysTreasuryAfterDelay() public onFork {
        _migrate(address(new SimplexPaymaster()));
        address anyone = makeAddr("anyone");

        if (v08Before.stake == 0) {
            vm.prank(anyone);
            vm.expectRevert("No stake to withdraw");
            paymaster.withdrawStakeV08();
            return;
        }

        vm.prank(anyone);
        vm.expectRevert("Stake withdrawal is not due");
        paymaster.withdrawStakeV08();

        vm.warp(block.timestamp + v08Before.unstakeDelaySec + 1);
        uint256 treasuryBefore = treasury.balance;
        IStakeManager.DepositInfo memory v09 = ENTRY_POINT_V09.getDepositInfo(address(paymaster));

        vm.prank(anyone);
        paymaster.withdrawStakeV08();

        assertEq(treasury.balance - treasuryBefore, v08Before.stake, "treasury received the v0.8 stake");
        assertEq(ENTRY_POINT_V08.getDepositInfo(address(paymaster)).stake, 0);
        IStakeManager.DepositInfo memory v09After = ENTRY_POINT_V09.getDepositInfo(address(paymaster));
        assertEq(v09After.stake, v09.stake);
        assertEq(v09After.deposit, v09.deposit);
    }

    /// The inherited stake and deposit entry points stay shut to every privileged identity on the
    /// migrated proxy; the v0.9 stake moves only through governance, and only to the treasury.
    function testMigratedStakeMovesOnlyThroughGovernance() public onFork {
        _migrate(address(new SimplexPaymaster()));
        IStakeManager.DepositInfo memory v09 = ENTRY_POINT_V09.getDepositInfo(address(paymaster));

        address[4] memory callers = [treasury, host, address(ENTRY_POINT_V09), makeAddr("anyone")];
        for (uint256 i = 0; i < callers.length; i++) {
            vm.prank(callers[i]);
            vm.expectRevert(HyperApp.UnauthorizedCall.selector);
            paymaster.unlockStake();

            vm.prank(callers[i]);
            vm.expectRevert(HyperApp.UnauthorizedCall.selector);
            paymaster.withdrawStake(payable(callers[i]));

            vm.prank(callers[i]);
            vm.expectRevert(HyperApp.UnauthorizedCall.selector);
            paymaster.withdraw(payable(callers[i]), 1);
        }
        IStakeManager.DepositInfo memory unchanged = ENTRY_POINT_V09.getDepositInfo(address(paymaster));
        assertEq(unchanged.staked, v09.staked);
        assertEq(unchanged.stake, v09.stake);
        assertEq(unchanged.deposit, v09.deposit);

        if (v09.stake == 0) return;

        _govern(SimplexPaymaster.RequestKind.UnlockStake, "");
        assertFalse(ENTRY_POINT_V09.getDepositInfo(address(paymaster)).staked);

        vm.warp(block.timestamp + v09.unstakeDelaySec);
        uint256 treasuryBefore = treasury.balance;
        _govern(SimplexPaymaster.RequestKind.WithdrawStake, "");
        assertEq(treasury.balance - treasuryBefore, v09.stake);
        assertEq(ENTRY_POINT_V09.getDepositInfo(address(paymaster)).stake, 0);
        assertEq(ENTRY_POINT_V09.getDepositInfo(address(paymaster)).deposit, v09.deposit);
    }

    /// The migrated proxy sponsors a PERMIT2 op from its v0.9 deposit through the real EntryPoint.
    function testMigratedProxySponsorsPermit2Op() public onFork {
        address token = _permit2Token();
        _migrate(address(new SimplexPaymaster()));
        (address solver, uint256 solverKey, uint256 unit) = _solver(token);
        vm.startPrank(solver);
        IERC20(token).forceApprove(PERMIT2, type(uint256).max);
        vm.stopPrank();

        uint256 permitAmount = 100 * unit;
        uint256 nonce = ENTRY_POINT_V09.getNonce(solver, 0);
        uint256 deadline = block.timestamp + 1 hours;
        bytes32 structHash = keccak256(
            abi.encode(
                PERMIT_TRANSFER_FROM_TYPEHASH,
                keccak256(abi.encode(TOKEN_PERMISSIONS_TYPEHASH, token, permitAmount)),
                address(paymaster),
                nonce,
                deadline
            )
        );
        (uint8 v, bytes32 r, bytes32 s) =
            vm.sign(solverKey, _digest(IPermit2Domain(PERMIT2).DOMAIN_SEPARATOR(), structHash));

        _sponsor(solver, solverKey, token, abi.encodePacked(uint8(2), token, permitAmount, nonce, deadline, v, r, s));
    }

    /// The migrated proxy sponsors a PERMIT op, running its EIP-2612 permit under the real EntryPoint.
    function testMigratedProxySponsorsPermitOp() public onFork {
        address token = _permitToken();
        vm.skip(token == address(0));
        _migrate(address(new SimplexPaymaster()));
        (address solver, uint256 solverKey, uint256 unit) = _solver(token);

        uint256 permitAmount = 100 * unit;
        uint256 permitNonce = IERC20Permit(token).nonces(solver);
        uint256 deadline = block.timestamp + 1 hours;
        bytes32 structHash =
            keccak256(abi.encode(PERMIT_TYPEHASH, solver, address(paymaster), permitAmount, permitNonce, deadline));
        (uint8 v, bytes32 r, bytes32 s) =
            vm.sign(solverKey, _digest(IERC20Permit(token).DOMAIN_SEPARATOR(), structHash));

        _sponsor(solver, solverKey, token, abi.encodePacked(uint8(0), token, permitAmount, deadline, v, r, s));
        assertEq(IERC20Permit(token).nonces(solver), permitNonce + 1, "permit consumed");
    }

    /// @dev A solver EOA delegated to the local SolverAccount build, holding 1,000 whole `token`.
    function _solver(address token) internal returns (address solver, uint256 solverKey, uint256 unit) {
        (solver, solverKey) = makeAddrAndKey("simplex-migration-solver");
        vm.etch(solver, address(new SolverAccount(INTENT_GATEWAY)).code);
        (,, uint8 tokenDecimals,) = paymaster.tokenConfigs(token);
        unit = 10 ** tokenDecimals;
        deal(token, solver, 1_000 * unit);
    }

    /// @dev Runs one sponsored no-op through the real v0.9 `handleOps`: the solver pays in `token`
    ///      and the gas comes out of the paymaster's v0.9 deposit.
    function _sponsor(address solver, uint256 solverKey, address token, bytes memory paymasterData) internal {
        PackedUserOperation memory op;
        op.sender = solver;
        op.nonce = ENTRY_POINT_V09.getNonce(solver, 0);
        op.accountGasLimits = bytes32((uint256(200_000) << 128) | uint256(50_000));
        op.preVerificationGas = 60_000;
        op.gasFees = bytes32((uint256(1 gwei) << 128) | (block.basefee + 1 gwei));
        op.paymasterAndData = abi.encodePacked(address(paymaster), uint128(300_000), uint128(40_000), paymasterData);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(solverKey, ENTRY_POINT_V09.getUserOpHash(op));
        op.signature = abi.encodePacked(r, s, v);
        PackedUserOperation[] memory ops = new PackedUserOperation[](1);
        ops[0] = op;

        uint256 solverBefore = IERC20(token).balanceOf(solver);
        uint256 surplusBefore = IERC20(token).balanceOf(address(paymaster));
        uint256 depositBefore = ENTRY_POINT_V09.getDepositInfo(address(paymaster)).deposit;

        address bundler = makeAddr("bundler");
        vm.prank(bundler, bundler);
        ENTRY_POINT_V09.handleOps(ops, payable(makeAddr("beneficiary")));

        uint256 charged = solverBefore - IERC20(token).balanceOf(solver);
        assertEq(ENTRY_POINT_V09.getNonce(solver, 0), op.nonce + 1);
        assertGt(charged, 0);
        assertEq(IERC20(token).balanceOf(address(paymaster)) - surplusBefore, charged);
        assertLt(ENTRY_POINT_V09.getDepositInfo(address(paymaster)).deposit, depositBefore);
    }

    function _digest(bytes32 domainSeparator, bytes32 structHash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));
    }

    function _migrate(address implementation) internal {
        _govern(
            SimplexPaymaster.RequestKind.UpgradeContract,
            abi.encode(implementation, abi.encodeCall(SimplexPaymaster.migrate, ()))
        );
    }

    function _govern(SimplexPaymaster.RequestKind kind, bytes memory payload) internal {
        IncomingPostRequest memory incoming;
        incoming.request.source = IDispatcher(host).hyperbridge();
        incoming.request.body = bytes.concat(bytes1(uint8(kind)), payload);
        incoming.relayer = relayer;
        vm.prank(host);
        paymaster.onAccept(incoming);
    }

    function _tokenConfigs(address[] memory tokens)
        internal
        view
        returns (SimplexPaymaster.TokenConfig[] memory configs)
    {
        configs = new SimplexPaymaster.TokenConfig[](tokens.length);
        for (uint256 i = 0; i < tokens.length; i++) {
            (AggregatorV3Interface oracle, uint8 oracleDecimals, uint8 tokenDecimals, bool active) =
                paymaster.tokenConfigs(tokens[i]);
            configs[i] = SimplexPaymaster.TokenConfig(oracle, oracleDecimals, tokenDecimals, active);
        }
    }
}

contract SimplexPaymasterMigrationEthereumForkTest is SimplexPaymasterMigrationForkTest {
    function _forkUrlEnv() internal pure override returns (string memory) {
        return "MAINNET_FORK_URL";
    }

    function _proxy() internal pure override returns (address) {
        return 0xD4340d7466e040626383cb9cda9307ba8E081149;
    }

    function _permit2Token() internal pure override returns (address) {
        return 0xdAC17F958D2ee523a2206206994597C13D831ec7; // USDT
    }

    function _permitToken() internal pure override returns (address) {
        return 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48; // USDC
    }
}

contract SimplexPaymasterMigrationBscForkTest is SimplexPaymasterMigrationForkTest {
    function _forkUrlEnv() internal pure override returns (string memory) {
        return "BSC_FORK_URL";
    }

    function _proxy() internal pure override returns (address) {
        return 0xeD02f9f0df8F562B89cC5b25867Ad3C2d61252A9;
    }

    function _permit2Token() internal pure override returns (address) {
        return 0x55d398326f99059fF775485246999027B3197955; // Binance-Peg USDT
    }

    /// @dev Binance-Peg USDC, the other registered token, has no EIP-2612 either.
    function _permitToken() internal pure override returns (address) {
        return address(0);
    }
}

contract SimplexPaymasterMigrationBaseForkTest is SimplexPaymasterMigrationForkTest {
    function _forkUrlEnv() internal pure override returns (string memory) {
        return "BASE_FORK_URL";
    }

    function _proxy() internal pure override returns (address) {
        return 0x15b3B03C870c7ef252029c35A12d3b339F5c8d7f;
    }

    function _permit2Token() internal pure override returns (address) {
        return 0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2; // bridged USDT
    }

    function _permitToken() internal pure override returns (address) {
        return 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913; // USDC
    }
}
