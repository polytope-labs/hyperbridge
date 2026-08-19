// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC4337Utils} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";

interface ISimplexPaymasterLive {
    function treasury() external view returns (address);

    function host() external view returns (address);

    function addStake(uint32 unstakeDelaySec) external payable;

    function unlockStake() external;

    function withdrawStake(address payable to) external;

    function withdraw(address payable to, uint256 value) external;
}

interface IEntryPointStakeView {
    function getDepositInfo(address account)
        external
        view
        returns (uint256 deposit, bool staked, uint112 stake, uint32 unstakeDelaySec, uint48 withdrawTime);
}

/// @notice Against the LIVE deployed paymasters: once native is staked with the EntryPoint,
///         no caller and no governance action can ever recover it, because every unlock path
///         routes through `_authorizeWithdraw()`, which reverts unconditionally, and the
///         governance `RequestKind` enum has no stake variant. `addStake` itself is
///         permissionless, so anyone can also extend the unstake delay.
contract SimplexPaymasterStakeLockForkTest is Test {
    IEntryPointStakeView constant ENTRY_POINT = IEntryPointStakeView(address(ERC4337Utils.ENTRYPOINT_V08));

    ISimplexPaymasterLive paymaster;

    function _setUpFork(string memory env, address deployed) internal returns (bool) {
        string memory url = vm.envOr(env, string(""));
        if (bytes(url).length == 0) return false;
        vm.selectFork(vm.createFork(url));
        paymaster = ISimplexPaymasterLive(deployed);
        return true;
    }

    function testLiveBscStakeIsPermanentlyLocked() public {
        if (!_setUpFork("BSC_FORK_URL", 0xeD02f9f0df8F562B89cC5b25867Ad3C2d61252A9)) return;
        _assertStakeUnrecoverable();
    }

    function testLiveEthereumStakeIsPermanentlyLocked() public {
        if (!_setUpFork("MAINNET_FORK_URL", 0xD4340d7466e040626383cb9cda9307ba8E081149)) return;
        _assertStakeUnrecoverable();
    }

    function _assertStakeUnrecoverable() internal {
        (, bool staked, uint112 stake,,) = ENTRY_POINT.getDepositInfo(address(paymaster));
        assertTrue(staked, "precondition: live paymaster is staked");
        assertGt(stake, 0, "precondition: stake is non-zero");
        emit log_named_uint("live stake at risk (wei)", stake);

        address treasury = paymaster.treasury();
        address host = paymaster.host();

        // Every privileged identity in the system is refused.
        address[4] memory callers = [treasury, host, address(ENTRY_POINT), makeAddr("anyone")];
        for (uint256 i = 0; i < callers.length; i++) {
            vm.prank(callers[i]);
            vm.expectRevert();
            paymaster.unlockStake();

            vm.prank(callers[i]);
            vm.expectRevert();
            paymaster.withdrawStake(payable(callers[i]));
        }

        // Stake is still there, and still locked.
        (, bool stakedAfter, uint112 stakeAfter,,) = ENTRY_POINT.getDepositInfo(address(paymaster));
        assertTrue(stakedAfter);
        assertEq(stakeAfter, stake, "stake unchanged: no recovery path exists");
    }

    /// `addStake` is permissionless, so a griefer can both lock fresh value into the
    /// contract and stretch the unstake delay a future upgrade would have to wait out.
    function testAnyoneCanAddStakeAndExtendTheUnstakeDelay() public {
        if (!_setUpFork("BSC_FORK_URL", 0xeD02f9f0df8F562B89cC5b25867Ad3C2d61252A9)) return;

        (,,, uint32 delayBefore,) = ENTRY_POINT.getDepositInfo(address(paymaster));
        address griefer = makeAddr("griefer");
        vm.deal(griefer, 1 ether);

        vm.prank(griefer);
        paymaster.addStake{value: 1 wei}(type(uint32).max);

        (,, uint112 stakeAfter, uint32 delayAfter,) = ENTRY_POINT.getDepositInfo(address(paymaster));
        emit log_named_uint("unstake delay before (s)", delayBefore);
        emit log_named_uint("unstake delay after  (s)", delayAfter);
        assertGt(delayAfter, delayBefore, "an unprivileged caller extended the unstake delay");
        assertGt(uint256(delayAfter), 100 * 365 days, "delay pushed beyond a century");
        assertGt(stakeAfter, 0);
    }
}
